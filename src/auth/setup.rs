//! The project end-user auth setup.
//!
//! The runner is deliberately boring: it resolves the target and the provider, asks
//! the provider for a plan, shows that plan, and only writes after the owner
//! confirmed the exact plan digest. Every write is bound to the plan digest and to
//! the project revision the plan was made against.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::core::ForgeError;

use super::contract::{
    plan_digest, project_revision, scan_answer_for_secrets, validate_response, ContractError,
    PlanChange, Population, ProviderOperation, ProviderRequest, ProviderResponse, ProviderStatus,
    RecoveryAction, CONTRACT_ID,
};
use super::packs::PackCatalog;

/// One setup request.
#[derive(Debug, Clone)]
pub struct AuthSetupRequest {
    /// The project id.
    pub project_id: String,
    /// The project directory that holds the manifest.
    pub project_dir: PathBuf,
    /// The provider pack the owner selected.
    pub provider_id: String,
    /// The provider input. Never carries a secret value.
    pub provider_input: Value,
    /// Whether the owner confirmed the plan.
    pub confirm: bool,
}

/// The outcome of a setup run.
#[derive(Debug, Clone)]
pub enum AuthSetupOutcome {
    /// The provider is not usable for this project. Nothing was written.
    Blocked {
        /// A stable reason code.
        reason: String,
        /// The recovery actions the provider named.
        recovery: Vec<RecoveryAction>,
    },
    /// The plan is ready and the owner has not confirmed it. Nothing was written.
    Preview {
        /// The plan the provider produced.
        plan: ProviderResponse,
        /// The digest the owner must confirm.
        digest: String,
    },
    /// The owner declined the plan. Nothing was written.
    Declined {
        /// The plan the provider produced.
        plan: ProviderResponse,
    },
    /// The setup finished and the manifest records the population.
    Applied {
        /// The plan the provider applied.
        plan: ProviderResponse,
        /// The population the provider created.
        population: Population,
        /// The manifest that now records the population.
        manifest_path: PathBuf,
    },
}

impl AuthSetupOutcome {
    /// A short, secret-free status word.
    pub fn status(&self) -> &'static str {
        match self {
            Self::Blocked { .. } => "blocked",
            Self::Preview { .. } => "preview",
            Self::Declined { .. } => "declined",
            Self::Applied { .. } => "applied",
        }
    }
}

/// Runs the setup for a project.
pub struct AuthSetupRunner {
    catalog: PackCatalog,
}

impl AuthSetupRunner {
    /// Creates a runner over a discovered pack catalog.
    pub fn new(catalog: PackCatalog) -> Self {
        Self { catalog }
    }

    /// The pack catalog.
    pub fn catalog(&self) -> &PackCatalog {
        &self.catalog
    }

    /// Runs the setup.
    pub fn run(&self, request: &AuthSetupRequest) -> Result<AuthSetupOutcome, ForgeError> {
        let pack = match self.catalog.find(&request.provider_id) {
            Some(pack) if pack.enabled && pack.available => pack.clone(),
            Some(pack) if !pack.enabled => {
                return Ok(AuthSetupOutcome::Blocked {
                    reason: "provider_disabled".to_string(),
                    recovery: vec![RecoveryAction {
                        reason: "provider_disabled".to_string(),
                        instruction: format!(
                            "Enable the provider pack '{}' before running the setup.",
                            pack.id
                        ),
                    }],
                })
            }
            Some(pack) => {
                return Ok(AuthSetupOutcome::Blocked {
                    reason: "provider_not_built".to_string(),
                    recovery: vec![RecoveryAction {
                        reason: "provider_not_built".to_string(),
                        instruction: format!(
                            "Build the provider pack '{}' before running the setup.",
                            pack.id
                        ),
                    }],
                })
            }
            None => {
                return Ok(AuthSetupOutcome::Blocked {
                    reason: "provider_unknown".to_string(),
                    recovery: vec![RecoveryAction {
                        reason: "provider_unknown".to_string(),
                        instruction: format!(
                            "The provider pack '{}' is not installed. Forge does not fall back to another provider.",
                            request.provider_id
                        ),
                    }],
                })
            }
        };

        let manifest_path = manifest_path(&request.project_dir)?;
        let manifest_bytes =
            fs::read(&manifest_path).map_err(|error| ForgeError::AuthSetupRefused {
                reason: format!("cannot read {}: {error}", manifest_path.display()),
            })?;
        let revision = project_revision(&manifest_bytes);

        if let Some(conflict) = existing_auth_conflict(&manifest_bytes, &request.provider_id)? {
            return Ok(AuthSetupOutcome::Blocked {
                reason: "existing_auth_conflict".to_string(),
                recovery: vec![RecoveryAction {
                    reason: "existing_auth_conflict".to_string(),
                    instruction: conflict,
                }],
            });
        }

        // The probe carries the same input as the plan, so the provider can
        // report what it needs before any plan exists.
        let probe = self.call(
            &pack,
            request,
            &revision,
            ProviderOperation::Probe,
            request.provider_input.clone(),
        )?;
        if probe.status != ProviderStatus::Ready {
            return Ok(AuthSetupOutcome::Blocked {
                reason: probe.status.as_str().to_string(),
                recovery: probe.recovery.clone(),
            });
        }

        let plan = self.call(
            &pack,
            request,
            &revision,
            ProviderOperation::Plan,
            request.provider_input.clone(),
        )?;
        if plan.status != ProviderStatus::Ready {
            return Ok(AuthSetupOutcome::Blocked {
                reason: plan.status.as_str().to_string(),
                recovery: plan.recovery.clone(),
            });
        }
        let digest = plan
            .plan_digest
            .clone()
            .ok_or_else(|| ForgeError::AuthSetupRefused {
                reason: "the provider plan carries no digest".to_string(),
            })?;

        if !request.confirm {
            return Ok(AuthSetupOutcome::Preview { plan, digest });
        }

        let mut apply_input = request.provider_input.clone();
        if let Value::Object(map) = &mut apply_input {
            map.insert("plan_digest".to_string(), Value::String(digest.clone()));
        }
        let applied = self.call(
            &pack,
            request,
            &revision,
            ProviderOperation::Apply,
            apply_input,
        )?;
        if applied.status != ProviderStatus::Ready {
            return Ok(AuthSetupOutcome::Blocked {
                reason: applied.status.as_str().to_string(),
                recovery: applied.recovery.clone(),
            });
        }
        let population = match applied.population.clone() {
            Some(population) => population,
            None => return Ok(no_population()),
        };

        write_auth_block(&manifest_path, &manifest_bytes, &population, &applied)?;

        Ok(AuthSetupOutcome::Applied {
            plan: applied,
            population,
            manifest_path,
        })
    }

    fn call(
        &self,
        pack: &super::packs::ProviderPack,
        request: &AuthSetupRequest,
        revision: &str,
        operation: ProviderOperation,
        input: Value,
    ) -> Result<ProviderResponse, ForgeError> {
        let operation_id = format!(
            "{}-{}",
            operation.as_str(),
            digest_seed(&request.project_id)
        );
        let provider_request = ProviderRequest {
            contract: CONTRACT_ID.to_string(),
            provider_id: pack.id.clone(),
            operation_id,
            project_id: request.project_id.clone(),
            project_revision: revision.to_string(),
            operation,
            input,
        };
        let response = call_pack(pack, &provider_request).map_err(|error| match error {
            ContractError::Unavailable(reason) => ForgeError::AuthSetupRefused {
                reason: format!("the provider pack is unavailable: {reason}"),
            },
            other => ForgeError::AuthSetupRefused {
                reason: other.to_string(),
            },
        })?;
        validate_response(&provider_request, &response).map_err(|error| {
            ForgeError::AuthSetupRefused {
                reason: error.to_string(),
            }
        })?;
        Ok(response)
    }
}

/// The manifest file name inside a project directory.
const MANIFEST_NAME: &str = "forge.yaml";

/// The blocked outcome for a provider that reported success without a
/// population.
fn no_population() -> AuthSetupOutcome {
    AuthSetupOutcome::Blocked {
        reason: "provider_returned_no_population".to_string(),
        recovery: vec![RecoveryAction {
            reason: "provider_returned_no_population".to_string(),
            instruction: "The provider reported success without a population. Run the setup again."
                .to_string(),
        }],
    }
}

fn manifest_path(project_dir: &Path) -> Result<PathBuf, ForgeError> {
    let path = project_dir.join(MANIFEST_NAME);
    if path.is_file() {
        return Ok(path);
    }
    Err(ForgeError::AuthSetupRefused {
        reason: format!(
            "the project has no {MANIFEST_NAME}: {}",
            project_dir.display()
        ),
    })
}

fn existing_auth_conflict(
    manifest_bytes: &[u8],
    provider_id: &str,
) -> Result<Option<String>, ForgeError> {
    let value: serde_yaml::Value =
        serde_yaml::from_slice(manifest_bytes).map_err(|error| ForgeError::AuthSetupRefused {
            reason: format!("the project manifest is not valid YAML: {error}"),
        })?;
    let existing = match value.get("auth") {
        Some(serde_yaml::Value::Mapping(map)) => map,
        Some(_) => {
            return Err(ForgeError::AuthSetupRefused {
                reason: "the project manifest carries an auth block that is not a mapping"
                    .to_string(),
            })
        }
        None => return Ok(None),
    };
    let existing_provider = existing
        .get(serde_yaml::Value::String("provider".to_string()))
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or_default();
    if existing_provider == provider_id {
        return Ok(None);
    }
    Ok(Some(format!(
        "The project already uses the identity provider '{existing_provider}'. Remove its auth block before switching provider."
    )))
}

fn write_auth_block(
    manifest_path: &Path,
    manifest_bytes: &[u8],
    population: &Population,
    applied: &ProviderResponse,
) -> Result<(), ForgeError> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_slice(manifest_bytes).map_err(|error| ForgeError::AuthSetupRefused {
            reason: format!("the project manifest is not valid YAML: {error}"),
        })?;
    if !value.is_mapping() {
        return Err(ForgeError::AuthSetupRefused {
            reason: "the project manifest is not a mapping".to_string(),
        });
    }

    let mut auth = serde_yaml::Mapping::new();
    auth.insert(
        serde_yaml::Value::String("schema_version".to_string()),
        serde_yaml::Value::Number(1.into()),
    );
    auth.insert(
        serde_yaml::Value::String("provider".to_string()),
        serde_yaml::Value::String(applied.provider_id.clone()),
    );
    auth.insert(
        serde_yaml::Value::String("population_id".to_string()),
        serde_yaml::Value::String(population.population_id.clone()),
    );
    auth.insert(
        serde_yaml::Value::String("issuer".to_string()),
        serde_yaml::Value::String(population.issuer.clone()),
    );
    auth.insert(
        serde_yaml::Value::String("client_id".to_string()),
        serde_yaml::Value::String(population.client_id.clone()),
    );
    auth.insert(
        serde_yaml::Value::String("callback_path".to_string()),
        serde_yaml::Value::String(population.callback_path.clone()),
    );
    auth.insert(
        serde_yaml::Value::String("scopes".to_string()),
        serde_yaml::Value::Sequence(
            population
                .scopes
                .iter()
                .map(|scope| serde_yaml::Value::String(scope.clone()))
                .collect(),
        ),
    );
    if let Some(registration) = &population.registration_url {
        auth.insert(
            serde_yaml::Value::String("registration_url".to_string()),
            serde_yaml::Value::String(registration.clone()),
        );
    }
    if !applied.secret_references.is_empty() {
        auth.insert(
            serde_yaml::Value::String("secret_references".to_string()),
            serde_yaml::Value::Sequence(
                applied
                    .secret_references
                    .iter()
                    .map(|reference| serde_yaml::Value::String(reference.clone()))
                    .collect(),
            ),
        );
    }

    if let serde_yaml::Value::Mapping(map) = &mut value {
        map.insert(
            serde_yaml::Value::String("auth".to_string()),
            serde_yaml::Value::Mapping(auth),
        );
    }

    let rendered = serde_yaml::to_string(&value).map_err(|error| ForgeError::AuthSetupRefused {
        reason: format!("the project manifest cannot be rendered: {error}"),
    })?;
    fs::write(manifest_path, rendered).map_err(|error| ForgeError::AuthSetupRefused {
        reason: format!("cannot write {}: {error}", manifest_path.display()),
    })?;
    Ok(())
}

/// Runs one pack and reads its answer.
fn call_pack(
    pack: &super::packs::ProviderPack,
    request: &ProviderRequest,
) -> Result<ProviderResponse, ContractError> {
    let mut child = Command::new(&pack.executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ContractError::Unavailable(error.to_string()))?;

    let payload = serde_json::to_string(request)
        .map_err(|error| ContractError::Unavailable(error.to_string()))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(payload.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|error| ContractError::Unavailable(error.to_string()))?;
    }
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| ContractError::Unavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(ContractError::Unavailable(format!(
            "the pack exited with status {}",
            output.status
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| ContractError::Unavailable("the pack answered nothing".to_string()))?;
    // The answer is scanned before it is parsed. The typed response drops the
    // fields the contract does not define, so a secret sent in an unknown
    // field would otherwise pass the scan unnoticed.
    let answer: Value = serde_json::from_str(line).map_err(|error| {
        ContractError::Unavailable(format!("the pack answer is not valid: {error}"))
    })?;
    scan_answer_for_secrets(&answer)?;
    serde_json::from_value::<ProviderResponse>(answer).map_err(|error| {
        ContractError::Unavailable(format!("the pack answer is not valid: {error}"))
    })
}

/// A short, stable seed for an operation identifier.
fn digest_seed(project_id: &str) -> String {
    let digest = plan_digest(project_id, "forge", "seed", &[]);
    digest
        .trim_start_matches("sha256:")
        .chars()
        .take(12)
        .collect()
}

/// The plan changes of a response, for rendering.
pub fn plan_changes(response: &ProviderResponse) -> &[PlanChange] {
    &response.proposed_changes
}

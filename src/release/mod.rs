//! Gated resumable releases and publication (`release-publishing`).
//!
//! Core owns the typed release contract. v0.1.0 supports:
//!
//! - `prepare` captures semver, source revision, changelog,
//!   configured documentation and the doctor/test/DriftWatch
//!   evidence into a reviewable plan. No release side effect
//!   runs from `prepare`.
//! - `apply` walks the captured stages (commit, tag, push,
//!   mirror, package, container, docs, notes) once a plan is
//!   verified. Each stage owns a per-stage record so a retry
//!   resumes from the last delivered stage instead of
//!   redoing prior work.
//! - A semver tag, a package version and a container tag are
//!   treated as immutable identities: a second attempt that
//!   points them at a different commit surfaces as
//!   `release-identity-conflict` and refuses to overwrite.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §29, §34, §43 require
//! gated resumable releases with verifiable preparation. The
//! contract is independent of any real provider: a missing
//! package binary, an unavailable DriftWatch adapter, an
//! unparseable changelog or a stale plan all surface as
//! typed `error[...]` responses before any tag, push,
//! package, container or notes file is written.
//!
//! ## Persistence
//!
//! [`ReleaseState`] lives under
//! `.forge/release/<project-id>/<release-id>/state.json` so a
//! retry sees exactly which stages already delivered and which
//! need another attempt. The state is local evidence, not a
//! record of authority: a successful run overwrites the prior
//! entry, a failed run leaves it untouched. The Core
//! registry's `operations` table receives one `release` row
//! per prepare/apply with a `done`/`partial` summary that
//! lists the captured checks and per-stage statuses.
//!
//! ## Risk model
//!
//! Published artifacts and pushed tags are irreversible. The
//! contract refuses to apply a stage without an explicit
//! `--confirm`; the contract refuses to overwrite an existing
//! tag, package version or container tag at a different
//! commit. Credentials embedded in evidence are redacted
//! through [`crate::policy::redact_credentials`].
//!
//! Real provider integration is out of scope for v0.1.0:
//! package/container/notes stages reuse the same
//! adapter-by-environment-variable pattern the docs and
//! policy contracts use, so a fixture binary stands in for a
//! real provider round trip.

pub mod engine;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::manifest::{Manifest, ReleaseMeta};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Contract data version for the release surface. The package
/// and container adapters speak the same version on
/// stdin/stdout.
pub const RELEASE_CONTRACT_VERSION: &str = "0.1.0";

/// Release state subdirectory inside the project. Each release
/// owns `<project>/.forge/release/<project-id>/<release-id>/state.json`.
pub const RELEASE_STATE_DIR: &str = ".forge/release";

/// Default changelog filename when the manifest omits one.
pub const DEFAULT_CHANGELOG: &str = "CHANGELOG.md";

/// Default release notes template when the manifest omits one.
pub const DEFAULT_NOTES_TEMPLATE: &str = "RELEASE_NOTES.md";

/// Default release notes output when the manifest omits one.
pub const DEFAULT_NOTES_OUTPUT: &str = "RELEASE_NOTES.md";

/// Default binary for the package adapter. Real provider
/// integration is out of scope; the binary is invoked with
/// argument arrays and a bounded timeout.
pub const DEFAULT_PACKAGE_BIN: &str = "forge-package-publisher";

/// Environment variable selecting the package adapter binary.
pub const PACKAGE_BIN_ENV: &str = "FORGE_PACKAGE_BIN";

/// Default binary for the container adapter.
pub const DEFAULT_CONTAINER_BIN: &str = "forge-container-publisher";

/// Environment variable selecting the container adapter binary.
pub const CONTAINER_BIN_ENV: &str = "FORGE_CONTAINER_BIN";

/// Default binary for the release notes adapter.
pub const DEFAULT_NOTES_BIN: &str = "forge-notes-renderer";

/// Environment variable selecting the release notes adapter.
pub const NOTES_BIN_ENV: &str = "FORGE_NOTES_BIN";

/// Per-run adapter timeout. Spawn plus bounded wait so an
/// unresponsive provider cannot hang the registry.
pub const ADAPTER_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable stage identifiers. Stages are applied in this
/// declaration order; the contract never reorders a stage
/// after a release identity is captured.
pub const STAGE_COMMIT: &str = "commit";
pub const STAGE_TAG: &str = "tag";
pub const STAGE_PUSH: &str = "push";
pub const STAGE_MIRROR: &str = "mirror";
pub const STAGE_PACKAGE: &str = "package";
pub const STAGE_CONTAINER: &str = "container";
pub const STAGE_DOCS: &str = "docs";
pub const STAGE_NOTES: &str = "notes";

/// Default stage list when the manifest does not override it.
pub const DEFAULT_STAGES: &[&str] = &[
    STAGE_COMMIT,
    STAGE_TAG,
    STAGE_PUSH,
    STAGE_MIRROR,
    STAGE_PACKAGE,
    STAGE_CONTAINER,
    STAGE_DOCS,
    STAGE_NOTES,
];

/// Stable per-stage statuses. The transport (CLI/MCP) renders
/// these labels verbatim; the Core contract owns the set.
pub const STATUS_SKIPPED: &str = "skipped";
pub const STATUS_DELIVERED: &str = "delivered";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_DISABLED: &str = "disabled";
pub const STATUS_DIVERGED: &str = "diverged";
pub const STATUS_UNAVAILABLE: &str = "unavailable";
pub const STATUS_CONFLICT: &str = "conflict";

/// Stable per-check statuses.
pub const CHECK_PASS: &str = "pass";
pub const CHECK_FAIL: &str = "fail";
pub const CHECK_UNAVAILABLE: &str = "unavailable";
pub const CHECK_STALE: &str = "stale";
pub const CHECK_DISABLED: &str = "disabled";

/// Semver components. `major.minor.patch` with optional
/// `-prerelease` and `+build`. The brief mandates semver for
/// releases; only semver is accepted so a manifest cannot
/// silently introduce an unsupportable versioning scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Semver {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    #[serde(default)]
    pub prerelease: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
}

impl Semver {
    pub fn parse(raw: &str) -> Result<Self, ForgeError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::ReleaseInvalid {
                reason: "version must not be empty".to_string(),
            });
        }
        // Strip build metadata
        let without_build = trimmed.split('+').next().unwrap_or(trimmed);
        // Split numeric + prerelease
        let (numeric, prerelease) = match without_build.split_once('-') {
            Some((num, pre)) => (num, Some(pre.to_string())),
            None => (without_build, None),
        };
        let parts: Vec<&str> = numeric.split('.').collect();
        if parts.len() != 3 {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "version `{raw}` is not a semver triple (expected major.minor.patch)"
                ),
            });
        }
        let major = parse_semver_component(parts[0], "major", raw)?;
        let minor = parse_semver_component(parts[1], "minor", raw)?;
        let patch = parse_semver_component(parts[2], "patch", raw)?;
        if let Some(ref pre) = prerelease {
            if pre.is_empty() {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!("version `{raw}` carries an empty prerelease"),
                });
            }
            if !pre
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!(
                        "version `{raw}` prerelease `{pre}` contains invalid characters"
                    ),
                });
            }
        }
        Ok(Semver {
            major,
            minor,
            patch,
            prerelease,
            build: None,
        })
    }

    pub fn label(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        match &self.prerelease {
            Some(pre) => match &self.build {
                Some(build) => format!("{base}-{pre}+{build}"),
                None => format!("{base}-{pre}"),
            },
            None => match &self.build {
                Some(build) => format!("{base}+{build}"),
                None => base,
            },
        }
    }
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

fn parse_semver_component(raw: &str, kind: &str, full: &str) -> Result<u64, ForgeError> {
    if raw.is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` is missing the {kind} component"),
        });
    }
    if !raw.chars().all(|c| c.is_ascii_digit()) {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` {kind} component is not numeric"),
        });
    }
    // No leading zeros except literal `0`.
    if raw.len() > 1 && raw.starts_with('0') {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("version `{full}` {kind} component has a leading zero"),
        });
    }
    raw.parse::<u64>().map_err(|_| ForgeError::ReleaseInvalid {
        reason: format!("version `{full}` {kind} component overflows u64"),
    })
}

/// Hex SHA-256 over bytes. Used for release identity hashing
/// and adapter contract signatures.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// One prepared check outcome. `evidence` is the typed reason
/// the check passed, failed, was unavailable, was stale or was
/// disabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub kind: String,
    pub status: String,
    pub applicable: bool,
    pub evidence: Vec<String>,
    pub detail: String,
}

/// One configured check kind. Mirrors the manifest's
/// `release.checks` array after normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSpec {
    pub kind: String,
}

/// One package destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSpec {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub version: String,
}

/// One container destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub name: String,
    pub dockerfile: String,
    pub registry: Option<String>,
    pub tag: String,
}

/// Notes destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesSpec {
    pub template: String,
    pub output: String,
}

/// Documentation translation locales from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsSpec {
    pub translate: Vec<String>,
}

/// Validated release config parsed from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseConfig {
    pub versioning: String,
    pub stages: Vec<String>,
    pub checks: Vec<CheckSpec>,
    pub changelog: String,
    pub packages: Vec<PackageSpec>,
    pub containers: Vec<ContainerSpec>,
    pub notes: Option<NotesSpec>,
    pub docs: Option<DocsSpec>,
}

impl ReleaseConfig {
    /// Parse the raw [`ReleaseMeta`] into a typed
    /// configuration. Defaults are applied here so the rest of
    /// the contract can read every field without checking for
    /// absence.
    pub fn from_manifest_meta(meta: &ReleaseMeta) -> Result<Self, ForgeError> {
        let versioning = meta
            .versioning
            .clone()
            .unwrap_or_else(|| "semver".to_string());
        if versioning != "semver" {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "release.versioning `{versioning}` is not supported; only `semver` is accepted in v0.1.0"
                ),
            });
        }
        let mut checks = Vec::new();
        for entry in &meta.checks {
            let kind = entry.kind.trim().to_string();
            if kind.is_empty() {
                return Err(ForgeError::ReleaseInvalid {
                    reason: "release.checks entries must carry a non-empty kind".to_string(),
                });
            }
            if !matches!(kind.as_str(), "doctor" | "test" | "driftwatch" | "gate") {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!(
                        "release.checks kind `{kind}` is not supported; expected one of doctor, test, driftwatch, gate"
                    ),
                });
            }
            if checks.iter().any(|c: &CheckSpec| c.kind == kind) {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!("release.checks kind `{kind}` is duplicated"),
                });
            }
            checks.push(CheckSpec { kind });
        }
        let changelog = meta
            .changelog
            .clone()
            .unwrap_or_else(|| DEFAULT_CHANGELOG.to_string());
        if changelog.trim().is_empty() {
            return Err(ForgeError::ReleaseInvalid {
                reason: "release.changelog must not be empty".to_string(),
            });
        }
        let mut packages = Vec::new();
        for entry in &meta.packages {
            let kind = entry.kind.clone().unwrap_or_else(|| "generic".to_string());
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| "release-artifact".to_string());
            let path = entry.path.clone().unwrap_or_else(|| ".".to_string());
            let version = entry.version.clone().unwrap_or_else(|| "0.0.0".to_string());
            if name.trim().is_empty() {
                return Err(ForgeError::ReleaseInvalid {
                    reason: "release.packages[].name must not be empty".to_string(),
                });
            }
            if path.trim().is_empty() {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!("release.packages[].path for `{name}` must not be empty"),
                });
            }
            if !lexically_inside(".", &path) {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!(
                        "release.packages[].path `{path}` for `{name}` resolves outside the project"
                    ),
                });
            }
            packages.push(PackageSpec {
                kind,
                name,
                path,
                version,
            });
        }
        let mut containers = Vec::new();
        for entry in &meta.containers {
            let name = entry
                .name
                .clone()
                .unwrap_or_else(|| "release-image".to_string());
            let dockerfile = entry
                .dockerfile
                .clone()
                .unwrap_or_else(|| "Dockerfile".to_string());
            let registry = entry.registry.clone();
            let tag = entry.tag.clone().unwrap_or_else(|| "0.0.0".to_string());
            if name.trim().is_empty() {
                return Err(ForgeError::ReleaseInvalid {
                    reason: "release.containers[].name must not be empty".to_string(),
                });
            }
            if !lexically_inside(".", &dockerfile) {
                return Err(ForgeError::ReleaseInvalid {
                    reason: format!(
                        "release.containers[].dockerfile `{dockerfile}` for `{name}` resolves outside the project"
                    ),
                });
            }
            containers.push(ContainerSpec {
                name,
                dockerfile,
                registry,
                tag,
            });
        }
        let notes = match &meta.notes {
            Some(notes) => {
                let template = notes
                    .template
                    .clone()
                    .unwrap_or_else(|| DEFAULT_NOTES_TEMPLATE.to_string());
                let output = notes
                    .output
                    .clone()
                    .unwrap_or_else(|| DEFAULT_NOTES_OUTPUT.to_string());
                if !lexically_inside(".", &output) {
                    return Err(ForgeError::ReleaseInvalid {
                        reason: format!(
                            "release.notes.output `{output}` resolves outside the project"
                        ),
                    });
                }
                Some(NotesSpec { template, output })
            }
            None => None,
        };
        let docs = match &meta.docs {
            Some(docs) => {
                let mut translate = Vec::new();
                for locale in &docs.translate {
                    if locale.trim().is_empty() {
                        return Err(ForgeError::ReleaseInvalid {
                            reason: "release.docs.translate must not contain empty entries"
                                .to_string(),
                        });
                    }
                    translate.push(locale.clone());
                }
                if translate.is_empty() {
                    None
                } else {
                    Some(DocsSpec { translate })
                }
            }
            None => None,
        };
        Ok(ReleaseConfig {
            versioning,
            stages: DEFAULT_STAGES.iter().map(|s| s.to_string()).collect(),
            checks,
            changelog,
            packages,
            containers,
            notes,
            docs,
        })
    }

    /// Overwrite the stage list. The CLI uses this to honor
    /// `--stages` overrides; the order is preserved as supplied.
    pub fn with_stages(mut self, stages: Vec<String>) -> Self {
        if !stages.is_empty() {
            self.stages = stages;
        }
        self
    }
}

/// Build a [`ReleaseConfig`] from a manifest's release section.
/// Convenience for callers that already have the manifest.
pub fn release_config_from_manifest(manifest: &Manifest) -> Result<ReleaseConfig, ForgeError> {
    match manifest.release.as_ref() {
        Some(meta) => ReleaseConfig::from_manifest_meta(meta),
        None => Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "project `{}` has no `release` section; declare one with at least `versioning: semver`",
                manifest.project.id
            ),
        }),
    }
}

/// Release identity. The id is derived from the project id,
/// the semver label and the first 12 hex characters of the
/// source revision hash. Two release attempts on the same
/// project at the same semver and source revision resolve to
/// the same id; a different revision or semver produces a
/// different id, so a prepared plan is never silently
/// reused for a different artifact.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseIdentity {
    pub project_id: String,
    pub version: String,
    pub source_revision: String,
    pub id: String,
}

impl ReleaseIdentity {
    pub fn derive(project_id: &str, version: &Semver, source_revision: &str) -> Self {
        let version_label = version.label();
        let mut hasher = Sha256::new();
        hasher.update(project_id.as_bytes());
        hasher.update(b"@");
        hasher.update(version_label.as_bytes());
        hasher.update(b"@");
        hasher.update(source_revision.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        let short = &digest[..12.min(digest.len())];
        let id = format!("{project_id}-{version_label}-{short}");
        ReleaseIdentity {
            project_id: project_id.to_string(),
            version: version_label,
            source_revision: source_revision.to_string(),
            id,
        }
    }
}

/// Release request from the transport layer. The CLI/MCP build
/// this; Core owns validation and execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleaseRequest {
    pub project_id: String,
    pub version: Semver,
    pub confirm: bool,
    pub dry_run: bool,
    pub retry: bool,
    pub stages: Vec<String>,
}

impl ReleaseRequest {
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.project_id.trim().is_empty() {
            return Err(ForgeError::ReleaseInvalid {
                reason: "project id must not be empty".to_string(),
            });
        }
        Ok(())
    }
}

/// One check captured during prepare. Distinct from
/// [`CheckOutcome`] so a plan can record the captured
/// revision alongside the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedCheck {
    pub kind: String,
    pub status: String,
    pub applicable: bool,
    pub evidence: Vec<String>,
    pub detail: String,
    /// Source revision at which the check was observed. The
    /// apply step refuses to consume a stale check.
    pub source_revision: String,
}

/// Bound changelog excerpt. The full content is captured at
/// prepare time so a re-run never reads a different file
/// after the working tree has moved on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedChangelog {
    pub path: String,
    pub content_hash: String,
    pub excerpt: String,
}

/// Read-only release plan. The transport renders this before
/// any side effect runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasePlan {
    pub contract: String,
    pub identity: ReleaseIdentity,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
    pub checks: Vec<CapturedCheck>,
    pub stages: Vec<String>,
    pub note: String,
    pub ready: bool,
}

impl ReleasePlan {
    pub fn healthy(&self) -> bool {
        self.ready
    }
}

/// Aggregate release report from a prepare/apply run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleaseReport {
    pub contract: String,
    pub project_id: String,
    pub identity: ReleaseIdentity,
    pub changelog: Option<CapturedChangelog>,
    pub docs_locales: Vec<String>,
    pub checks: Vec<CapturedCheck>,
    pub stages: Vec<String>,
    pub stage_outcomes: Vec<StageOutcome>,
    pub dry_run: bool,
    pub retry: bool,
    pub state_path: String,
    pub note: String,
    pub healthy: bool,
}

impl ReleaseReport {
    pub fn healthy(&self) -> bool {
        self.healthy
    }
}

/// One captured per-stage result. The status set is stable
/// across CLI, MCP and journal rows; the contract owns the
/// labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageOutcome {
    pub stage: String,
    pub target: String,
    pub status: String,
    pub identity: Option<String>,
    pub note: String,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
}

/// Persisted release state. Tracks which stage already
/// delivered at which commit so a retry only re-runs
/// outstanding stages. Stored under
/// `.forge/release/<project-id>/<release-id>/state.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseState {
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub identity: ReleaseIdentity,
    #[serde(default)]
    pub changelog: Option<CapturedChangelog>,
    #[serde(default)]
    pub docs_locales: Vec<String>,
    #[serde(default)]
    pub checks: Vec<CapturedCheck>,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub stage_outcomes: Vec<StageOutcome>,
    #[serde(default)]
    pub last_run_at: String,
}

impl ReleaseState {
    pub fn delivered_stages(&self) -> Vec<&StageOutcome> {
        self.stage_outcomes
            .iter()
            .filter(|s| s.status == STATUS_DELIVERED || s.status == STATUS_SKIPPED)
            .collect()
    }
}

/// Compute the on-disk state path for one release. The path
/// is lexically scoped to the project directory, the project
/// id and the release identity so two projects (or two
/// release attempts on the same project) can never share a
/// state file.
pub fn state_path_for(
    project_dir: &Path,
    project_id: &str,
    identity: &ReleaseIdentity,
) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "project id is required to resolve the release state path".to_string(),
        });
    }
    if identity.id.trim().is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release id is required to resolve the release state path".to_string(),
        });
    }
    Ok(project_dir
        .join(RELEASE_STATE_DIR)
        .join(project_id)
        .join(&identity.id)
        .join("state.json"))
}

pub fn load_release_state(path: &Path) -> Result<ReleaseState, ForgeError> {
    if !path.exists() {
        return Ok(ReleaseState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read release state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(ReleaseState::default());
    }
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!(
            "release state at {} is not valid JSON: {err}",
            path.display()
        ),
    })
}

pub fn save_release_state(path: &Path, state: &ReleaseState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!(
                "cannot create release state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot serialize release state: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot write release state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot rename release state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Adapter configuration. The CLI/MCP build this from
/// environment variables; Core owns the default values so
/// contract fixtures can stand in for real provider
/// integration.
#[derive(Debug, Clone)]
pub struct ReleaseAdapterConfig {
    pub package_bin: String,
    pub container_bin: String,
    pub notes_bin: String,
}

impl ReleaseAdapterConfig {
    pub fn from_env() -> Self {
        let package_bin = std::env::var(PACKAGE_BIN_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PACKAGE_BIN.to_string());
        let container_bin = std::env::var(CONTAINER_BIN_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CONTAINER_BIN.to_string());
        let notes_bin = std::env::var(NOTES_BIN_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_NOTES_BIN.to_string());
        ReleaseAdapterConfig {
            package_bin,
            container_bin,
            notes_bin,
        }
    }
}

/// Redact credential-like substrings from a piece of evidence.
/// The redaction is delegated to
/// [`crate::policy::redact_credentials`] so the release,
/// policy and distribution contracts share one definition
/// of "secret".
pub fn redact_release_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Load a file from `project_dir` and return its content
/// hash plus a bounded excerpt. The full content is not
/// embedded in the report so the report stays small even
/// for large changelogs.
pub fn load_changelog(project_dir: &Path, relative: &str) -> Result<CapturedChangelog, ForgeError> {
    if !lexically_inside(".", relative) {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!("changelog `{relative}` resolves outside the project"),
        });
    }
    let path = project_dir.join(relative);
    if !path.is_file() {
        return Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "changelog file `{}` does not exist; release requires a present changelog",
                path.display()
            ),
        });
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::ReleaseInvalid {
        reason: format!("cannot read changelog {}: {err}", path.display()),
    })?;
    let content_hash = hash_bytes(&bytes);
    let text = String::from_utf8_lossy(&bytes);
    let excerpt = excerpt_first_lines(&text, 40);
    Ok(CapturedChangelog {
        path: relative.to_string(),
        content_hash,
        excerpt,
    })
}

fn excerpt_first_lines(text: &str, max_lines: usize) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i >= max_lines {
            out.push_str("…\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Capture the current working-tree revision. A non-git
/// project is refused before any release side effect runs.
pub fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ForgeError::ReleaseInvalid {
            reason: format!(
                "directory `{}` is not a git working tree with a HEAD commit: {}",
                dir.display(),
                stderr.trim()
            ),
        });
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        return Err(ForgeError::ReleaseInvalid {
            reason: "git rev-parse returned an empty SHA".to_string(),
        });
    }
    Ok(sha)
}

/// Render a [`ReleaseReport`] for human output. The transport
/// renders the same data the JSON envelope carries so a
/// partial run is observable on stdout.
pub fn render_report_human(report: &ReleaseReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!("release_id: {}", report.identity.id));
    lines.push(format!("version: {}", report.identity.version));
    lines.push(format!(
        "source_revision: {}",
        report.identity.source_revision
    ));
    if let Some(changelog) = &report.changelog {
        lines.push(format!("changelog: {}", changelog.path));
        lines.push(format!("changelog_hash: {}", changelog.content_hash));
    }
    if !report.docs_locales.is_empty() {
        lines.push(format!("docs_locales: {}", report.docs_locales.join(", ")));
    }
    lines.push(format!("stages: {}", report.stages.join(", ")));
    lines.push(format!("mode: {}", release_mode_label(report)));
    lines.push(format!("state: {}", report.state_path));
    if !report.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &report.checks {
            lines.push(format!(
                "  - {kind} {status} applicable={applicable} revision={revision}: {detail}",
                kind = check.kind,
                status = check.status,
                applicable = check.applicable,
                revision = check.source_revision,
                detail = check.detail
            ));
        }
    }
    if !report.stage_outcomes.is_empty() {
        lines.push("stage_outcomes:".to_string());
        for outcome in &report.stage_outcomes {
            lines.push(format!(
                "  - {stage} target=`{target}` {status}: {note}",
                stage = outcome.stage,
                target = outcome.target,
                status = outcome.status,
                note = outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

fn release_mode_label(report: &ReleaseReport) -> String {
    if report.dry_run {
        "dry-run".to_string()
    } else if report.retry {
        "retry".to_string()
    } else {
        "apply".to_string()
    }
}

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem. Absolute paths must already sit inside
/// the project, and `..` segments must never escape the
/// root. Symlinks are rechecked canonically at write time.
fn lexically_inside(_project_dir: &str, rel: &str) -> bool {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return false;
    }
    let mut depth = 0i32;
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn meta_with(text: &str) -> ReleaseMeta {
        let manifest = Manifest::parse(
            Path::new("forge.yaml"),
            format!(
                "schema: 1\nproject:\n  id: app\n  name: App\n  profile: rust-web\nrelease:\n{text}"
            )
            .as_bytes(),
        )
        .expect("manifest");
        manifest.release.expect("release section")
    }

    #[test]
    fn semver_parses_full_triple() {
        let v = Semver::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.label(), "1.2.3");
    }

    #[test]
    fn semver_parses_prerelease() {
        let v = Semver::parse("1.2.3-rc.1").unwrap();
        assert_eq!(v.label(), "1.2.3-rc.1");
    }

    #[test]
    fn semver_rejects_non_triple() {
        let err = Semver::parse("1.2").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("1.2.3.4").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn semver_rejects_non_numeric_components() {
        let err = Semver::parse("1.x.3").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
        let err = Semver::parse("01.0.0").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn semver_rejects_unsupported_versioning() {
        let meta = meta_with("  versioning: calver\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_defaults_to_eight_stages() {
        let meta = meta_with("  versioning: semver\n");
        let cfg = ReleaseConfig::from_manifest_meta(&meta).unwrap();
        let expected: Vec<String> = DEFAULT_STAGES.iter().map(|s| s.to_string()).collect();
        assert_eq!(cfg.stages, expected);
    }

    #[test]
    fn release_config_rejects_unknown_check_kind() {
        let meta = meta_with("  versioning: semver\n  checks:\n    - kind: lint\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_accepts_known_checks() {
        let meta = meta_with(
            "  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n    - kind: driftwatch\n",
        );
        let cfg = ReleaseConfig::from_manifest_meta(&meta).unwrap();
        let kinds: Vec<&str> = cfg.checks.iter().map(|c| c.kind.as_str()).collect();
        assert_eq!(kinds, vec!["doctor", "test", "driftwatch"]);
    }

    #[test]
    fn release_config_rejects_package_path_outside_project() {
        let meta =
            meta_with("  versioning: semver\n  packages:\n    - name: x\n      path: ../evil\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_config_rejects_duplicate_check_kind() {
        let meta =
            meta_with("  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: doctor\n");
        let err = ReleaseConfig::from_manifest_meta(&meta).unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn release_identity_is_stable_for_same_inputs() {
        let v = Semver::parse("1.2.3").unwrap();
        let a = ReleaseIdentity::derive("app", &v, "deadbeefcafe");
        let b = ReleaseIdentity::derive("app", &v, "deadbeefcafe");
        assert_eq!(a, b);
        let c = ReleaseIdentity::derive("app", &v, "different");
        assert_ne!(a, c);
    }

    #[test]
    fn load_changelog_reads_present_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("CHANGELOG.md"),
            "## 1.0.0\n- initial release\n",
        )
        .unwrap();
        let captured = load_changelog(tmp.path(), "CHANGELOG.md").unwrap();
        assert_eq!(captured.path, "CHANGELOG.md");
        assert!(captured.excerpt.contains("1.0.0"));
    }

    #[test]
    fn load_changelog_refuses_missing_file() {
        let tmp = TempDir::new().unwrap();
        let err = load_changelog(tmp.path(), "CHANGELOG.md").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }

    #[test]
    fn load_changelog_refuses_outside_project_path() {
        let err = load_changelog(Path::new("."), "../etc/passwd").unwrap_err();
        assert_eq!(err.code(), "release-invalid");
    }
}

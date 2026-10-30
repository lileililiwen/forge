//! The delegated container-image build.
//!
//! `forge build --image` turns a source tree into an image without Forge
//! owning a builder. The builder is an operator-installed adapter that
//! speaks `forge-image-builder/0.1.0`: one JSON request on stdin, one JSON
//! response on stdout. Forge resolves the revision, derives the tag, runs
//! the adapter under the shared native-command bound, and refuses any
//! claim it cannot check — a different image, a digest that is not a
//! `sha256` digest, an SBOM path that leaves the project.
//!
//! A successful build writes one revision-bound provenance record under
//! `.forge/build/<project-id>/<sha12>.json`. That record is its own
//! evidence: it is not Gate evidence and it does not feed the deploy lane.

pub mod image_tests;

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::ForgeError;
use crate::extension::ContractVersion;
use crate::native::run_native_with_input;
use crate::policy::redact_credentials;

/// The contract the image-builder adapter speaks.
pub const IMAGE_BUILDER_CONTRACT: &str = "forge-image-builder/0.1.0";

/// The contract of the provenance record Forge writes.
pub const IMAGE_BUILD_RECORD_CONTRACT: &str = "forge-build-image/0.1.0";

/// The variable that names the adapter binary.
pub const IMAGE_BUILDER_ENV: &str = "FORGE_IMAGE_BUILDER_BIN";

/// The adapter binary used when nothing else names one.
pub const IMAGE_BUILDER_DEFAULT: &str = "forge-image-builder";

/// The largest SBOM Forge will read and hash.
pub const SBOM_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The most evidence lines a response may carry.
pub const EVIDENCE_MAX: usize = 20;

/// The longest evidence line Forge keeps.
pub const EVIDENCE_LINE_MAX: usize = 800;

/// One image build's provenance record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBuildOutcome {
    pub contract: String,
    pub project_id: String,
    pub revision: String,
    pub tag: String,
    pub builder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder_version: Option<String>,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbom: Option<SbomRecord>,
    pub evidence: Vec<String>,
    pub observed_at: String,
}

/// The SBOM Forge observed, hashed itself and recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomRecord {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// The adapter's answer, before Forge trusts any of it.
#[derive(Debug, Deserialize)]
struct BuilderResponse {
    contract: String,
    #[serde(default)]
    builder: Option<String>,
    #[serde(default)]
    builder_version: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    sbom_path: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Serialize)]
struct BuilderRequest<'a> {
    contract: &'a str,
    project_id: &'a str,
    revision: &'a str,
    directory: String,
    tag: &'a str,
    dry_run: bool,
}

/// The tag Forge derives when the operator names none.
pub fn default_tag(project_id: &str, revision: &str) -> String {
    format!("forge-{project_id}:{}", &revision[..12])
}

/// Refuse a tag an adapter could read as a flag or an invalid reference.
pub fn validate_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty() || tag.len() > 255 {
        return Err("the image tag must be 1..=255 characters".to_string());
    }
    if tag.starts_with('-') || tag.starts_with('.') {
        return Err(format!(
            "the image tag `{tag}` must not start with `-` or `.`"
        ));
    }
    if !tag
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ':' | '@'))
    {
        return Err(format!(
            "the image tag `{tag}` carries a character outside `[A-Za-z0-9._-/:@]`"
        ));
    }
    Ok(())
}

/// Refuse anything that is not a `sha256` digest.
pub fn validate_digest(digest: &str) -> Result<(), String> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(format!(
            "the image digest `{digest}` does not start with `sha256:`"
        ));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(format!(
            "the image digest `{digest}` is not `sha256:` followed by 64 lower-case hex characters"
        ));
    }
    Ok(())
}

/// Build an image through the resolved adapter.
pub fn run_image_build(
    project_id: &str,
    dir: &Path,
    tag: Option<&str>,
    builder_bin: Option<&str>,
) -> Result<ImageBuildOutcome, ForgeError> {
    if project_id.is_empty()
        || !project_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(ForgeError::BuildInvalid {
            reason: format!(
                "the project id `{project_id}` is not a safe record name; expected \
                 lower-case letters, digits, `-` or `_`"
            ),
        });
    }
    let revision = git_revision(dir)?;
    let tag = match tag {
        Some(tag) => {
            validate_tag(tag).map_err(|reason| ForgeError::BuildInvalid { reason })?;
            tag.to_string()
        }
        None => default_tag(project_id, &revision),
    };
    let adapter = resolve_adapter(builder_bin);
    let request = BuilderRequest {
        contract: IMAGE_BUILDER_CONTRACT,
        project_id,
        revision: &revision,
        directory: dir.display().to_string(),
        tag: &tag,
        dry_run: false,
    };
    let input = serde_json::to_vec(&request).map_err(|err| ForgeError::BuildInvalid {
        reason: format!("cannot encode the image-builder request: {err}"),
    })?;

    let run = run_native_with_input(dir, &adapter, &[], &[], &input).map_err(|err| {
        ForgeError::BuildInvalid {
            reason: format!("the image builder `{adapter}` could not run: {err}"),
        }
    })?;
    if let Some(start_error) = run.start_error {
        return Err(ForgeError::BuildInvalid {
            reason: format!(
                "the image builder `{adapter}` could not start: {start_error}; install it or set \
                 `{IMAGE_BUILDER_ENV}`; Forge never installs a builder"
            ),
        });
    }
    if run.timed_out {
        return Err(ForgeError::BuildFailed {
            reason: format!(
                "the image builder `{adapter}` exceeded the build time bound; its process group \
                 was killed and no record was written"
            ),
        });
    }
    if !run.success {
        let status = run
            .exit_code
            .map(|code| format!("status {code}"))
            .unwrap_or_else(|| "no exit status".to_string());
        return Err(ForgeError::BuildFailed {
            reason: format!(
                "the image builder `{adapter}` exited with {status}: {}",
                run.tail(1200)
            ),
        });
    }

    let response: BuilderResponse =
        serde_json::from_str(&run.stdout).map_err(|err| ForgeError::BuildInvalid {
            reason: format!("the image builder `{adapter}` returned no readable response: {err}"),
        })?;
    match response.status.as_deref().unwrap_or("") {
        "built" => {}
        "failed" => {
            return Err(ForgeError::BuildFailed {
                reason: format!(
                    "the image builder `{adapter}` reported a failed build: {}",
                    bounded_evidence(&response.evidence).join(" | ")
                ),
            })
        }
        other => {
            return Err(ForgeError::BuildInvalid {
                reason: format!(
                    "the image builder `{adapter}` returned status `{other}`, which is neither \
                     `built` nor `failed`"
                ),
            })
        }
    }
    let (builder, builder_version, digest, sbom) = validate_response(&response, &tag, dir)
        .map_err(|reason| ForgeError::BuildInvalid {
            reason: format!(
                "the image builder `{adapter}` returned a response Forge refuses: {reason}"
            ),
        })?;
    let evidence = bounded_evidence(&response.evidence);

    let outcome = ImageBuildOutcome {
        contract: IMAGE_BUILD_RECORD_CONTRACT.to_string(),
        project_id: project_id.to_string(),
        revision: revision.clone(),
        tag: tag.clone(),
        builder,
        builder_version,
        digest,
        sbom,
        evidence,
        observed_at: Utc::now().to_rfc3339(),
    };
    write_record(dir, &outcome)?;
    Ok(outcome)
}

/// Resolve the adapter binary: the explicit flag, then the variable, then
/// the default name on `PATH`.
pub fn resolve_adapter(explicit: Option<&str>) -> String {
    explicit
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::env::var(IMAGE_BUILDER_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| IMAGE_BUILDER_DEFAULT.to_string())
}

/// Check every claim the adapter made and return what Forge will record.
fn validate_response(
    response: &BuilderResponse,
    requested_tag: &str,
    project_dir: &Path,
) -> Result<(String, Option<String>, String, Option<SbomRecord>), String> {
    let required = ContractVersion::new("forge-image-builder", 0, 1, 0);
    let declared = ContractVersion::parse(&response.contract)
        .map_err(|err| format!("the contract `{}` cannot be read: {err}", response.contract))?;
    if let Some(detail) = declared.incompatibility_detail(&required) {
        return Err(format!(
            "it declares `{}`, which is incompatible with `{}`: {detail}",
            response.contract, IMAGE_BUILDER_CONTRACT
        ));
    }
    let status = response.status.as_deref().unwrap_or("");
    if status != "built" {
        return Err(format!("the status `{status}` is not `built`"));
    }
    let builder = response
        .builder
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "it names no builder".to_string())?;
    if builder.len() > 128 {
        return Err(format!(
            "the builder name `{builder}` is longer than 128 characters"
        ));
    }
    let builder_version = response
        .builder_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if builder_version
        .as_deref()
        .map(|value| value.len() > 128)
        .unwrap_or(false)
    {
        return Err("the builder version is longer than 128 characters".to_string());
    }
    let image = response
        .image
        .as_deref()
        .ok_or_else(|| "it names no image".to_string())?;
    if image != requested_tag {
        return Err(format!(
            "it built `{image}` while `{requested_tag}` was requested"
        ));
    }
    let digest = response
        .digest
        .as_deref()
        .ok_or_else(|| "it records no digest".to_string())?;
    validate_digest(digest)?;
    let sbom = match response.sbom_path.as_deref() {
        Some(path) => Some(record_sbom(project_dir, path)?),
        None => None,
    };
    Ok((
        builder.to_string(),
        builder_version,
        digest.to_string(),
        sbom,
    ))
}

/// Read and hash the SBOM the adapter named.
fn record_sbom(project_dir: &Path, raw: &str) -> Result<SbomRecord, String> {
    let relative = Path::new(raw);
    if relative.as_os_str().is_empty() {
        return Err("the SBOM path is empty".to_string());
    }
    if relative.is_absolute() {
        return Err(format!(
            "the SBOM path `{raw}` is absolute; it must be project-relative"
        ));
    }
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "the SBOM path `{raw}` carries a component other than a plain name; it must stay \
             inside the project"
        ));
    }
    let root = project_dir.canonicalize().map_err(|err| {
        format!(
            "the project directory `{}` cannot be resolved: {err}",
            project_dir.display()
        )
    })?;
    let joined = project_dir.join(relative);
    let resolved = joined
        .canonicalize()
        .map_err(|err| format!("the SBOM path `{raw}` does not name a readable file: {err}"))?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        return Err(format!(
            "the SBOM path `{raw}` resolves outside the project or is not a file"
        ));
    }
    let metadata = std::fs::metadata(&resolved)
        .map_err(|err| format!("the SBOM `{raw}` cannot be read: {err}"))?;
    if metadata.len() > SBOM_MAX_BYTES {
        return Err(format!(
            "the SBOM `{raw}` is {} bytes, over the {SBOM_MAX_BYTES}-byte bound",
            metadata.len()
        ));
    }
    let mut file = std::fs::File::open(&resolved)
        .map_err(|err| format!("the SBOM `{raw}` cannot be opened: {err}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| format!("the SBOM `{raw}` cannot be read: {err}"))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(SbomRecord {
        path: raw.to_string(),
        sha256,
        bytes: bytes.len() as u64,
    })
}

/// Keep the evidence bounded and free of credential shapes.
fn bounded_evidence(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .take(EVIDENCE_MAX)
        .map(|line| {
            let redacted = redact_credentials(line);
            if redacted.chars().count() > EVIDENCE_LINE_MAX {
                let mut bounded: String = redacted.chars().take(EVIDENCE_LINE_MAX - 1).collect();
                bounded.push('…');
                bounded
            } else {
                redacted
            }
        })
        .collect()
}

/// The project's committed revision.
fn git_revision(dir: &Path) -> Result<String, ForgeError> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(dir)
        .output()
        .map_err(|err| ForgeError::BuildInvalid {
            reason: format!(
                "an image build is bound to a committed revision and Git could not run: {err}"
            ),
        })?;
    if !output.status.success() {
        return Err(ForgeError::BuildInvalid {
            reason: "an image build is bound to a committed revision, and this project has no Git \
                     HEAD; commit the source and retry"
                .to_string(),
        });
    }
    let revision = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ForgeError::BuildInvalid {
            reason: format!("Git reported `{revision}`, which is not a 40-character hex revision"),
        });
    }
    Ok(revision)
}

/// Write the record through a staged file and an atomic rename.
fn write_record(dir: &Path, outcome: &ImageBuildOutcome) -> Result<(), ForgeError> {
    let record_dir = dir.join(".forge/build").join(&outcome.project_id);
    std::fs::create_dir_all(&record_dir).map_err(|err| ForgeError::BuildFailed {
        reason: format!(
            "the build record directory `{}` cannot be created: {err}",
            record_dir.display()
        ),
    })?;
    let name = format!("{}.json", &outcome.revision[..12]);
    let staged = record_dir.join(format!(".{name}.tmp"));
    let final_path = record_dir.join(&name);
    let bytes = serde_json::to_vec_pretty(outcome).map_err(|err| ForgeError::BuildFailed {
        reason: format!("the build record cannot be encoded: {err}"),
    })?;
    std::fs::write(&staged, bytes).map_err(|err| ForgeError::BuildFailed {
        reason: format!(
            "the build record `{}` cannot be staged: {err}",
            staged.display()
        ),
    })?;
    std::fs::rename(&staged, &final_path).map_err(|err| {
        let _ = std::fs::remove_file(&staged);
        ForgeError::BuildFailed {
            reason: format!(
                "the build record `{}` cannot be published: {err}",
                final_path.display()
            ),
        }
    })?;
    Ok(())
}

/// The record path for one project and revision.
pub fn record_path(project_dir: &Path, project_id: &str, revision: &str) -> PathBuf {
    project_dir
        .join(".forge/build")
        .join(project_id)
        .join(format!("{}.json", &revision[..12]))
}

/// Verify the build record for the revision at `HEAD`.
///
/// Read-only: it reads the record, proves its identity, re-hashes the SBOM it
/// names and returns the record it trusted. It resolves no builder, starts no
/// process and writes nothing. A bound Gate check can run it as its command.
pub fn verify_image_build(project_id: &str, dir: &Path) -> Result<ImageBuildOutcome, ForgeError> {
    let revision = git_revision(dir)?;
    let relative = format!(".forge/build/{project_id}/{}", record_name(&revision));
    let path = record_path(dir, project_id, &revision);
    let bytes = std::fs::read(&path).map_err(|err| ForgeError::BuildInvalid {
        reason: format!(
            "no readable image build record at `{relative}` for revision `{revision}` ({err}); \
             run `forge build --image` at this revision"
        ),
    })?;
    let record: ImageBuildOutcome =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::BuildInvalid {
            reason: format!("the image build record `{relative}` cannot be read: {err}"),
        })?;

    let required = ContractVersion::new("forge-build-image", 0, 1, 0);
    let declared =
        ContractVersion::parse(&record.contract).map_err(|err| ForgeError::BuildInvalid {
            reason: format!(
                "the image build record `{relative}` declares `{}`, which cannot be read: {err}",
                record.contract
            ),
        })?;
    if let Some(detail) = declared.incompatibility_detail(&required) {
        return Err(ForgeError::BuildInvalid {
            reason: format!(
                "the image build record `{relative}` declares `{}`, which is incompatible with \
                 `{IMAGE_BUILD_RECORD_CONTRACT}`: {detail}",
                record.contract
            ),
        });
    }
    if record.project_id != project_id {
        return Err(ForgeError::BuildInvalid {
            reason: format!(
                "the image build record `{relative}` names project `{}`, not `{project_id}`",
                record.project_id
            ),
        });
    }
    if record.revision != revision {
        return Err(ForgeError::BuildInvalid {
            reason: format!(
                "the image build record `{relative}` names revision `{}`, not `{revision}`",
                record.revision
            ),
        });
    }
    validate_digest(&record.digest).map_err(|reason| ForgeError::BuildInvalid {
        reason: format!("the image build record `{relative}` carries an invalid digest: {reason}"),
    })?;

    if let Some(sbom) = record.sbom.as_ref() {
        let observed = record_sbom(dir, &sbom.path).map_err(|reason| ForgeError::BuildInvalid {
            reason: format!("the recorded SBOM cannot be read: {reason}"),
        })?;
        if observed.sha256 != sbom.sha256 || observed.bytes != sbom.bytes {
            return Err(ForgeError::BuildFailed {
                reason: format!(
                    "the SBOM `{}` no longer matches the recorded build: recorded sha256:{} \
                     ({} bytes), observed sha256:{} ({} bytes); rebuild with `forge build --image`",
                    sbom.path, sbom.sha256, sbom.bytes, observed.sha256, observed.bytes
                ),
            });
        }
    }
    Ok(record)
}

/// The record's file name for one revision.
fn record_name(revision: &str) -> String {
    format!("{}.json", &revision[..12])
}

//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::ReleaseMeta;
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::contract::{
    CONTAINER_BIN_ENV, DEFAULT_CHANGELOG, DEFAULT_CONTAINER_BIN, DEFAULT_NOTES_BIN,
    DEFAULT_NOTES_OUTPUT, DEFAULT_NOTES_TEMPLATE, DEFAULT_PACKAGE_BIN, DEFAULT_STAGES,
    NOTES_BIN_ENV, PACKAGE_BIN_ENV, STATUS_DELIVERED, STATUS_SKIPPED,
};
use super::prepare::{lexically_inside, parse_semver_component};

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
        let without_build = trimmed.split('+').next().unwrap_or(trimmed);
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
/// One package destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSpec {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub version: String,
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
/// One configured check kind. Mirrors the manifest's
/// `release.checks` array after normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSpec {
    pub kind: String,
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
/// Notes destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesSpec {
    pub template: String,
    pub output: String,
}
/// One container destination from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub name: String,
    pub dockerfile: String,
    pub registry: Option<String>,
    pub tag: String,
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
/// Documentation translation locales from the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsSpec {
    pub translate: Vec<String>,
}

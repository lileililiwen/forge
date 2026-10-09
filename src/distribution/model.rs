//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{DistributionMeta, MirrorEntry};
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::contract::PRIMARY_REMOTE;
use super::engine::{mirror_provider_label, parse_mirror_provider};

/// Per-remote mirror result. Roles are `"primary"` or
/// `"mirror"`. Status values are stable:
/// - `delivered`: the ref reached the remote
/// - `skipped`: the ref was already at the remote (retry path)
/// - `disabled`: the entry is disabled in the manifest
/// - `diverged`: the remote has diverging history; force refused
/// - `unavailable`: provider is planned, not yet supported
/// - `failed`: the push exited non-zero; see `evidence`/`recovery`
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MirrorRemoteOutcome {
    pub provider: String,
    pub role: String,
    pub status: String,
    pub commit_sha: Option<String>,
    pub note: String,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
}
/// Persisted mirror state. Tracks which provider received
/// which ref at which commit, so a retry only re-pushes
/// outstanding work and never rewrites evidence the host
/// already has.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorState {
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub primary_delivered: BTreeMap<String, String>,
    #[serde(default)]
    pub mirrors_delivered: BTreeMap<String, BTreeMap<String, String>>,
}
impl MirrorState {
    pub(super) fn record_primary(&mut self, ref_name: &str, sha: &str) {
        self.primary_delivered
            .insert(ref_name.to_string(), sha.to_string());
    }
    pub(super) fn record_mirror(&mut self, provider: &str, ref_name: &str, sha: &str) {
        let entry = self
            .mirrors_delivered
            .entry(provider.to_string())
            .or_default();
        entry.insert(ref_name.to_string(), sha.to_string());
    }
}
/// Distribution provider. New providers are added here and
/// matched in [`parse_mirror_provider`] and
/// [`mirror_support_status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorProvider {
    Github,
    Gitee,
    Gitlab,
    Codeberg,
}
/// Support status of a provider. Only `github` and `gitee` are
/// in the supported set for v0.5; the others stay discoverable
/// through [`parse_mirror_provider`] but [`plan_mirror`] refuses
/// to schedule a push to a planned provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MirrorSupportStatus {
    Supported,
    Planned,
}
/// One configured mirror from the manifest. The remote name
/// is the git remote alias the local working tree uses to
/// publish to the mirror; Forge does not rewrite or store
/// remote URLs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MirrorConfigEntry {
    pub provider: MirrorProvider,
    pub enabled: bool,
    pub remote_name: String,
}
/// Validated distribution config parsed from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DistributionConfig {
    pub primary: Option<MirrorConfigEntry>,
    pub mirrors: Vec<MirrorConfigEntry>,
}
impl DistributionConfig {
    /// Parse the raw [`DistributionMeta`] into a typed
    /// configuration. The first mirror is treated as the
    /// fallback primary when no explicit primary is set so a
    /// manifest that lists only `mirrors` is still usable.
    pub fn from_manifest_meta(meta: &DistributionMeta) -> Result<Self, ForgeError> {
        let mut mirrors: Vec<MirrorConfigEntry> = Vec::new();
        for (index, entry) in meta.mirrors.iter().enumerate() {
            let (provider_raw, enabled) = match entry {
                MirrorEntry::Name(name) => (name.clone(), true),
                MirrorEntry::Detailed { provider, enabled } => {
                    (provider.clone(), enabled.unwrap_or(true))
                }
            };
            let provider = parse_mirror_provider(&provider_raw)?;
            let remote_name = format!("mirror-{}", mirror_provider_label(provider));
            let remote_name = if index == 0 {
                format!("mirror-{}", mirror_provider_label(provider))
            } else {
                remote_name
            };
            if mirrors.iter().any(|m| m.provider == provider && m.enabled) {
                return Err(ForgeError::DistributionInvalid {
                    reason: format!(
                        "duplicate enabled mirror for provider `{}`; configure each provider at most once",
                        mirror_provider_label(provider)
                    ),
                });
            }
            mirrors.push(MirrorConfigEntry {
                provider,
                enabled,
                remote_name,
            });
        }
        let primary = match meta.primary.as_deref() {
            Some(name) => {
                let provider = parse_mirror_provider(name)?;
                Some(MirrorConfigEntry {
                    provider,
                    enabled: true,
                    remote_name: PRIMARY_REMOTE.to_string(),
                })
            }
            None => None,
        };
        if primary.is_none() && mirrors.is_empty() {
            return Err(ForgeError::DistributionInvalid {
                reason: "manifest distribution section is empty; declare a primary or at least one mirror"
                    .to_string(),
            });
        }
        Ok(DistributionConfig { primary, mirrors })
    }
    /// All enabled entries (primary first, then mirrors in
    /// declaration order).
    pub fn enabled_targets(&self) -> Vec<&MirrorConfigEntry> {
        let mut out: Vec<&MirrorConfigEntry> = Vec::new();
        if let Some(primary) = &self.primary {
            if primary.enabled {
                out.push(primary);
            }
        }
        for mirror in &self.mirrors {
            if mirror.enabled {
                out.push(mirror);
            }
        }
        out
    }
}
/// Aggregate mirror report. The report is the typed outcome
/// the CLI and MCP transport render; Core owns every field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MirrorReport {
    pub contract: String,
    pub project_id: String,
    pub primary: Option<String>,
    pub refs: Vec<String>,
    pub dry_run: bool,
    pub retry_failed: bool,
    pub state_path: String,
    pub outcomes: Vec<MirrorRemoteOutcome>,
    pub note: String,
    /// Aggregate health. The CLI and MCP transport render this
    /// field directly so a caller can read the verdict without
    /// re-evaluating the per-remote outcomes.
    pub healthy: bool,
}
impl MirrorReport {
    /// `true` when every enabled remote is in a successful
    /// state (`delivered` or `skipped` from a retry). A run
    /// whose primary is `failed` or `diverged`, or whose any
    /// enabled mirror is `failed` or `diverged`, is not
    /// healthy. A `disabled` mirror outcome is always
    /// non-fatal because the configuration opted out of
    /// writing. A dry run is reported as healthy because no
    /// remote was contacted.
    pub fn healthy(&self) -> bool {
        self.healthy_check()
    }
    pub(super) fn healthy_check(&self) -> bool {
        if self.dry_run {
            return true;
        }
        for outcome in &self.outcomes {
            match outcome.status.as_str() {
                "delivered" | "skipped" | "disabled" => continue,
                _ => return false,
            }
        }
        if self.outcomes.is_empty() {
            return false;
        }
        true
    }
}
/// Mirror execution request. The CLI and the MCP transport
/// build this from their input args; Core owns the validation
/// and the per-remote execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MirrorRequest {
    pub project_id: String,
    pub refs: Vec<String>,
    pub confirm: bool,
    pub dry_run: bool,
    pub retry_failed: bool,
}
impl MirrorRequest {
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.project_id.trim().is_empty() {
            return Err(ForgeError::DistributionInvalid {
                reason: "project id must not be empty".to_string(),
            });
        }
        if self.refs.is_empty() {
            return Err(ForgeError::DistributionInvalid {
                reason: "at least one ref is required; pass --ref <name>".to_string(),
            });
        }
        for r in &self.refs {
            if r.trim().is_empty() {
                return Err(ForgeError::DistributionInvalid {
                    reason: "ref name must not be empty".to_string(),
                });
            }
            if r.starts_with('-') {
                return Err(ForgeError::DistributionInvalid {
                    reason: format!(
                        "ref name `{r}` starts with a dash; refs are treated as data, not CLI options"
                    ),
                });
            }
            if r.contains(' ') || r.contains('\n') || r.contains('\t') {
                return Err(ForgeError::DistributionInvalid {
                    reason: format!("ref name `{r}` contains whitespace; refusing"),
                });
            }
        }
        if !self.confirm && !self.dry_run {
            return Err(ForgeError::DistributionInvalid {
                reason: "distribution to a primary or mirror requires --confirm; refusing implicit remote write"
                    .to_string(),
            });
        }
        Ok(())
    }
}
pub(super) enum PushResult {
    Delivered(String),
    Diverged(String),
    Unauthorized(String),
    Failed(String),
}

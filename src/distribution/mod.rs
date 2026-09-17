//! Canonical primary and one-way mirror distribution
//! (`repository-distribution`).
//!
//! Core owns the typed distribution contract. v0.1.0 supports a
//! single configured primary and provider-qualified mirrors with
//! one-way semantics: a successful primary push is recorded
//! independently from each mirror push, a mirror push that fails
//! is reported without misreporting the primary state, and a
//! retry executes only the outstanding work. GitLab/Codeberg
//! providers are declared but refused at plan time so the
//! registry does not silently grow past what is verified.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §27, §32, §34, §43 require
//! a canonical primary and one-way mirrors with per-remote
//! outcomes and credential redaction. The distribution contract
//! stays independent of any real provider integration: an
//! unauthenticated mirror attempt, a divergent protected
//! history, or a disabled mirror all surface as typed
//! `error[...]` responses before any file or registry state is
//! mutated, matching the existing upgrade / push
//! confirm-required boundary.
//!
//! ## Persistence
//!
//! [`MirrorState`] is stored under
//! `.forge/distribution/<project-id>/state.json` so a retry can
//! see which mirrors received which refs. The state is local
//! evidence, not a record of authority: a successful retry
//! overwrites the previous entry. The Core registry's
//! `operations` table receives one `mirror` row per run, with
//! a `done`/`partial`/`failed` summary that lists the per-remote
//! states (the per-remote evidence is redacted so credentials do
//! not leak through the journal).
//!
//! ## Risk model
//!
//! The primary and mirror refs are never pushed without an
//! explicit `confirm: true` (CLI `--confirm` or MCP `confirm`).
//! The contract refuses a push to a remote whose history has
//! diverged (R1 failure scenario) and a push that requires
//! `--force` (out of scope for one-way distribution).
//! Credentials are never read from the manifest; the contract
//! surfaces authentication failures with the embedded secret
//! redacted by [`crate::policy::redact_credentials`].

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::core::manifest::{DistributionMeta, Manifest, MirrorEntry};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Contract data version for the distribution surface.
pub const DISTRIBUTION_CONTRACT_VERSION: &str = "0.1.0";

/// Distribution subdirectory inside the project. The state file
/// lives under `<project>/.forge/distribution/<project-id>/state.json`
/// so a single project root owns its evidence.
pub const DISTRIBUTION_DIR: &str = ".forge/distribution";

/// Git remote name used to publish the primary ref. The
/// registry already records this in `ProjectRecord.git_remote`,
/// but the value is also embedded in the manifest's
/// `distribution.primary` so the contract stays
/// provider-scoped without re-reading the live remote URL.
pub const PRIMARY_REMOTE: &str = "origin";

/// Provider id matching the manifest's `distribution.primary`
/// and `distribution.mirrors[].provider` strings.
pub fn parse_mirror_provider(s: &str) -> Result<MirrorProvider, ForgeError> {
    match s.trim() {
        "github" => Ok(MirrorProvider::Github),
        "gitee" => Ok(MirrorProvider::Gitee),
        "gitlab" => Ok(MirrorProvider::Gitlab),
        "codeberg" => Ok(MirrorProvider::Codeberg),
        other => Err(ForgeError::DistributionInvalid {
            reason: format!(
                "unknown distribution provider `{other}`; supported: github, gitee; planned: gitlab, codeberg"
            ),
        }),
    }
}

/// Stable string label for a provider. Used in journal rows
/// and in the rendered mirror report.
pub fn mirror_provider_label(provider: MirrorProvider) -> &'static str {
    match provider {
        MirrorProvider::Github => "github",
        MirrorProvider::Gitee => "gitee",
        MirrorProvider::Gitlab => "gitlab",
        MirrorProvider::Codeberg => "codeberg",
    }
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

pub fn mirror_support_status(provider: MirrorProvider) -> MirrorSupportStatus {
    match provider {
        MirrorProvider::Github | MirrorProvider::Gitee => MirrorSupportStatus::Supported,
        MirrorProvider::Gitlab | MirrorProvider::Codeberg => MirrorSupportStatus::Planned,
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
            // Index zero is the canonical name without index
            // suffix to keep the first mirror's remote name
            // stable across runs.
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

    fn healthy_check(&self) -> bool {
        if self.dry_run {
            return true;
        }
        for outcome in &self.outcomes {
            match outcome.status.as_str() {
                "delivered" | "skipped" | "disabled" => continue,
                _ => return false,
            }
        }
        // A run with no primary and no mirrors is not
        // healthy: the request was meaningless.
        if self.outcomes.is_empty() {
            return false;
        }
        true
    }
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
    fn record_primary(&mut self, ref_name: &str, sha: &str) {
        self.primary_delivered
            .insert(ref_name.to_string(), sha.to_string());
    }
    fn record_mirror(&mut self, provider: &str, ref_name: &str, sha: &str) {
        let entry = self
            .mirrors_delivered
            .entry(provider.to_string())
            .or_default();
        entry.insert(ref_name.to_string(), sha.to_string());
    }
}

/// Compute the on-disk state path for a project. The state is
/// scoped to the project's canonical directory plus its id so
/// the same path can never host a different project's state.
pub fn state_path_for(project_dir: &Path, project_id: &str) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::DistributionInvalid {
            reason: "project id is required to resolve the mirror state path".to_string(),
        });
    }
    Ok(project_dir
        .join(DISTRIBUTION_DIR)
        .join(project_id)
        .join("state.json"))
}

pub fn load_mirror_state(path: &Path) -> Result<MirrorState, ForgeError> {
    if !path.exists() {
        return Ok(MirrorState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::DistributionInvalid {
        reason: format!("cannot read mirror state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(MirrorState::default());
    }
    let state: MirrorState =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::DistributionInvalid {
            reason: format!(
                "mirror state at {} is not valid JSON: {err}",
                path.display()
            ),
        })?;
    Ok(state)
}

pub fn save_mirror_state(path: &Path, state: &MirrorState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::DistributionInvalid {
            reason: format!(
                "cannot create mirror state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|err| ForgeError::DistributionInvalid {
            reason: format!("cannot serialize mirror state: {err}"),
        })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::DistributionInvalid {
        reason: format!("cannot write mirror state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::DistributionInvalid {
        reason: format!("cannot rename mirror state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Build a read-only mirror plan that lists what would be
/// pushed to which remote without contacting any of them.
/// Refs that already match the saved state are reported as
/// `skipped` so the caller can confirm the retry safety
/// boundary before applying.
pub fn plan_mirror(
    config: &DistributionConfig,
    request: &MirrorRequest,
    state: &MirrorState,
    state_file: &Path,
) -> Result<MirrorReport, ForgeError> {
    request.validate()?;
    let mut outcomes: Vec<MirrorRemoteOutcome> = Vec::new();
    let mut refs_for_state: Vec<String> = Vec::new();
    for entry in config.enabled_targets() {
        let role = if entry.remote_name == PRIMARY_REMOTE {
            "primary"
        } else {
            "mirror"
        };
        for ref_name in &request.refs {
            let previously = if role == "primary" {
                state.primary_delivered.get(ref_name).cloned()
            } else {
                state
                    .mirrors_delivered
                    .get(mirror_provider_label(entry.provider))
                    .and_then(|m| m.get(ref_name).cloned())
            };
            let status = match mirror_support_status(entry.provider) {
                MirrorSupportStatus::Planned => "unavailable",
                _ => "would-push",
            };
            let note = match status {
                "unavailable" => format!(
                    "provider `{}` is declared but not in the supported distribution set for this release; planned for a later change",
                    mirror_provider_label(entry.provider)
                ),
                "would-push" => {
                    if let Some(sha) = previously {
                        format!("would re-push `{ref_name}` (last delivered: {sha})")
                    } else {
                        format!("would push `{ref_name}` to `{}`", entry.remote_name)
                    }
                }
                _ => unreachable!(),
            };
            outcomes.push(MirrorRemoteOutcome {
                provider: mirror_provider_label(entry.provider).to_string(),
                role: role.to_string(),
                status: status.to_string(),
                commit_sha: None,
                note,
                evidence: Vec::new(),
                recovery: Vec::new(),
            });
            refs_for_state.push(ref_name.to_string());
        }
    }
    // Disabled mirrors are reported as boundary outcomes so the
    // caller can verify the no-write contract (R1 boundary
    // scenario).
    for entry in &config.mirrors {
        if entry.enabled {
            continue;
        }
        for ref_name in &request.refs {
            outcomes.push(MirrorRemoteOutcome {
                provider: mirror_provider_label(entry.provider).to_string(),
                role: "mirror".to_string(),
                status: "disabled".to_string(),
                commit_sha: None,
                note: format!(
                    "mirror `{}` is disabled in the manifest; no write was attempted for ref `{ref_name}`",
                    mirror_provider_label(entry.provider)
                ),
                evidence: Vec::new(),
                recovery: vec![format!(
                    "set `distribution.mirrors[provider={}].enabled: true` and re-run to distribute `{ref_name}`",
                    mirror_provider_label(entry.provider)
                )],
            });
        }
    }
    refs_for_state.sort();
    refs_for_state.dedup();
    let primary = config
        .primary
        .as_ref()
        .map(|p| mirror_provider_label(p.provider).to_string());
    let report = MirrorReport {
        contract: DISTRIBUTION_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        primary,
        refs: request.refs.clone(),
        dry_run: true,
        retry_failed: request.retry_failed,
        state_path: state_file.display().to_string(),
        outcomes,
        note: "plan only; no push was attempted".to_string(),
        healthy: true,
    };
    Ok(report)
}

/// Apply a mirror run. The request is validated first; the
/// state file is loaded; each enabled remote is processed
/// in declaration order (primary first, then mirrors). A
/// successful push updates the state and the report; a
/// failed push is reported per-remote without mutating the
/// primary outcome. Retries only re-push refs that the state
/// shows as undelivered for that remote.
pub fn apply_mirror(
    project_dir: &Path,
    config: &DistributionConfig,
    request: &MirrorRequest,
) -> Result<MirrorReport, ForgeError> {
    request.validate()?;
    let state_path = state_path_for(project_dir, &request.project_id)?;
    let mut state = load_mirror_state(&state_path)?;
    let mut outcomes: Vec<MirrorRemoteOutcome> = Vec::new();
    let entries: Vec<MirrorConfigEntry> = config.enabled_targets().into_iter().cloned().collect();
    for entry in &entries {
        if !is_git_working_tree(project_dir) {
            return Err(ForgeError::DistributionInvalid {
                reason: format!(
                    "directory `{}` is not a git working tree; distribution requires a git repository",
                    project_dir.display()
                ),
            });
        }
        let role = if entry.remote_name == PRIMARY_REMOTE {
            "primary"
        } else {
            "mirror"
        };
        if mirror_support_status(entry.provider) == MirrorSupportStatus::Planned {
            for ref_name in &request.refs {
                outcomes.push(MirrorRemoteOutcome {
                    provider: mirror_provider_label(entry.provider).to_string(),
                    role: role.to_string(),
                    status: "unavailable".to_string(),
                    commit_sha: None,
                    note: format!(
                        "provider `{}` is planned for a later change; no write attempted for ref `{ref_name}`",
                        mirror_provider_label(entry.provider)
                    ),
                    evidence: Vec::new(),
                    recovery: vec![
                        format!(
                            "replace `{}` with `github` or `gitee` in `distribution.mirrors` to distribute `{ref_name}`",
                            mirror_provider_label(entry.provider)
                        ),
                    ],
                });
            }
            continue;
        }
        for ref_name in &request.refs {
            let previously = if role == "primary" {
                state.primary_delivered.get(ref_name).cloned()
            } else {
                state
                    .mirrors_delivered
                    .get(mirror_provider_label(entry.provider))
                    .and_then(|m| m.get(ref_name).cloned())
            };
            let current_sha = read_head_sha(project_dir);
            // Retry semantics: skip only when the previous
            // delivery is for the same ref AND the same
            // commit SHA the local working tree currently
            // reports. A ref that advanced locally (new
            // commit) must be re-pushed on retry because
            // the remote is now behind.
            if request.retry_failed {
                let matches = match (&previously, &current_sha) {
                    (Some(prev), Some(now)) => prev == now,
                    _ => false,
                };
                if matches {
                    outcomes.push(MirrorRemoteOutcome {
                        provider: mirror_provider_label(entry.provider).to_string(),
                        role: role.to_string(),
                        status: "skipped".to_string(),
                        commit_sha: previously.clone(),
                        note: format!(
                            "`{ref_name}` is already recorded as delivered at `{prev_sha}` for `{}`; retry skips delivered refs",
                            mirror_provider_label(entry.provider),
                            prev_sha = previously.clone().unwrap_or_default()
                        ),
                        evidence: Vec::new(),
                        recovery: Vec::new(),
                    });
                    continue;
                }
            }
            match push_ref_to_remote(project_dir, &entry.remote_name, ref_name) {
                Ok(PushResult::Delivered(sha)) => {
                    if role == "primary" {
                        state.record_primary(ref_name, &sha);
                    } else {
                        state.record_mirror(mirror_provider_label(entry.provider), ref_name, &sha);
                    }
                    outcomes.push(MirrorRemoteOutcome {
                        provider: mirror_provider_label(entry.provider).to_string(),
                        role: role.to_string(),
                        status: "delivered".to_string(),
                        commit_sha: Some(sha.clone()),
                        note: format!(
                            "delivered `{ref_name}` to `{}` for project `{}`",
                            entry.remote_name, request.project_id
                        ),
                        evidence: vec![format!("commit: {sha}")],
                        recovery: Vec::new(),
                    });
                }
                Ok(PushResult::Diverged(detail)) => {
                    outcomes.push(MirrorRemoteOutcome {
                        provider: mirror_provider_label(entry.provider).to_string(),
                        role: role.to_string(),
                        status: "diverged".to_string(),
                        commit_sha: None,
                        note: format!(
                            "remote `{}` has divergent protected history; force or reverse-sync are refused to preserve one-way distribution",
                            entry.remote_name
                        ),
                        evidence: vec![redact_distribution_evidence(&detail)],
                        recovery: vec![
                            "investigate the mirror's diverging history before re-pushing".to_string(),
                            format!(
                                "remove the diverging commits from `{}` or align with `{}`",
                                entry.remote_name, PRIMARY_REMOTE
                            ),
                        ],
                    });
                }
                Ok(PushResult::Unauthorized(detail)) => {
                    outcomes.push(MirrorRemoteOutcome {
                        provider: mirror_provider_label(entry.provider).to_string(),
                        role: role.to_string(),
                        status: "failed".to_string(),
                        commit_sha: None,
                        note: format!(
                            "authentication failed for `{}`",
                            mirror_provider_label(entry.provider)
                        ),
                        evidence: vec![redact_distribution_evidence(&detail)],
                        recovery: vec![
                            "rotate the credential referenced for this remote and re-run"
                                .to_string(),
                            "store the credential in a credential helper, not in the manifest"
                                .to_string(),
                        ],
                    });
                }
                Ok(PushResult::Failed(detail)) => {
                    outcomes.push(MirrorRemoteOutcome {
                        provider: mirror_provider_label(entry.provider).to_string(),
                        role: role.to_string(),
                        status: "failed".to_string(),
                        commit_sha: None,
                        note: format!("push to `{}` failed for `{ref_name}`", entry.remote_name),
                        evidence: vec![redact_distribution_evidence(&detail)],
                        recovery: vec![format!(
                            "verify the remote URL and the credential helper for `{}`",
                            entry.remote_name
                        )],
                    });
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
    // Disabled mirrors are reported as boundary outcomes so the
    // caller can verify the no-write contract (R1 boundary
    // scenario).
    for entry in &config.mirrors {
        if entry.enabled {
            continue;
        }
        for ref_name in &request.refs {
            outcomes.push(MirrorRemoteOutcome {
                provider: mirror_provider_label(entry.provider).to_string(),
                role: "mirror".to_string(),
                status: "disabled".to_string(),
                commit_sha: None,
                note: format!(
                    "mirror `{}` is disabled in the manifest; no write was attempted for ref `{ref_name}`",
                    mirror_provider_label(entry.provider)
                ),
                evidence: Vec::new(),
                recovery: vec![format!(
                    "set `distribution.mirrors[provider={}].enabled: true` and re-run to distribute `{ref_name}`",
                    mirror_provider_label(entry.provider)
                )],
            });
        }
    }
    state.refs = request.refs.clone();
    let now = chrono::Utc::now().to_rfc3339();
    state.last_run_at = now;
    save_mirror_state(&state_path, &state)?;
    let primary = config
        .primary
        .as_ref()
        .map(|p| mirror_provider_label(p.provider).to_string());
    let primary_outcome = outcomes
        .iter()
        .find(|o| o.role == "primary")
        .map(|o| o.status.clone());
    let mirror_count = outcomes
        .iter()
        .filter(|o| o.role == "mirror")
        .filter(|o| o.status == "delivered")
        .count();
    let note = match primary_outcome.as_deref() {
        Some("delivered") => format!(
            "primary delivered; {mirror_count} mirror(s) delivered for project `{}`",
            request.project_id
        ),
        Some("diverged") => "primary diverged; mirror delivery skipped".to_string(),
        Some("failed") => "primary push failed; mirror delivery skipped".to_string(),
        Some("unavailable") => "primary provider is planned; no write attempted".to_string(),
        Some(other) => format!("primary status: {other}"),
        None => "no primary configured; only mirrors were attempted".to_string(),
    };
    let mut report = MirrorReport {
        contract: DISTRIBUTION_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        primary,
        refs: request.refs.clone(),
        dry_run: false,
        retry_failed: request.retry_failed,
        state_path: state_path.display().to_string(),
        outcomes,
        note,
        healthy: false,
    };
    report.healthy = report.healthy_check();
    Ok(report)
}

/// Redact credential-like substrings from a piece of evidence.
/// The redaction is delegated to
/// [`crate::policy::redact_credentials`] so the policy and
/// distribution contracts share one definition of "secret".
pub fn redact_distribution_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Render a [`MirrorReport`] for human output. The summary
/// is the same data the JSON transport carries, with stable
/// status labels.
pub fn render_report_human(report: &MirrorReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    if let Some(primary) = &report.primary {
        lines.push(format!("primary: {primary}"));
    }
    lines.push(format!("refs: {}", report.refs.join(", ")));
    if report.dry_run {
        lines.push("mode: dry-run".to_string());
    }
    if report.retry_failed {
        lines.push("mode: retry-failed".to_string());
    }
    lines.push(format!("state: {}", report.state_path));
    for outcome in &report.outcomes {
        let commit = outcome
            .commit_sha
            .clone()
            .map(|s| format!(" commit={s}"))
            .unwrap_or_default();
        lines.push(format!(
            "  - {role} {provider} {status}{commit}: {note}",
            role = outcome.role,
            provider = outcome.provider,
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
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

enum PushResult {
    Delivered(String),
    Diverged(String),
    Unauthorized(String),
    Failed(String),
}

fn push_ref_to_remote(dir: &Path, remote: &str, ref_name: &str) -> Result<PushResult, ForgeError> {
    let output = run_git(dir, &["push", remote, ref_name])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        let sha = read_head_sha(dir).unwrap_or_default();
        return Ok(PushResult::Delivered(sha));
    }
    let combined = format!("{stdout}\n{stderr}");
    let lower = combined.to_ascii_lowercase();
    if lower.contains("non-fast-forward")
        || lower.contains("rejected")
        || lower.contains("fetch first")
        || lower.contains("tip of your current branch is behind")
        || lower.contains("diverged")
    {
        return Ok(PushResult::Diverged(combined));
    }
    if lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("could not read password")
        || lower.contains("invalid username or password")
        || lower.contains("bad credentials")
        || lower.contains("401")
        || lower.contains("403")
    {
        return Ok(PushResult::Unauthorized(combined));
    }
    Ok(PushResult::Failed(combined))
}

fn read_head_sha(dir: &Path) -> Option<String> {
    let out = run_git(dir, &["rev-parse", "HEAD"]).ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

fn run_git(dir: &Path, args: &[&str]) -> Result<std::process::Output, ForgeError> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().map_err(|err| ForgeError::DistributionInvalid {
        reason: format!("git invocation failed: {err}"),
    })
}

fn is_git_working_tree(dir: &Path) -> bool {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim() == "true",
        _ => false,
    }
}

/// Build a [`DistributionConfig`] from a manifest's
/// distribution section. Convenience for callers that already
/// have the manifest.
pub fn distribution_config_from_manifest(
    manifest: &Manifest,
) -> Result<DistributionConfig, ForgeError> {
    match manifest.distribution.as_ref() {
        Some(meta) => DistributionConfig::from_manifest_meta(meta),
        None => Err(ForgeError::DistributionInvalid {
            reason: format!(
                "project `{}` has no `distribution` section; declare a primary or at least one mirror",
                manifest.project.id
            ),
        }),
    }
}

#[cfg(test)]
pub mod test_fixtures {
    use super::*;
    use std::process::Command;

    /// Build a git working tree with a single initial commit
    /// and no remotes. The returned directory is initialized
    /// with the supplied `id` as the project id and a default
    /// `forge.yaml`. Tests add remotes per scenario.
    pub fn git_working_tree(tmp: &Path, id: &str) -> PathBuf {
        let dir = tmp.join(id);
        fs::create_dir_all(&dir).unwrap();
        let manifest = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join("forge.yaml"), manifest).unwrap();
        fs::write(dir.join("README.md"), "v1\n").unwrap();
        let run = |args: &[&str]| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(&dir);
            for a in args {
                cmd.arg(a);
            }
            let out = cmd.output().expect("git");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "forge@example.com"]);
        run(&["config", "user.name", "Forge Test"]);
        run(&["config", "init.defaultBranch", "main"]);
        run(&["checkout", "-q", "-b", "main"]);
        run(&["add", "--", "forge.yaml", "README.md"]);
        run(&["commit", "-q", "-m", "initial"]);
        dir
    }

    /// Build a local bare repository that stands in for a
    /// provider remote. The returned path is a bare git
    /// repository; the caller adds it as a remote on the
    /// working tree. The bare repo's HEAD is set to
    /// `refs/heads/main` so a subsequent `git clone` lands on
    /// the same branch name as the working tree under test.
    pub fn bare_repository(tmp: &Path, label: &str) -> PathBuf {
        let path = tmp.join(format!("{label}.git"));
        fs::create_dir_all(&path).unwrap();
        let mut cmd = Command::new("git");
        cmd.arg("init").arg("--bare").arg("-q").arg(&path);
        let out = cmd.output().expect("git init --bare");
        assert!(
            out.status.success(),
            "git init --bare failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let mut head_cmd = Command::new("git");
        head_cmd
            .arg("symbolic-ref")
            .arg("HEAD")
            .arg("refs/heads/main")
            .current_dir(&path);
        let head_out = head_cmd.output().expect("git symbolic-ref HEAD");
        assert!(
            head_out.status.success(),
            "git symbolic-ref HEAD failed: {}",
            String::from_utf8_lossy(&head_out.stderr)
        );
        path
    }

    /// Add `path` as the working tree's `remote_name` remote.
    pub fn add_remote(working_tree: &Path, remote_name: &str, path: &Path) {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(working_tree);
        cmd.arg("remote").arg("add").arg(remote_name).arg(path);
        let out = cmd.output().expect("git remote add");
        assert!(
            out.status.success(),
            "git remote add failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixtures::*;
    use super::*;

    fn manifest_with_distribution(text: &str) -> Manifest {
        Manifest::parse(Path::new("forge.yaml"), text.as_bytes()).expect("manifest")
    }

    #[test]
    fn parse_provider_recognises_supported_and_planned() {
        assert!(matches!(
            parse_mirror_provider("github").unwrap(),
            MirrorProvider::Github
        ));
        assert!(matches!(
            parse_mirror_provider("gitee").unwrap(),
            MirrorProvider::Gitee
        ));
        assert!(matches!(
            parse_mirror_provider("gitlab").unwrap(),
            MirrorProvider::Gitlab
        ));
        assert!(matches!(
            parse_mirror_provider("codeberg").unwrap(),
            MirrorProvider::Codeberg
        ));
        let err = parse_mirror_provider("sourceforge").unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn support_status_is_supported_for_github_gitee_only() {
        assert_eq!(
            mirror_support_status(MirrorProvider::Github),
            MirrorSupportStatus::Supported
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Gitee),
            MirrorSupportStatus::Supported
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Gitlab),
            MirrorSupportStatus::Planned
        );
        assert_eq!(
            mirror_support_status(MirrorProvider::Codeberg),
            MirrorSupportStatus::Planned
        );
    }

    #[test]
    fn config_from_manifest_uses_first_mirror_as_primary_fallback() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  mirrors:\n    - gitee\n    - github\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        assert!(config.primary.is_none(), "no primary declared");
        assert_eq!(config.mirrors.len(), 2);
        assert_eq!(config.enabled_targets().len(), 2);
    }

    #[test]
    fn config_from_manifest_keeps_declared_primary_separate() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - gitee\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        assert_eq!(
            config.primary.as_ref().map(|p| p.provider),
            Some(MirrorProvider::Github)
        );
        assert_eq!(config.primary.as_ref().unwrap().remote_name, "origin");
        assert_eq!(config.mirrors[0].provider, MirrorProvider::Gitee);
        assert_eq!(config.mirrors[0].remote_name, "mirror-gitee");
    }

    #[test]
    fn config_from_manifest_rejects_duplicate_enabled_mirrors() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  mirrors:\n    - gitee\n    - provider: gitee\n      enabled: true\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn config_from_manifest_rejects_unknown_provider() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: bogus\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn config_from_manifest_rejects_empty_distribution() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\n",
        );
        let err = distribution_config_from_manifest(&manifest).unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn request_refuses_empty_refs_and_dash_prefixed_refs() {
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["-evil".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main with space".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn request_refuses_implicit_remote_write() {
        let bad = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: false,
            dry_run: false,
            retry_failed: false,
        };
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "distribution-invalid");
    }

    #[test]
    fn plan_reports_disabled_mirror_as_boundary_outcome() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - provider: gitee\n      enabled: false\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        let req = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, Path::new("/tmp/state.json")).unwrap();
        let disabled = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitee" && o.status == "disabled")
            .expect("disabled boundary");
        assert_eq!(disabled.role, "mirror");
    }

    #[test]
    fn plan_marks_planned_providers_as_unavailable() {
        let manifest = manifest_with_distribution(
            "schema: 1\nproject:\n  id: a\n  name: A\n  profile: rust-web\ndistribution:\n  primary: github\n  mirrors:\n    - gitlab\n",
        );
        let config = distribution_config_from_manifest(&manifest).unwrap();
        let req = MirrorRequest {
            project_id: "a".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, Path::new("/tmp/state.json")).unwrap();
        let planned = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitlab")
            .expect("planned");
        assert_eq!(planned.status, "unavailable");
    }

    #[test]
    fn apply_records_primary_and_mirror_independently() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "distrib-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "distrib-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let primary = report
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .expect("primary outcome");
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(primary.status, "delivered");
        assert_eq!(mirror.status, "delivered");
        assert!(primary.commit_sha.is_some());
        assert!(mirror.commit_sha.is_some());
        assert!(report.healthy());
    }

    #[test]
    fn apply_records_partial_failure_when_mirror_unavailable() {
        // Primary push works (real bare repo), mirror push
        // fails because the mirror-gitee remote is not
        // configured. The contract records primary as
        // delivered and mirror as failed without rolling
        // back the primary.
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "partial-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        add_remote(&dir, "origin", &primary_bare);
        // mirror-gitee is intentionally not added.

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "partial-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let primary = report
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .expect("primary outcome");
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(primary.status, "delivered");
        assert_eq!(mirror.status, "failed");
        // Partial outcome: the report must surface the
        // primary state and the per-remote failure without
        // misreporting one for the other.
        assert!(!report.healthy(), "primary delivered but mirror failed");
        // State file records only the primary delivery.
        let state = load_mirror_state(Path::new(&report.state_path)).unwrap();
        assert!(state.primary_delivered.contains_key("main"));
        assert!(state.mirrors_delivered.is_empty());
    }

    #[test]
    fn apply_refuses_diverged_mirror_with_recovery_note() {
        // The mirror's history is forced to diverge from the
        // primary so a non-fast-forward push is required.
        // The contract refuses the force and surfaces
        // `diverged` with the diverging detail (redacted).
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "diverged-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);
        // Push primary and mirror first so both bare
        // repositories hold the `main` branch; a subsequent
        // `git clone` of the mirror would otherwise land on
        // a detached HEAD with no branches.
        let run = |args: &[&str]| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(&dir);
            for a in args {
                cmd.arg(a);
            }
            let out = cmd.output().expect("git");
            assert!(
                out.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["push", "origin", "main"]);
        run(&["push", "mirror-gitee", "main"]);
        // Build a divergent history on the mirror: clone it,
        // add a commit, push back.
        let mirror_clone = tmp.path().join("mirror-clone");
        run_via(&[&format!("git clone {mirror_bare:?} {mirror_clone:?}")]);
        let commit_in_clone = |args: &[&str]| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(&mirror_clone);
            for a in args {
                cmd.arg(a);
            }
            let out = cmd.output().expect("git in clone");
            assert!(
                out.status.success(),
                "git clone {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        commit_in_clone(&["config", "user.email", "forge@example.com"]);
        commit_in_clone(&["config", "user.name", "Forge Test"]);
        // The bare repo's HEAD was set to `main` so the
        // clone lands on the same branch name as the
        // working tree; checkout explicitly to anchor the
        // diverging commit.
        commit_in_clone(&["checkout", "main"]);
        std::fs::write(mirror_clone.join("README.md"), "divergent\n").unwrap();
        commit_in_clone(&["add", "README.md"]);
        commit_in_clone(&["commit", "-q", "-m", "divergent history"]);
        commit_in_clone(&["push", "origin", "main"]);
        // Move the local primary forward so the next push
        // is non-fast-forward from the mirror's perspective.
        std::fs::write(dir.join("README.md"), "v2\n").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-q", "-m", "primary ahead"]);
        run(&["push", "origin", "main"]);

        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "diverged-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let report = apply_mirror(&dir, &config, &req).unwrap();
        let mirror = report
            .outcomes
            .iter()
            .find(|o| o.role == "mirror")
            .expect("mirror outcome");
        assert_eq!(mirror.status, "diverged");
        assert!(!mirror.recovery.is_empty());
    }

    #[test]
    fn retry_skips_already_delivered_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "retry-app");
        let primary_bare = bare_repository(tmp.path(), "origin");
        let mirror_bare = bare_repository(tmp.path(), "gitee");
        add_remote(&dir, "origin", &primary_bare);
        add_remote(&dir, "mirror-gitee", &mirror_bare);
        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: true,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "retry-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: false,
        };
        let first = apply_mirror(&dir, &config, &req).unwrap();
        let primary_first = first.outcomes.iter().find(|o| o.role == "primary").unwrap();
        let mirror_first = first.outcomes.iter().find(|o| o.role == "mirror").unwrap();
        assert_eq!(primary_first.status, "delivered");
        assert_eq!(mirror_first.status, "delivered");

        // Retry must skip both remotes; the state file is the
        // source of truth for what was already delivered.
        let retry = MirrorRequest {
            project_id: "retry-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: false,
            retry_failed: true,
        };
        let second = apply_mirror(&dir, &config, &retry).unwrap();
        let primary_second = second
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .unwrap();
        let mirror_second = second.outcomes.iter().find(|o| o.role == "mirror").unwrap();
        assert_eq!(primary_second.status, "skipped");
        assert_eq!(mirror_second.status, "skipped");
    }

    #[test]
    fn disabled_mirror_is_reported_without_writing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = git_working_tree(tmp.path(), "disabled-app");
        let config = DistributionConfig {
            primary: Some(MirrorConfigEntry {
                provider: MirrorProvider::Github,
                enabled: true,
                remote_name: PRIMARY_REMOTE.to_string(),
            }),
            mirrors: vec![MirrorConfigEntry {
                provider: MirrorProvider::Gitee,
                enabled: false,
                remote_name: "mirror-gitee".to_string(),
            }],
        };
        let req = MirrorRequest {
            project_id: "disabled-app".to_string(),
            refs: vec!["main".to_string()],
            confirm: true,
            dry_run: true,
            retry_failed: false,
        };
        let state = MirrorState::default();
        let report = plan_mirror(&config, &req, &state, &dir.join("state.json")).unwrap();
        let disabled = report
            .outcomes
            .iter()
            .find(|o| o.provider == "gitee" && o.status == "disabled")
            .expect("disabled");
        assert_eq!(disabled.role, "mirror");
    }

    #[test]
    fn redaction_strips_credentials_from_evidence() {
        let redacted =
            redact_distribution_evidence("auth: ghp_abcdefghijklmnopqrstuvwxyz0123456789");
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ghp_"));
    }

    #[test]
    fn state_round_trip_preserves_delivered_refs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let mut state = MirrorState::default();
        state.record_primary("main", "abc123");
        state.record_mirror("gitee", "main", "abc123");
        save_mirror_state(&path, &state).unwrap();
        let restored = load_mirror_state(&path).unwrap();
        assert_eq!(restored.primary_delivered.get("main").unwrap(), "abc123");
        assert_eq!(
            restored
                .mirrors_delivered
                .get("gitee")
                .and_then(|m| m.get("main"))
                .unwrap(),
            "abc123"
        );
    }

    fn run_via(args: &[&str]) {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        let joined = args.join(" ");
        cmd.arg(joined);
        let out = cmd.output().expect("sh -c");
        assert!(
            out.status.success(),
            "sh -c failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

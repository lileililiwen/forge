//! Per-project Studio session state (`forge-studio-session/0.1.0`).
//!
//! The session is the source of truth for the saved AppSpec
//! revision, the saved refinement revision, and the recorded
//! preview state. It is persisted as a single JSON document at
//! `<project>/.forge/studio/session.json`; the journal rows the
//! existing `operations` table records (`studio.spec.save`,
//! `studio.preview.start`, `studio.preview.stop`, `studio.refine`)
//! are the audit trail.
//!
//! All operations are bounded: validation is read-only,
//! persistence is atomic, and the journal row records the
//! exact spec/revision + the bounded and redacted detail.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::registry::Registry;

use super::spec::{validate_spec, AppSpec, APP_SPEC_CONTRACT};

/// Wire contract id for the persisted Studio session record.
pub const STUDIO_SESSION_CONTRACT: &str = "forge-studio-session/0.1.0";

/// Relative path under the project root where the session record
/// lives. The directory is created lazily by [`save_spec`].
pub const STUDIO_DIR: &str = ".forge/studio";

/// Single filename under [`STUDIO_DIR`] for the session record.
pub const SESSION_FILENAME: &str = "session.json";

/// Bound on the saved `detail` text on a journal row. Mirrors the
/// bound other transports apply so a Studio journal cannot pin a
/// host log with unbounded detail text.
pub const MAX_JOURNAL_DETAIL_CHARS: usize = 4096;

/// Bound on the per-refinement `selected_files` list.
pub const MAX_SELECTED_FILES: usize = 32;

/// Bound on a single `selected_files` entry.
pub const MAX_SELECTED_FILE_CHARS: usize = 4096;

/// One persisted Studio session record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioSession {
    pub contract: String,
    pub project_id: String,
    pub profile: String,
    pub app_schema_major: String,
    pub spec_revision: String,
    pub app_revision: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub preview: SessionPreviewState,
    /// The last saved AppSpec. Persisted next to the session
    /// metadata so a project reload restores the operator's last
    /// reviewed proposal. The JSON shape is the closed
    /// `forge-app-spec/0.1.0` payload — not the original YAML
    /// — so a future YAML drift cannot silently desync the
    /// on-disk record.
    pub spec: AppSpec,
}

/// Recorded preview state. The on-disk mirror of the
/// `forge-studio-preview/0.1.0` envelope, with the bounded log
/// captured between start and stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPreviewState {
    pub state: PreviewState,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error_code: Option<String>,
    #[serde(default)]
    pub log_tail: String,
}

/// Persisted-preview state vocabulary. Mirrors the bounded
/// `PreviewState` envelope so the on-disk record and the
/// transport envelope agree on the same closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewState {
    None,
    Starting,
    Ready,
    Failed,
    Stopping,
    Stopped,
}

impl PreviewState {
    pub fn label(self) -> &'static str {
        match self {
            PreviewState::None => "none",
            PreviewState::Starting => "starting",
            PreviewState::Ready => "ready",
            PreviewState::Failed => "failed",
            PreviewState::Stopping => "stopping",
            PreviewState::Stopped => "stopped",
        }
    }
}

/// One journal row kind emitted by the Studio Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioJournalKind {
    SpecSave,
    PreviewStart,
    PreviewStop,
    Refine,
}

impl StudioJournalKind {
    pub fn label(self) -> &'static str {
        match self {
            StudioJournalKind::SpecSave => "studio.spec.save",
            StudioJournalKind::PreviewStart => "studio.preview.start",
            StudioJournalKind::PreviewStop => "studio.preview.stop",
            StudioJournalKind::Refine => "studio.refine",
        }
    }
}

/// Persisted preview record (the bounded log cap lives here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewRecord {
    pub state: PreviewState,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error_code: Option<String>,
    #[serde(default)]
    pub log_tail: String,
}

impl Default for PreviewRecord {
    fn default() -> Self {
        Self {
            state: PreviewState::None,
            port: None,
            started_at: None,
            last_error_code: None,
            log_tail: String::new(),
        }
    }
}

impl Default for SessionPreviewState {
    fn default() -> Self {
        Self {
            state: PreviewState::None,
            port: None,
            started_at: None,
            last_error_code: None,
            log_tail: String::new(),
        }
    }
}

/// Compute the on-disk session path for a project. The directory
/// is *not* created here; persistence is responsible for the
/// atomic write.
pub fn session_path(project_root: &Path) -> PathBuf {
    project_root.join(STUDIO_DIR).join(SESSION_FILENAME)
}

/// Load the persisted session for a project. Returns `None`
/// when no session has ever been saved (a fresh project); a
/// malformed or missing file is refused as `studio-invalid-spec`
/// because the operator can no longer reason about a Studio
/// state that was not produced by this contract.
pub fn load_session(project_root: &Path) -> Result<Option<StudioSession>, ForgeError> {
    let path = session_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!(
            "could not read existing Studio session {}: {err}",
            path.display()
        ),
    })?;
    let session: StudioSession =
        serde_json::from_str(&raw).map_err(|err| ForgeError::StudioInvalidSpec {
            reason: format!(
                "existing Studio session at {} is malformed: {err}",
                path.display()
            ),
        })?;
    if session.contract != STUDIO_SESSION_CONTRACT {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "existing Studio session contract '{}' does not match expected '{}'",
                session.contract, STUDIO_SESSION_CONTRACT
            ),
        });
    }
    Ok(Some(session))
}

/// Persist a Studio session atomically. The file is written to a
/// sibling temp path and renamed into place so a concurrent
/// reader cannot observe a partial document. The session's
/// `updated_at` is set to `now` before the write.
pub fn persist_session(project_root: &Path, session: &StudioSession) -> Result<(), ForgeError> {
    let dir = project_root.join(STUDIO_DIR);
    std::fs::create_dir_all(&dir).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!("could not create Studio directory {}: {err}", dir.display()),
    })?;
    let final_path = dir.join(SESSION_FILENAME);
    let tmp_path = dir.join(format!("{SESSION_FILENAME}.tmp"));
    let serialized =
        serde_json::to_string_pretty(session).map_err(|err| ForgeError::StudioInvalidSpec {
            reason: format!("could not serialize Studio session: {err}"),
        })?;
    std::fs::write(&tmp_path, serialized).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!(
            "could not write Studio session to {}: {err}",
            tmp_path.display()
        ),
    })?;
    std::fs::rename(&tmp_path, &final_path).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!(
            "could not finalize Studio session at {}: {err}",
            final_path.display()
        ),
    })?;
    Ok(())
}

/// Save a validated AppSpec and record the journal row. The
/// `expected_revision` is the operator-supplied optimistic-
/// concurrency token: `r0` for the first save (no prior session),
/// the current `spec_revision` thereafter. A mismatch is refused
/// as `studio-revision-conflict` before any project file is
/// touched.
pub fn save_spec(
    registry: &Registry,
    project_id: &str,
    project_root: &Path,
    spec: AppSpec,
    expected_revision: &str,
) -> Result<StudioSession, ForgeError> {
    if spec.project_id != project_id {
        return Err(ForgeError::StudioProjectScope {
            reason: format!(
                "spec.project_id '{}' does not match registered project '{project_id}'",
                spec.project_id
            ),
        });
    }
    validate_spec(&spec)?;
    let now = Utc::now();
    let session = match load_session(project_root)? {
        Some(existing) => {
            if existing.spec_revision != expected_revision {
                return Err(ForgeError::StudioRevisionConflict {
                    reason: format!(
                        "expected_revision '{expected_revision}' does not match current spec_revision '{}'",
                        existing.spec_revision
                    ),
                });
            }
            if existing.app_schema_major != spec.schema_version {
                return Err(ForgeError::StudioRevisionConflict {
                    reason: format!(
                        "schema_version '{}' would not preserve the existing app_schema_major '{}'; bump the contract first",
                        spec.schema_version, existing.app_schema_major
                    ),
                });
            }
            let next_spec_rev = bump_revision(&existing.spec_revision, "spec_revision")?;
            StudioSession {
                contract: STUDIO_SESSION_CONTRACT.to_string(),
                project_id: project_id.to_string(),
                profile: spec.profile.clone(),
                app_schema_major: spec.schema_version.clone(),
                spec_revision: next_spec_rev.clone(),
                app_revision: next_spec_rev,
                created_at: existing.created_at,
                updated_at: now,
                preview: existing.preview,
                spec,
            }
        }
        None => {
            if expected_revision != "r0" {
                return Err(ForgeError::StudioRevisionConflict {
                    reason: format!(
                        "no prior Studio session exists; expected_revision must be 'r0' on the first save (got '{expected_revision}')"
                    ),
                });
            }
            StudioSession {
                contract: STUDIO_SESSION_CONTRACT.to_string(),
                project_id: project_id.to_string(),
                profile: spec.profile.clone(),
                app_schema_major: spec.schema_version.clone(),
                spec_revision: "r1".to_string(),
                app_revision: "r1".to_string(),
                created_at: now,
                updated_at: now,
                preview: SessionPreviewState::default(),
                spec,
            }
        }
    };
    persist_session(project_root, &session)?;
    let detail = bounded_detail(&format!(
        "spec_revision={} app_revision={} app_schema_major={} profile={}",
        session.spec_revision, session.app_revision, session.app_schema_major, session.profile
    ));
    record_journal(
        registry,
        StudioJournalKind::SpecSave,
        project_id,
        "done",
        &detail,
    )?;
    Ok(session)
}

/// Record a refinement request. The journal row is the only
/// durable artefact in this cycle: the agent-adapter runtime
/// hook is a follow-up (see `tasks.md` §5.3). The validation is
/// bounded and read-only; the new `app_revision` is derived from
/// the session's current `app_revision`.
pub fn record_refinement(
    registry: &Registry,
    project_root: &Path,
    expected_revision: &str,
    request_text: &str,
    selected_files: &[String],
) -> Result<StudioSession, ForgeError> {
    let mut session =
        load_session(project_root)?.ok_or_else(|| ForgeError::StudioRevisionConflict {
            reason: "no Studio session exists; save a spec before refining".to_string(),
        })?;
    if expected_revision != session.app_revision && expected_revision != session.spec_revision {
        return Err(ForgeError::StudioRevisionConflict {
            reason: format!(
                "expected_revision '{expected_revision}' does not match the current app_revision '{}' or spec_revision '{}'",
                session.app_revision, session.spec_revision
            ),
        });
    }
    if request_text.is_empty() {
        return Err(ForgeError::StudioInvalidSpec {
            reason: "refinement request must not be empty".to_string(),
        });
    }
    if redact_credentials(request_text) != request_text {
        return Err(ForgeError::StudioInvalidSpec {
            reason: "refinement request contains a credential-shaped string; refusing to journal"
                .to_string(),
        });
    }
    if selected_files.len() > MAX_SELECTED_FILES {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "selected_files contains {} entries; bound is {MAX_SELECTED_FILES}",
                selected_files.len()
            ),
        });
    }
    for entry in selected_files {
        if entry.len() > MAX_SELECTED_FILE_CHARS {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "selected_files entry is {} chars; bound is {MAX_SELECTED_FILE_CHARS}",
                    entry.len()
                ),
            });
        }
        if entry.contains("..") || entry.starts_with('/') || entry.starts_with('\\') {
            return Err(ForgeError::StudioProjectScope {
                reason: format!(
                    "selected_files entry '{entry}' resolves outside the registered project root"
                ),
            });
        }
        if &redact_credentials(entry) != entry {
            return Err(ForgeError::StudioInvalidSpec {
                reason: "selected_files entry contains a credential-shaped string".to_string(),
            });
        }
    }
    let next_app_rev = bump_revision(&session.app_revision, "app_revision")?;
    let files_summary = if selected_files.is_empty() {
        "(none)".to_string()
    } else {
        selected_files.join(",")
    };
    let detail = bounded_detail(&format!(
        "app_revision={next_app_rev} request={} selected_files={files_summary}",
        bounded(request_text, 512)
    ));
    record_journal(
        registry,
        StudioJournalKind::Refine,
        &session.project_id,
        "done",
        &detail,
    )?;
    session.app_revision = next_app_rev;
    session.updated_at = Utc::now();
    persist_session(project_root, &session)?;
    Ok(session)
}

/// Append one Studio journal row to the existing `operations`
/// table. Errors are returned to the caller so an authentication
/// or registry failure surfaces as a typed `forge-studio-*` code
/// rather than a silent loss.
pub fn record_journal(
    registry: &Registry,
    kind: StudioJournalKind,
    project_id: &str,
    state: &str,
    detail: &str,
) -> Result<(), ForgeError> {
    let bounded = bounded_detail(detail);
    registry
        .record_operation(kind.label(), project_id, state, &bounded)
        .map_err(|err| match err {
            ForgeError::Registry { reason } => ForgeError::StudioInvalidSpec { reason },
            other => other,
        })
}

fn bump_revision(current: &str, field: &str) -> Result<String, ForgeError> {
    let numeric = current
        .strip_prefix('r')
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| ForgeError::StudioInvalidSpec {
            reason: format!("{field} '{current}' is not a 'r<n>' revision token"),
        })?;
    Ok(format!("r{}", numeric + 1))
}

fn bounded_detail(detail: &str) -> String {
    let redacted = redact_credentials(detail);
    bounded(&redacted, MAX_JOURNAL_DETAIL_CHARS)
}

fn bounded(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out = String::with_capacity(max + 16);
    for c in value.chars().take(max) {
        out.push(c);
    }
    out.push_str("[truncated]");
    out
}

/// Envelope accessor kept for symmetry with [`super::spec::AppSpec::envelope`].
pub fn session_envelope(session: &StudioSession) -> serde_json::Value {
    serde_json::json!({
        "contract": STUDIO_SESSION_CONTRACT,
        "project_id": session.project_id,
        "profile": session.profile,
        "app_schema_major": session.app_schema_major,
        "spec_revision": session.spec_revision,
        "app_revision": session.app_revision,
        "created_at": session.created_at,
        "updated_at": session.updated_at,
        "preview": {
            "state": session.preview.state.label(),
            "port": session.preview.port,
            "started_at": session.preview.started_at,
            "last_error_code": session.preview.last_error_code,
            "log_tail": session.preview.log_tail,
        },
        "spec": session.spec.envelope(),
        "app_spec_contract": APP_SPEC_CONTRACT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::spec::parse_spec_text;
    use tempfile::tempdir;

    const MINIMAL_YAML: &str = "schema_version: \"1\"\nproject_id: studio-fixture\nname: Studio Fixture\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n";

    fn build_registry() -> (tempfile::TempDir, Registry, PathBuf) {
        let dir = tempdir().expect("tempdir");
        let registry = Registry::open(&dir.path().join("registry.db")).expect("open");
        let project = dir.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        // Materialize the minimum manifest the registry requires so
        // `register` does not refuse the project before the Studio
        // touches anything. The minimal `forge.yaml` carries the
        // closed schema the existing generator already enforces.
        std::fs::write(
            project.join("forge.yaml"),
            "schema: 1\n\
             project:\n  id: studio-fixture\n  name: Studio Fixture\n\
             \x20\x20profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
        )
        .unwrap();
        (dir, registry, project)
    }

    fn register_and_use<F>(f: F)
    where
        F: FnOnce(&Registry, &Path),
    {
        let (dir, mut registry, project) = build_registry();
        registry.register(&project, None).expect("register project");
        let read_only = Registry::open(&dir.path().join("registry.db")).expect("reopen");
        f(&read_only, &project);
    }

    #[test]
    fn first_save_starts_at_r1() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            let session = save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            assert_eq!(session.spec_revision, "r1");
            assert_eq!(session.app_revision, "r1");
            assert_eq!(session.app_schema_major, "1");
        });
    }

    #[test]
    fn revision_conflict_is_refused_before_writing() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec.clone(), "r0").unwrap();
            let err = save_spec(registry, "studio-fixture", project, spec, "r9").unwrap_err();
            assert_eq!(err.code(), "studio-revision-conflict");
            let loaded = load_session(project).unwrap().unwrap();
            assert_eq!(loaded.spec_revision, "r1");
        });
    }

    #[test]
    fn refinement_bumps_app_revision_only() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            let refined = record_refinement(
                registry,
                project,
                "r1",
                "add a contact form",
                &["src/App.tsx".to_string()],
            )
            .unwrap();
            assert_eq!(refined.app_revision, "r2");
            assert_eq!(refined.spec_revision, "r1");
        });
    }

    #[test]
    fn refinement_rejects_path_escape() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            let err = record_refinement(
                registry,
                project,
                "r1",
                "escape",
                &["../etc/passwd".to_string()],
            )
            .unwrap_err();
            assert_eq!(err.code(), "studio-project-scope");
        });
    }

    #[test]
    fn refinement_rejects_stale_revision() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            let err = record_refinement(registry, project, "r9", "stale", &[]).unwrap_err();
            assert_eq!(err.code(), "studio-revision-conflict");
        });
    }

    #[test]
    fn refinement_rejects_credential_shaped_request() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            let err =
                record_refinement(registry, project, "r1", "use AKIAIOSFODNN7EXAMPLE key", &[])
                    .unwrap_err();
            assert_eq!(err.code(), "studio-invalid-spec");
        });
    }

    #[test]
    fn save_refuses_when_spec_project_id_mismatches_route() {
        register_and_use(|registry, project| {
            let mut spec = parse_spec_text(MINIMAL_YAML).unwrap();
            spec.project_id = "some-other-id".to_string();
            let err = save_spec(registry, "studio-fixture", project, spec, "r0").unwrap_err();
            assert_eq!(err.code(), "studio-project-scope");
        });
    }

    #[test]
    fn journal_rows_are_recorded() {
        register_and_use(|registry, project| {
            let spec = parse_spec_text(MINIMAL_YAML).unwrap();
            save_spec(registry, "studio-fixture", project, spec, "r0").unwrap();
            record_refinement(registry, project, "r1", "polish the hero", &[]).unwrap();
            let rows = registry
                .operations_for_project("studio-fixture", 32)
                .expect("operations");
            let kinds: Vec<String> = rows.iter().map(|row| row.kind.clone()).collect();
            assert!(kinds.iter().any(|k| k == "studio.spec.save"));
            assert!(kinds.iter().any(|k| k == "studio.refine"));
        });
    }
}

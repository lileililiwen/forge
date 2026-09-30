//! Site Studio: versioned AppSpec, bounded preview, scoped refinement
//! (`site-studio-preview-refinement`).
//!
//! Core owns the data-flow contract for the bounded local preview slice:
//!
//! - [`spec`] owns the versioned `forge.app.yaml` parser and validator.
//!   The parser is closed to the `app-spec-v1.json` vocabulary; the
//!   validator refuses unknown schema majors, unsupported profiles,
//!   duplicate routes, embedded shell metacharacters, path escapes, and
//!   secret-shaped strings, and is read-only — it never mutates project
//!   files until the operator explicitly confirms a save.
//! - [`state`] owns the per-project Studio session record persisted at
//!   `<project>/.forge/studio/session.json` and the journal rows the
//!   existing `operations` table records for `studio.spec.save`,
//!   `studio.preview.start`, `studio.preview.stop`, and `studio.refine`.
//! - [`preview`] owns the bounded preview lifecycle: port reservation,
//!   profile-runner spawn through `crate::process::spawn_with_timeout`,
//!   bounded stdout/stderr capture (1 MiB / 2 KiB), kill-on-startup-
//!   timeout, credential redaction, and idempotent stop.
//!
//! The wire contracts are versioned:
//!
//! - `forge-app-spec/0.1.0` (the AppSpec payload),
//! - `forge-studio-session/0.1.0` (the persisted session record),
//! - `forge-studio-preview/0.1.0` (the bounded preview envelope).
//!
//! The Studio page, the agent-adapter refinement hook, the same-origin
//! preview proxy, and the live `react-web` smoke are recorded as
//! explicit deferred work in `openspec/changes/site-studio-preview-refinement/tasks.md`.

pub mod preview;
pub mod spec;
pub mod state;

pub use preview::{
    allocate_port, envelope_from_session, start_preview, startup_timeout, stop_preview,
    validate_profile, FakeRunner, PreviewEnvelope, PreviewOutcome, PreviewRunner, PreviewSession,
    PreviewState, ProcessRunner, MAX_PREVIEW_LOG_BYTES, PREVIEW_CONTRACT,
    PREVIEW_LOG_TRUNCATION_MARKER, RUNTIME_BIN_ENV, RUNTIME_PORT_ENV, STARTUP_TIMEOUT_DEFAULT_SECS,
};
pub use spec::{
    parse_spec_file, parse_spec_text, validate_spec, AppPage, AppSection, AppSpec, AppTheme,
    ThemePreset, APP_SCHEMA_MAJOR, APP_SPEC_CONTRACT, MAX_PAGES, MAX_PAGE_TITLE_CHARS,
    MAX_PROJECT_NAME_CHARS, MAX_ROUTE_CHARS, MAX_SECTIONS_PER_PAGE, MAX_SECTION_BODY_CHARS,
    MAX_SECTION_ID_CHARS, MAX_SECTION_TITLE_CHARS, SUPPORTED_PROFILES,
};
pub use state::{
    load_session, record_journal, record_refinement, save_spec, session_path, PreviewRecord,
    SessionPreviewState, StudioSession, STUDIO_SESSION_CONTRACT,
};

use std::path::Path;

use crate::core::manifest::Manifest;
use crate::core::ForgeError;

/// Studio env-var: bounded startup window for the profile runner.
/// Clamped to `1..=600` seconds; the default is 30 seconds.
pub const STARTUP_TIMEOUT_ENV: &str = "FORGE_STUDIO_STARTUP_TIMEOUT_SECS";

/// Studio env-var: override the lower bound of the port allocator.
/// Defaults to `4100`. The allocator walks upwards through the range
/// and refuses every candidate already in use; it never kills an
/// unrelated process.
pub const STUDIO_PORT_RANGE_START_ENV: &str = "FORGE_STUDIO_PORT_RANGE_START";

/// Default lower bound of the Studio port range.
pub const DEFAULT_PORT_RANGE_START: u16 = 4100;

/// Width of the Studio port range. The allocator refuses with
/// `studio-port-unavailable` after this many consecutive busy ports.
pub const PORT_RANGE_WIDTH: u16 = 64;

/// Resolve the canonical project root for a Studio request, scoped to
/// the registered project. The manifest is loaded once so the caller
/// has the project identity and path without re-touching the registry.
pub fn load_project_root(
    project_root: &Path,
) -> Result<(Manifest, std::path::PathBuf), ForgeError> {
    Manifest::load_from_dir(project_root, None)
}

/// Resolve a CLI-supplied project reference. CLI accepts either a
/// registered id (resolved through the registry) or an explicit
/// registered path. Unknown references surface as
/// [`ForgeError::UnknownProject`] so a missing or unregistered
/// project is refused before any Studio file is touched.
pub fn resolve_project_root(
    registry: &crate::registry::Registry,
    project_ref: &str,
) -> Result<std::path::PathBuf, ForgeError> {
    let record = registry.inspect(project_ref)?;
    Ok(std::path::PathBuf::from(record.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_profiles_are_exactly_react_web_today() {
        assert_eq!(SUPPORTED_PROFILES, ["react-web"]);
    }

    #[test]
    fn schema_major_is_one() {
        assert_eq!(APP_SCHEMA_MAJOR, "1");
    }

    #[test]
    fn contracts_are_versioned() {
        assert_eq!(APP_SPEC_CONTRACT, "forge-app-spec/0.1.0");
        assert_eq!(STUDIO_SESSION_CONTRACT, "forge-studio-session/0.1.0");
    }
}

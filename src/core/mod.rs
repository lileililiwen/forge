//! Shared Core types: stable error codes and project identity rules.

pub mod manifest;

use thiserror::Error;

/// Typed Core failure. Each variant maps to a stable machine-readable
/// `code()` used by every transport; transports render, never reinterpret.
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("unsupported schema version {found} in {path}: only schema 1 is supported; file left unchanged")]
    UnsupportedSchema { path: String, found: String },

    #[error("ambiguous manifest sources in {dir}: both {first} and {second} exist; re-run with an explicit --manifest and neither file was changed")]
    AmbiguousManifest {
        dir: String,
        first: String,
        second: String,
    },

    #[error("legacy manifest {found} requires an explicit source: re-run with --manifest {found}; nothing was changed")]
    LegacyManifestRequiresExplicit { found: String },

    #[error("manifest not found: {path}")]
    ManifestNotFound { path: String },

    #[error("invalid manifest {path}: {reason}; file left unchanged")]
    ManifestInvalid { path: String, reason: String },

    #[error("project id collision: '{id}' is already registered to a different path; original record unchanged")]
    IdCollision { id: String },

    #[error("project path collision: '{path}' is already registered under a different id; original record unchanged")]
    PathCollision { path: String },

    #[error("unknown project: '{query}' matches no registered id or path")]
    UnknownProject { query: String },

    #[error("project path unavailable: '{path}'")]
    PathUnavailable { path: String },

    #[error("unknown profile: '{id}' matches no MVP profile descriptor")]
    UnknownProfile { id: String },

    #[error("invalid profile descriptor: {reason}")]
    InvalidProfile { reason: String },

    #[error("incompatible profile: {reason}")]
    IncompatibleProfile { reason: String },

    #[error("toolchain '{toolchain}' required by profile '{profile}' is unavailable locally; profile was not tested")]
    ToolchainMissing { toolchain: String, profile: String },

    #[error("unsupported profile: {reason}; plan a tracked future profile and re-run with --profile <id>")]
    UnsupportedProfile { reason: String },

    #[error(
        "ambiguous import: {detail}; re-run with an explicit --profile and no files were changed"
    )]
    AmbiguousImport { detail: String },

    #[error("import conflict: {reason}; source and conflicting metadata left unchanged")]
    ImportConflict { reason: String },

    #[error("generation conflict: {reason}; destination left unchanged")]
    GenerationConflict { reason: String },

    #[error("generation failed: {reason}; destination was cleaned and nothing was registered")]
    GenerationFailed { reason: String },

    #[error("generation cancelled: {reason}; no project or registry entry was created")]
    GenerationCancelled { reason: String },

    #[error("unknown feature: '{id}' matches no catalog feature descriptor")]
    UnknownFeature { id: String },

    #[error("incompatible feature: {reason}")]
    IncompatibleFeature { reason: String },

    #[error("feature ownership conflict: {reason}")]
    FeatureOwnershipConflict { reason: String },

    #[error("policy adapter unavailable: {reason}")]
    PolicyUnavailable { reason: String },

    #[error("spec invalid: {reason}")]
    SpecInvalid { reason: String },

    #[error("spec write failed at {path}: {reason}")]
    SpecWrite { path: String, reason: String },

    #[error("registry error: {reason}")]
    Registry { reason: String },

    #[error("agent adapter unavailable: {reason}")]
    AgentUnavailable { reason: String },

    #[error("agent operation unsupported: {reason}")]
    AgentUnsupported { reason: String },

    #[error("test command failed: {reason}")]
    TestFailed { reason: String },

    #[error("git working tree has unrelated changes: {reason}; commit the reviewed paths explicitly and leave others untouched")]
    GitDirty { reason: String },

    #[error("git push requires explicit --confirm; refusing implicit remote write")]
    PushConfirmRequired,

    #[error("mcp invalid: {reason}")]
    McpInvalid { reason: String },

    #[error("mcp tool unauthorized: {reason}")]
    McpUnauthorized { reason: String },

    #[error("distribution invalid: {reason}")]
    DistributionInvalid { reason: String },

    #[error("mirror `{provider}` is disabled in the project distribution config; no write was attempted")]
    MirrorDisabled { provider: String },

    #[error(
        "mirror `{provider}` has divergent protected history ({detail}); force or reverse-sync are refused to preserve one-way distribution"
    )]
    MirrorDiverged { provider: String, detail: String },

    #[error("mirror `{provider}` authentication failed: {reason}")]
    MirrorCredentials { provider: String, reason: String },

    #[error("docs invalid: {reason}")]
    DocsInvalid { reason: String },

    #[error("translation failed: {reason}; prior derivative left intact")]
    TranslationFailed { reason: String },

    #[error("release invalid: {reason}")]
    ReleaseInvalid { reason: String },

    #[error("release check failed: {reason}; release execution blocks before tag or publication")]
    ReleaseCheckFailed { reason: String },

    #[error("release identity conflict: {reason}; existing tag or immutable version points to different content")]
    ReleaseIdentityConflict { reason: String },

    #[error("deploy invalid: {reason}")]
    DeployInvalid { reason: String },

    #[error("deploy target unavailable: {reason}")]
    DeployTargetUnavailable { reason: String },

    #[error("deploy target stale: {reason}")]
    DeployTargetStale { reason: String },

    #[error("deploy health failed: {reason}; deployment state recorded as failed")]
    DeployHealthFailed { reason: String },

    #[error("component invalid: {reason}")]
    ComponentInvalid { reason: String },

    #[error("component quality conflict: {reason}")]
    ComponentQualityConflict { reason: String },
}

impl ForgeError {
    /// Stable machine code for structured error output.
    pub fn code(&self) -> &'static str {
        match self {
            ForgeError::UnsupportedSchema { .. } => "unsupported-schema",
            ForgeError::AmbiguousManifest { .. } => "ambiguous-manifest",
            ForgeError::LegacyManifestRequiresExplicit { .. } => {
                "legacy-manifest-requires-explicit"
            }
            ForgeError::ManifestNotFound { .. } => "manifest-not-found",
            ForgeError::ManifestInvalid { .. } => "manifest-invalid",
            ForgeError::IdCollision { .. } => "id-collision",
            ForgeError::PathCollision { .. } => "path-collision",
            ForgeError::UnknownProject { .. } => "unknown-project",
            ForgeError::PathUnavailable { .. } => "path-unavailable",
            ForgeError::UnknownProfile { .. } => "unknown-profile",
            ForgeError::InvalidProfile { .. } => "invalid-profile",
            ForgeError::IncompatibleProfile { .. } => "incompatible-profile",
            ForgeError::ToolchainMissing { .. } => "toolchain-missing",
            ForgeError::UnsupportedProfile { .. } => "unsupported-profile",
            ForgeError::AmbiguousImport { .. } => "ambiguous-import",
            ForgeError::ImportConflict { .. } => "import-conflict",
            ForgeError::GenerationConflict { .. } => "generation-conflict",
            ForgeError::GenerationFailed { .. } => "generation-failed",
            ForgeError::GenerationCancelled { .. } => "generation-cancelled",
            ForgeError::UnknownFeature { .. } => "unknown-feature",
            ForgeError::IncompatibleFeature { .. } => "incompatible-feature",
            ForgeError::FeatureOwnershipConflict { .. } => "feature-ownership-conflict",
            ForgeError::PolicyUnavailable { .. } => "policy-unavailable",
            ForgeError::SpecInvalid { .. } => "spec-invalid",
            ForgeError::SpecWrite { .. } => "spec-write-failed",
            ForgeError::Registry { .. } => "registry-error",
            ForgeError::AgentUnavailable { .. } => "agent-unavailable",
            ForgeError::AgentUnsupported { .. } => "agent-unsupported",
            ForgeError::TestFailed { .. } => "test-failed",
            ForgeError::GitDirty { .. } => "git-dirty",
            ForgeError::PushConfirmRequired => "push-confirm-required",
            ForgeError::McpInvalid { .. } => "mcp-invalid",
            ForgeError::McpUnauthorized { .. } => "mcp-unauthorized",
            ForgeError::DistributionInvalid { .. } => "distribution-invalid",
            ForgeError::MirrorDisabled { .. } => "mirror-disabled",
            ForgeError::MirrorDiverged { .. } => "mirror-diverged",
            ForgeError::MirrorCredentials { .. } => "mirror-credentials",
            ForgeError::DocsInvalid { .. } => "docs-invalid",
            ForgeError::TranslationFailed { .. } => "translation-failed",
            ForgeError::ReleaseInvalid { .. } => "release-invalid",
            ForgeError::ReleaseCheckFailed { .. } => "release-check-failed",
            ForgeError::ReleaseIdentityConflict { .. } => "release-identity-conflict",
            ForgeError::DeployInvalid { .. } => "deploy-invalid",
            ForgeError::DeployTargetUnavailable { .. } => "deploy-target-unavailable",
            ForgeError::DeployTargetStale { .. } => "deploy-target-stale",
            ForgeError::DeployHealthFailed { .. } => "deploy-health-failed",
            ForgeError::ComponentInvalid { .. } => "component-invalid",
            ForgeError::ComponentQualityConflict { .. } => "component-quality-conflict",
        }
    }

    /// Process exit code for CLI failures: 1 for operational errors.
    /// Usage errors (exit 2) are owned by the argument parser.
    pub fn exit_code(&self) -> i32 {
        1
    }
}

impl From<rusqlite::Error> for ForgeError {
    fn from(err: rusqlite::Error) -> Self {
        ForgeError::Registry {
            reason: err.to_string(),
        }
    }
}

/// Project ids are kebab-case: `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`.
pub fn validate_project_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("project id must not be empty".to_string());
    }
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => {
            return Err(format!(
                "invalid project id '{id}': must start with a lowercase letter"
            ))
        }
    }
    let mut prev_dash = false;
    for c in chars {
        if c == '-' {
            if prev_dash {
                return Err(format!(
                    "invalid project id '{id}': must not contain consecutive dashes"
                ));
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return Err(format!(
                "invalid project id '{id}': use lowercase letters, digits and single dashes"
            ));
        }
    }
    if prev_dash {
        return Err(format!(
            "invalid project id '{id}': must not end with a dash"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_ids() {
        for id in ["a", "rust-web", "mortality-reflection", "gpa-sim2"] {
            assert!(validate_project_id(id).is_ok(), "{id}");
        }
    }

    #[test]
    fn rejects_invalid_ids() {
        for id in ["", "Rust-web", "1abc", "-abc", "abc-", "a--b", "a_b", "a b"] {
            assert!(validate_project_id(id).is_err(), "{id}");
        }
    }

    #[test]
    fn error_codes_are_stable() {
        let err = ForgeError::UnknownProject {
            query: "x".to_string(),
        };
        assert_eq!(err.code(), "unknown-project");
        assert_eq!(err.exit_code(), 1);
    }
}

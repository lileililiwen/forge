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

    #[error("ui pattern invalid: {reason}")]
    UiPatternInvalid { reason: String },

    #[error("ui pattern unsupported platform: {reason}")]
    UiPatternUnsupportedPlatform { reason: String },

    #[error("ui pattern quality conflict: {reason}")]
    UiPatternQualityConflict { reason: String },

    #[error("ui pattern deprecated dependency: {reason}")]
    UiPatternDeprecatedDep { reason: String },

    #[error("ui pattern ownership conflict: {reason}")]
    UiPatternOwnershipConflict { reason: String },

    #[error("intent invalid: {reason}")]
    IntentInvalid { reason: String },

    #[error("intent ambiguous: {reason}; the planner refused to silently select a profile or capability")]
    IntentAmbiguous { reason: String },

    #[error("plan stale: {reason}; revalidate the intent before applying")]
    PlanStale { reason: String },

    #[error("plan conflict: {reason}; the planner refused to assemble the requested intent")]
    PlanConflict { reason: String },

    #[error("plan apply failed at step '{step}': {reason}")]
    PlanApplyFailed { step: String, reason: String },

    #[error("procedure invalid: {reason}")]
    ProcedureInvalid { reason: String },

    #[error("procedure unsupported operation: {reason}; the procedure references a Core operation that is not part of the current supported set")]
    ProcedureUnsupportedOperation { reason: String },

    #[error("procedure bypass refused: {reason}; Core still validates every operation; procedures do not override Core outcomes")]
    ProcedureBypassRefused { reason: String },

    #[error("procedure run failed at step '{step}': {reason}")]
    ProcedureRunFailed { step: String, reason: String },

    #[error("identity invalid: {reason}")]
    IdentityInvalid { reason: String },

    #[error("identity auth failed: {reason}")]
    IdentityAuthFailed { reason: String },

    #[error("identity session expired: {reason}")]
    IdentitySessionExpired { reason: String },

    #[error("identity session not found: {reason}")]
    IdentitySessionNotFound { reason: String },

    #[error("identity session cross-project: {reason}; sessions are project-scoped and may not be presented to a different project")]
    IdentitySessionCrossProject { reason: String },

    #[error(
        "identity permission denied: {reason}; provider login does not imply admin authorization"
    )]
    IdentityPermissionDenied { reason: String },

    #[error("analytics invalid: {reason}")]
    AnalyticsInvalid { reason: String },

    #[error("analytics adapter unavailable: {reason}")]
    AnalyticsAdapterUnavailable { reason: String },

    #[error("analytics mapping ambiguous: {reason}; refusing to attach another project's data")]
    AnalyticsMappingAmbiguous { reason: String },

    #[error("api invalid: {reason}")]
    ApiInvalid { reason: String },

    #[error("api unauthorized: {reason}")]
    ApiUnauthorized { reason: String },

    #[error(
        "api project mismatch: {reason}; the session does not authorize the requested project"
    )]
    ApiProjectMismatch { reason: String },

    #[error(
        "idempotency key conflict: {reason}; the same key was reused with a different request body"
    )]
    IdempotencyKeyConflict { reason: String },

    #[error("portal invalid: {reason}")]
    PortalInvalid { reason: String },

    #[error("readiness invalid: {reason}")]
    ReadinessInvalid { reason: String },

    #[error("readiness not ready: {reason}")]
    ReadinessNotReady { reason: String },

    #[error("provider invalid: {reason}")]
    ProviderInvalid { reason: String },

    #[error("governance invalid: {reason}")]
    GovernanceInvalid { reason: String },

    #[error("checker invalid: {reason}")]
    CheckInvalid { reason: String },

    #[error("governance provider unavailable: {reason}")]
    GovernanceUnavailable { reason: String },

    #[error("fleet registry invalid: {reason}")]
    FleetRegistryInvalid { reason: String },

    #[error("gate invalid: {reason}")]
    GateInvalid { reason: String },

    #[error("gate runtime unavailable: {reason}")]
    GateRuntimeUnavailable { reason: String },

    #[error("contract invalid: {reason}")]
    ContractInvalid { reason: String },
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
            ForgeError::UiPatternInvalid { .. } => "ui-pattern-invalid",
            ForgeError::UiPatternUnsupportedPlatform { .. } => "ui-pattern-unsupported-platform",
            ForgeError::UiPatternQualityConflict { .. } => "ui-pattern-quality-conflict",
            ForgeError::UiPatternDeprecatedDep { .. } => "ui-pattern-deprecated-dependency",
            ForgeError::UiPatternOwnershipConflict { .. } => "ui-pattern-ownership-conflict",
            ForgeError::IntentInvalid { .. } => "intent-invalid",
            ForgeError::IntentAmbiguous { .. } => "intent-ambiguous",
            ForgeError::PlanStale { .. } => "plan-stale",
            ForgeError::PlanConflict { .. } => "plan-conflict",
            ForgeError::PlanApplyFailed { .. } => "plan-apply-failed",
            ForgeError::ProcedureInvalid { .. } => "procedure-invalid",
            ForgeError::ProcedureUnsupportedOperation { .. } => "procedure-unsupported-operation",
            ForgeError::ProcedureBypassRefused { .. } => "procedure-bypass-refused",
            ForgeError::ProcedureRunFailed { .. } => "procedure-run-failed",
            ForgeError::IdentityInvalid { .. } => "identity-invalid",
            ForgeError::IdentityAuthFailed { .. } => "identity-auth-failed",
            ForgeError::IdentitySessionExpired { .. } => "identity-session-expired",
            ForgeError::IdentitySessionNotFound { .. } => "identity-session-not-found",
            ForgeError::IdentitySessionCrossProject { .. } => "identity-session-cross-project",
            ForgeError::IdentityPermissionDenied { .. } => "identity-permission-denied",
            ForgeError::AnalyticsInvalid { .. } => "analytics-invalid",
            ForgeError::AnalyticsAdapterUnavailable { .. } => "analytics-adapter-unavailable",
            ForgeError::AnalyticsMappingAmbiguous { .. } => "analytics-mapping-ambiguous",
            ForgeError::ApiInvalid { .. } => "api-invalid",
            ForgeError::ApiUnauthorized { .. } => "api-unauthorized",
            ForgeError::ApiProjectMismatch { .. } => "api-project-mismatch",
            ForgeError::IdempotencyKeyConflict { .. } => "idempotency-key-conflict",
            ForgeError::PortalInvalid { .. } => "portal-invalid",
            ForgeError::ReadinessInvalid { .. } => "readiness-invalid",
            ForgeError::ReadinessNotReady { .. } => "readiness-not-ready",
            ForgeError::ProviderInvalid { .. } => "provider-invalid",
            ForgeError::GovernanceInvalid { .. } => "governance-invalid",
            ForgeError::CheckInvalid { .. } => "check-invalid",
            ForgeError::GovernanceUnavailable { .. } => "governance-provider-unavailable",
            ForgeError::FleetRegistryInvalid { .. } => "fleet-registry-invalid",
            ForgeError::GateInvalid { .. } => "gate-invalid",
            ForgeError::GateRuntimeUnavailable { .. } => "gate-runtime-unavailable",
            ForgeError::ContractInvalid { .. } => "contract-invalid",
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

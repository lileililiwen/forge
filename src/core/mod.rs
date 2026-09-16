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

    #[error("registry error: {reason}")]
    Registry { reason: String },
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
            ForgeError::Registry { .. } => "registry-error",
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

//! Versioned MVP profile descriptors and the stack compatibility contract.
//!
//! Core owns the descriptors; transports render Core outcomes without
//! reinterpreting them. Descriptors are compiled-in, versioned metadata —
//! they establish what each stack supports, not working templates.
//! Generation validates each supported stack separately with its native
//! toolchain before any profile support is advertised as verified.
//!
//! MVP IDs: `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`,
//! `python-service`. Versions are explicit; there is no implicit `latest`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::core::ForgeError;

/// Capabilities that require a server-side backend. A client-only profile
/// (today: `flutter-app`) rejects these at resolution time, before any file
/// change, and suggests a backend boundary instead.
const SERVER_SIDE_CAPABILITIES: &[&str] = &["postgres", "redis", "background-jobs", "storage"];

/// Versioned descriptor for one MVP profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDescriptor {
    pub id: String,
    pub version: String,
    pub adapter: String,
    pub language: String,
    pub toolchain: String,
    #[serde(default)]
    pub toolchain_version: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub layout: Vec<String>,
    #[serde(default)]
    pub conventions: Vec<String>,
    pub build_command: String,
    pub test_command: String,
    #[serde(default)]
    pub deployment_type: Option<String>,
    #[serde(default)]
    pub deployment_target: Option<String>,
    #[serde(default)]
    pub quality_policies: Vec<String>,
    #[serde(default)]
    pub requires_database: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// Successful resolution: exact pinned version plus owning adapter identity.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedProfile {
    pub id: String,
    pub version: String,
    pub adapter: String,
    pub language: String,
    pub toolchain: String,
}

/// Toolchain preflight outcome. A present toolchain is a prerequisite, not
/// proof: it never claims the profile was tested.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PreflightReport {
    pub id: String,
    pub version: String,
    pub toolchain: String,
    pub available: bool,
}

#[allow(clippy::too_many_arguments)]
fn descriptor(
    id: &str,
    version: &str,
    adapter: &str,
    language: &str,
    toolchain: &str,
    toolchain_version: Option<&str>,
    capabilities: &[&str],
    packages: &[&str],
    layout: &[&str],
    conventions: &[&str],
    build_command: &str,
    test_command: &str,
    deployment_type: Option<&str>,
    deployment_target: Option<&str>,
    quality_policies: &[&str],
    requires_database: bool,
    description: &str,
) -> ProfileDescriptor {
    ProfileDescriptor {
        id: id.to_string(),
        version: version.to_string(),
        adapter: adapter.to_string(),
        language: language.to_string(),
        toolchain: toolchain.to_string(),
        toolchain_version: toolchain_version.map(str::to_string),
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        packages: packages.iter().map(|s| s.to_string()).collect(),
        layout: layout.iter().map(|s| s.to_string()).collect(),
        conventions: conventions.iter().map(|s| s.to_string()).collect(),
        build_command: build_command.to_string(),
        test_command: test_command.to_string(),
        deployment_type: deployment_type.map(str::to_string),
        deployment_target: deployment_target.map(str::to_string),
        quality_policies: quality_policies.iter().map(|s| s.to_string()).collect(),
        requires_database,
        description: Some(description.to_string()),
    }
}

/// All five MVP profile descriptors in stable ID order.
pub fn mvp_profiles() -> Vec<ProfileDescriptor> {
    vec![
        descriptor(
            "aspnet-web",
            "0.1.0",
            "adapter-dotnet",
            "csharp",
            "dotnet",
            Some("8.0"),
            &[
                "auth",
                "admin",
                "postgres",
                "redis",
                "email",
                "storage",
                "audit",
                "telemetry",
                "health-check",
                "rate-limit",
                "background-jobs",
                "search",
                "billing",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
            ],
            &[
                "Microsoft.AspNetCore.OpenApi",
                "Npgsql.EntityFrameworkCore.PostgreSQL",
                "StackExchange.Redis",
            ],
            &["src/", "tests/", "Dockerfile", "appsettings.json"],
            &["async-controllers", "ef-core-migrations", "openapi-first"],
            "dotnet build",
            "dotnet test",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "ASP.NET Core server-rendered web stack",
        ),
        descriptor(
            "flutter-app",
            "0.1.0",
            "adapter-flutter",
            "dart",
            "flutter",
            Some("3.22"),
            &[
                "auth",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
                "telemetry",
                "health-check",
            ],
            &["flutter_bloc", "go_router", "dio"],
            &["lib/", "test/", "pubspec.yaml", "android/", "ios/"],
            &["bloc-state", "go-router-navigation", "offline-first-cache"],
            "flutter build appbundle",
            "flutter test",
            Some("app-store"),
            Some("store-internal"),
            &["PRIVACY-003", "A11Y-001"],
            false,
            "Flutter client application; server-side capabilities live behind a backend boundary",
        ),
        descriptor(
            "nextjs-web",
            "0.1.0",
            "adapter-nextjs",
            "typescript",
            "node",
            Some("20"),
            &[
                "auth",
                "admin",
                "email",
                "audit",
                "telemetry",
                "health-check",
                "rate-limit",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
                "search",
            ],
            &["next", "react", "next-auth", "tailwindcss"],
            &["app/", "components/", "tests/", "next.config.mjs"],
            &[
                "app-router",
                "server-components-default",
                "edge-safe-middleware",
            ],
            "npm run build",
            "npm test",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "A11Y-001", "DEPLOY-001"],
            false,
            "Next.js web stack without a forced database dependency",
        ),
        descriptor(
            "python-service",
            "0.1.0",
            "adapter-python",
            "python",
            "python3",
            Some("3.12"),
            &[
                "auth",
                "admin",
                "postgres",
                "redis",
                "email",
                "storage",
                "audit",
                "telemetry",
                "health-check",
                "rate-limit",
                "background-jobs",
                "search",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
            ],
            &["fastapi", "sqlalchemy", "alembic", "pytest"],
            &["app/", "tests/", "pyproject.toml", "Dockerfile"],
            &[
                "router-per-resource",
                "alembic-migrations",
                "pydantic-boundaries",
            ],
            "python3 -m build",
            "python3 -m pytest",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "Python API service stack",
        ),
        descriptor(
            "rust-web",
            "0.1.0",
            "adapter-rust",
            "rust",
            "cargo",
            Some("stable"),
            &[
                "auth",
                "admin",
                "postgres",
                "redis",
                "email",
                "storage",
                "audit",
                "telemetry",
                "health-check",
                "rate-limit",
                "background-jobs",
                "search",
                "billing",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
            ],
            &["axum", "tokio", "sqlx", "serde"],
            &["src/", "tests/", "Cargo.toml", "Dockerfile"],
            &["axum-handlers", "sqlx-migrations", "tracing-spans"],
            "cargo build",
            "cargo test",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "Rust Axum web service stack",
        ),
    ]
}

/// List all MVP descriptors in stable ID order.
pub fn list_profiles() -> Vec<ProfileDescriptor> {
    mvp_profiles()
}

/// Inspect one descriptor by ID.
pub fn inspect_profile(id: &str) -> Result<ProfileDescriptor, ForgeError> {
    mvp_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| ForgeError::UnknownProfile { id: id.to_string() })
}

/// Validate a descriptor parsed from external input. Names the missing
/// field so callers can report it without guessing.
pub fn validate_descriptor(profile: &ProfileDescriptor) -> Result<(), ForgeError> {
    let name = if profile.id.trim().is_empty() {
        "unknown"
    } else {
        profile.id.as_str()
    };
    let missing = if profile.id.trim().is_empty() {
        Some("id")
    } else if profile.version.trim().is_empty() {
        Some("version")
    } else if profile.adapter.trim().is_empty() {
        Some("adapter")
    } else if profile.language.trim().is_empty() {
        Some("language")
    } else if profile.toolchain.trim().is_empty() {
        Some("toolchain")
    } else if profile.build_command.trim().is_empty() {
        Some("build_command")
    } else if profile.test_command.trim().is_empty() {
        Some("test_command")
    } else {
        None
    };
    if let Some(field) = missing {
        return Err(ForgeError::InvalidProfile {
            reason: format!("profile descriptor '{name}' missing required field '{field}'"),
        });
    }
    Ok(())
}

/// Parse an external descriptor document and validate required fields.
pub fn descriptor_from_yaml(bytes: &[u8]) -> Result<ProfileDescriptor, ForgeError> {
    let profile: ProfileDescriptor =
        serde_yaml::from_slice(bytes).map_err(|err| ForgeError::InvalidProfile {
            reason: format!("malformed profile descriptor: {err}"),
        })?;
    validate_descriptor(&profile)?;
    Ok(profile)
}

/// Resolve a profile plus requested capabilities. Fails before any file
/// change when the combination is unsupported and explains the boundary.
pub fn resolve_profile(id: &str, requested: &[String]) -> Result<ResolvedProfile, ForgeError> {
    let profile = inspect_profile(id)?;
    for capability in requested {
        if profile.capabilities.iter().any(|c| c == capability) {
            continue;
        }
        if SERVER_SIDE_CAPABILITIES.contains(&capability.as_str()) {
            return Err(ForgeError::IncompatibleProfile {
                reason: format!(
                    "profile '{id}' does not support server-side capability '{capability}'; \
                     use a backend profile (e.g. 'rust-web' or 'python-service') for '{capability}' \
                     and keep '{id}' as client; no files were changed"
                ),
            });
        }
        return Err(ForgeError::IncompatibleProfile {
            reason: format!(
                "profile '{id}' version '{}' does not support capability '{capability}' \
                 (adapter '{}'); no files were changed",
                profile.version, profile.adapter
            ),
        });
    }
    Ok(ResolvedProfile {
        id: profile.id,
        version: profile.version,
        adapter: profile.adapter,
        language: profile.language,
        toolchain: profile.toolchain,
    })
}

/// Preflight the required toolchain. With `available` set, membership is
/// checked directly (deterministic for tests); otherwise the host `PATH` is
/// probed. A missing toolchain is reported as unavailable — never as tested.
pub fn preflight_profile(
    id: &str,
    available: Option<&HashSet<String>>,
) -> Result<PreflightReport, ForgeError> {
    let profile = inspect_profile(id)?;
    let present = match available {
        Some(set) => set.contains(profile.toolchain.as_str()),
        None => toolchain_on_path(profile.toolchain.as_str()),
    };
    if !present {
        return Err(ForgeError::ToolchainMissing {
            toolchain: profile.toolchain.clone(),
            profile: profile.id,
        });
    }
    Ok(PreflightReport {
        id: profile.id,
        version: profile.version,
        toolchain: profile.toolchain,
        available: true,
    })
}

fn toolchain_on_path(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return true;
        }
        if Path::new(&candidate).exists() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn lists_all_five_mvp_ids_with_versions() {
        let profiles = list_profiles();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "aspnet-web",
                "flutter-app",
                "nextjs-web",
                "python-service",
                "rust-web"
            ]
        );
        for p in &profiles {
            assert!(!p.version.trim().is_empty(), "{} needs a version", p.id);
            validate_descriptor(p).expect("built-in descriptors must be valid");
        }
    }

    #[test]
    fn descriptors_carry_full_metadata() {
        for p in list_profiles() {
            assert!(!p.adapter.trim().is_empty(), "{}", p.id);
            assert!(!p.language.trim().is_empty(), "{}", p.id);
            assert!(!p.toolchain.trim().is_empty(), "{}", p.id);
            assert!(!p.capabilities.is_empty(), "{}", p.id);
            assert!(!p.packages.is_empty(), "{}", p.id);
            assert!(!p.layout.is_empty(), "{}", p.id);
            assert!(!p.conventions.is_empty(), "{}", p.id);
            assert!(!p.build_command.trim().is_empty(), "{}", p.id);
            assert!(!p.test_command.trim().is_empty(), "{}", p.id);
            assert!(!p.quality_policies.is_empty(), "{}", p.id);
        }
    }

    #[test]
    fn malformed_descriptor_names_missing_field() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: ''\n";
        let err = descriptor_from_yaml(yaml).expect_err("empty test_command must fail");
        assert_eq!(err.code(), "invalid-profile");
        assert!(
            err.to_string().contains("test_command"),
            "must name the field: {err}"
        );
    }

    #[test]
    fn valid_profile_without_database_forces_no_dependency() {
        let flutter = inspect_profile("flutter-app").unwrap();
        assert!(!flutter.requires_database);
        assert!(
            !flutter.packages.iter().any(|p| p.contains("postgres")),
            "client profile must not force a database package"
        );
        assert!(!flutter.capabilities.iter().any(|c| c == "postgres"));
        // Resolution without features succeeds and carries no db requirement.
        let resolved = resolve_profile("flutter-app", &[]).unwrap();
        assert_eq!(resolved.version, flutter.version);
    }

    #[test]
    fn resolve_returns_exact_version_and_adapter() {
        let resolved = resolve_profile("rust-web", &strings(&["auth"])).unwrap();
        assert_eq!(resolved.version, "0.1.0");
        assert_eq!(resolved.adapter, "adapter-rust");
        assert_eq!(resolved.language, "rust");
    }

    #[test]
    fn flutter_rejects_server_postgres_with_backend_hint() {
        let err = resolve_profile("flutter-app", &strings(&["postgres"]))
            .expect_err("flutter + postgres must fail");
        assert_eq!(err.code(), "incompatible-profile");
        let text = err.to_string();
        assert!(text.contains("postgres"), "{text}");
        assert!(text.contains("backend"), "{text}");
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn missing_toolchain_reports_without_claiming_tested() {
        let empty = HashSet::new();
        let err = preflight_profile("rust-web", Some(&empty)).expect_err("no toolchain");
        assert_eq!(err.code(), "toolchain-missing");
        let text = err.to_string();
        assert!(text.contains("cargo"), "{text}");
        assert!(text.contains("not tested"), "{text}");
        assert!(!text.contains("tested ok"), "{text}");
    }

    #[test]
    fn unknown_profile_is_reported() {
        let err = inspect_profile("react-web").expect_err("react-web is later");
        assert_eq!(err.code(), "unknown-profile");
    }
}

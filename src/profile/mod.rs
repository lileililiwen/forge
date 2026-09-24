//! Versioned profile descriptors and the stack compatibility contract.
//!
//! Core owns the descriptors; transports render Core outcomes without
//! reinterpreting them. Descriptors are compiled-in, versioned metadata —
//! they establish what each stack supports, not working templates.
//! Generation validates each supported stack separately with its native
//! toolchain before any profile support is advertised as verified.
//!
//! MVP IDs: `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`,
//! `python-service`, plus `react-web` (promoted from the v0.2 catalog with
//! the same descriptor contract and a tested native scaffold). Future
//! specialist candidates are reserved as [`ProfileSupportStatus::Planned`]
//! descriptors so the roadmap stays discoverable without advertising
//! unsupported generation: a planned id is not selectable through
//! resolution, preflight or generation until it is promoted to
//! [`ProfileSupportStatus::Supported`].
//!
//! Versions are explicit; there is no implicit `latest`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::core::ForgeError;

/// Capabilities that require a server-side backend. A client-only profile
/// (today: `flutter-app`, `react-web`) rejects these at resolution time,
/// before any file change, and suggests a backend boundary instead.
const SERVER_SIDE_CAPABILITIES: &[&str] = &["postgres", "redis", "background-jobs", "storage"];

/// Catalog support status for one profile id. A descriptor marked
/// [`ProfileSupportStatus::Supported`] is selectable through every
/// owning-domain entry point (resolution, preflight, generation);
/// [`ProfileSupportStatus::Planned`] is reserved on the roadmap and
/// inspectable, but operations that require generation evidence fail
/// before any mutation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileSupportStatus {
    /// Fully implemented, has a tested template and resolves like the MVP.
    Supported,
    /// Reserved on the roadmap; no tested template, not selectable.
    Planned,
}

/// Versioned descriptor for one profile. The `support_status` field
/// disambiguates a `Supported` profile (selectable, has a tested template)
/// from a `Planned` candidate (discoverable, not yet implemented).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDescriptor {
    pub id: String,
    pub version: String,
    #[serde(default = "default_support_status")]
    pub support_status: ProfileSupportStatus,
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
    /// Workspace Governance metadata mapping (`.project.json` emission).
    /// `None` is the no-mapping sentinel: generation omits the file and
    /// prints a note, and never guesses a governance profile.
    #[serde(default)]
    pub workspace: Option<WorkspaceMapping>,
}

/// Explicit per-profile Workspace Governance mapping used by generation to
/// emit an honest initial `.project.json` (sibling `schema_version: 1`).
/// The governance profile string must exist in the sibling's vocabulary;
/// the mapping lives in the descriptor so each profile decides its own
/// adoption shape (or opts out with no mapping).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMapping {
    /// Governance profile vocabulary value (e.g. `rust-product`).
    pub governance_profile: String,
    /// Declaration kind (e.g. `product`; `control-plane` when declared).
    pub kind: String,
    /// Gate runtime name, emitted only when the profile declares one.
    /// No supported profile declares a gate runtime today.
    #[serde(default)]
    pub gate_runtime: Option<String>,
}

fn default_support_status() -> ProfileSupportStatus {
    ProfileSupportStatus::Supported
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
    descriptor_with_status(
        id,
        version,
        ProfileSupportStatus::Supported,
        adapter,
        language,
        toolchain,
        toolchain_version,
        capabilities,
        packages,
        layout,
        conventions,
        build_command,
        test_command,
        deployment_type,
        deployment_target,
        quality_policies,
        requires_database,
        description,
    )
}

#[allow(clippy::too_many_arguments)]
fn descriptor_with_status(
    id: &str,
    version: &str,
    support_status: ProfileSupportStatus,
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
        support_status,
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
        workspace: None,
    }
}

/// Attach a Workspace Governance mapping to a descriptor (generation
/// emits `.project.json` only for profiles carrying one). The gate runtime
/// stays undeclared: no supported profile runs a shared Gate Runtime.
fn with_governance(
    mut profile: ProfileDescriptor,
    governance_profile: &str,
    kind: &str,
) -> ProfileDescriptor {
    profile.workspace = Some(WorkspaceMapping {
        governance_profile: governance_profile.to_string(),
        kind: kind.to_string(),
        gate_runtime: None,
    });
    profile
}

/// All supported profile descriptors in stable ID order. Resolvers,
/// preflight and generation consume this list; planned candidates stay
/// out of it until they are promoted to supported.
pub fn mvp_profiles() -> Vec<ProfileDescriptor> {
    mvp_profiles_supported()
}

/// Reserved specialist profiles that are on the roadmap but not yet
/// implemented. They are inspectable so the catalog stays discoverable,
/// but resolution, preflight and generation refuse them with
/// `unsupported-profile` before any file change. The future boundary hint
/// for each planned id lives in the descriptor description.
pub fn planned_profiles() -> Vec<ProfileDescriptor> {
    vec![
        descriptor_with_status(
            "aspnet-saas",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-dotnet-saas",
            "csharp",
            "dotnet",
            Some("8.0"),
            &[],
            &[],
            &["src/", "tests/", "Dockerfile", "appsettings.json"],
            &["multi-tenant", "subscription-aware"],
            "dotnet build",
            "dotnet test",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "BILLING-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "Planned multi-tenant ASP.NET SaaS profile; no tested template yet. \
             Promotion requires a versioned descriptor with a tested scaffold before \
             it becomes selectable.",
        ),
        descriptor_with_status(
            "flutter-client",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-flutter-client",
            "dart",
            "flutter",
            Some("3.22"),
            &[],
            &[],
            &["lib/", "test/", "pubspec.yaml", "android/", "ios/"],
            &["client-only", "backend-boundary"],
            "flutter build appbundle",
            "flutter test",
            Some("app-store"),
            Some("store-internal"),
            &["PRIVACY-003", "A11Y-001"],
            false,
            "Planned Flutter client-only profile; server capabilities (postgres, \
             redis, email, storage, background-jobs, rate-limit, audit) must live \
             behind a backend profile (e.g. 'rust-web' or 'python-service'); \
             'flutter-app' is the supported UI-bearing client profile today",
        ),
        descriptor_with_status(
            "nextjs-content",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-nextjs-content",
            "typescript",
            "node",
            Some("20"),
            &[],
            &[],
            &[
                "app/",
                "components/",
                "content/",
                "tests/",
                "next.config.mjs",
            ],
            &["static-first", "content-collections"],
            "npm run build",
            "npm test",
            Some("container"),
            Some("home-server-01"),
            &["A11Y-001", "DEPLOY-001"],
            false,
            "Planned Next.js content-first profile; no tested template yet. \
             Promotion requires a versioned descriptor with a tested scaffold before \
             it becomes selectable.",
        ),
        descriptor_with_status(
            "python-ai",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-python-ai",
            "python",
            "python3",
            Some("3.12"),
            &[],
            &[],
            &[
                "app/",
                "notebooks/",
                "tests/",
                "pyproject.toml",
                "Dockerfile",
            ],
            &["model-per-task", "deterministic-pipelines"],
            "python3 -m build",
            "python3 -m pytest",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "Planned Python AI service profile; no tested template yet. Promotion \
             requires a versioned descriptor with a tested scaffold before it \
             becomes selectable.",
        ),
        descriptor_with_status(
            "python-data",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-python-data",
            "python",
            "python3",
            Some("3.12"),
            &[],
            &[],
            &[
                "app/",
                "pipelines/",
                "tests/",
                "pyproject.toml",
                "Dockerfile",
            ],
            &["pipeline-per-dataset", "schema-versioning"],
            "python3 -m build",
            "python3 -m pytest",
            Some("container"),
            Some("home-server-01"),
            &["PRIVACY-003", "DEPLOY-001"],
            true,
            "Planned Python data pipeline profile; no tested template yet. Promotion \
             requires a versioned descriptor with a tested scaffold before it \
             becomes selectable.",
        ),
        descriptor_with_status(
            "rust-cli",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-rust-cli",
            "rust",
            "cargo",
            Some("stable"),
            &[],
            &[],
            &["src/", "tests/", "Cargo.toml"],
            &["clap-derived", "zero-runtime"],
            "cargo build",
            "cargo test",
            None,
            None,
            &["CLI-001"],
            false,
            "Planned Rust CLI profile; no tested template yet. Promotion requires a \
             versioned descriptor with a tested scaffold before it becomes selectable.",
        ),
        descriptor_with_status(
            "rust-worker",
            "0.0.0",
            ProfileSupportStatus::Planned,
            "adapter-rust-worker",
            "rust",
            "cargo",
            Some("stable"),
            &[],
            &[],
            &["src/", "tests/", "Cargo.toml", "Dockerfile"],
            &["queue-consumer", "retry-with-backoff"],
            "cargo build",
            "cargo test",
            Some("container"),
            Some("home-server-01"),
            &["DEPLOY-001"],
            true,
            "Planned Rust worker profile; no tested template yet. Promotion requires \
             a versioned descriptor with a tested scaffold before it becomes \
             selectable.",
        ),
    ]
}

/// Internal: the list of [`ProfileSupportStatus::Supported`] descriptors
/// in stable ID order. [`mvp_profiles`] is the public alias.
fn mvp_profiles_supported() -> Vec<ProfileDescriptor> {
    vec![
        with_governance(
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
            "dotnet-product",
            "product",
        ),
        with_governance(
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
            "flutter-product",
            "product",
        ),
        with_governance(
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
            "typescript-product",
            "product",
        ),
        with_governance(
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
            "python-product",
            "product",
        ),
        with_governance(
            descriptor(
                "react-web",
            "0.1.0",
            "adapter-react",
            "typescript",
            "npm",
            Some("20"),
            &[
                "auth",
                "admin",
                "notifications",
                "i18n",
                "privacy",
                "content",
                "analytics",
                "telemetry",
                "health-check",
                "search",
                "billing",
            ],
            &["react", "react-dom", "react-router-dom", "vite"],
            &[
                "src/",
                "public/",
                "index.html",
                "package.json",
                "vite.config.js",
                "tests/",
            ],
            &["component-per-file", "hooks-at-top", "client-side-routing"],
            "npm run build",
            "npm test",
            Some("container"),
            Some("home-server-01"),
            &["A11Y-001", "DEPLOY-001"],
            false,
            "React SPA web stack; client-only rendering with no server-side runtime; \
             server capabilities (postgres, redis, email, storage, background-jobs, \
             rate-limit, audit) live behind a backend profile",
            ),
            "typescript-product",
            "product",
        ),
        with_governance(
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
            "rust-product",
            "product",
        ),
    ]
}

/// List all supported descriptors in stable ID order. Resolution, preflight
/// and generation consume this list; planned candidates stay discoverable
/// through [`inspect_profile`] and [`planned_profiles`] but never enter the
/// selectable catalog until they are promoted to supported.
pub fn list_profiles() -> Vec<ProfileDescriptor> {
    mvp_profiles()
}

/// Inspect one descriptor by ID. Searches the supported catalog first and
/// then the planned catalog so the roadmap stays discoverable. The
/// descriptor's `support_status` field is the authoritative signal for
/// whether the id is selectable through resolution or generation.
pub fn inspect_profile(id: &str) -> Result<ProfileDescriptor, ForgeError> {
    if let Some(p) = mvp_profiles().into_iter().find(|p| p.id == id) {
        return Ok(p);
    }
    if let Some(p) = planned_profiles().into_iter().find(|p| p.id == id) {
        return Ok(p);
    }
    Err(ForgeError::UnknownProfile { id: id.to_string() })
}

/// True iff the id is in the supported catalog and therefore selectable
/// through resolution, preflight and generation.
pub fn is_supported(id: &str) -> bool {
    mvp_profiles().iter().any(|p| p.id == id)
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
    } else if profile
        .workspace
        .as_ref()
        .is_some_and(|w| w.governance_profile.trim().is_empty())
    {
        Some("workspace.governance_profile")
    } else if profile
        .workspace
        .as_ref()
        .is_some_and(|w| w.kind.trim().is_empty())
    {
        Some("workspace.kind")
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
/// Planned profiles are reported as `unsupported-profile` so callers can
/// distinguish a missing template from a missing capability.
pub fn resolve_profile(id: &str, requested: &[String]) -> Result<ResolvedProfile, ForgeError> {
    let profile = inspect_profile(id)?;
    if profile.support_status == ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{id}' is reserved on the catalog as a planned candidate \
                 with no tested template; promote it to a versioned supported \
                 descriptor before selection; no files were changed"
            ),
        });
    }
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
/// Planned profiles are refused with `unsupported-profile` so the
/// preflight never claims a planned candidate was tested.
pub fn preflight_profile(
    id: &str,
    available: Option<&HashSet<String>>,
) -> Result<PreflightReport, ForgeError> {
    let profile = inspect_profile(id)?;
    if profile.support_status == ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{id}' is reserved on the catalog as a planned candidate \
                 with no tested template; preflight refuses planned profiles and \
                 no toolchain was probed"
            ),
        });
    }
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
    fn lists_six_supported_ids_including_react_web() {
        let profiles = list_profiles();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "aspnet-web",
                "flutter-app",
                "nextjs-web",
                "python-service",
                "react-web",
                "rust-web",
            ]
        );
        for p in &profiles {
            assert!(!p.version.trim().is_empty(), "{} needs a version", p.id);
            assert_eq!(
                p.support_status,
                ProfileSupportStatus::Supported,
                "{}",
                p.id
            );
            validate_descriptor(p).expect("built-in descriptors must be valid");
        }
    }

    #[test]
    fn react_web_resolves_with_client_capabilities_only() {
        let resolved = resolve_profile("react-web", &strings(&["auth", "i18n"])).unwrap();
        assert_eq!(resolved.version, "0.1.0");
        assert_eq!(resolved.adapter, "adapter-react");
        assert_eq!(resolved.language, "typescript");
        assert_eq!(resolved.toolchain, "npm");

        // Boundary: react-web is client-only, the v0.1 server-side
        // capabilities must be refused with the backend hint.
        let err = resolve_profile("react-web", &strings(&["postgres"]))
            .expect_err("react-web + postgres must fail");
        assert_eq!(err.code(), "incompatible-profile");
        let text = err.to_string();
        assert!(text.contains("postgres"), "{text}");
        assert!(text.contains("backend"), "{text}");
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn react_web_descriptor_has_no_database_dependency() {
        let react = inspect_profile("react-web").unwrap();
        assert!(!react.requires_database);
        assert!(
            !react.packages.iter().any(|p| p.contains("postgres")),
            "client profile must not force a database package"
        );
        assert!(!react.capabilities.iter().any(|c| c == "postgres"));
        assert!(!react.capabilities.iter().any(|c| c == "redis"));
        assert!(!react.capabilities.iter().any(|c| c == "background-jobs"));
        assert!(!react.capabilities.iter().any(|c| c == "storage"));
    }

    #[test]
    fn supported_profiles_declare_workspace_mappings() {
        let expected = [
            ("aspnet-web", "dotnet-product"),
            ("flutter-app", "flutter-product"),
            ("nextjs-web", "typescript-product"),
            ("python-service", "python-product"),
            ("react-web", "typescript-product"),
            ("rust-web", "rust-product"),
        ];
        for (id, governance) in expected {
            let p = inspect_profile(id).unwrap();
            let w = p
                .workspace
                .as_ref()
                .unwrap_or_else(|| panic!("{id} declares a governance mapping"));
            assert_eq!(w.governance_profile, governance, "{id}");
            assert_eq!(w.kind, "product", "{id}");
            // No shared Gate Runtime is configured, so no profile declares one.
            assert_eq!(w.gate_runtime, None, "{id}");
        }
        // Planned candidates hold the no-mapping sentinel: nothing is ever
        // guessed for a profile without a tested scaffold.
        for p in planned_profiles() {
            assert!(p.workspace.is_none(), "{} must have no mapping", p.id);
        }
    }

    #[test]
    fn workspace_mapping_parses_and_rejects_blank_governance_profile() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\nworkspace:\n  governance_profile: '  '\n  kind: product\n";
        let err = descriptor_from_yaml(yaml).expect_err("blank governance profile must fail");
        assert_eq!(err.code(), "invalid-profile");
        assert!(
            err.to_string().contains("workspace.governance_profile"),
            "{err}"
        );

        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\n";
        let p = descriptor_from_yaml(yaml).unwrap();
        assert!(
            p.workspace.is_none(),
            "absent mapping parses to the sentinel"
        );
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
        let err = inspect_profile("not-a-real-profile").expect_err("must be unknown");
        assert_eq!(err.code(), "unknown-profile");
    }

    #[test]
    fn planned_profiles_are_inspectable_but_not_selectable() {
        // Discoverability: every reserved candidate has a descriptor with
        // status `planned` and a description that names the boundary.
        let planned = planned_profiles();
        let ids: Vec<&str> = planned.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "aspnet-saas",
                "flutter-client",
                "nextjs-content",
                "python-ai",
                "python-data",
                "rust-cli",
                "rust-worker",
            ]
        );
        for p in &planned {
            assert_eq!(p.support_status, ProfileSupportStatus::Planned, "{}", p.id);
            assert!(
                p.description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("planned"),
                "{} must mark itself as planned",
                p.id
            );
            validate_descriptor(p).expect("planned descriptor must be well-formed");
        }
        // The supported catalog stays at 6 and never includes planned ids.
        for planned_id in &ids {
            assert!(
                !is_supported(planned_id),
                "{planned_id} must not be supported"
            );
            assert!(!list_profiles().iter().any(|p| p.id == *planned_id));
        }
    }

    #[test]
    fn planned_profile_resolve_reports_unsupported_profile() {
        let err = resolve_profile("aspnet-saas", &[]).expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("aspnet-saas"), "{text}");
        assert!(
            text.contains("planned"),
            "must name the planned status: {text}"
        );
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn planned_profile_preflight_refuses_without_probing_toolchain() {
        let err = preflight_profile("flutter-client", None).expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("flutter-client"), "{text}");
        assert!(text.contains("planned"), "{text}");
    }

    #[test]
    fn flutter_client_planned_descriptor_describes_backend_boundary() {
        // Boundary scenario: a planned client profile that needs server
        // capabilities must describe a separate backend boundary rather
        // than embedding server infrastructure in the client.
        let flutter_client = inspect_profile("flutter-client").unwrap();
        assert_eq!(flutter_client.support_status, ProfileSupportStatus::Planned);
        let desc = flutter_client.description.unwrap_or_default();
        assert!(
            desc.contains("backend"),
            "flutter-client description must reference a backend boundary: {desc}"
        );
        assert!(
            desc.contains("server")
                || desc.contains("'rust-web'")
                || desc.contains("'python-service'"),
            "flutter-client description must name the supported backend alternatives: {desc}"
        );
    }
}

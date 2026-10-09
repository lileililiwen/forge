//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{ProfileDescriptor, ProfileSupportStatus, WorkspaceMapping};

pub(super) fn default_support_status() -> ProfileSupportStatus {
    ProfileSupportStatus::Supported
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
        kit: None,
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
        capabilities: Vec::new(),
    });
    profile
}

/// Attach a compiled-in shared-layer kit reference to a descriptor.
///
/// Generation resolves the reference against the compiled-in kit registry,
/// checks its ecosystem against the profile's own toolchain, evaluates the
/// declared consumption floor, and renders the pre-wired manifest block.
fn with_kit(
    mut profile: ProfileDescriptor,
    reference: crate::kit::registry::KitReference,
) -> ProfileDescriptor {
    profile.kit = Some(reference);
    profile
}

/// The shared-layer kit each supported profile declares.
///
/// Three states are represented and never conflated: a registered kit with a
/// confirmed set and a floor (`aspnet-web`), a registered kit whose single
/// confirmed unit is the vendored token source (`react-web`, `nextjs-web`),
/// and a **declared zero** naming the missing evidence (`flutter-app`,
/// `rust-web`, `python-service`). The zero is recorded rather than left
/// implicit, and is never reported as a met floor.
fn kit_reference_for(profile: &str) -> crate::kit::registry::KitReference {
    use crate::kit::registry::{
        declared_zero, Ecosystem, FeedKind, FeedRef, KitReference, PLATFORM_FEED_PATH,
        PLATFORM_PACKAGE_VERSION, PLATFORM_TFM, PLATFORM_UI_KIT_VERSION,
    };
    match profile {
        "aspnet-web" => KitReference {
            id: "platform-dotnet".to_string(),
            version: Some(PLATFORM_PACKAGE_VERSION.to_string()),
            ecosystem: Ecosystem::Dotnet,
            feed: Some(FeedRef {
                name: "platform".to_string(),
                kind: FeedKind::Nuget,
                // Relative to the generated project: the feed travels with the
                // repository, so a fresh clone restores at any path with no
                // sibling library and no environment variable.
                path: PLATFORM_FEED_PATH.to_string(),
            }),
            // A net8.0 project cannot reference a net10.0 package. The owner
            // ruled that the workspace baseline is net10.0 and SDK 8 is
            // removed, so the profile renders the TFM its kit requires rather
            // than multi-targeting the library down.
            tfm: Some(PLATFORM_TFM.to_string()),
            minimum_packages: crate::kit::floor::DEFAULT_MINIMUM_PACKAGES,
            zero_reason: None,
        },
        "react-web" | "nextjs-web" => KitReference {
            id: "platform-ui-web".to_string(),
            version: Some(PLATFORM_UI_KIT_VERSION.to_string()),
            ecosystem: Ecosystem::Npm,
            // The token artifacts are vendored as ordinary source, not added
            // as registry dependencies, so the offline `npm run build` /
            // `npm test` contract is preserved and no scaffold acquires a new
            // network requirement to build.
            feed: None,
            tfm: None,
            // One confirmed unit: the digest-pinned vendored token pair.
            minimum_packages: 1,
            zero_reason: None,
        },
        "flutter-app" => declared_zero(
            Ecosystem::Pub,
            "no registered token kit for pub; the flutter-app ui_pattern adapter remains the \
             behaviour layer and no Dart tokens are synthesized",
        ),
        "rust-web" => declared_zero(
            Ecosystem::Cargo,
            "no crate in rust-platform-libs has two external consumers (1 consumer: trailCrew); \
             the 8 crates stay commented-out Cargo.toml entries",
        ),
        _ => declared_zero(Ecosystem::None, "no registered kit for this ecosystem"),
    }
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
        with_kit(
        with_governance(
            descriptor(
                "aspnet-web",
            "0.1.0",
            "adapter-dotnet",
            "csharp",
            "dotnet",
            // Raised with the kit: a net8.0 project cannot reference a
            // net10.0 package, and the workspace baseline is net10.0.
            Some("10.0"),
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
        kit_reference_for("aspnet-web"),
        ),
        with_kit(
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
            "flutter analyze",
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
        kit_reference_for("flutter-app"),
        ),
        with_kit(
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
        kit_reference_for("nextjs-web"),
        ),
        with_kit(
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
            "python3 -m unittest discover -s tests -v",
            Some("container"),
            Some("home-server-01"),
            &["AUTH-001", "PRIVACY-003", "DEPLOY-001"],
            true,
            "Python API service stack",
            ),
            "python-product",
            "product",
        ),
        kit_reference_for("python-service"),
        ),
        with_kit(
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
        kit_reference_for("react-web"),
        ),
        with_kit(
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
        kit_reference_for("rust-web"),
        ),
    ]
}

//! Read-only health and maturity assessment (`doctor-maturity-assessment`).
//!
//! Core owns the typed finding inventory and the evidence-based maturity
//! policy; transports render Core outcomes without reinterpreting them.
//! Every probe is read-only: filesystem checks use metadata/file reads,
//! Git is inspected via argument arrays (never shell), and neither project
//! files nor remotes are modified. Registry access, when the caller
//! supplies it, is limited to a read-only observation used to mark stale
//! results as stale.
//!
//! Finding model (per design): each finding carries a stable rule ID, a
//! `PASS`/`WARN`/`FAIL`/`UNAVAILABLE` status, evidence, applicability and a
//! remediation class (`automatic`, `AI`, `manual`). Unknown or unexecuted
//! checks are reported explicitly and never converted to `PASS`.
//! Configured maturity is an intent, not proof: only evidence grants a
//! level, and stale observations are shown as stale.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::core::manifest::{Manifest, Maturity};
use crate::core::ForgeError;
use crate::policy::{redact_report_in_place, PolicyOutcome, PolicyReport, PolicySeverity};

/// Version of the maturity policy descriptors compiled into this release.
pub const DOCTOR_POLICY_VERSION: &str = "0.1.0";

/// Outcome of one inspection rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingStatus {
    Pass,
    Warn,
    Fail,
    Unavailable,
}

impl std::fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingStatus::Pass => write!(f, "PASS"),
            FindingStatus::Warn => write!(f, "WARN"),
            FindingStatus::Fail => write!(f, "FAIL"),
            FindingStatus::Unavailable => write!(f, "UNAVAILABLE"),
        }
    }
}

/// Who (or what) can apply the fix for a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Remediation {
    Automatic,
    Ai,
    Manual,
}

impl std::fmt::Display for Remediation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Remediation::Automatic => write!(f, "automatic"),
            Remediation::Ai => write!(f, "ai"),
            Remediation::Manual => write!(f, "manual"),
        }
    }
}

/// One typed doctor finding with a stable rule ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub id: String,
    pub status: FindingStatus,
    pub evidence: Vec<String>,
    pub applicable: bool,
    pub remediation: Remediation,
    pub detail: String,
}

impl Finding {
    fn new(
        id: &str,
        status: FindingStatus,
        evidence: Vec<String>,
        applicable: bool,
        remediation: Remediation,
        detail: impl Into<String>,
    ) -> Self {
        Finding {
            id: id.to_string(),
            status,
            evidence,
            applicable,
            remediation,
            detail: detail.into(),
        }
    }
}

/// One maturity control from the versioned policy descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaturityControl {
    pub id: String,
    pub level: String,
    pub description: String,
    pub applicable: bool,
    pub met: bool,
    pub evidence: Vec<String>,
}

/// Read-only registry observation supplied by the caller. `registered`
/// tells whether the project is known; `observed_at` (RFC 3339) is compared
/// against the manifest mtime so stale observations are shown as stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryObservation {
    pub registered: bool,
    pub observed_at: Option<String>,
}

/// Full doctor outcome for one project directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub path: String,
    pub profile: Option<String>,
    pub policy_version: String,
    pub current_maturity: Option<String>,
    pub target_maturity: Option<String>,
    pub findings: Vec<Finding>,
    pub controls: Vec<MaturityControl>,
    pub healthy: bool,
    pub stale: bool,
}

impl DoctorReport {
    /// Findings that deny health: every `FAIL`/`UNAVAILABLE`.
    pub fn blocking_findings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Fail | FindingStatus::Unavailable))
            .collect()
    }

    /// Applicable controls without satisfying evidence.
    pub fn unmet_controls(&self) -> Vec<&MaturityControl> {
        self.controls
            .iter()
            .filter(|c| c.applicable && !c.met)
            .collect()
    }
}

/// Parse an explicit `--target` maturity level (`L0`..`L4`).
pub fn parse_target_level(raw: &str) -> Result<Maturity, ForgeError> {
    match raw {
        "L0" => Ok(Maturity::L0),
        "L1" => Ok(Maturity::L1),
        "L2" => Ok(Maturity::L2),
        "L3" => Ok(Maturity::L3),
        "L4" => Ok(Maturity::L4),
        other => Err(ForgeError::ManifestInvalid {
            path: "<--target>".to_string(),
            reason: format!("invalid maturity '{other}': expected one of L0, L1, L2, L3, L4"),
        }),
    }
}

fn maturity_rank(level: Maturity) -> u8 {
    match level {
        Maturity::L0 => 0,
        Maturity::L1 => 1,
        Maturity::L2 => 2,
        Maturity::L3 => 3,
        Maturity::L4 => 4,
    }
}

fn maturity_name(level: Maturity) -> &'static str {
    match level {
        Maturity::L0 => "L0",
        Maturity::L1 => "L1",
        Maturity::L2 => "L2",
        Maturity::L3 => "L3",
        Maturity::L4 => "L4",
    }
}

fn file_exists(dir: &Path, name: &str) -> bool {
    dir.join(name).is_file()
}

fn dir_has_entries(dir: &Path, name: &str) -> bool {
    match fs::read_dir(dir.join(name)) {
        Ok(mut entries) => entries.any(|e| e.is_ok()),
        Err(_) => false,
    }
}

/// Bounded text scan for dependency/feature markers; binary or oversized
/// files yield an empty scan rather than failing the inspection.
fn scan_text(path: &Path) -> String {
    const MAX_SCAN_BYTES: usize = 128 * 1024;
    let bytes = fs::read(path).unwrap_or_default();
    let len = bytes.len().min(MAX_SCAN_BYTES);
    String::from_utf8_lossy(&bytes[..len]).to_lowercase()
}

fn dependency_text(dir: &Path) -> String {
    let mut text = String::new();
    for file in [
        "Cargo.toml",
        "package.json",
        "pubspec.yaml",
        "pyproject.toml",
        "requirements.txt",
        "setup.py",
        "setup.cfg",
    ] {
        text.push_str(&scan_text(&dir.join(file)));
        text.push('\n');
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_csproj = path
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("csproj"));
            if is_csproj {
                text.push_str(&scan_text(&path));
                text.push('\n');
            }
        }
    }
    text
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Expected dependency-manifest file per MVP profile.
fn expected_build_file(profile: &str) -> Option<&'static str> {
    match profile {
        "rust-web" => Some("Cargo.toml"),
        "nextjs-web" => Some("package.json"),
        "flutter-app" => Some("pubspec.yaml"),
        "aspnet-web" => None,     // any *.csproj
        "python-service" => None, // pyproject.toml or requirements.txt
        _ => None,
    }
}

fn has_build_definition(dir: &Path, profile: &str) -> (bool, Vec<String>) {
    match profile {
        "aspnet-web" => {
            let found = fs::read_dir(dir).map(|entries| {
                entries.flatten().any(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| x.eq_ignore_ascii_case("csproj"))
                })
            });
            match found {
                Ok(true) => (true, vec!["*.csproj".to_string()]),
                _ => (false, vec!["no .csproj file in project root".to_string()]),
            }
        }
        "python-service" => {
            let mut evidence = Vec::new();
            for file in [
                "pyproject.toml",
                "requirements.txt",
                "setup.py",
                "setup.cfg",
            ] {
                if file_exists(dir, file) {
                    evidence.push(file.to_string());
                }
            }
            if evidence.is_empty() {
                (
                    false,
                    vec!["no pyproject.toml, requirements.txt, setup.py or setup.cfg".to_string()],
                )
            } else {
                (true, evidence)
            }
        }
        _ => match expected_build_file(profile) {
            Some(file) if file_exists(dir, file) => (true, vec![file.to_string()]),
            Some(file) => (false, vec![format!("missing build definition '{file}'")]),
            None => (
                false,
                vec!["unknown profile: no expected build definition".to_string()],
            ),
        },
    }
}

fn has_database_markers(deps: &str) -> (bool, Vec<String>) {
    let mut found = Vec::new();
    if contains_any(
        deps,
        &[
            "postgres", "psycopg", "sqlx", "diesel", "npgsql", "typeorm", "pg_",
        ],
    ) {
        found.push("postgresql markers in dependency manifests".to_string());
    }
    if deps.contains("redis") {
        found.push("redis markers in dependency manifests".to_string());
    }
    if found.is_empty() {
        (
            false,
            vec!["no database markers in dependency manifests".to_string()],
        )
    } else {
        (true, found)
    }
}

fn has_auth_markers(deps: &str, dir: &Path) -> (bool, Vec<String>) {
    let file_markers = file_exists(dir, "auth.config.ts")
        || file_exists(dir, "src/auth.ts")
        || file_exists(dir, "src/auth/mod.rs");
    if contains_any(
        deps,
        &[
            "next-auth",
            "passport",
            "auth0",
            "clerk",
            "lucia",
            "axum-login",
            "jsonwebtoken",
            "oauth2",
            "openidconnect",
            "tower-sessions",
            "fastapi-users",
            "flask-login",
            "django-allauth",
            "authlib",
            "python-jose",
            "firebase_auth",
            "flutter_appauth",
            "microsoft.aspnetcore.identity",
            "identityserver",
        ],
    ) || file_markers
    {
        (
            true,
            vec!["auth markers in dependencies or auth files".to_string()],
        )
    } else {
        (
            false,
            vec!["no auth markers in dependencies or auth files".to_string()],
        )
    }
}

fn has_admin_markers(
    deps: &str,
    dir: &Path,
    features: &std::collections::BTreeMap<String, String>,
) -> (bool, Vec<String>) {
    if features.contains_key("admin")
        && (deps.contains("admin")
            || dir.join("src/admin").exists()
            || dir.join("app/admin").exists())
    {
        (
            true,
            vec!["admin markers in dependencies or admin sources".to_string()],
        )
    } else if features.contains_key("admin") {
        (
            false,
            vec!["feature 'admin' declared but no admin markers found".to_string()],
        )
    } else if deps.contains("admin") {
        (true, vec!["admin markers in dependencies".to_string()])
    } else {
        (
            false,
            vec!["no admin markers in dependencies or sources".to_string()],
        )
    }
}

fn has_audit_markers(
    deps: &str,
    features: &std::collections::BTreeMap<String, String>,
) -> (bool, Vec<String>) {
    if deps.contains("audit") {
        (true, vec!["audit markers in dependencies".to_string()])
    } else if features.contains_key("audit") {
        (
            false,
            vec!["feature 'audit' declared but no audit markers found".to_string()],
        )
    } else {
        (false, vec!["no audit markers in dependencies".to_string()])
    }
}

fn has_observability_markers(
    deps: &str,
    features: &std::collections::BTreeMap<String, String>,
) -> (bool, Vec<String>) {
    if contains_any(
        deps,
        &[
            "opentelemetry",
            "tracing",
            "prometheus",
            "telemetry",
            "sentry",
            "applicationinsights",
        ],
    ) {
        (
            true,
            vec!["observability markers in dependencies".to_string()],
        )
    } else if features.contains_key("telemetry") || features.contains_key("analytics") {
        (
            false,
            vec!["telemetry/analytics declared but no observability markers found".to_string()],
        )
    } else {
        (
            false,
            vec!["no observability markers in dependencies".to_string()],
        )
    }
}

fn requests_db(features: &std::collections::BTreeMap<String, String>) -> bool {
    features.keys().any(|k| {
        let k = k.to_lowercase();
        k.contains("postgres")
            || k.contains("redis")
            || k.contains("storage")
            || k.contains("database")
    })
}

/// Git remote probe via argument arrays (never shell). A repository without
/// an `origin` remote is `missing`; a non-repository is `unknown`.
enum GitState {
    Detected(String),
    Missing,
    Unknown,
}

fn probe_git_remote(dir: &Path) -> GitState {
    let remote = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(url) = remote {
        return GitState::Detected(url);
    }
    let is_repo = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--git-dir")
        .output()
        .is_ok_and(|out| out.status.success());
    if is_repo {
        GitState::Missing
    } else {
        GitState::Unknown
    }
}

fn detect_ci(dir: &Path) -> (bool, Vec<String>) {
    let mut found = Vec::new();
    let mut evidence = Vec::new();
    if dir_has_entries(dir, ".github/workflows") {
        found.push("github-actions");
        evidence.push(".github/workflows/".to_string());
    }
    for (file, provider) in [
        (".gitlab-ci.yml", "gitlab-ci"),
        ("Jenkinsfile", "jenkins"),
        ("azure-pipelines.yml", "azure-pipelines"),
        (".circleci/config.yml", "circleci"),
        ("buildspec.yml", "codebuild"),
    ] {
        if file_exists(dir, file) {
            found.push(provider);
            evidence.push(file.to_string());
        }
    }
    if found.is_empty() {
        (false, vec!["no ci configuration".to_string()])
    } else {
        (
            true,
            vec![format!("{} ({})", found.join("+"), evidence.join(", "))],
        )
    }
}

fn detect_driftwatch(dir: &Path) -> (bool, Vec<String>) {
    let mut evidence = Vec::new();
    for file in [
        "driftwatch.yaml",
        "driftwatch.yml",
        "driftwatch.json",
        ".driftwatch.yaml",
        ".driftwatch.yml",
    ] {
        if file_exists(dir, file) {
            evidence.push(file.to_string());
        }
    }
    if dir.join(".driftwatch").is_dir() {
        evidence.push(".driftwatch/".to_string());
    }
    if evidence.is_empty() {
        (false, vec!["no driftwatch configuration".to_string()])
    } else {
        (true, evidence)
    }
}

fn detect_deployment(dir: &Path) -> (bool, Vec<String>) {
    let mut found = Vec::new();
    let mut evidence = Vec::new();
    if file_exists(dir, "Dockerfile") || file_exists(dir, "Containerfile") {
        found.push("container");
        evidence.push("Dockerfile".to_string());
    }
    for (file, provider) in [
        ("fly.toml", "fly.io"),
        ("vercel.json", "vercel"),
        ("render.yaml", "render"),
        ("render.yml", "render"),
    ] {
        if file_exists(dir, file) {
            found.push(provider);
            evidence.push(file.to_string());
        }
    }
    if dir.join("android").is_dir() && dir.join("ios").is_dir() {
        found.push("app-store");
        evidence.push("android/+ios/".to_string());
    }
    if found.is_empty() {
        (false, vec!["no deployment configuration".to_string()])
    } else {
        (
            true,
            vec![format!("{} ({})", found.join("+"), evidence.join(", "))],
        )
    }
}

fn detect_docs(dir: &Path, manifest: &Manifest) -> (bool, Vec<String>) {
    let mut evidence = Vec::new();
    for file in ["README.md", "README", "readme.md", "docs/README.md"] {
        if file_exists(dir, file) {
            evidence.push(file.to_string());
        }
    }
    if manifest.docs.is_some() {
        evidence.push("manifest docs section".to_string());
    }
    if evidence.is_empty() {
        (
            false,
            vec!["no readme or manifest docs section".to_string()],
        )
    } else {
        (true, evidence)
    }
}

fn marker_files_present(dir: &Path, names: &[&str], dir_markers: &[&str]) -> (bool, Vec<String>) {
    let mut evidence = Vec::new();
    for name in names {
        if file_exists(dir, name) {
            evidence.push((*name).to_string());
        }
    }
    for name in dir_markers {
        if dir.join(name).is_dir() {
            evidence.push(format!("{name}/"));
        }
    }
    if evidence.is_empty() {
        (false, vec![format!("none of {} present", names.join(", "))])
    } else {
        (true, evidence)
    }
}

fn manifest_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(dir.join("forge.yaml"))
        .and_then(|m| m.modified())
        .ok()
}

fn observation_is_stale(observed_at: &str, mtime: Option<std::time::SystemTime>) -> bool {
    let mtime = match mtime {
        Some(t) => t,
        None => return false,
    };
    let observed = match chrono::DateTime::parse_from_rfc3339(observed_at) {
        Ok(dt) => dt,
        Err(_) => return false,
    };
    let mtime_chrono: chrono::DateTime<chrono::Utc> = mtime.into();
    mtime_chrono > observed.with_timezone(&chrono::Utc)
}

/// Run the read-only doctor inspection over a project directory.
///
/// Manifest IO failures (missing/invalid/ambiguous/legacy/unsupported) are
/// hard errors; everything else — including an unknown profile — is
/// reported as findings so the caller sees evidence instead of a refusal.
/// `policy_outcome` is the live DriftWatch result for this project; when
/// supplied, doctor normalizes its findings into the typed inventory and
/// flags the absence of evidence as `unavailable`.
pub fn run_doctor(
    dir: &Path,
    target_override: Option<Maturity>,
    observation: Option<&RegistryObservation>,
    policy_outcome: Option<&PolicyOutcome>,
) -> Result<DoctorReport, ForgeError> {
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: dir.display().to_string(),
        });
    }
    let (manifest, _) = Manifest::load_from_dir(dir, None)?;

    let profile_known = crate::profile::inspect_profile(&manifest.project.profile).is_ok();
    let descriptor = crate::profile::inspect_profile(&manifest.project.profile).ok();

    let requested: Vec<String> = manifest.features.keys().cloned().collect();
    let features_compatible = if profile_known {
        crate::profile::resolve_profile(&manifest.project.profile, &requested).is_ok()
    } else {
        false
    };

    let deps = dependency_text(dir);
    let current = manifest.project.maturity;
    let manifest_target = manifest.project.target_maturity;
    let target = target_override.or(manifest_target).unwrap_or(Maturity::L1);

    let mut findings: Vec<Finding> = Vec::new();

    findings.push(Finding::new(
        "manifest-valid",
        FindingStatus::Pass,
        vec!["forge.yaml parses against schema 1".to_string()],
        true,
        Remediation::Manual,
        "manifest is present and schema-valid",
    ));

    if profile_known {
        findings.push(Finding::new(
            "profile-known",
            FindingStatus::Pass,
            vec![format!(
                "profile '{}' matches a versioned MVP descriptor",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "profile is known",
        ));
    } else {
        findings.push(Finding::new(
            "profile-known",
            FindingStatus::Fail,
            vec![format!(
                "profile '{}' matches no MVP profile descriptor",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "unknown profile: descriptor-gated checks are unavailable",
        ));
    }

    if !profile_known {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Unavailable,
            vec!["profile descriptor is unknown; compatibility cannot be evaluated".to_string()],
            true,
            Remediation::Manual,
            "required inspector (profile descriptor) cannot run",
        ));
    } else if features_compatible {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Pass,
            vec![if requested.is_empty() {
                "no capabilities requested".to_string()
            } else {
                format!("capabilities compatible: {}", requested.join(", "))
            }],
            true,
            Remediation::Manual,
            "requested capabilities are compatible with the profile",
        ));
    } else {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Fail,
            vec![format!(
                "capabilities incompatible with profile '{}'",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "requested capabilities are incompatible with the profile",
        ));
    }

    // Dependency drift: manifest runtime language versus on-disk evidence.
    if !profile_known {
        findings.push(Finding::new(
            "dependency-drift",
            FindingStatus::Unavailable,
            vec!["profile descriptor is unknown; drift cannot be evaluated".to_string()],
            true,
            Remediation::Manual,
            "required inspector (profile descriptor) cannot run",
        ));
    } else {
        let profile = manifest.project.profile.as_str();
        let expected_lang = descriptor
            .as_ref()
            .map(|d| d.language.as_str())
            .unwrap_or("");
        let manifest_lang = manifest
            .runtime
            .as_ref()
            .and_then(|r| r.language.clone())
            .unwrap_or_default();
        let mut drift: Vec<String> = Vec::new();
        if !manifest_lang.is_empty() && manifest_lang != expected_lang {
            drift.push(format!(
                "manifest runtime language '{manifest_lang}' disagrees with profile language '{expected_lang}'"
            ));
        }
        let (build_present, build_evidence) = has_build_definition(dir, profile);
        if !build_present {
            drift.push(build_evidence.join("; "));
        }
        if drift.is_empty() {
            findings.push(Finding::new(
                "dependency-drift",
                FindingStatus::Pass,
                vec!["manifest runtime agrees with on-disk dependency evidence".to_string()],
                true,
                Remediation::Automatic,
                "no dependency drift detected",
            ));
        } else {
            findings.push(Finding::new(
                "dependency-drift",
                FindingStatus::Warn,
                drift,
                true,
                Remediation::Automatic,
                "dependency evidence disagrees with the manifest",
            ));
        }
    }

    // Build / deployment configuration.
    if profile_known {
        let (present, evidence) = has_build_definition(dir, &manifest.project.profile);
        findings.push(Finding::new(
            "build-config",
            if present {
                FindingStatus::Pass
            } else {
                FindingStatus::Fail
            },
            evidence,
            true,
            Remediation::Automatic,
            if present {
                "build definition is present"
            } else {
                "build definition required by L1 is missing"
            },
        ));
    } else {
        findings.push(Finding::new(
            "build-config",
            FindingStatus::Unavailable,
            vec![
                "profile descriptor is unknown; build expectations cannot be evaluated".to_string(),
            ],
            true,
            Remediation::Automatic,
            "required inspector (profile descriptor) cannot run",
        ));
    }

    let (deploy_present, deploy_evidence) = detect_deployment(dir);
    findings.push(Finding::new(
        "deployment-config",
        if deploy_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        deploy_evidence,
        true,
        Remediation::Manual,
        if deploy_present {
            "deployment configuration is present"
        } else {
            "no deployment configuration; required for L2 deployment"
        },
    ));

    // Repository: detected / missing / unknown (unknown is never PASS).
    match probe_git_remote(dir) {
        GitState::Detected(url) => findings.push(Finding::new(
            "repository",
            FindingStatus::Pass,
            vec![format!("git remote origin: {url}")],
            true,
            Remediation::Manual,
            "git remote is configured",
        )),
        GitState::Missing => findings.push(Finding::new(
            "repository",
            FindingStatus::Warn,
            vec!["git repository has no origin remote".to_string()],
            true,
            Remediation::Manual,
            "repository exists but no origin remote is configured",
        )),
        GitState::Unknown => findings.push(Finding::new(
            "repository",
            FindingStatus::Unavailable,
            vec!["not a git repository; remote cannot be determined".to_string()],
            true,
            Remediation::Manual,
            "required inspector (git repository) cannot run",
        )),
    }

    let (ci_present, ci_evidence) = detect_ci(dir);
    findings.push(Finding::new(
        "ci-config",
        if ci_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        ci_evidence,
        true,
        Remediation::Automatic,
        if ci_present {
            "ci configuration is present"
        } else {
            "no ci configuration; required for L2 ci"
        },
    ));

    let (docs_present, docs_evidence) = detect_docs(dir, &manifest);
    findings.push(Finding::new(
        "docs-present",
        if docs_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        docs_evidence,
        true,
        Remediation::Ai,
        if docs_present {
            "documentation evidence is present"
        } else {
            "no readme or manifest docs section"
        },
    ));

    // Derivative documentation freshness. Read-only: the
    // recorded source hash per enabled locale is compared
    // against the current source so stale, missing or
    // needs-review derivatives surface with the
    // `forge docs translate` recovery. Disabled locales are
    // skipped entirely (automatic-workflow boundary).
    findings.extend(docs_freshness_findings(dir, &manifest));

    let (dw_present, dw_evidence) = detect_driftwatch(dir);
    findings.push(Finding::new(
        "driftwatch-config",
        if dw_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        dw_evidence,
        true,
        Remediation::Automatic,
        if dw_present {
            "driftwatch configuration is present"
        } else {
            "no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)"
        },
    ));

    // DriftWatch policy findings. The execution path is delegated to
    // the configured adapter (see `policy::run_driftwatch`); doctor maps
    // each finding to the typed inventory and reports a `pass`/`warn`/
    // `fail`/`unavailable` finding while retaining the original rule ID,
    // category, severity, tool version and redacted evidence.
    findings.extend(policy_findings(dir, policy_outcome));

    // Registry observation: registered + fresh / stale / not registered.
    let mtime = manifest_mtime(dir);
    let mut stale = false;
    match observation {
        Some(obs) if obs.registered => {
            let is_stale = obs
                .observed_at
                .as_deref()
                .map(|at| observation_is_stale(at, mtime))
                .unwrap_or(false);
            stale = is_stale;
            if is_stale {
                findings.push(Finding::new(
                    "registry-observation",
                    FindingStatus::Warn,
                    vec![format!(
                        "registry observation {} predates the current forge.yaml; shown as stale",
                        obs.observed_at.as_deref().unwrap_or("unknown")
                    )],
                    true,
                    Remediation::Automatic,
                    "stale observation: re-run forge register to refresh",
                ));
            } else {
                findings.push(Finding::new(
                    "registry-observation",
                    FindingStatus::Pass,
                    vec!["registry observation is current".to_string()],
                    true,
                    Remediation::Automatic,
                    "project is registered and the observation is fresh",
                ));
            }
        }
        _ => {
            findings.push(Finding::new(
                "registry-observation",
                FindingStatus::Warn,
                vec!["project is not registered; maturity evidence is local-only".to_string()],
                true,
                Remediation::Automatic,
                "project is not registered",
            ));
        }
    }

    let controls = assess_maturity(
        dir,
        &manifest,
        descriptor.as_ref(),
        &deps,
        target,
        observation,
    );

    let unmet = controls.iter().filter(|c| c.applicable && !c.met).count();
    if unmet == 0 {
        findings.push(Finding::new(
            "maturity-requirements",
            FindingStatus::Pass,
            vec![format!(
                "all applicable controls for target {} are met",
                maturity_name(target)
            )],
            true,
            Remediation::Manual,
            "maturity requirements satisfied for the target level",
        ));
    } else {
        findings.push(Finding::new(
            "maturity-requirements",
            FindingStatus::Fail,
            vec![format!(
                "{unmet} applicable control(s) for target {} lack evidence",
                maturity_name(target)
            )],
            true,
            Remediation::Manual,
            "maturity requirements are not satisfied for the target level",
        ));
    }

    let healthy = findings
        .iter()
        .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
        && unmet == 0
        && !stale;

    Ok(DoctorReport {
        path: dir.display().to_string(),
        profile: Some(manifest.project.profile.clone()),
        policy_version: DOCTOR_POLICY_VERSION.to_string(),
        current_maturity: current.map(|m| m.to_string()),
        target_maturity: Some(maturity_name(target).to_string()),
        findings,
        controls,
        healthy,
        stale,
    })
}

fn control(
    id: &str,
    level: Maturity,
    description: &str,
    applicable: bool,
    met: bool,
    evidence: Vec<String>,
) -> MaturityControl {
    MaturityControl {
        id: id.to_string(),
        level: maturity_name(level).to_string(),
        description: description.to_string(),
        applicable,
        met: met && applicable,
        evidence,
    }
}

/// Evidence-based maturity assessment against the target level. Controls
/// above the target are recorded as nonapplicable rather than forced;
/// database/auth-gated controls are nonapplicable when the project does
/// not request them, so L0 prototypes are respected.
fn assess_maturity(
    dir: &Path,
    manifest: &Manifest,
    descriptor: Option<&crate::profile::ProfileDescriptor>,
    deps: &str,
    target: Maturity,
    observation: Option<&RegistryObservation>,
) -> Vec<MaturityControl> {
    let rank = maturity_rank(target);
    let at_least = |level: Maturity| rank >= maturity_rank(level);
    let profile_id = manifest.project.profile.as_str();
    let capabilities: Vec<&str> = descriptor
        .map(|d| d.capabilities.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let supports = |cap: &str| capabilities.contains(&cap);

    let (build_present, build_evidence) = descriptor
        .map(|_| has_build_definition(dir, profile_id))
        .unwrap_or((false, vec!["unknown profile".to_string()]));
    let structure_met = build_present && file_exists(dir, "forge.yaml");
    let structure_evidence = if structure_met {
        vec!["forge.yaml and profile build definition are present".to_string()]
    } else {
        build_evidence.clone()
    };

    let want_db = requests_db(&manifest.features);
    let db_applicable = at_least(Maturity::L1)
        && want_db
        && descriptor.map(|d| d.requires_database).unwrap_or(false);
    let (db_met, db_evidence) = has_database_markers(deps);

    let want_logging = manifest.features.contains_key("telemetry")
        || manifest.features.contains_key("analytics")
        || manifest.features.contains_key("logging");
    let (obs_met, obs_evidence) = has_observability_markers(deps, &manifest.features);

    let want_health = manifest.features.contains_key("health-check");
    let health_met =
        deps.contains("health") || dir.join("src/health").exists() || want_health && build_present;
    let health_evidence =
        if health_met && (deps.contains("health") || dir.join("src/health").exists()) {
            vec!["health markers found".to_string()]
        } else if want_health && build_present {
            vec![
                "health-check declared; build definition present but no health markers found"
                    .to_string(),
            ]
        } else {
            vec!["no health markers found".to_string()]
        };

    let (auth_met, auth_evidence) = has_auth_markers(deps, dir);
    let (admin_met, admin_evidence) = has_admin_markers(deps, dir, &manifest.features);
    let (audit_met, audit_evidence) = has_audit_markers(deps, &manifest.features);
    let (ci_present, ci_evidence) = detect_ci(dir);
    let (dw_present, dw_evidence) = detect_driftwatch(dir);
    let (deploy_present, deploy_evidence) = detect_deployment(dir);

    let registered = observation.map(|o| o.registered).unwrap_or(false);
    let distribution_met = manifest
        .distribution
        .as_ref()
        .is_some_and(|d| d.primary.is_some() || !d.mirrors.is_empty());
    let distribution_evidence = match &manifest.distribution {
        Some(d) if d.primary.is_some() || !d.mirrors.is_empty() => {
            vec!["manifest distribution declares primary or mirrors".to_string()]
        }
        _ => vec!["manifest declares no distribution primary or mirrors".to_string()],
    };
    let release_evidence = marker_files_present(
        dir,
        &["CHANGELOG.md", "RELEASES.md", "release.yaml", "release.yml"],
        &[],
    );
    let backup_evidence = marker_files_present(
        dir,
        &["BACKUP.md", "backup.yaml", "backup.yml"],
        &["backup"],
    );
    let recovery_evidence = marker_files_present(
        dir,
        &[
            "RECOVERY.md",
            "DISASTER_RECOVERY.md",
            "recovery.yaml",
            "restore.sh",
        ],
        &["recovery", "restore"],
    );
    let monitoring_evidence = marker_files_present(
        dir,
        &[
            "prometheus.yml",
            "grafana.yaml",
            "monitoring.yaml",
            "MONITORING.md",
        ],
        &["monitoring", "grafana"],
    );
    let security_evidence = marker_files_present(dir, &["SECURITY.md", "security.yaml"], &[]);
    let privacy_evidence = marker_files_present(dir, &["PRIVACY.md", "privacy.yaml"], &[]);
    let secrets_evidence = marker_files_present(
        dir,
        &[".sops.yaml", ".secrets.yaml", "vault.yaml", "SECRETS.md"],
        &["secrets", "sealed-secrets"],
    );
    let alerts_evidence = marker_files_present(
        dir,
        &["alertmanager.yml", "alerts.yaml", "ALERTS.md"],
        &["alerts"],
    );

    // Upgrade evidence: version-controlled automation that can repeat a
    // migration (git remote + CI). Recorded explicitly, never inferred.
    let upgrades_met = matches!(probe_git_remote(dir), GitState::Detected(_)) && ci_present;
    let upgrades_evidence = if upgrades_met {
        vec!["git remote and ci configuration provide a repeatable upgrade path".to_string()]
    } else {
        vec!["no combined git-remote and ci evidence for repeatable upgrades".to_string()]
    };

    vec![
        control(
            "L1-configuration",
            Maturity::L1,
            "manifest is schema-valid (L1 configuration)",
            at_least(Maturity::L1),
            true,
            vec!["forge.yaml parses against schema 1".to_string()],
        ),
        control(
            "L1-structure",
            Maturity::L1,
            "project structure matches the profile layout (L1 structure)",
            at_least(Maturity::L1),
            structure_met,
            structure_evidence,
        ),
        control(
            "L1-database",
            Maturity::L1,
            "database evidence where a database is requested (L1 database)",
            db_applicable,
            db_met,
            db_evidence,
        ),
        control(
            "L1-logging",
            Maturity::L1,
            "logging/telemetry evidence where requested (L1 logging)",
            at_least(Maturity::L1) && want_logging,
            obs_met,
            obs_evidence.clone(),
        ),
        control(
            "L1-health",
            Maturity::L1,
            "health evidence where a health check is requested (L1 health)",
            at_least(Maturity::L1) && want_health,
            health_met && (deps.contains("health") || dir.join("src/health").exists()),
            health_evidence,
        ),
        control(
            "L1-build",
            Maturity::L1,
            "build definition is present (L1 build definition)",
            at_least(Maturity::L1),
            build_present,
            build_evidence,
        ),
        control(
            "L2-auth",
            Maturity::L2,
            "auth evidence where the profile supports auth (L2 auth)",
            at_least(Maturity::L2) && supports("auth"),
            auth_met,
            auth_evidence,
        ),
        control(
            "L2-admin",
            Maturity::L2,
            "admin evidence where the profile supports admin (L2 admin)",
            at_least(Maturity::L2) && supports("admin"),
            admin_met,
            admin_evidence,
        ),
        control(
            "L2-ci",
            Maturity::L2,
            "continuous integration is configured (L2 CI)",
            at_least(Maturity::L2),
            ci_present,
            ci_evidence,
        ),
        control(
            "L2-driftwatch",
            Maturity::L2,
            "driftwatch is configured; execution belongs to v0.3 (L2 DriftWatch)",
            at_least(Maturity::L2),
            dw_present,
            dw_evidence,
        ),
        control(
            "L2-deployment",
            Maturity::L2,
            "deployment automation is configured (L2 deployment)",
            at_least(Maturity::L2),
            deploy_present,
            deploy_evidence,
        ),
        control(
            "L2-audit",
            Maturity::L2,
            "audit evidence where the profile supports audit (L2 audit)",
            at_least(Maturity::L2) && supports("audit"),
            audit_met,
            audit_evidence,
        ),
        control(
            "L3-identity-compat",
            Maturity::L3,
            "identity compatibility evidence where auth applies (L3 identity compatibility)",
            at_least(Maturity::L3) && supports("auth"),
            auth_met,
            vec!["same evidence as L2-auth".to_string()],
        ),
        control(
            "L3-observability",
            Maturity::L3,
            "observability evidence (L3 observability)",
            at_least(Maturity::L3),
            obs_met,
            obs_evidence,
        ),
        control(
            "L3-release",
            Maturity::L3,
            "release evidence such as a changelog (L3 release)",
            at_least(Maturity::L3),
            release_evidence.0,
            release_evidence.1,
        ),
        control(
            "L3-registry",
            Maturity::L3,
            "project is registered (L3 registry)",
            at_least(Maturity::L3),
            registered,
            vec![if registered {
                "project is registered".to_string()
            } else {
                "project is not registered".to_string()
            }],
        ),
        control(
            "L3-distribution",
            Maturity::L3,
            "distribution primary or mirrors declared (L3 distribution)",
            at_least(Maturity::L3),
            distribution_met,
            distribution_evidence,
        ),
        control(
            "L3-upgrades",
            Maturity::L3,
            "repeatable upgrade path via git remote and CI (L3 upgrades)",
            at_least(Maturity::L3),
            upgrades_met,
            upgrades_evidence,
        ),
        control(
            "L4-backup",
            Maturity::L4,
            "backup evidence (L4 backup)",
            at_least(Maturity::L4),
            backup_evidence.0,
            backup_evidence.1,
        ),
        control(
            "L4-recovery",
            Maturity::L4,
            "recovery evidence (L4 recovery)",
            at_least(Maturity::L4),
            recovery_evidence.0,
            recovery_evidence.1,
        ),
        control(
            "L4-monitoring",
            Maturity::L4,
            "monitoring evidence (L4 monitoring)",
            at_least(Maturity::L4),
            monitoring_evidence.0,
            monitoring_evidence.1,
        ),
        control(
            "L4-security",
            Maturity::L4,
            "security policy evidence (L4 security)",
            at_least(Maturity::L4),
            security_evidence.0,
            security_evidence.1,
        ),
        control(
            "L4-privacy",
            Maturity::L4,
            "privacy policy evidence (L4 privacy)",
            at_least(Maturity::L4),
            privacy_evidence.0,
            privacy_evidence.1,
        ),
        control(
            "L4-secrets",
            Maturity::L4,
            "secrets management evidence (L4 secrets)",
            at_least(Maturity::L4),
            secrets_evidence.0,
            secrets_evidence.1,
        ),
        control(
            "L4-alerts",
            Maturity::L4,
            "alerting evidence (L4 alerts)",
            at_least(Maturity::L4),
            alerts_evidence.0,
            alerts_evidence.1,
        ),
    ]
}

/// Render a report for human CLI output.
pub fn render_report_human(report: &DoctorReport) -> String {
    let mut lines = vec![
        format!("doctor: {}", report.path),
        format!(
            "profile: {}",
            report.profile.as_deref().unwrap_or("unknown")
        ),
        format!(
            "maturity: current {} -> target {} (policy {})",
            report.current_maturity.as_deref().unwrap_or("unknown"),
            report.target_maturity.as_deref().unwrap_or("unknown"),
            report.policy_version,
        ),
        format!(
            "verdict: {}",
            if report.healthy {
                "healthy"
            } else if report.stale {
                "stale (re-run forge register to refresh)"
            } else {
                "not healthy"
            }
        ),
        String::new(),
        "Findings:".to_string(),
    ];
    for finding in &report.findings {
        lines.push(format!(
            "  [{}] {} ({})",
            finding.status, finding.id, finding.remediation
        ));
        for evidence in &finding.evidence {
            lines.push(format!("    evidence: {evidence}"));
        }
        lines.push(format!("    detail: {}", finding.detail));
        if !finding.applicable {
            lines.push("    applicability: not applicable".to_string());
        }
    }
    lines.push(String::new());
    lines.push("Maturity controls:".to_string());
    for c in &report.controls {
        let state = if !c.applicable {
            "N/A"
        } else if c.met {
            "met"
        } else {
            "UNMET"
        };
        lines.push(format!(
            "  [{state}] {} ({}): {}",
            c.id, c.level, c.description
        ));
        for evidence in &c.evidence {
            lines.push(format!("    evidence: {evidence}"));
        }
    }
    lines.join("\n")
}

/// Convert a DriftWatch policy outcome into one or more typed findings
/// while preserving the original rule ID, category, severity, tool
/// version and (already redacted) evidence. A missing binary, non-zero
/// exit, timeout or unparseable payload all surface as a single
/// `driftwatch-policy` finding with status `unavailable`; never as
/// `pass`. Findings reported with `applicable == false` are surfaced
/// separately so a future detector does not invent a result.
fn policy_findings(dir: &Path, outcome: Option<&PolicyOutcome>) -> Vec<Finding> {
    let mut out = Vec::new();
    let Some(outcome) = outcome else {
        return out;
    };
    match outcome {
        PolicyOutcome::Unavailable { reason } => {
            out.push(Finding::new(
                "driftwatch-policy",
                FindingStatus::Unavailable,
                vec![format!("adapter did not produce a report: {reason}")],
                true,
                Remediation::Manual,
                "delegated policy execution is unavailable; see evidence for the reason",
            ));
        }
        PolicyOutcome::Reported(report) => {
            out.extend(reported_policy_findings(dir, report));
        }
    }
    out
}

fn reported_policy_findings(dir: &Path, report: &PolicyReport) -> Vec<Finding> {
    let mut safe_report = report.clone();
    // Defense in depth: every evidence line passes through the
    // redaction pipeline even if the adapter already ran it, so a
    // report constructed by a different caller cannot leak
    // credentials into storage or display.
    redact_report_in_place(&mut safe_report);
    let report = &safe_report;
    let mut out = Vec::new();
    let tool_version = report.tool_version.clone();
    let stale = observation_is_stale_from_report(dir, report);
    for pf in &report.findings {
        if !pf.applicable {
            out.push(Finding::new(
                format!("driftwatch-{}", pf.id).as_str(),
                FindingStatus::Pass,
                vec![format!(
                    "tool {} {} reports policy not applicable: {}",
                    report.tool,
                    tool_version,
                    pf.reason.clone().unwrap_or_else(|| pf.message.clone())
                )],
                false,
                Remediation::Manual,
                "policy is not applicable to this profile; reason preserved",
            ));
            continue;
        }
        let status = match pf.severity {
            PolicySeverity::Pass => FindingStatus::Pass,
            PolicySeverity::Warn => FindingStatus::Warn,
            PolicySeverity::Fail => FindingStatus::Fail,
        };
        let mut evidence: Vec<String> = pf
            .evidence
            .iter()
            .map(|line| format!("{} {}: {}", report.tool, tool_version, line))
            .collect();
        if stale {
            evidence.push(format!(
                "observation source is older than current {}; stale",
                crate::core::manifest::CANONICAL_MANIFEST
            ));
        }
        out.push(Finding::new(
            format!("driftwatch-{}", pf.id).as_str(),
            if stale && matches!(status, FindingStatus::Pass) {
                FindingStatus::Warn
            } else {
                status
            },
            evidence,
            true,
            Remediation::Manual,
            format!(
                "[{}:{}] {}",
                pf.category,
                pf.severity.severity_label(),
                pf.message
            ),
        ));
    }
    // Aggregate rollup so the doctor verdict reflects DriftWatch's
    // overall outcome, matching the typed finding contract.
    let rollup = aggregate_rollup(report, stale);
    out.push(Finding::new(
        "driftwatch-policy",
        rollup.status,
        rollup.evidence,
        true,
        Remediation::Manual,
        rollup.detail,
    ));
    out
}

struct Rollup {
    status: FindingStatus,
    evidence: Vec<String>,
    detail: String,
}

fn aggregate_rollup(report: &PolicyReport, stale: bool) -> Rollup {
    let mut applicable = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;
    let mut pass = 0usize;
    for pf in &report.findings {
        if !pf.applicable {
            continue;
        }
        applicable += 1;
        match pf.severity {
            PolicySeverity::Pass => pass += 1,
            PolicySeverity::Warn => warn += 1,
            PolicySeverity::Fail => fail += 1,
        }
    }
    let detail = format!(
        "driftwatch {} (contract {}) reported {} finding(s): {} pass, {} warn, {} fail",
        report.tool_version,
        report.contract,
        report.findings.len(),
        pass,
        warn,
        fail
    );
    let status = if fail > 0 {
        FindingStatus::Fail
    } else if warn > 0 || stale {
        FindingStatus::Warn
    } else if applicable == 0 {
        FindingStatus::Unavailable
    } else {
        FindingStatus::Pass
    };
    let evidence = if stale {
        vec![format!(
            "driftwatch {} reported at {}; source revision older than current {}",
            report.tool,
            report
                .source_revision
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string()),
            crate::core::manifest::CANONICAL_MANIFEST
        )]
    } else {
        vec![format!(
            "driftwatch {} reported {} finding(s) ({} pass, {} warn, {} fail)",
            report.tool,
            report.findings.len(),
            pass,
            warn,
            fail
        )]
    };
    Rollup {
        status,
        evidence,
        detail,
    }
}

/// Convert the read-only docs freshness assessment into typed
/// findings. Each enabled locale gets a stable `docs-<locale>`
/// rule: `pass` when the recorded source hash matches, `warn`
/// when the derivative is stale, never translated or flagged
/// `needs-review`, and `fail` when the locale is misconfigured
/// (missing source, escaping paths). Disabled locales are
/// skipped entirely so automatic workflows never touch them.
/// A `docs-freshness` rollup carries the worst status; it is
/// present only when at least one locale is enabled.
fn docs_freshness_findings(dir: &Path, manifest: &Manifest) -> Vec<Finding> {
    let assessments = match crate::docs::assess_freshness(dir, manifest) {
        Ok(a) => a,
        Err(err) => {
            return vec![Finding::new(
                "docs-freshness",
                FindingStatus::Fail,
                vec![format!("docs configuration is invalid: {err}")],
                true,
                Remediation::Manual,
                "docs configuration is invalid; fix `docs` in forge.yaml",
            )];
        }
    };
    if assessments.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut pass = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;
    for assessment in &assessments {
        let id = format!("docs-{}", assessment.locale);
        let mut evidence = vec![format!(
            "locale `{}` status: {}",
            assessment.locale,
            assessment.status_label()
        )];
        if let Some(hash) = &assessment.source_hash {
            evidence.push(format!("source_hash: {hash}"));
        }
        if let Some(derivative) = &assessment.derivative {
            evidence.push(format!("derivative: {derivative}"));
        }
        let (status, remediation, detail) = match &assessment.status {
            crate::docs::FreshnessStatus::Current { review } => {
                pass += 1;
                (
                    FindingStatus::Pass,
                    Remediation::Ai,
                    format!(
                        "derivative for locale `{}` is current (review: {})",
                        assessment.locale,
                        review.label()
                    ),
                )
            }
            crate::docs::FreshnessStatus::Stale => {
                warn += 1;
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "source changed since locale `{}` was translated; re-run `forge docs translate {}`",
                        assessment.locale, assessment.locale
                    ),
                )
            }
            crate::docs::FreshnessStatus::NeverTranslated => {
                warn += 1;
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "locale `{}` is enabled but has no derivative; run `forge docs translate {}`",
                        assessment.locale, assessment.locale
                    ),
                )
            }
            crate::docs::FreshnessStatus::NeedsReview { reasons } => {
                warn += 1;
                for reason in reasons {
                    evidence.push(format!("review-reason: {reason}"));
                }
                (
                    FindingStatus::Warn,
                    Remediation::Ai,
                    format!(
                        "derivative for locale `{}` needs review: {}",
                        assessment.locale,
                        if reasons.is_empty() {
                            "unspecified".to_string()
                        } else {
                            reasons.join("; ")
                        }
                    ),
                )
            }
            crate::docs::FreshnessStatus::Misconfigured { reason } => {
                fail += 1;
                (
                    FindingStatus::Fail,
                    Remediation::Manual,
                    format!("locale `{}` is misconfigured: {reason}", assessment.locale),
                )
            }
        };
        out.push(Finding::new(
            id.as_str(),
            status,
            evidence,
            true,
            remediation,
            detail,
        ));
    }
    let status = if fail > 0 {
        FindingStatus::Fail
    } else if warn > 0 {
        FindingStatus::Warn
    } else {
        FindingStatus::Pass
    };
    out.push(Finding::new(
        "docs-freshness",
        status,
        vec![format!(
            "translation locales: {} pass, {warn} warn, {fail} fail",
            pass
        )],
        true,
        Remediation::Ai,
        if fail > 0 {
            "one or more translation locales are misconfigured".to_string()
        } else if warn > 0 {
            "one or more derivatives are stale, missing or need review".to_string()
        } else {
            "all enabled translation locales are current".to_string()
        },
    ));
    out
}

fn observation_is_stale_from_report(dir: &Path, report: &PolicyReport) -> bool {
    let Some(revision) = report.source_revision else {
        return false;
    };
    let current = manifest_mtime(dir);
    let Some(current) = current else {
        return false;
    };
    let now_secs = current
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let now = chrono::DateTime::<chrono::Utc>::from_timestamp(now_secs, 0);
    match now {
        Some(now) => now > revision,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PolicyFinding;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    fn rust_manifest(id: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        )
    }

    fn no_registry() -> Option<RegistryObservation> {
        None
    }

    #[test]
    fn reports_stable_findings_for_missing_config_and_drift() {
        let tmp = TempDir::new().unwrap();
        // Manifest claims rust but no Cargo.toml: drift + build failure.
        write(tmp.path(), "forge.yaml", &rust_manifest("drift-proj"));
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let ids: Vec<&str> = report.findings.iter().map(|f| f.id.as_str()).collect();
        for expected in [
            "manifest-valid",
            "profile-known",
            "features-compatible",
            "dependency-drift",
            "build-config",
            "deployment-config",
            "repository",
            "ci-config",
            "docs-present",
            "driftwatch-config",
            "registry-observation",
            "maturity-requirements",
        ] {
            assert!(
                ids.contains(&expected),
                "missing finding {expected}: {ids:?}"
            );
        }
        let drift = report
            .findings
            .iter()
            .find(|f| f.id == "dependency-drift")
            .unwrap();
        assert_eq!(drift.status, FindingStatus::Warn);
        assert!(!drift.evidence.is_empty());
        let build = report
            .findings
            .iter()
            .find(|f| f.id == "build-config")
            .unwrap();
        assert_eq!(build.status, FindingStatus::Fail);
        assert!(!report.healthy);
    }

    #[test]
    fn docs_freshness_warns_for_never_translated_enabled_locale() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: docs-proj\n  name: docs-proj\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\ndocs:\n  source_language: en\n  translations:\n    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
        );
        write(tmp.path(), "README.md", "Hello.\n");
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let ids: Vec<&str> = report.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"docs-zh-CN"), "{ids:?}");
        assert!(ids.contains(&"docs-freshness"), "{ids:?}");
        // Disabled `fr` is skipped entirely: no finding, no output.
        assert!(!ids.contains(&"docs-fr"), "{ids:?}");
        let locale = report
            .findings
            .iter()
            .find(|f| f.id == "docs-zh-CN")
            .unwrap();
        assert_eq!(locale.status, FindingStatus::Warn);
        assert_eq!(locale.remediation, Remediation::Ai);
        assert!(
            locale.detail.contains("forge docs translate zh-CN"),
            "{}",
            locale.detail
        );
        let rollup = report
            .findings
            .iter()
            .find(|f| f.id == "docs-freshness")
            .unwrap();
        assert_eq!(rollup.status, FindingStatus::Warn);
    }

    #[test]
    fn docs_freshness_fails_for_misconfigured_locale() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: docs-bad\n  name: docs-bad\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\ndocs:\n  source: MISSING.md\n  translations:\n    zh-CN:\n      enabled: true\n",
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let locale = report
            .findings
            .iter()
            .find(|f| f.id == "docs-zh-CN")
            .unwrap();
        assert_eq!(locale.status, FindingStatus::Fail);
        assert_eq!(locale.remediation, Remediation::Manual);
        let rollup = report
            .findings
            .iter()
            .find(|f| f.id == "docs-freshness")
            .unwrap();
        assert_eq!(rollup.status, FindingStatus::Fail);
        assert!(!report.healthy);
    }

    #[test]
    fn unavailable_inspector_is_never_healthy() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("no-git-proj"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        // TempDir is not a git repository: repository check is unavailable.
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let repo = report
            .findings
            .iter()
            .find(|f| f.id == "repository")
            .unwrap();
        assert_eq!(repo.status, FindingStatus::Unavailable);
        assert!(!report.healthy);
        assert!(report
            .blocking_findings()
            .iter()
            .any(|f| f.id == "repository"));
    }

    #[test]
    fn repeat_runs_are_equivalent_and_change_nothing() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("clean-proj"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        write(tmp.path(), "README.md", "# demo\n");
        fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
            let mut out = Vec::new();
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_file() {
                    out.push((
                        path.file_name().unwrap().to_string_lossy().to_string(),
                        fs::read(&path).unwrap(),
                    ));
                }
            }
            out.sort();
            out
        }
        let before = snapshot(tmp.path());
        let first = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let second = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        assert_eq!(first, second);
        assert_eq!(snapshot(tmp.path()), before);
    }

    #[test]
    fn l2_assessment_reports_missing_l1_and_l2_controls() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: l2-proj\n  name: l2-proj\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\n",
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        assert_eq!(report.target_maturity.as_deref(), Some("L2"));
        let unmet: Vec<&str> = report
            .unmet_controls()
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        for expected in [
            "L2-auth",
            "L2-ci",
            "L2-driftwatch",
            "L2-deployment",
            "L2-audit",
        ] {
            assert!(
                unmet.contains(&expected),
                "missing unmet {expected}: {unmet:?}"
            );
        }
        // L1 build/structure hold on this fixture.
        assert!(!unmet.contains(&"L1-build"));
        let maturity = report
            .findings
            .iter()
            .find(|f| f.id == "maturity-requirements")
            .unwrap();
        assert_eq!(maturity.status, FindingStatus::Fail);
    }

    #[test]
    fn l4_without_recovery_is_denied_production() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: l4-proj\n  name: l4-proj\n  profile: rust-web\n  maturity: L2\n  target_maturity: L4\nruntime:\n  language: rust\n",
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let unmet: Vec<&str> = report
            .unmet_controls()
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert!(unmet.contains(&"L4-recovery"), "{unmet:?}");
        assert!(!report.healthy);
        let maturity = report
            .findings
            .iter()
            .find(|f| f.id == "maturity-requirements")
            .unwrap();
        assert_eq!(maturity.status, FindingStatus::Fail);
        assert!(maturity.evidence.iter().any(|e| e.contains("L4")));
    }

    #[test]
    fn l0_prototype_records_nonapplicability() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: l0-proj\n  name: l0-proj\n  profile: flutter-app\n  maturity: L0\n  target_maturity: L0\nruntime:\n  language: dart\n",
        );
        write(
            tmp.path(),
            "pubspec.yaml",
            "name: demo\nenvironment:\n  flutter: 3.22\n",
        );
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        assert_eq!(report.target_maturity.as_deref(), Some("L0"));
        // No deployment automation and no database requirement: L2+ and
        // database controls are nonapplicable rather than forced.
        for id in [
            "L2-deployment",
            "L2-ci",
            "L3-release",
            "L4-recovery",
            "L1-database",
        ] {
            let c = report.controls.iter().find(|c| c.id == id).unwrap();
            assert!(!c.applicable, "{id} must be nonapplicable for L0");
        }
        assert!(report.unmet_controls().is_empty());
    }

    #[test]
    fn stale_registry_observation_is_shown_as_stale() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("stale-proj"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = RegistryObservation {
            registered: true,
            observed_at: Some("2000-01-01T00:00:00Z".to_string()),
        };
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        assert!(report.stale);
        assert!(!report.healthy);
        let finding = report
            .findings
            .iter()
            .find(|f| f.id == "registry-observation")
            .unwrap();
        assert!(finding.evidence.iter().any(|e| e.contains("stale")));
    }

    #[test]
    fn unknown_profile_reports_unavailable_without_hard_failure() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: odd-proj\n  name: odd-proj\n  profile: not-a-real-profile\n",
        );
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let known = report
            .findings
            .iter()
            .find(|f| f.id == "profile-known")
            .unwrap();
        assert_eq!(known.status, FindingStatus::Fail);
        let compat = report
            .findings
            .iter()
            .find(|f| f.id == "features-compatible")
            .unwrap();
        assert_eq!(compat.status, FindingStatus::Unavailable);
        assert!(!report.healthy);
    }

    #[test]
    fn planned_profile_reports_unsupported_without_hard_failure() {
        // R2 boundary: a planned candidate must be inspectable but
        // identified as not-yet-supported so doctor reports it as a
        // blocked control rather than a missing profile.
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: planned-proj\n  name: planned-proj\n  profile: rust-cli\n",
        );
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        // Planned profiles are on the catalog, so profile-known is a
        // Pass. The `features-compatible` check (which routes through
        // resolve_profile) refuses planned profiles, so the
        // unsupported status surfaces there.
        let known = report
            .findings
            .iter()
            .find(|f| f.id == "profile-known")
            .unwrap();
        assert_eq!(known.status, FindingStatus::Pass);
        let compat = report
            .findings
            .iter()
            .find(|f| f.id == "features-compatible")
            .unwrap();
        assert_eq!(compat.status, FindingStatus::Fail);
        assert!(compat.evidence.iter().any(|e| e.contains("rust-cli")));
        assert!(!report.healthy);
    }

    #[test]
    fn invalid_target_is_rejected() {
        let err = parse_target_level("L9").expect_err("L9 must fail");
        assert_eq!(err.code(), "manifest-invalid");
    }

    fn report_with(report: &PolicyReport) -> PolicyOutcome {
        PolicyOutcome::Reported(report.clone())
    }

    fn sample_report(findings: Vec<PolicyFinding>) -> PolicyReport {
        PolicyReport {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            contract: crate::policy::POLICY_CONTRACT_VERSION.to_string(),
            source_revision: None,
            findings,
        }
    }

    #[test]
    fn policy_unavailable_outcome_maps_to_unavailable_finding() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("policy-unavail"));
        let outcome = PolicyOutcome::Unavailable {
            reason: "binary not found on PATH".to_string(),
        };
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
        let finding = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-policy")
            .expect("driftwatch-policy finding");
        assert_eq!(finding.status, FindingStatus::Unavailable);
        assert!(finding.evidence[0].contains("binary not found"));
        assert!(!report.healthy);
    }

    #[test]
    fn policy_findings_keep_rule_ids_and_severity() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("policy-finds"));
        let report_in = sample_report(vec![
            PolicyFinding {
                id: "AUTH-001".to_string(),
                category: "security".to_string(),
                severity: PolicySeverity::Fail,
                applicable: true,
                message: "missing auth markers".to_string(),
                evidence: vec!["Cargo.toml has no auth dep".to_string()],
                reason: None,
            },
            PolicyFinding {
                id: "DEPLOY-002".to_string(),
                category: "deployment".to_string(),
                severity: PolicySeverity::Warn,
                applicable: true,
                message: "deployment target unset".to_string(),
                evidence: vec!["forge.yaml has no deployment.target".to_string()],
                reason: None,
            },
        ]);
        let outcome = report_with(&report_in);
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
        let auth = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-AUTH-001")
            .expect("auth finding");
        assert_eq!(auth.status, FindingStatus::Fail);
        let deploy = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-DEPLOY-002")
            .expect("deploy finding");
        assert_eq!(deploy.status, FindingStatus::Warn);
        let rollup = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-policy")
            .expect("rollup finding");
        assert_eq!(rollup.status, FindingStatus::Fail);
    }

    #[test]
    fn policy_not_applicable_preserves_reason_and_applicability() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("policy-na"));
        let report_in = sample_report(vec![PolicyFinding {
            id: "FLUTTER-AUTH-001".to_string(),
            category: "security".to_string(),
            severity: PolicySeverity::Pass,
            applicable: false,
            message: "not applicable".to_string(),
            evidence: vec![],
            reason: Some("policy not applicable to flutter-app profile".to_string()),
        }]);
        let outcome = report_with(&report_in);
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
        let finding = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-FLUTTER-AUTH-001")
            .expect("flutter auth finding");
        assert!(!finding.applicable);
        assert!(finding.evidence[0].contains("not applicable"));
    }

    #[test]
    fn policy_observation_with_credentials_is_redacted_before_finding() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("policy-redact"));
        let report_in = sample_report(vec![PolicyFinding {
            id: "LEAK-001".to_string(),
            category: "security".to_string(),
            severity: PolicySeverity::Fail,
            applicable: true,
            message: "leaked github token ghp_abcdefghijklmnopqrstuvwxyz0123456789 in config"
                .to_string(),
            evidence: vec!["token=abcdef0123456789 and password=hunter2hunter2".to_string()],
            reason: None,
        }]);
        let outcome = report_with(&report_in);
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
        let finding = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-LEAK-001")
            .expect("leak finding");
        let combined = format!("{} {}", finding.detail, finding.evidence.join(" "));
        assert!(combined.contains("[REDACTED]"), "{combined}");
        for secret in [
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "abcdef0123456789",
            "hunter2hunter2",
        ] {
            assert!(!combined.contains(secret), "secret leaked: {combined}");
        }
    }

    #[test]
    fn policy_finding_with_stale_source_marks_observation_stale() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("policy-stale"));
        // Backdate the source revision so the current manifest mtime is
        // newer than the report's source revision.
        let past: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
        let report_in = PolicyReport {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            contract: crate::policy::POLICY_CONTRACT_VERSION.to_string(),
            source_revision: Some(past),
            findings: vec![PolicyFinding {
                id: "AUTH-001".to_string(),
                category: "security".to_string(),
                severity: PolicySeverity::Pass,
                applicable: true,
                message: "all clean".to_string(),
                evidence: vec!["no issues".to_string()],
                reason: None,
            }],
        };
        let outcome = report_with(&report_in);
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
        let finding = report
            .findings
            .iter()
            .find(|f| f.id == "driftwatch-AUTH-001")
            .expect("auth finding");
        // A passing finding whose source is stale is shown as warn.
        assert_eq!(finding.status, FindingStatus::Warn);
        assert!(finding.evidence.iter().any(|e| e.contains("stale")));
        assert!(!report.healthy);
    }
}

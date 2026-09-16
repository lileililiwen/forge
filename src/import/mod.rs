//! Conservative onboarding of existing repositories (`project-import`).
//!
//! Detection is strictly read-only: filesystem probes and argument-array
//! Git inspection never modify the target directory. A manifest file is
//! written only by [`adopt_import`], only after the proposal is accepted
//! (explicit `--accept`), and only after registry identity is pre-checked,
//! so failures leave source and conflicting metadata unchanged.
//!
//! Ambiguous repositories (monorepos, mixed frameworks) never resolve by
//! silently taking the first detector: inspection fails with
//! `error[ambiguous-import]` until the caller selects a profile or a more
//! specific path.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::core::{validate_project_id, ForgeError};
use crate::profile::inspect_profile;
use crate::registry::Registry;

/// How an import inventory area was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldStatus {
    /// Direct evidence was found.
    Detected,
    /// The area was probed and is absent.
    Missing,
    /// No evidence was available to decide.
    Unknown,
}

impl std::fmt::Display for FieldStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldStatus::Detected => write!(f, "detected"),
            FieldStatus::Missing => write!(f, "missing"),
            FieldStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// One inventoried area with its evidence files or probe notes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Detection {
    pub status: FieldStatus,
    pub value: Option<String>,
    pub evidence: Vec<String>,
}

impl Detection {
    fn detected(value: impl Into<String>, evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Detected,
            value: Some(value.into()),
            evidence,
        }
    }

    fn missing(evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Missing,
            value: None,
            evidence,
        }
    }

    fn unknown(evidence: Vec<String>) -> Self {
        Detection {
            status: FieldStatus::Unknown,
            value: None,
            evidence,
        }
    }
}

/// Read-only import proposal for one directory.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportProposal {
    pub path: String,
    pub language: Detection,
    pub framework: Detection,
    pub package_manager: Detection,
    pub database: Detection,
    pub docker: Detection,
    pub ci: Detection,
    pub auth: Detection,
    pub features: Detection,
    pub driftwatch: Detection,
    pub git_remote: Detection,
    pub deployment: Detection,
    pub suggested_profile: Option<String>,
    pub suggested_maturity: Option<String>,
    pub confidence: String,
    pub alternatives: Vec<String>,
    pub explicit_profile: bool,
    pub manifest_exists: bool,
}

struct Candidate {
    profile: &'static str,
    score: i32,
}

const MAX_SCAN_BYTES: u64 = 128 * 1024;
const MAX_SUBDIRS: usize = 32;

fn profile_language(profile: &str) -> &str {
    match profile {
        "aspnet-web" => "csharp",
        "rust-web" => "rust",
        "nextjs-web" => "typescript",
        "flutter-app" => "dart",
        "python-service" => "python",
        _ => "unknown",
    }
}

fn profile_package_manager(profile: &str) -> &str {
    match profile {
        "aspnet-web" => "dotnet",
        "rust-web" => "cargo",
        "nextjs-web" => "npm",
        "flutter-app" => "pub",
        "python-service" => "pip",
        _ => "unknown",
    }
}

fn exists(dir: &Path, name: &str) -> bool {
    dir.join(name).is_file()
}

fn dir_has_entries(dir: &Path, name: &str) -> bool {
    match fs::read_dir(dir.join(name)) {
        Ok(mut entries) => entries.any(|e| e.is_ok()),
        Err(_) => false,
    }
}

/// Bounded text read for dependency scans; binary or oversized files yield
/// an empty scan rather than failing detection.
fn scan_text(path: &Path) -> String {
    let metadata = fs::metadata(path);
    if let Ok(meta) = metadata {
        if meta.len() > MAX_SCAN_BYTES * 4 {
            return String::new();
        }
    }
    let bytes = fs::read(path).unwrap_or_default();
    let len = bytes.len().min(MAX_SCAN_BYTES as usize);
    String::from_utf8_lossy(&bytes[..len]).to_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn top_level_candidates(dir: &Path) -> Vec<Candidate> {
    let mut out = Vec::new();

    if exists(dir, "Cargo.toml") {
        let mut score = 3;
        if exists(dir, "Cargo.lock") {
            score += 1;
        }
        if exists(dir, "src/main.rs") || exists(dir, "src/lib.rs") {
            score += 1;
        }
        out.push(Candidate {
            profile: "rust-web",
            score,
        });
    }

    if exists(dir, "package.json") {
        let text = scan_text(&dir.join("package.json"));
        // Require the dependency shape, not prose mentioning "next".
        if text.contains("\"next\"") || text.contains("nextjs") {
            let mut score = 3;
            if dir.join("app").is_dir() || dir.join("pages").is_dir() {
                score += 1;
            }
            if exists(dir, "package-lock.json")
                || exists(dir, "pnpm-lock.yaml")
                || exists(dir, "yarn.lock")
            {
                score += 1;
            }
            out.push(Candidate {
                profile: "nextjs-web",
                score,
            });
        }
    }

    if exists(dir, "pubspec.yaml") {
        let text = scan_text(&dir.join("pubspec.yaml"));
        if text.contains("flutter") {
            let mut score = 3;
            if exists(dir, "lib/main.dart") {
                score += 1;
            }
            out.push(Candidate {
                profile: "flutter-app",
                score,
            });
        }
    }

    let has_dotnet = fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("csproj"))
                    || e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| x.eq_ignore_ascii_case("sln"))
            })
        })
        .unwrap_or(false);
    if has_dotnet {
        out.push(Candidate {
            profile: "aspnet-web",
            score: 3,
        });
    }

    if exists(dir, "pyproject.toml")
        || exists(dir, "requirements.txt")
        || exists(dir, "setup.py")
        || exists(dir, "setup.cfg")
    {
        let mut score = 3;
        if dir.join("app").is_dir() || dir.join("src").is_dir() {
            score += 1;
        }
        out.push(Candidate {
            profile: "python-service",
            score,
        });
    }

    out
}

/// Distinct profiles signaled by immediate subdirectories (monorepo roots).
fn subdir_profiles(dir: &Path) -> Vec<(String, String)> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten().take(MAX_SUBDIRS + 1) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if out.len() >= MAX_SUBDIRS {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        for candidate in top_level_candidates(&path) {
            if candidate.score >= 3 {
                out.push((name.clone(), candidate.profile.to_string()));
                break;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn detect_framework(dir: &Path, profile: &str) -> Detection {
    let unknown = || Detection::unknown(vec!["no framework markers".to_string()]);
    match profile {
        "rust-web" => {
            let text = scan_text(&dir.join("Cargo.toml"));
            for fw in ["axum", "actix-web", "rocket", "warp", "poem", "salvo"] {
                if text.contains(fw) {
                    return Detection::detected(fw.to_string(), vec!["Cargo.toml".to_string()]);
                }
            }
            if exists(dir, "Cargo.toml") {
                return Detection::unknown(vec!["Cargo.toml names no known framework".to_string()]);
            }
            unknown()
        }
        "python-service" => {
            let mut text = String::new();
            for file in [
                "pyproject.toml",
                "requirements.txt",
                "setup.py",
                "setup.cfg",
            ] {
                text.push_str(&scan_text(&dir.join(file)));
                text.push('\n');
            }
            for fw in ["fastapi", "django", "flask", "litestar", "tornado"] {
                if text.contains(fw) {
                    return Detection::detected(
                        fw.to_string(),
                        vec!["python-manifest".to_string()],
                    );
                }
            }
            unknown()
        }
        "nextjs-web" => {
            Detection::detected("next.js".to_string(), vec!["package.json".to_string()])
        }
        "aspnet-web" => Detection::detected(
            "asp.net core".to_string(),
            vec!["*.csproj|*.sln".to_string()],
        ),
        "flutter-app" => {
            Detection::detected("flutter".to_string(), vec!["pubspec.yaml".to_string()])
        }
        _ => unknown(),
    }
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

fn detect_database(deps: &str, recognized: bool) -> Detection {
    let mut found = Vec::new();
    if contains_any(
        deps,
        &[
            "postgres", "psycopg", "sqlx", "diesel", "npgsql", "typeorm", "pg_",
        ],
    ) {
        found.push("postgresql");
    }
    if deps.contains("redis") {
        found.push("redis");
    }
    if found.is_empty() {
        if recognized {
            Detection::missing(vec!["no database markers in manifests".to_string()])
        } else {
            Detection::unknown(vec!["no dependency evidence".to_string()])
        }
    } else {
        Detection::detected(found.join("+"), vec!["dependency-manifest".to_string()])
    }
}

fn detect_auth(deps: &str, dir: &Path, recognized: bool) -> Detection {
    let file_markers = exists(dir, "auth.config.ts")
        || exists(dir, "src/auth.ts")
        || exists(dir, "src/auth/mod.rs");
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
        Detection::detected("auth".to_string(), vec!["dependency-manifest".to_string()])
    } else if recognized {
        Detection::missing(vec!["no auth markers in manifests".to_string()])
    } else {
        Detection::unknown(vec!["no dependency evidence".to_string()])
    }
}

fn detect_docker(dir: &Path) -> Detection {
    let mut evidence = Vec::new();
    for file in [
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yaml",
        "compose.yml",
        "Containerfile",
    ] {
        if exists(dir, file) {
            evidence.push(file.to_string());
        }
    }
    if evidence.is_empty() {
        Detection::missing(vec!["no dockerfile or compose file".to_string()])
    } else {
        let value = if evidence
            .iter()
            .any(|e| e.to_lowercase().contains("compose"))
        {
            "docker+compose"
        } else {
            "docker"
        };
        Detection::detected(value, evidence)
    }
}

fn detect_ci(dir: &Path) -> Detection {
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
        if exists(dir, file) {
            found.push(provider);
            evidence.push(file.to_string());
        }
    }
    if found.is_empty() {
        Detection::missing(vec!["no ci configuration".to_string()])
    } else {
        Detection::detected(found.join("+"), evidence)
    }
}

fn detect_driftwatch(dir: &Path) -> Detection {
    let mut evidence = Vec::new();
    for file in [
        "driftwatch.yaml",
        "driftwatch.yml",
        "driftwatch.json",
        ".driftwatch.yaml",
        ".driftwatch.yml",
    ] {
        if exists(dir, file) {
            evidence.push(file.to_string());
        }
    }
    if dir.join(".driftwatch").is_dir() {
        evidence.push(".driftwatch/".to_string());
    }
    if evidence.is_empty() {
        Detection::missing(vec!["no driftwatch configuration".to_string()])
    } else {
        Detection::detected("driftwatch".to_string(), evidence)
    }
}

/// Best-effort Git probe via argument arrays (never shell). A repository
/// without an `origin` remote is `missing`; a non-repository is `unknown`.
fn detect_git_remote(dir: &Path) -> Detection {
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
        return Detection::detected(url, vec!["git remote origin".to_string()]);
    }
    let is_repo = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--git-dir")
        .output()
        .is_ok_and(|out| out.status.success());
    if is_repo {
        Detection::missing(vec!["git repository has no origin remote".to_string()])
    } else {
        Detection::unknown(vec!["not a git repository".to_string()])
    }
}

fn detect_deployment(dir: &Path, recognized: bool) -> Detection {
    let mut found = Vec::new();
    let mut evidence = Vec::new();
    if exists(dir, "Dockerfile") || exists(dir, "Containerfile") {
        found.push("container (docker)");
        evidence.push("Dockerfile".to_string());
    }
    if exists(dir, "fly.toml") {
        found.push("fly.io");
        evidence.push("fly.toml".to_string());
    }
    if exists(dir, "vercel.json") {
        found.push("vercel");
        evidence.push("vercel.json".to_string());
    }
    if exists(dir, "render.yaml") || exists(dir, "render.yml") {
        found.push("render");
        evidence.push("render.yaml".to_string());
    }
    if dir.join("android").is_dir() && dir.join("ios").is_dir() {
        found.push("app-store");
        evidence.push("android/+ios/".to_string());
    }
    if found.is_empty() {
        if recognized {
            Detection::missing(vec!["no deployment configuration".to_string()])
        } else {
            Detection::unknown(vec!["no dependency evidence".to_string()])
        }
    } else {
        Detection::detected(found.join("+"), evidence)
    }
}

/// Read-only inspection of an existing repository. Fails with
/// `ambiguous-import` (before any write) when detectors disagree and no
/// explicit profile selects the outcome.
pub fn inspect_import(
    dir: &Path,
    explicit_profile: Option<&str>,
) -> Result<ImportProposal, ForgeError> {
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: dir.display().to_string(),
        });
    }
    if let Some(id) = explicit_profile {
        inspect_profile(id)?;
    }

    let candidates = top_level_candidates(dir);
    let max_score = candidates.iter().map(|c| c.score).max().unwrap_or(0);
    let winners: Vec<&Candidate> = candidates.iter().filter(|c| c.score == max_score).collect();

    let manifest_exists = dir.join("forge.yaml").is_file();

    // Detectors disagreeing about the profile never resolve silently: an
    // explicit --profile is the accepted resolution, otherwise fail before
    // any write.
    if max_score > 0 && winners.len() > 1 && explicit_profile.is_none() {
        let mut names: Vec<String> = winners.iter().map(|w| w.profile.to_string()).collect();
        names.sort();
        names.dedup();
        return Err(ForgeError::AmbiguousImport {
            detail: format!(
                "directory '{}' matches multiple profiles ({}) with equal evidence; \
                 re-run with one of --profile {} or import a more specific subdirectory",
                dir.display(),
                names.join(", "),
                names.join("|"),
            ),
        });
    }

    // No top-level winner: a monorepo whose subdirectories disagree about
    // the root is ambiguous rather than silently taking the first subdir.
    if max_score == 0 && explicit_profile.is_none() {
        let subdirs = subdir_profiles(dir);
        let mut profiles: Vec<String> = subdirs.iter().map(|(_, p)| p.clone()).collect();
        profiles.sort();
        profiles.dedup();
        if profiles.len() > 1 || (profiles.len() == 1 && subdirs.len() > 1) {
            let roots: Vec<String> = subdirs.iter().map(|(name, _)| name.clone()).collect();
            return Err(ForgeError::AmbiguousImport {
                detail: format!(
                    "directory '{}' looks like a monorepo with candidate subprojects ({}); \
                     import one subdirectory directly or re-run with an explicit --profile",
                    dir.display(),
                    roots.join(", "),
                ),
            });
        }
    }

    let suggested: Option<&str> = match explicit_profile {
        Some(id) => Some(id),
        None => {
            if max_score > 0 {
                Some(winners[0].profile)
            } else {
                None
            }
        }
    };
    let recognized = suggested.is_some();

    let deps = dependency_text(dir);
    let (language, package_manager) = match suggested {
        Some(profile) => (
            Detection::detected(
                profile_language(profile).to_string(),
                vec!["profile-markers".to_string()],
            ),
            Detection::detected(
                profile_package_manager(profile).to_string(),
                vec!["profile-markers".to_string()],
            ),
        ),
        None => (
            Detection::unknown(vec!["no recognizable stack markers".to_string()]),
            Detection::unknown(vec!["no recognizable stack markers".to_string()]),
        ),
    };
    let framework = match suggested {
        Some(profile) => detect_framework(dir, profile),
        None => Detection::unknown(vec!["no recognizable stack markers".to_string()]),
    };
    let database = detect_database(&deps, recognized);
    let auth = detect_auth(&deps, dir, recognized);
    let mut feature_names = Vec::new();
    if auth.status == FieldStatus::Detected {
        feature_names.push("auth".to_string());
    }
    if let Some(db) = database.value.clone() {
        for part in db.split('+') {
            feature_names.push(part.to_string());
        }
    }
    feature_names.sort();
    feature_names.dedup();
    let features = if feature_names.is_empty() {
        if recognized {
            Detection::missing(vec!["no forge-compatible feature markers".to_string()])
        } else {
            Detection::unknown(vec!["no dependency evidence".to_string()])
        }
    } else {
        Detection::detected(
            feature_names.join(","),
            vec!["dependency-manifest".to_string()],
        )
    };

    let mut alternatives: Vec<String> = candidates.iter().map(|c| c.profile.to_string()).collect();
    alternatives.sort();
    alternatives.dedup();
    if let Some(profile) = suggested {
        alternatives.retain(|p| p != profile);
    }

    let confidence = match (explicit_profile, suggested, max_score) {
        (Some(_), _, s) if s > 0 => "medium".to_string(),
        (Some(_), _, _) => "low".to_string(),
        (None, Some(_), s) if s >= 3 => "high".to_string(),
        (None, Some(_), _) => "medium".to_string(),
        (None, None, _) => "none".to_string(),
    };

    Ok(ImportProposal {
        path: dir.display().to_string(),
        language,
        framework,
        package_manager,
        database,
        docker: detect_docker(dir),
        ci: detect_ci(dir),
        auth,
        features,
        driftwatch: detect_driftwatch(dir),
        git_remote: detect_git_remote(dir),
        deployment: detect_deployment(dir, recognized),
        suggested_profile: suggested.map(str::to_string),
        suggested_maturity: suggested.map(|_| "L1".to_string()),
        confidence,
        alternatives,
        explicit_profile: explicit_profile.is_some(),
        manifest_exists,
    })
}

/// Derive a kebab-case project id from the directory name.
pub fn derive_project_id(dir: &Path, id_override: Option<&str>) -> Result<String, ForgeError> {
    if let Some(id) = id_override {
        return match validate_project_id(id) {
            Ok(()) => Ok(id.to_string()),
            Err(reason) => Err(ForgeError::ImportConflict { reason }),
        };
    }
    let raw = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let mut cleaned = String::with_capacity(raw.len());
    let mut prev_dash = true; // trim leading dashes
    for ch in raw.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            cleaned.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            cleaned.push('-');
            prev_dash = true;
        }
    }
    while cleaned.ends_with('-') {
        cleaned.pop();
    }
    match validate_project_id(&cleaned) {
        Ok(()) => Ok(cleaned),
        Err(_) => Err(ForgeError::ImportConflict {
            reason: format!(
                "directory name '{}' yields no valid project id; re-run with --id <kebab-case-id>",
                dir.display()
            ),
        }),
    }
}

/// Minimal validated manifest for an accepted proposal. The manifest carries
/// no `features` section: detected features are reported in the proposal but
/// only compatible, explicitly resolved capabilities belong in the registry.
pub fn build_manifest_text(id: &str, profile: &str, language: Option<&str>) -> String {
    let mut text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: {profile}\n  maturity: L1\n  target_maturity: L1\n"
    );
    if let Some(lang) = language.filter(|l| !l.trim().is_empty()) {
        text.push_str(&format!("runtime:\n  language: {lang}\n"));
    }
    text
}

fn canonicalize_dir(dir: &Path) -> Result<String, ForgeError> {
    dir.canonicalize()
        .map(|p| p.display().to_string())
        .map_err(|_| ForgeError::PathUnavailable {
            path: dir.display().to_string(),
        })
}

/// Accept an inspected proposal: write the minimal manifest (only when no
/// manifest exists and no legacy file would become ambiguous) and register
/// the project. Any validation failure happens before any write.
pub fn adopt_import(
    registry: &mut Registry,
    dir: &Path,
    explicit_profile: Option<&str>,
    id_override: Option<&str>,
) -> Result<crate::registry::ProjectRecord, ForgeError> {
    let proposal = inspect_import(dir, explicit_profile)?;
    let profile = proposal
        .suggested_profile
        .clone()
        .ok_or_else(|| ForgeError::ImportConflict {
            reason: format!(
                "directory '{}' has no recognizable profile; re-run with --profile <id>",
                dir.display()
            ),
        })?;
    // Compatibility is validated before any mutation.
    inspect_profile(&profile)?;
    let canonical = canonicalize_dir(dir)?;

    let forge_path = dir.join("forge.yaml");
    let legacy_path = dir.join("platform.yaml");

    if forge_path.is_file() {
        let (manifest, _) = crate::core::manifest::Manifest::load_from_dir(dir, None)?;
        if let Some(explicit) = explicit_profile {
            if manifest.project.profile != explicit {
                return Err(ForgeError::ImportConflict {
                    reason: format!(
                        "existing forge.yaml declares profile '{}' which conflicts with --profile '{explicit}'; refusing to overwrite",
                        manifest.project.profile
                    ),
                });
            }
        }
        registry.check_identity_available(&manifest.project.id, &canonical)?;
        return registry.register(dir, None);
    }

    if legacy_path.is_file() {
        return Err(ForgeError::ImportConflict {
            reason: "legacy platform.yaml is present and no forge.yaml exists; refusing to create a second manifest (ambiguous-manifest); register with --manifest platform.yaml instead".to_string(),
        });
    }

    let id = derive_project_id(dir, id_override)?;
    registry.check_identity_available(&id, &canonical)?;

    let language = proposal.language.value.as_deref();
    let text = build_manifest_text(&id, &profile, language);
    // The generated text is minimal and must always validate; a failure here
    // is a bug, reported without claiming unrelated files changed.
    crate::core::manifest::Manifest::parse(&forge_path, text.as_bytes())?;
    fs::write(&forge_path, &text).map_err(|err| ForgeError::ImportConflict {
        reason: format!(
            "manifest destination '{}' is unwritable: {err}; nothing was changed",
            forge_path.display()
        ),
    })?;

    match registry.register(dir, None) {
        Ok(record) => Ok(record),
        Err(err) => {
            // Registration re-validates identity; a failure after the write
            // must not leave a half-adopted manifest behind.
            let _ = fs::remove_file(&forge_path);
            Err(err)
        }
    }
}

/// Render a proposal for human CLI output, following the requirement §8 shape.
pub fn render_proposal_human(proposal: &ImportProposal) -> String {
    fn field(label: &str, detection: &Detection) -> String {
        match (&detection.status, &detection.value) {
            (FieldStatus::Detected, Some(value)) => format!("{label}: {value}"),
            (FieldStatus::Detected, None) => format!("{label}: present"),
            (FieldStatus::Missing, _) => format!("{label}: missing"),
            (FieldStatus::Unknown, _) => format!("{label}: unknown"),
        }
    }
    let mut lines = vec![
        "Detected:".to_string(),
        String::new(),
        field("Language", &proposal.language),
        field("Framework", &proposal.framework),
        field("Package manager", &proposal.package_manager),
        field("Database", &proposal.database),
        field("Container", &proposal.docker),
        field("CI", &proposal.ci),
        field("Auth", &proposal.auth),
        field("Features", &proposal.features),
        field("DriftWatch", &proposal.driftwatch),
        field("Git remote", &proposal.git_remote),
        field("Deployment", &proposal.deployment),
        String::new(),
        format!(
            "Suggested profile:\n{}",
            proposal.suggested_profile.as_deref().unwrap_or("unknown")
        ),
        String::new(),
        format!(
            "Suggested maturity:\n{}",
            proposal.suggested_maturity.as_deref().unwrap_or("unknown")
        ),
        String::new(),
        format!("Confidence: {}", proposal.confidence),
    ];
    if !proposal.alternatives.is_empty() {
        lines.push(format!(
            "Alternatives: {}",
            proposal.alternatives.join(", ")
        ));
    }
    if proposal.manifest_exists {
        lines.push(
            "Manifest: forge.yaml already exists; --accept will register it without overwriting."
                .to_string(),
        );
    } else {
        lines.push(
            "Manifest: missing; --accept will create a minimal forge.yaml and register."
                .to_string(),
        );
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    fn rust_fixture(dir: &Path) {
        write(
            dir,
            "Cargo.toml",
            "[package]\nname = \"demo\"\n[dependencies]\naxum = \"0.7\"\nsqlx = { version = \"0.8\", features = [\"postgres\"] }\njsonwebtoken = \"9\"\n",
        );
        write(dir, "src/main.rs", "fn main() {}\n");
        write(dir, "Dockerfile", "FROM rust\n");
        write(dir, ".github/workflows/ci.yml", "on: push\n");
    }

    #[test]
    fn detects_rust_stack_with_evidence() {
        let tmp = TempDir::new().unwrap();
        rust_fixture(tmp.path());
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert_eq!(proposal.suggested_maturity.as_deref(), Some("L1"));
        assert_eq!(proposal.language.value.as_deref(), Some("rust"));
        assert_eq!(proposal.framework.value.as_deref(), Some("axum"));
        assert_eq!(proposal.package_manager.value.as_deref(), Some("cargo"));
        assert!(proposal
            .database
            .value
            .as_deref()
            .unwrap()
            .contains("postgresql"));
        assert_eq!(proposal.docker.status, FieldStatus::Detected);
        assert_eq!(proposal.ci.status, FieldStatus::Detected);
        assert_eq!(proposal.auth.status, FieldStatus::Detected);
        // No git repo in tempdir: unknown, not missing.
        assert_eq!(proposal.git_remote.status, FieldStatus::Unknown);
        assert_eq!(proposal.driftwatch.status, FieldStatus::Missing);
    }

    #[test]
    fn missing_remote_and_driftwatch_do_not_block_inspection() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert_eq!(proposal.git_remote.status, FieldStatus::Unknown);
        assert_eq!(proposal.driftwatch.status, FieldStatus::Missing);
    }

    #[test]
    fn mixed_frameworks_are_ambiguous_and_selectable() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        write(
            tmp.path(),
            "pubspec.yaml",
            "name: demo\nenvironment:\n  flutter: 3.22\n",
        );
        let err = inspect_import(tmp.path(), None).expect_err("mixed stacks must be ambiguous");
        assert_eq!(err.code(), "ambiguous-import");
        assert!(err.to_string().contains("--profile"), "{err}");
        // Explicit selection resolves without writes.
        let before: Vec<PathBuf> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        let proposal = inspect_import(tmp.path(), Some("rust-web")).unwrap();
        assert_eq!(proposal.suggested_profile.as_deref(), Some("rust-web"));
        assert!(proposal.explicit_profile);
        let after: Vec<PathBuf> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn monorepo_roots_are_ambiguous() {
        let tmp = TempDir::new().unwrap();
        let rust_sub = tmp.path().join("backend");
        let app_sub = tmp.path().join("mobile");
        fs::create_dir(&rust_sub).unwrap();
        fs::create_dir(&app_sub).unwrap();
        write(&rust_sub, "Cargo.toml", "[package]\nname = \"backend\"\n");
        write(
            &app_sub,
            "pubspec.yaml",
            "name: mobile\nenvironment:\n  flutter: 3.22\n",
        );
        let err = inspect_import(tmp.path(), None).expect_err("monorepo must be ambiguous");
        assert_eq!(err.code(), "ambiguous-import");
        assert!(err.to_string().contains("backend"), "{err}");
    }

    #[test]
    fn empty_directory_has_unknown_profile() {
        let tmp = TempDir::new().unwrap();
        let proposal = inspect_import(tmp.path(), None).unwrap();
        assert_eq!(proposal.suggested_profile, None);
        assert_eq!(proposal.confidence, "none");
        assert_eq!(proposal.language.status, FieldStatus::Unknown);
    }

    #[test]
    fn derives_kebab_ids_and_rejects_garbage() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("My_Cool App!");
        fs::create_dir(&dir).unwrap();
        assert_eq!(derive_project_id(&dir, None).unwrap(), "my-cool-app");
        assert_eq!(derive_project_id(&dir, Some("ok-id")).unwrap(), "ok-id");
        assert_eq!(
            derive_project_id(&dir, Some("Bad_ID")).unwrap_err().code(),
            "import-conflict"
        );
    }

    #[test]
    fn generated_manifest_is_minimal_and_valid() {
        let text = build_manifest_text("demo-app", "rust-web", Some("rust"));
        let manifest =
            crate::core::manifest::Manifest::parse(Path::new("forge.yaml"), text.as_bytes())
                .expect("generated manifest must validate");
        assert_eq!(manifest.project.id, "demo-app");
        assert!(manifest.features.is_empty());
        crate::profile::resolve_profile(&manifest.project.profile, &[]).unwrap();
    }

    #[test]
    fn unknown_explicit_profile_fails_read_only() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let err = inspect_import(tmp.path(), Some("react-web")).expect_err("unknown profile");
        assert_eq!(err.code(), "unknown-profile");
    }
}

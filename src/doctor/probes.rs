//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{Manifest, Maturity};
use crate::core::ForgeError;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::model::{Finding, FindingStatus, GitState, Remediation};

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

pub(super) fn maturity_rank(level: Maturity) -> u8 {
    match level {
        Maturity::L0 => 0,
        Maturity::L1 => 1,
        Maturity::L2 => 2,
        Maturity::L3 => 3,
        Maturity::L4 => 4,
    }
}

pub(super) fn maturity_name(level: Maturity) -> &'static str {
    match level {
        Maturity::L0 => "L0",
        Maturity::L1 => "L1",
        Maturity::L2 => "L2",
        Maturity::L3 => "L3",
        Maturity::L4 => "L4",
    }
}

pub(super) fn file_exists(dir: &Path, name: &str) -> bool {
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

pub(super) fn dependency_text(dir: &Path) -> String {
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

pub(super) fn has_build_definition(dir: &Path, profile: &str) -> (bool, Vec<String>) {
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

pub(super) fn has_database_markers(deps: &str) -> (bool, Vec<String>) {
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

pub(super) fn has_auth_markers(deps: &str, dir: &Path) -> (bool, Vec<String>) {
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

pub(super) fn has_admin_markers(
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

pub(super) fn has_audit_markers(
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

pub(super) fn has_observability_markers(
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

pub(super) fn requests_db(features: &std::collections::BTreeMap<String, String>) -> bool {
    features.keys().any(|k| {
        let k = k.to_lowercase();
        k.contains("postgres")
            || k.contains("redis")
            || k.contains("storage")
            || k.contains("database")
    })
}

pub(super) fn probe_git_remote(dir: &Path) -> GitState {
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

pub(super) fn detect_ci(dir: &Path) -> (bool, Vec<String>) {
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

/// Gate runtime evidence finding (`gate-runtime-evidence`). Doctor
/// reads the persisted revision-bound record for the project: pass on a
/// fresh passing aggregate, fail on a blocked or failing snapshot, warn
/// on stale passing or unclassifiable evidence, and unavailable
/// (`unverified`) when a declared/managed gate has never run or the
/// record cannot be read. Absence is never rendered as a pass; a project
/// with no declaration and no evidence is not-applicable, mirroring the
/// policy plane's non-applicable convention without inventing a verdict.
pub(super) fn gate_evidence_finding(dir: &Path) -> Finding {
    let current = crate::gate::capture_revision(dir);
    match crate::gate::load_latest_evidence(dir) {
        Ok(Some(evidence)) => {
            let freshness = crate::gate::evidence_freshness(&evidence, current.as_deref());
            let summary = crate::gate::evidence_summary(&evidence);
            let (status, detail) = match (evidence.aggregate, freshness) {
                (crate::gate::GateAggregate::Passed, crate::gate::GateFreshness::Fresh) => (
                    FindingStatus::Pass,
                    "gate evidence is fresh and passing",
                ),
                (crate::gate::GateAggregate::Passed, _) => (
                    FindingStatus::Warn,
                    "gate evidence passed but is stale: the working revision moved, never a current verification",
                ),
                (crate::gate::GateAggregate::Blocked, crate::gate::GateFreshness::Fresh) => (
                    FindingStatus::Fail,
                    "the gate runtime blocked this revision; unresolved REVIEW_REQUIRED or FAIL cannot pass",
                ),
                (crate::gate::GateAggregate::Blocked, _) => (
                    FindingStatus::Fail,
                    "the gate runtime blocked this revision and the snapshot is stale; never a pass",
                ),
                (crate::gate::GateAggregate::Failed, _) => (
                    FindingStatus::Fail,
                    "the gate runtime reported a non-blocking failure on this snapshot",
                ),
                (crate::gate::GateAggregate::Unknown, _) => (
                    FindingStatus::Warn,
                    "the gate document aggregate is unclassifiable and is never treated as passing",
                ),
            };
            Finding::new(
                "gate-evidence",
                status,
                vec![summary],
                true,
                Remediation::Manual,
                detail,
            )
        }
        Ok(None) => {
            let opted_in = !matches!(
                crate::gate::declared_gate_runtime(dir),
                crate::gate::DeclaredRuntime::Undeclared
            ) || crate::gate::is_gate_managed(dir);
            if opted_in {
                Finding::new(
                    "gate-evidence",
                    FindingStatus::Unavailable,
                    vec![format!("{}/", crate::gate::GATE_EVIDENCE_DIR)],
                    true,
                    Remediation::Manual,
                    "gate runtime is declared but the gate has never run: unverified, not a pass",
                )
            } else {
                Finding::new(
                    "gate-evidence",
                    FindingStatus::Pass,
                    vec![format!("{}/", crate::gate::GATE_EVIDENCE_DIR)],
                    false,
                    Remediation::Manual,
                    "no gate runtime declared or gate manifest present: not applicable (never a claim that a gate passed)",
                )
            }
        }
        Err(err) => Finding::new(
            "gate-evidence",
            FindingStatus::Unavailable,
            vec![err.to_string()],
            true,
            Remediation::Manual,
            "persisted gate evidence cannot be read and is never assumed healthy",
        ),
    }
}

/// Gate release-evidence finding (`gate-evidence-export-consumption`).
/// Doctor reads the persisted consumed release-evidence record for the
/// project and reports the state distribution: pass when at least one field
/// is `verified` and the record is fresh; warn on stale, partially
/// evidenced or refused entries; fail for the specific inconsistency a
/// declaration asserting `verified` in `evidence_status` while carrying no
/// `release_evidence` block at all (the pattern the structural audit cannot
/// see today). Non-gating by construction: not-applicable when neither a
/// declaration nor an export exists, so absence never lowers health,
/// never changes a maturity verdict and never alters an exit code.
pub(super) fn release_evidence_finding(dir: &Path) -> Option<Finding> {
    use crate::gate::evidence::EvidenceFreshness;

    let _current = crate::gate::capture_revision(dir);
    let metadata_path = dir.join(crate::generate::workspace::METADATA_PATH);
    let has_declaration = metadata_path.is_file();

    match crate::gate::evidence::load_latest_release_evidence(dir) {
        Ok(Some(record)) => {
            let freshness_label = record.freshness.label();
            let state_summary = format!(
                "freshness={} verified={} configured={} declared={} unverified={} refused={}",
                freshness_label,
                record.counts.verified,
                record.counts.configured,
                record.counts.declared,
                record.counts.unverified,
                record.counts.refused,
            );
            let mut evidence_lines = vec![state_summary];

            // Check for the specific inconsistency: declaration says verified
            // but no release_evidence block exists.
            if has_declaration {
                let raw = fs::read(&metadata_path).ok()?;
                let value: serde_json::Value = serde_json::from_slice(&raw).ok()?;
                let declared_verified = value
                    .get("verification")
                    .and_then(|v| v.get("evidence_status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_lowercase())
                    .map(|s| s == "verified")
                    .unwrap_or(false);
                if declared_verified && record.counts.verified == 0 {
                    // The declaration asserts verified but the consumed record
                    // has zero verified fields.
                    return Some(Finding::new(
                        "release-evidence",
                        FindingStatus::Fail,
                        evidence_lines,
                        true,
                        Remediation::Manual,
                        "declaration asserts evidence_status=verified but the consumed gate export has zero verified fields; this is the assertion-without-evidence pattern the structural audit cannot see",
                    ));
                }
            }

            let (status, detail) = match (record.counts.verified, record.freshness) {
                (n, _) if n > 0 && record.freshness == EvidenceFreshness::Fresh => (
                    FindingStatus::Pass,
                    "release-evidence record is fresh with verified fields",
                ),
                (n, EvidenceFreshness::Stale) if n > 0 => (
                    FindingStatus::Warn,
                    "release-evidence record has verified fields but is stale: the working revision moved",
                ),
                (0, EvidenceFreshness::Fresh) if record.counts.configured > 0 => (
                    FindingStatus::Warn,
                    "release-evidence record is fresh but all fields are configured or below; no field is verified yet",
                ),
                (0, _) if record.counts.refused > 0 => (
                    FindingStatus::Warn,
                    "release-evidence record has refused entries that could not be consumed",
                ),
                (0, _) => (
                    FindingStatus::Warn,
                    "release-evidence record exists but no field is verified",
                ),
                _ => (
                    FindingStatus::Warn,
                    "release-evidence record has a mixed state distribution",
                ),
            };

            if record.counts.refused > 0 {
                evidence_lines.push(format!(
                    "{} field(s) refused during consumption",
                    record.counts.refused
                ));
            }

            Some(Finding::new(
                "release-evidence",
                status,
                evidence_lines,
                true,
                Remediation::Manual,
                detail,
            ))
        }
        Ok(None) => {
            // No consumed export. If the project has a declaration asserting
            // verified without a release_evidence block, that is the failure
            // mode this finding exists to surface.
            if has_declaration {
                let raw = fs::read(&metadata_path).ok()?;
                let value: serde_json::Value = serde_json::from_slice(&raw).ok()?;
                let declared_verified = value
                    .get("verification")
                    .and_then(|v| v.get("evidence_status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_lowercase())
                    .map(|s| s == "verified")
                    .unwrap_or(false);
                if declared_verified {
                    return Some(Finding::new(
                        "release-evidence",
                        FindingStatus::Fail,
                        vec![format!(
                            "{}/",
                            crate::gate::evidence::release_evidence_path("*")
                                .parent()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| format!("{}/release-evidence", crate::gate::GATE_EVIDENCE_DIR))
                        )],
                        true,
                        Remediation::Manual,
                        "declaration asserts evidence_status=verified but no release_evidence block exists and no gate evidence export has been consumed",
                    ));
                }
            }
            // No declaration or declaration does not assert verified.
            None
        }
        Err(err) => Some(Finding::new(
            "release-evidence",
            FindingStatus::Unavailable,
            vec![err.to_string()],
            true,
            Remediation::Manual,
            "persisted release-evidence record cannot be read",
        )),
    }
}

/// Declaration-vocabulary divergence (`governance-vocabulary-consumption`).
/// Doctor compares the project's `.project.json` declaration against the
/// consumed governance vocabulary and reports divergence without ever
/// gating on it: a project with no declaration gets no finding at all
/// (checker stays silent, like `workspace-metadata` absent); a fully
/// canonical declaration passes as not-applicable (checker stays silent,
/// like `gate-evidence` for undeclared projects); divergence warns
/// naming field, value and vocabulary source; only a Forge-authored
/// claim whose evidence reference no longer resolves fails.
///
/// Capability *states* are never adjudicated here: Workspace Governance
/// owns that vocabulary and its audit reports on it. Forge checks only
/// that a `configured`/`verified` entry's `evidence_ref` resolves to a
/// file inside the project.
pub(super) fn declaration_vocabulary_finding(dir: &Path) -> Option<Finding> {
    let raw = fs::read(dir.join(crate::generate::workspace::METADATA_PATH)).ok()?;
    let authored = crate::generate::workspace::declaration_is_forge_authored(dir, &raw);
    let scrub = |text: &str| {
        let mut out = crate::policy::redact_credentials(text);
        for form in [dir.display().to_string()]
            .into_iter()
            .chain(dir.canonicalize().ok().map(|c| c.display().to_string()))
        {
            if !form.is_empty() {
                out = out.replace(&form, "<project>");
            }
        }
        let kept: String = out.chars().take(crate::gate::MAX_NOTE_CHARS).collect();
        if kept.len() < out.len() {
            format!("{kept}…[truncated]")
        } else {
            kept
        }
    };
    let value: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(err) => {
            return Some(Finding::new(
                "declaration-vocabulary",
                FindingStatus::Warn,
                vec![scrub(&format!(
                    "{}: declaration does not parse ({err}); values not validated",
                    crate::generate::workspace::METADATA_PATH
                ))],
                true,
                Remediation::Manual,
                "project declaration exists but does not parse, so no declared value can be validated against the governance vocabulary",
            ));
        }
    };
    let vocab = match crate::vocabulary::load(None) {
        Ok(vocab) => vocab,
        Err(err) => {
            return Some(Finding::new(
                "declaration-vocabulary",
                FindingStatus::Unavailable,
                vec![scrub(&err.to_string())],
                true,
                Remediation::Manual,
                "governance vocabulary unavailable; declared values not validated and never assumed canonical",
            ));
        }
    };
    let provenance = vocab.provenance();
    let mut problems: Vec<String> = Vec::new();
    let mut failed = false;
    let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    if kind.is_empty() {
        problems.push("declaration carries no kind".to_string());
    } else if !vocab.is_canonical_kind(kind) {
        problems.push(format!(
            "kind '{kind}' is not in the consumed canonical set ({provenance})"
        ));
    }
    let profile = value.get("profile").and_then(|v| v.as_str()).unwrap_or("");
    if profile.is_empty() {
        problems.push("declaration carries no profile".to_string());
    } else if !vocab.is_canonical_profile(profile) {
        problems.push(format!(
            "profile '{profile}' is not in the consumed canonical set ({provenance})"
        ));
    }
    // The consumed vocabulary defines no canonical evidence-status set,
    // so the value is stated as evidence but never adjudicated: it cannot
    // make an otherwise canonical declaration warn, and the gap stays a
    // companion request rather than a Forge-side invention.
    let evidence_status = value
        .pointer("/verification/evidence_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let status_note = if evidence_status.is_empty() {
        problems.push("verification.evidence_status is missing".to_string());
        "verification.evidence_status is missing".to_string()
    } else {
        format!(
            "verification.evidence_status '{evidence_status}' has no canonical set in the consumed vocabulary ({provenance}); stated, not adjudicated"
        )
    };
    if let Some(caps) = value.get("capabilities").and_then(|v| v.as_object()) {
        for (name, entry) in caps {
            let state = entry.get("evidence_state").and_then(|v| v.as_str());
            if !matches!(state, Some("configured") | Some("verified")) {
                continue;
            }
            let broken = match entry.get("evidence_ref").and_then(|v| v.as_str()) {
                None | Some("") => Some("missing evidence_ref".to_string()),
                Some(reference)
                    if reference.starts_with('/')
                        || Path::new(reference).components().any(|c| {
                            matches!(
                                c,
                                std::path::Component::ParentDir | std::path::Component::RootDir
                            )
                        }) =>
                {
                    Some(format!("evidence_ref escapes the project: '{reference}'"))
                }
                Some(reference) if !dir.join(reference).is_file() => {
                    Some(format!("evidence_ref missing: '{reference}'"))
                }
                _ => None,
            };
            if let Some(reason) = broken {
                // Only a claim Forge itself authored fails; foreign or
                // user-edited declarations warn so Forge never fails a
                // project over words it did not write.
                if authored {
                    failed = true;
                    problems.push(format!("capability '{name}' {reason}"));
                } else {
                    problems.push(format!("capability '{name}' {reason} (not Forge-authored)"));
                }
            }
        }
    }
    if value.get("deployment").and_then(|v| v.get("deployable"))
        == Some(&serde_json::Value::Bool(true))
        && value.get("release_evidence").is_none()
    {
        problems.push(
            "deployment.deployable is true while the declaration carries no release_evidence block"
                .to_string(),
        );
    }
    if problems.is_empty() {
        let mut evidence = vec![scrub(&format!(
            "declaration values are canonical against {provenance}"
        ))];
        evidence.push(scrub(&status_note));
        Some(Finding::new(
            "declaration-vocabulary",
            FindingStatus::Pass,
            evidence,
            false,
            Remediation::Manual,
            "declaration kind, profile and capability references agree with the consumed governance vocabulary (informational; not maturity evidence)",
        ))
    } else if failed {
        let mut evidence: Vec<String> = problems.iter().map(|p| scrub(p)).collect();
        evidence.push(scrub(&status_note));
        Some(Finding::new(
            "declaration-vocabulary",
            FindingStatus::Fail,
            evidence,
            true,
            Remediation::Manual,
            "a capability claim Forge authored no longer resolves; divergence never gates maturity, only the broken claim fails",
        ))
    } else {
        let mut evidence: Vec<String> = problems.iter().map(|p| scrub(p)).collect();
        evidence.push(scrub(&status_note));
        Some(Finding::new(
            "declaration-vocabulary",
            FindingStatus::Warn,
            evidence,
            true,
            Remediation::Manual,
            "declaration diverges from the consumed governance vocabulary; reported without changing health, maturity or exit codes",
        ))
    }
}

pub(super) fn detect_driftwatch(dir: &Path) -> (bool, Vec<String>) {
    let mut evidence = Vec::new();
    for file in [
        "driftwatch.yaml",
        "driftwatch.yml",
        "driftwatch.json",
        ".driftwatch.yaml",
        ".driftwatch.yml",
        "driftwatch.toml",
        "gate.toml",
        ".ai-gate/gate.yaml",
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

pub(super) fn detect_deployment(dir: &Path) -> (bool, Vec<String>) {
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

pub(super) fn detect_docs(dir: &Path, manifest: &Manifest) -> (bool, Vec<String>) {
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

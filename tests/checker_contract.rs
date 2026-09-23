//! CLI contract for `forge check` (`external-checker-emission`).
//!
//! Exercises the external-checker projection end to end through the
//! built binary:
//!
//! - Emission contract: stdout carries exactly one protocol document
//!   with an always-present `alerts` array; severities stay inside the
//!   sibling vocabulary; both output formats print the identical bytes.
//! - Read-only guarantee: manifest bytes, registry file bytes, `.forge/`
//!   state, Git HEAD and the journal are unchanged across two runs; the
//!   documents differ only in `generated_at`.
//! - Bounded failure surface: findings never change the exit code; an
//!   unregistered target exits non-zero with a typed error and an empty
//!   stdout; `--max-alerts` is bounded and truncation is explicit.
//! - Feedback-loop guard: the DriftWatch adapter is contacted only with
//!   an explicit `--include-policy`.
//! - Sibling protocol fixture: a tolerant parser mirroring
//!   `config-checker-protocol` (driftwatchdog) accepts every document
//!   this surface emits and rejects a `{}` without `alerts`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DRIFTWATCH_BIN");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_with_env(db: &Path, args: &[&str], key: &str, value: &Path) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.env(key, value);
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn write_file(dir: &Path, name: &str, text: &str) {
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

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
}

/// A registered project whose every assessment plane passes: schema-
/// valid manifest, build definition, README, deployment config, CI,
/// DriftWatch config, git repository with an origin remote and a fresh
/// registry observation.
fn clean_project(parent: &Path, db: &Path, id: &str) -> PathBuf {
    let proj = parent.join(id);
    fs::create_dir_all(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest(id));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write_file(&proj, "README.md", "# demo\n");
    write_file(&proj, "Dockerfile", "FROM scratch\n");
    write_file(&proj, "driftwatch.yaml", "checks: []\n");
    write_file(&proj, ".github/workflows/ci.yml", "name: ci\n");
    git(&proj, &["init", "-q"]);
    git(
        &proj,
        &["remote", "add", "origin", "https://example.test/demo/x.git"],
    );
    let out = run(db, &["register", &proj.display().to_string()]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    proj
}

fn check_value(db: &Path, proj: &Path, extra: &[&str]) -> serde_json::Value {
    let path = proj.display().to_string();
    let mut args: Vec<&str> = vec!["check", &path];
    args.extend(extra);
    let out = run(db, &args);
    assert!(
        out.status.success(),
        "exit {:?}; stderr: {}",
        out.status.code(),
        lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "stderr must stay empty: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "checker stdout must be pure JSON: {err}; stdout: {}",
            lossy(&out.stdout)
        )
    })
}

fn alerts(doc: &serde_json::Value) -> Vec<serde_json::Value> {
    doc["alerts"].as_array().cloned().unwrap_or_default()
}

/// Mirrors the driftwatchdog `config-checker-protocol` tolerance rules:
/// `alerts` must be present, each alert requires non-empty
/// severity/message/source/symbol, unknown top-level and per-alert
/// fields are ignored, count and message size stay under the sibling
/// caps. A `{}` is a protocol error, never success.
fn parse_like_driftwatch(bytes: &[u8]) -> Result<usize, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|err| format!("invalid JSON: {err}"))?;
    let obj = value.as_object().ok_or("not a JSON object")?;
    let alerts = obj
        .get("alerts")
        .ok_or_else(|| String::from("missing \"alerts\""))?;
    let list = alerts.as_array().ok_or("`alerts` is not an array")?;
    if list.len() > 10_000 {
        return Err(format!("{} alerts exceeds the 10000 cap", list.len()));
    }
    for (index, alert) in list.iter().enumerate() {
        for field in ["severity", "message", "source", "symbol"] {
            let text = alert[field]
                .as_str()
                .ok_or_else(|| format!("alert #{index} missing field {field}"))?;
            if text.is_empty() {
                return Err(format!("alert #{index} has an empty {field}"));
            }
        }
        if alert["message"].as_str().unwrap_or_default().len() > 64 * 1024 {
            return Err(format!("alert #{index} message is oversized"));
        }
    }
    Ok(list.len())
}

#[test]
fn top_level_help_advertises_the_checker_surface() {
    let out = run(Path::new("/unused"), &["--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("check"), "{text}");
}

#[test]
fn check_help_names_bounded_options() {
    let out = run(Path::new("/unused"), &["check", "--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("--include-policy"), "{text}");
    assert!(text.contains("--max-alerts"), "{text}");
}

#[test]
fn clean_project_emits_an_empty_alerts_document() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "clean-app");

    let doc = check_value(&db, &proj, &[]);
    assert_eq!(doc["schema"], "forge-checker/0.1.0");
    let generated = doc["generated_at"].as_str().unwrap();
    assert!(!generated.is_empty());
    assert_eq!(alerts(&doc).len(), 0);
    // The alerts key must be serialized even when empty.
    let text = lossy(&run(&db, &["check", &proj.display().to_string()]).stdout);
    assert!(text.contains("\"alerts\":[]"), "{text}");
    assert_eq!(parse_like_driftwatch(text.as_bytes()).unwrap(), 0);
}

#[test]
fn format_json_does_not_alter_checker_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "parity-app");
    let human = run(&db, &["check", &proj.display().to_string()]);
    let json = run(
        &db,
        &["--format", "json", "check", &proj.display().to_string()],
    );
    assert!(human.status.success() && json.status.success());
    let (mut a, mut b) = (human.stdout.clone(), json.stdout.clone());
    a.pop(); // trailing newline from println
    b.pop();
    assert_eq!(
        String::from_utf8_lossy(&a).replace("\"generated_at\":\"", ""),
        String::from_utf8_lossy(&b).replace("\"generated_at\":\"", ""),
        "--format json must not alter the emitted document"
    );
}

#[test]
fn mixed_findings_carry_protocol_severities_and_stable_symbols() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "mixed-app");
    // User edits the manifest after registration: a planned profile
    // breaks descriptor-gated planes and makes readiness unverified.
    write_file(
        &proj,
        "forge.yaml",
        &rust_manifest("mixed-app").replace("profile: rust-web", "profile: python-ai"),
    );

    let doc = check_value(&db, &proj, &[]);
    let list = alerts(&doc);
    assert!(!list.is_empty());
    let severities: Vec<&str> = list
        .iter()
        .map(|a| a["severity"].as_str().unwrap())
        .collect();
    assert!(
        severities.contains(&"error"),
        "fail findings must surface as error: {list:?}"
    );
    assert!(
        severities.contains(&"warning"),
        "warn/unavailable findings must surface: {list:?}"
    );
    for alert in &list {
        assert!(matches!(
            alert["severity"].as_str().unwrap_or(""),
            "error" | "warning"
        ));
        assert!(!alert["message"].as_str().unwrap().is_empty());
        assert!(!alert["symbol"].as_str().unwrap().is_empty());
        let source = alert["source"].as_str().unwrap();
        assert!(!source.is_empty());
        assert!(!source.starts_with('/'), "absolute source leaked: {source}");
        assert!(!source.contains(".."), "traversal source leaked: {source}");
    }
    let symbols: Vec<&str> = list.iter().map(|a| a["symbol"].as_str().unwrap()).collect();
    assert!(
        symbols.contains(&"doctor/features-compatible"),
        "{symbols:?}"
    );
    assert!(
        symbols.contains(&"readiness/python-ai"),
        "planned profile must surface as unverified readiness: {symbols:?}"
    );
    assert_eq!(
        list.iter()
            .find(|a| a["symbol"] == "readiness/python-ai")
            .unwrap()["source"],
        "readiness"
    );
    assert!(
        parse_like_driftwatch(&run(&db, &["check", &proj.display().to_string()]).stdout).is_ok()
    );
}

#[test]
fn missing_evidence_surfaces_as_warning_naming_the_gap() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // Registered project without a git repository: the repository
    // plane is unavailable and must not be dropped silently.
    let proj = tmp.path().join("nogit-app");
    fs::create_dir_all(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("nogit-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write_file(&proj, "README.md", "# demo\n");
    let out = run(&db, &["register", &proj.display().to_string()]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));

    let doc = check_value(&db, &proj, &[]);
    let repo = alerts(&doc)
        .into_iter()
        .find(|a| a["symbol"] == "doctor/repository")
        .expect("repository alert");
    assert_eq!(repo["severity"], "warning");
    assert!(repo["message"]
        .as_str()
        .unwrap()
        .contains("not a git repository"));
}

#[test]
fn governance_plane_projects_selected_provider_status() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "gov-app");
    write_file(
        &proj,
        ".forge/providers.yaml",
        "provider:\n  provider: acme-governance\n  adapter: /nonexistent/adapter\n  enabled: true\n  protocol_version: 0.1.0\n  timeout_ms: 1000\n",
    );
    let doc = check_value(&db, &proj, &[]);
    let alert = alerts(&doc)
        .into_iter()
        .find(|a| a["symbol"] == "governance/acme-governance")
        .expect("governance alert");
    assert_eq!(alert["severity"], "warning");
    assert_eq!(alert["source"], "governance");
    let text = lossy(&run(&db, &["check", &proj.display().to_string()]).stdout);
    assert!(
        !text.contains(&proj.display().to_string()),
        "absolute project path leaked: {text}"
    );
}

#[test]
fn unregistered_target_fails_without_any_document() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ghost = tmp.path().join("ghost");
    fs::create_dir_all(&ghost).unwrap();
    let out = run(&db, &["check", &ghost.display().to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        out.stdout.is_empty(),
        "stdout must stay empty: {}",
        lossy(&out.stdout)
    );
    let err = lossy(&out.stderr);
    assert!(err.contains("error[unknown-project]"), "{err}");
}

#[test]
fn truncation_is_bounded_and_explicit() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "bound-app");
    // Drop the CI, DriftWatch and deployment configs: three warn
    // findings to exceed a small bound.
    fs::remove_file(proj.join(".github/workflows/ci.yml")).unwrap();
    fs::remove_file(proj.join("driftwatch.yaml")).unwrap();
    fs::remove_file(proj.join("Dockerfile")).unwrap();
    let full = alerts(&check_value(&db, &proj, &[]));
    assert!(full.len() >= 3, "fixture must produce warnings: {full:?}");

    let doc = check_value(&db, &proj, &["--max-alerts", "2"]);
    let list = alerts(&doc);
    assert_eq!(list.len(), 2, "the document must respect the bound");
    assert_eq!(list[1]["symbol"], "check/truncated");
    let message = list[1]["message"].as_str().unwrap();
    assert!(
        message.contains(&(full.len() - 1).to_string()),
        "summary alert must name the dropped count: {message}"
    );
}

#[test]
fn max_alerts_bound_is_validated() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "range-app");
    for bad in ["0", "10001"] {
        let out = run(
            &db,
            &["check", &proj.display().to_string(), "--max-alerts", bad],
        );
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty(), "stdout must stay empty");
        assert!(
            lossy(&out.stderr).contains("error[check-invalid]"),
            "{}",
            lossy(&out.stderr)
        );
    }
}

fn driftwatch_fixture(parent: &Path, marker: &Path) -> PathBuf {
    let script = parent.join("driftwatch-fixture.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch {}\ncat <<'JSON'\n{{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{{\"id\":\"DEPLOY-002\",\"category\":\"deployment\",\"severity\":\"fail\",\"applicable\":true,\"message\":\"deployment target not configured\",\"evidence\":[\"forge.yaml has no deployment.target\"]}}]}}\nJSON\n",
            marker.display()
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();
    script
}

#[test]
fn policy_plane_is_opt_in_only() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "policy-app");
    let marker = tmp.path().join("driftwatch-was-invoked");
    let fixture = driftwatch_fixture(tmp.path(), &marker);

    // Default: the checker never drives DriftWatch (feedback-loop guard).
    let doc = check_value(&db, &proj, &[]);
    assert!(
        !marker.exists(),
        "a default check run must not invoke the adapter"
    );
    assert!(!alerts(&doc)
        .iter()
        .any(|a| a["symbol"] == "driftwatch-DEPLOY-002"));

    // Explicit opt-in: the policy finding re-emits with its stable
    // rule symbol and error severity.
    let out = run_with_env(
        &db,
        &["check", &proj.display().to_string(), "--include-policy"],
        "FORGE_DRIFTWATCH_BIN",
        &fixture,
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    assert!(marker.exists(), "--include-policy must invoke the adapter");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let alert = alerts(&doc)
        .into_iter()
        .find(|a| a["symbol"] == "driftwatch-DEPLOY-002")
        .expect("policy alert");
    assert_eq!(alert["severity"], "error");
}

#[test]
fn credential_shaped_policy_evidence_is_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "leak-app");
    let marker = tmp.path().join("invoked");
    let script = tmp.path().join("leaky-driftwatch.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch {}\ncat <<'JSON'\n{{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{{\"id\":\"LEAK-001\",\"category\":\"security\",\"severity\":\"fail\",\"applicable\":true,\"message\":\"leaked github token ghp_abcdefghijklmnopqrstuvwxyz0123456789\",\"evidence\":[\"password=hunter2hunter2\"]}}]}}\nJSON\n",
            marker.display()
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let out = run_with_env(
        &db,
        &["check", &proj.display().to_string(), "--include-policy"],
        "FORGE_DRIFTWATCH_BIN",
        &script,
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(
        !text.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "{text}"
    );
    assert!(!text.contains("hunter2hunter2"), "{text}");
    assert!(text.contains("[REDACTED]"), "{text}");
}

#[test]
fn checker_run_mutates_nothing_and_repeats_stably() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "stable-app");

    let manifest_before = fs::read(proj.join("forge.yaml")).unwrap();
    let registry_before = fs::read(&db).unwrap();
    let head_before = {
        let out = Command::new("git")
            .arg("-C")
            .arg(&proj)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        // Freshly git-init-ed repositories have no HEAD; tolerate that.
        lossy(&out.stdout)
    };
    let tree_before = {
        let mut names: Vec<_> = fs::read_dir(&proj)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    };

    let first = run(&db, &["check", &proj.display().to_string()]);
    let second = run(&db, &["check", &proj.display().to_string()]);
    assert!(first.status.success() && second.status.success());

    let mut doc_a: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let mut doc_b: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    let (ga, gb) = (
        doc_a["generated_at"].as_str().unwrap().to_string(),
        doc_b["generated_at"].as_str().unwrap().to_string(),
    );
    doc_a["generated_at"] = serde_json::Value::Null;
    doc_b["generated_at"] = serde_json::Value::Null;
    assert_eq!(doc_a, doc_b, "only generated_at may differ: {ga} vs {gb}");

    assert_eq!(fs::read(proj.join("forge.yaml")).unwrap(), manifest_before);
    assert_eq!(fs::read(&db).unwrap(), registry_before);
    assert_eq!(head_before, {
        let out = Command::new("git")
            .arg("-C")
            .arg(&proj)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        lossy(&out.stdout)
    });
    let tree_after = {
        let mut names: Vec<_> = fs::read_dir(&proj)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    assert_eq!(tree_before, tree_after, "the project tree must not change");
}

#[test]
fn emitted_documents_parse_under_the_sibling_checker_protocol() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = clean_project(tmp.path(), &db, "proto-app");

    // Documents this surface emits are accepted, including the empty
    // document (a valid success under the sibling report rules).
    let clean = run(&db, &["check", &proj.display().to_string()]).stdout;
    assert_eq!(parse_like_driftwatch(&clean).unwrap(), 0);

    write_file(
        &proj,
        ".forge/providers.yaml",
        "provider:\n  provider: acme-governance\n  adapter: /nonexistent/adapter\n  enabled: true\n  protocol_version: 0.1.0\n  timeout_ms: 1000\n",
    );
    let findings = run(&db, &["check", &proj.display().to_string()]).stdout;
    assert!(parse_like_driftwatch(&findings).unwrap() > 0);

    // Protocol mirrors of the sibling's named scenarios: unknown fields
    // are tolerated, `{}` without `alerts` is a protocol error, missing
    // alert fields are refused.
    assert_eq!(
        parse_like_driftwatch(br#"{"alerts": [], "futureField": {"a": 1}}"#).unwrap(),
        0
    );
    assert!(parse_like_driftwatch(b"{}").is_err());
    assert!(parse_like_driftwatch(br#"{"alerts": [{"severity": "error"}]}"#).is_err());
    assert!(parse_like_driftwatch(
        br#"{"alerts": [{"severity": "error", "message": "m", "source": "s", "symbol": "x", "note": "tolerated"}]}"#
    ).is_ok());
}

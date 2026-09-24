//! DriftWatch CLI alignment contract (`driftwatch-cli-alignment`).
//!
//! Drives the real `forge` binary with `FORGE_DRIFTWATCH_BIN` pointed at
//! stub scripts that emit the verbatim sibling documents captured in
//! `tests/fixtures/driftwatch/` (see `NOTES.md` there). Covers what the
//! fabricated pre-alignment surface could never prove: only supported
//! flags are sent (no `--project`), the gate surface for gate-managed
//! projects is the real `gate --format json` (its `--dry-run` sibling
//! composition prints a human plan, never JSON), a parseable document
//! is findings whatever the exit code, an unknown contract names the
//! version instead of ever reading PASS, and the ordered binary probe
//! resolves `driftwatchdog` before the `driftwatch` alias.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/driftwatch")
        .join(name)
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn run_with_driftwatch(db: &Path, args: &[&str], driftwatch_bin: &Path) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env("FORGE_DRIFTWATCH_BIN", driftwatch_bin);
    cmd.arg("--registry").arg(db).arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn doctor_json(out: &std::process::Output) -> serde_json::Value {
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        !stdout.is_empty(),
        "empty stdout; stderr: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("doctor json")
}

fn findings_of<'a>(value: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    value["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["id"] == id)
        .collect()
}

/// A stub that answers the checker surface with the captured mixed
/// envelope (one alerting error row carrying a fake GitHub token and
/// one protocol-error row) and exits like a real non-dry-run failing
/// aggregate.
fn checker_stub(dir: &Path) -> PathBuf {
    let doc = fixture("checker-report-alerting.json")
        .display()
        .to_string();
    write_script(
        dir,
        "dw-checker.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit 1\n"
        ),
    )
}

#[test]
fn checker_envelope_produces_findings_and_redaction_through_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("alerted-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("alerted-app"));
    let stub = checker_stub(tmp.path());

    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &stub);
    let value = doctor_json(&out);
    let stdout = lossy(&out.stdout).to_string();
    let findings = value["doctor"]["findings"].as_array().unwrap();

    // Alert rows keep checker + symbol as the rule identity and the
    // checker-declared category; severity error maps to fail.
    let auth = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-auth/AUTH-001")
        .expect("checker/symbol finding present");
    assert_eq!(auth["status"], "fail");
    assert!(
        auth["detail"].as_str().unwrap().contains("[security:fail]"),
        "{auth}"
    );
    // The document's fake credential never escapes the redaction
    // pipeline, through any output surface.
    assert!(auth["detail"].as_str().unwrap().contains("[REDACTED]"));
    assert!(
        !stdout.contains("ghp_"),
        "credential leaked through doctor stdout"
    );
    // A failed checker row is evidence, not an adapter failure: the
    // rollup reports the run, and no unavailable placeholder appears.
    let broken = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-broken")
        .expect("protocol-error row surfaces");
    assert_eq!(broken["status"], "fail");
    let rollup = findings_of(&value, "driftwatch-policy");
    assert_eq!(rollup.len(), 1);
    assert_eq!(rollup[0]["status"], "fail");
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn only_supported_flags_are_sent_and_the_gate_surface_is_real() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // The stub records every invocation's argv so the test can assert
    // the exact flag surface Forge drives.
    let argv_log = tmp.path().join("argv.log");
    let checker_doc = fixture("checker-report-passing.json").display().to_string();
    let gate_doc = fixture("gate-status-pass.json").display().to_string();
    let stub = write_script(
        tmp.path(),
        "dw-argv.sh",
        &format!(
            "#!/bin/sh\necho \"$@\" >> '{}'\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nif [ \"$1\" = \"gate\" ]; then cat '{}'; exit 0; fi\ncat '{}'\nexit 0\n",
            argv_log.display(),
            gate_doc,
            checker_doc
        ),
    );

    let plain = tmp.path().join("plain-app");
    fs::create_dir(&plain).unwrap();
    write_file(&plain, "forge.yaml", &rust_manifest("plain-app"));
    let out = run_with_driftwatch(&db, &["doctor", &plain.display().to_string()], &stub);
    doctor_json(&out);

    let gated = tmp.path().join("gated-app");
    fs::create_dir(&gated).unwrap();
    write_file(&gated, "forge.yaml", &rust_manifest("gated-app"));
    write_file(&gated, "gate.toml", "version = 1\nprofile = \"minimal\"\n");
    let out = run_with_driftwatch(&db, &["doctor", &gated.display().to_string()], &stub);
    doctor_json(&out);

    let log = fs::read_to_string(&argv_log).unwrap();
    let invocations: Vec<&str> = log
        .lines()
        .filter(|l| !l.starts_with("--version"))
        .collect();
    assert_eq!(
        invocations,
        vec!["check --dry-run --format json", "gate --format json"],
        "Forge must send exactly the surfaces the sibling supports"
    );
    assert!(
        !log.contains("--project"),
        "the fabricated --project flag must be gone: {log}"
    );
    assert!(
        !log.contains("gate --dry-run"),
        "gate --dry-run prints the human plan, never a document: {log}"
    );
}

#[test]
fn blocked_gate_document_lowers_the_verdict_as_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("blocked-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("blocked-app"));
    write_file(&proj, ".ai-gate/gate.yaml", "version: 1\n");
    let doc = fixture("gate-status-blocked.json").display().to_string();
    let stub = write_script(
        &proj,
        "dw-gate.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit 1\n"
        ),
    );

    // A blocked gate exits non-zero; Forge must report its failing
    // checks as findings, not the plane as unavailable.
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &stub);
    let value = doctor_json(&out);
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let docs = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-docs")
        .expect("blocked gate row surfaces as a finding");
    assert_eq!(docs["status"], "fail");
    assert!(docs["evidence"]
        .to_string()
        .contains("Fix the failing command"));
    let rollup = findings_of(&value, "driftwatch-policy");
    assert_eq!(rollup[0]["status"], "fail");
    assert!(
        !findings
            .iter()
            .any(|f| f["id"] == "driftwatch-policy" && f["status"] == "unavailable"),
        "a parseable blocked gate is never adapter-unavailable"
    );
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn unknown_contract_names_the_version_and_never_passes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("future-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("future-app"));
    let doc = fixture("checker-report-unknown-contract.json")
        .display()
        .to_string();
    let stub = write_script(
        &proj,
        "dw-future.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit 1\n"
        ),
    );

    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &stub);
    let value = doctor_json(&out);
    let rollup = findings_of(&value, "driftwatch-policy");
    assert_eq!(rollup.len(), 1);
    assert_eq!(rollup[0]["status"], "unavailable");
    assert!(
        rollup[0]["evidence"]
            .to_string()
            .contains("driftwatch-checker/9.0.0"),
        "the refusal must name the unsupported contract: {rollup:?}"
    );
    // No per-rule findings are invented from an unparsed document.
    assert!(
        !value["doctor"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"]
                .as_str()
                .unwrap_or("")
                .starts_with("driftwatch-auth")),
        "unknown contract must not produce findings"
    );
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn stale_sibling_rejection_surfaces_exit_and_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("old-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("old-app"));
    let rejection = fixture("stale-binary-rejection.txt").display().to_string();
    // Mirrors the pre-checker-machine-output binary on this host: clap
    // rejects --format on stderr, empty stdout, exit 2.
    let stub = write_script(
        &proj,
        "dw-stale.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.0.9'; exit 0; fi\ncat '{rejection}' >&2\nexit 2\n"
        ),
    );

    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &stub);
    let value = doctor_json(&out);
    let rollup = findings_of(&value, "driftwatch-policy");
    assert_eq!(rollup[0]["status"], "unavailable");
    let evidence = rollup[0]["evidence"].to_string();
    assert!(evidence.contains("unexpected argument"), "{evidence}");
    assert!(evidence.contains("exit status"), "{evidence}");
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn ordered_path_probe_prefers_the_cargo_name_over_the_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("probe-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("probe-app"));

    // A PATH containing both sibling names under distinct documents:
    // resolution must run `driftwatchdog` (cargo/installer name) and
    // never reach the `driftwatch` alias.
    let bin = tmp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let alias_log = tmp.path().join("alias-argv.log");
    let doggy_doc = r#"{"contract":"driftwatch-checker/0.1.0","tool":"driftwatchdog","version":"0.1.0","generated_at":"2026-09-24T00:00:00+00:00","checkers":[{"name":"probe","status":"alerting","alerts":[{"severity":"warning","message":"resolved via the cargo name","source":"forge.yaml","symbol":"FROM-DOGGY"}]}],"summary":{"total":1,"ok":0,"alerting":1,"failed":0,"timeout":0,"protocol_error":0,"alerts":1}}"#;
    let alias_doc = doggy_doc.replace("FROM-DOGGY", "FROM-ALIAS");
    let doggy = bin.join("driftwatchdog");
    fs::write(
        &doggy,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\necho '{doggy_doc}'\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&doggy, fs::Permissions::from_mode(0o755)).unwrap();
    let alias = bin.join("driftwatch");
    fs::write(
        &alias,
        format!(
            "#!/bin/sh\necho \"$@\" >> '{}'\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\necho '{}'\n",
            alias_log.display(),
            alias_doc
        ),
    )
    .unwrap();
    fs::set_permissions(&alias, fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(forge_bin())
        .env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bin)
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args(["doctor", &proj.display().to_string()])
        .output()
        .expect("run forge");
    let value = doctor_json(&out);
    let findings = value["doctor"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["id"] == "driftwatch-probe/FROM-DOGGY"),
        "cargo-name binary must win: {findings:?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["id"].as_str().unwrap_or("").contains("FROM-ALIAS")),
        "alias must never run while driftwatchdog resolves"
    );
    assert!(
        !alias_log.exists() || fs::read_to_string(&alias_log).unwrap().is_empty(),
        "the alias binary was invoked"
    );
    // The observation names the resolved binary and its version.
    let probe = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-probe/FROM-DOGGY")
        .unwrap();
    let evidence = probe["evidence"].to_string();
    assert!(
        evidence.contains("driftwatchdog") && evidence.contains("0.1.0"),
        "{evidence}"
    );
}

#[test]
fn doctor_recognizes_the_real_driftwatch_markers() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bogus = tmp.path().join("no-such-binary");

    for (marker, project) in [
        ("driftwatch.toml", "checker-marker"),
        ("gate.toml", "gate-marker"),
        (".ai-gate/gate.yaml", "aigate-marker"),
    ] {
        let proj = tmp.path().join(project);
        fs::create_dir_all(&proj).unwrap();
        write_file(&proj, "forge.yaml", &rust_manifest(project));
        write_file(&proj, marker, "version = 1\n");
        let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &bogus);
        let value = doctor_json(&out);
        let config = findings_of(&value, "driftwatch-config");
        assert_eq!(config[0]["status"], "pass", "{marker} must be recognized");
        assert!(
            config[0]["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e.as_str().unwrap_or("") == marker),
            "{marker} must be named in evidence: {config:?}"
        );
        // Presence is evidence only: the policy plane still reports its
        // own availability honestly, as a distinct finding.
        let policy = findings_of(&value, "driftwatch-policy");
        assert_eq!(policy[0]["status"], "unavailable");
    }

    let bare = tmp.path().join("bare-marker");
    fs::create_dir_all(&bare).unwrap();
    write_file(&bare, "forge.yaml", &rust_manifest("bare-marker"));
    let out = run_with_driftwatch(&db, &["doctor", &bare.display().to_string()], &bogus);
    let value = doctor_json(&out);
    let config = findings_of(&value, "driftwatch-config");
    assert_eq!(config[0]["status"], "warn");
    assert!(
        config[0]["evidence"].as_array().unwrap().iter().any(|e| e
            .as_str()
            .unwrap_or("")
            .contains("no driftwatch configuration")),
        "absence is its own distinct state: {config:?}"
    );
}

#[test]
fn legacy_adapter_reports_keep_prior_semantics() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("legacy-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("legacy-app"));
    // Existing fixture scripts emit a bare PolicyReport document; that
    // path must keep working unchanged (Reported on success,
    // Unavailable on non-zero exit).
    let legacy = r#"{"tool":"driftwatch","tool_version":"0.1.0","contract":"0.1.0","source_revision":null,"findings":[{"id":"AUTH-001","category":"security","severity":"fail","applicable":true,"message":"missing auth markers","evidence":[]}]}"#;
    let stub = write_script(
        &proj,
        "dw-legacy.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\necho '{legacy}'\n"
        ),
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &stub);
    let value = doctor_json(&out);
    let findings = value["doctor"]["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["id"] == "driftwatch-AUTH-001"),
        "legacy rules keep their ids: {findings:?}"
    );
}

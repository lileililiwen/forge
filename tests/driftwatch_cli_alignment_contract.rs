//! DriftWatch CLI alignment contract (`driftwatch-cli-alignment`).
//!
//! Drives the real adapter and provider surfaces against stub binaries
//! planted on a controlled PATH, proving: the ordered resolution
//! (`FORGE_DRIFTWATCH_BIN` → `driftwatchdog` → `driftwatch`), the
//! fabricated-`--project` removal with cwd confinement, checker-report
//! and gate-status envelope consumption, blocked-gate-is-evidence
//! classification, unknown-contract refusal, host-path scrubbing, and
//! doctor workspace-marker detection.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Forge CLI with a fully controlled environment: no registry leakage,
/// no inherited driftwatch override, and an explicit PATH.
fn forge(db: &Path, path_env: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env("PATH", path_env)
        .arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn write_bin(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// Stub answering the real grammar: version probe plus a checker-report
/// envelope with one warning alert. `$2` records the invocation argv.
fn checker_stub(version_tag: &str, symbol: &str) -> String {
    format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'driftwatch {version_tag}'; exit 0; fi\n\
         if [ -n \"${{DRIFTWATCH_STUB_LOG:-}}\" ]; then echo \"$@\" >> \"$DRIFTWATCH_STUB_LOG\"; fi\n\
         printf '%s' '{{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"{version_tag}\",\"checkers\":[{{\"name\":\"sec-scan\",\"status\":\"alerting\",\"alerts\":[{{\"severity\":\"warning\",\"message\":\"stub finding via real grammar\",\"source\":\"src/main.rs\",\"symbol\":\"{symbol}\"}}]}}],\"summary\":{{\"total\":1,\"ok\":0,\"alerting\":1,\"failed\":0,\"timeout\":0,\"protocol_error\":0,\"alerts\":1}}}}'\n"
    )
}

fn rust_proj(tmp: &Path) -> PathBuf {
    let proj = tmp.join("app");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: app\n  name: app\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(proj.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
    proj
}

fn doctor_dw_findings(out: &std::process::Output) -> Vec<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("json: {err}; stderr={}", lossy(&out.stderr)));
    value["doctor"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn ordered_probe_prefers_driftwatchdog_then_the_npm_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let log = tmp.path().join("stub.log");
    write_bin(&bins, "driftwatchdog", &checker_stub("9.1.0-cargo", "WD-1"));
    write_bin(&bins, "driftwatch", &checker_stub("9.2.0-npm", "DW-1"));
    let proj = rust_proj(tmp.path());

    let out = {
        let mut cmd = Command::new(forge_bin());
        cmd.env_remove("FORGE_REGISTRY")
            .env_remove("FORGE_DRIFTWATCH_BIN")
            .env("PATH", &bins)
            .env("DRIFTWATCH_STUB_LOG", &log)
            .arg("--registry")
            .arg(&db)
            .arg("--format")
            .arg("json")
            .args(["doctor", proj.to_str().unwrap()]);
        cmd.output().expect("forge doctor")
    };
    let findings = doctor_dw_findings(&out);
    let alert = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-sec-scan/WD-1")
        .unwrap_or_else(|| panic!("cargo-name binary must answer: {findings:?}"));
    assert_eq!(alert["status"], "warn");
    assert!(
        alert["evidence"]
            .to_string()
            .contains("driftwatchdog 9.1.0-cargo"),
        "{alert}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["id"] == "driftwatch-sec-scan/DW-1"),
        "the alias must not be reached while driftwatchdog exists"
    );

    // Alias-only hosts resolve the npm launcher name.
    fs::remove_file(bins.join("driftwatchdog")).unwrap();
    fs::write(&log, "").unwrap();
    let out = {
        let mut cmd = Command::new(forge_bin());
        cmd.env_remove("FORGE_REGISTRY")
            .env_remove("FORGE_DRIFTWATCH_BIN")
            .env("PATH", &bins)
            .env("DRIFTWATCH_STUB_LOG", &log)
            .arg("--registry")
            .arg(&db)
            .arg("--format")
            .arg("json")
            .args(["doctor", proj.to_str().unwrap()]);
        cmd.output().expect("forge doctor")
    };
    let findings = doctor_dw_findings(&out);
    let alert = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-sec-scan/DW-1")
        .unwrap_or_else(|| panic!("alias must answer when the cargo name is absent: {findings:?}"));
    assert!(
        alert["evidence"]
            .to_string()
            .contains("driftwatch 9.2.0-npm"),
        "{alert}"
    );

    // The real grammar only: no fabricated --project flag ever sent, and
    // the surface is check --dry-run --format json.
    let recorded = fs::read_to_string(&log).unwrap();
    assert!(
        recorded.contains("check --dry-run --format json"),
        "{recorded}"
    );
    assert!(!recorded.contains("--project"), "{recorded}");
}

#[test]
fn no_binary_on_path_is_unavailable_naming_attempts() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let empty = tmp.path().join("emptybin");
    fs::create_dir_all(&empty).unwrap();
    let proj = rust_proj(tmp.path());
    let out = forge(&db, &empty, &["doctor", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let findings = doctor_dw_findings(&out);
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .expect("unavailable plane must be reported");
    assert_eq!(dw["status"], "unavailable");
    let text = dw["evidence"].to_string();
    assert!(text.contains("driftwatchdog"), "{text}");
    assert!(text.contains("driftwatch"), "{text}");
}

#[test]
fn operator_override_runs_exactly_that_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    write_bin(&bins, "driftwatchdog", &checker_stub("PATH-wins", "WD-1"));
    write_bin(&bins, "driftwatch", &checker_stub("PATH-alias", "DW-1"));
    let fixture_dir = tmp.path().join("fix");
    fs::create_dir_all(&fixture_dir).unwrap();
    let fixture = write_bin(
        &fixture_dir,
        "override-fixture",
        &checker_stub("OVERRIDE-1.0", "OV-1"),
    );
    let proj = rust_proj(tmp.path());
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env("PATH", &bins)
        .env("FORGE_DRIFTWATCH_BIN", &fixture)
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args(["doctor", proj.to_str().unwrap()]);
    let out = cmd.output().expect("forge doctor");
    let findings = doctor_dw_findings(&out);
    assert!(
        findings
            .iter()
            .any(|f| f["id"] == "driftwatch-sec-scan/OV-1"),
        "override binary must answer: {findings:?}"
    );
    assert!(
        !findings
            .iter()
            .any(|f| f["id"] == "driftwatch-sec-scan/WD-1"),
        "PATH names must not run when an override is set: {findings:?}"
    );
}

#[test]
fn blocked_gate_document_lowers_the_verdict_as_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let log = tmp.path().join("stub.log");
    // Answers only the gate surface; exits non-zero like a real blocked
    // gate (exit-code authority) while keeping the document on stdout.
    write_bin(
        &bins,
        "driftwatchdog",
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'driftwatchdog 0.4.2'; exit 0; fi\n\
         echo \"$@\" >> \"$DRIFTWATCH_STUB_LOG\"\n\
         printf '%s' '{\"status\":\"FAIL\",\"blocked\":true,\"failures\":[\"quality\"],\"pending_reviews\":[],\"not_applicable\":[],\"manifest_digest\":\"sha\",\"rule_pack_version\":1,\"results\":[{\"gate_id\":\"build\",\"source\":\"ci\",\"status\":\"PASS\",\"severity\":\"info\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[]},{\"gate_id\":\"quality\",\"source\":\"driftwatch\",\"status\":\"FAIL\",\"severity\":\"error\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[\"tests:run\"],\"diagnostic\":\"product tests failing\",\"remediation\":\"run cargo test\"}]}'\n\
         exit 1\n",
    );
    let proj = rust_proj(tmp.path());
    fs::write(proj.join("gate.toml"), "[gate]\nprofile = \"product\"\n").unwrap();
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bins)
        .env("DRIFTWATCH_STUB_LOG", &log)
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args(["doctor", proj.to_str().unwrap()]);
    let out = cmd.output().expect("forge doctor");
    let findings = doctor_dw_findings(&out);
    let quality = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-quality")
        .unwrap_or_else(|| panic!("blocked gate maps to fail findings: {findings:?}"));
    assert_eq!(quality["status"], "fail");
    assert!(quality["evidence"]
        .to_string()
        .contains("missing=tests:run"));
    assert_eq!(
        findings
            .iter()
            .filter(|f| f["id"] == "driftwatch-build")
            .count(),
        0
    );
    let rollup = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .expect("reported plane carries the rollup finding");
    assert_eq!(
        rollup["status"], "fail",
        "a parseable gate document is evidence, never 'unavailable': {rollup}"
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["doctor"]["healthy"], false);
    let recorded = fs::read_to_string(&log).unwrap();
    assert!(recorded.contains("gate --format json"), "{recorded}");
    assert!(
        !recorded.contains("--dry-run"),
        "the gate JSON writer is only reached without --dry-run (see tests/fixtures/driftwatch/NOTES.md): {recorded}"
    );
}

#[test]
fn unknown_contract_never_reports_and_host_paths_are_scrubbed() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    write_bin(
        &bins,
        "driftwatchdog",
        "#!/bin/sh\nprintf '%s' '{\"contract\":\"driftwatch-checker/2.0.0\",\"tool\":\"driftwatch\",\"version\":\"2\",\"checkers\":[],\"summary\":{}}'\n",
    );
    let proj = rust_proj(tmp.path());
    let out = forge(&db, &bins, &["doctor", proj.to_str().unwrap()]);
    let findings = doctor_dw_findings(&out);
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .expect("unknown contract is unavailable");
    assert_eq!(dw["status"], "unavailable");
    let text = dw["evidence"].to_string();
    assert!(text.contains("driftwatch-checker/2.0.0"), "{text}");

    // A 2.0.0 document claiming a PASSing checker still cannot report.
    write_bin(
        &bins,
        "driftwatchdog",
        "#!/bin/sh\nprintf '%s' '{\"contract\":\"driftwatch-checker/2.0.0\",\"tool\":\"driftwatch\",\"version\":\"2\",\"checkers\":[{\"name\":\"x\",\"status\":\"ok\",\"alerts\":[]}],\"summary\":{}}'\n",
    );
    let out = forge(&db, &bins, &["doctor", proj.to_str().unwrap()]);
    let findings = doctor_dw_findings(&out);
    assert!(
        !findings
            .iter()
            .any(|f| f["id"].as_str() == Some("driftwatch-x")),
        "unsupported contract must never contribute findings: {findings:?}"
    );
}

#[test]
fn alert_evidence_secrets_and_absolute_paths_scrub_through_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let proj = rust_proj(tmp.path());
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let body = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.4.2'; exit 0; fi\n\
         printf '%s' '{{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"project\":\"{}\",\"checkers\":[{{\"name\":\"leaky\",\"status\":\"alerting\",\"alerts\":[{{\"severity\":\"error\",\"message\":\"found {secret} under {}\",\"source\":\"{}\",\"symbol\":\"SEC-9\"}}]}}],\"summary\":{{\"total\":1,\"ok\":0,\"alerting\":1,\"failed\":0,\"timeout\":0,\"protocol_error\":0,\"alerts\":1}}}}'\n",
        proj.display(),
        proj.display(),
        proj.join("src/main.rs").display()
    );
    write_bin(&bins, "driftwatchdog", &body);
    let out = forge(&db, &bins, &["doctor", proj.to_str().unwrap()]);
    let stdout = lossy(&out.stdout);
    assert!(!stdout.contains(secret), "secret leaked: {stdout}");
    // The finding itself is scrubbed: Forge's own report legitimately
    // names the assessed path, so check the finding object, not stdout.
    let findings = doctor_dw_findings(&out);
    let leaky = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-leaky/SEC-9")
        .unwrap_or_else(|| panic!("scrubbed alert missing: {findings:?}"));
    let text = leaky.to_string();
    assert!(text.contains("[REDACTED]"), "{text}");
    assert!(text.contains("<project>"), "{text}");
    assert!(
        !text.contains(&proj.display().to_string()),
        "host path leaked into finding: {text}"
    );
}

#[test]
fn doctor_recognizes_sibling_markers() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let empty = tmp.path().join("emptybin");
    fs::create_dir_all(&empty).unwrap();
    for marker in ["driftwatch.toml", "gate.toml", ".ai-gate/gate.yaml"] {
        let proj = tmp.path().join(format!(
            "marker-{}",
            marker.replace('/', "-").replace('.', "")
        ));
        let path = proj.join(marker);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "marker\n").unwrap();
        fs::write(
            proj.join("forge.yaml"),
            "schema: 1\nproject:\n  id: marker-app\n  name: marker\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
        )
        .unwrap();
        fs::write(proj.join("Cargo.toml"), "[package]\nname = \"m\"\n").unwrap();
        let out = forge(&db, &empty, &["doctor", proj.to_str().unwrap()]);
        let findings = doctor_dw_findings(&out);
        let dw = findings
            .iter()
            .find(|f| f["id"] == "driftwatch-config")
            .expect("config finding present");
        assert_eq!(dw["status"], "pass", "{marker}: {dw}");
        assert!(
            dw["evidence"].to_string().contains(marker),
            "{marker}: {dw}"
        );
    }
    // No markers stay a distinct state from any configured outcome.
    let proj = rust_proj(tmp.path());
    let out = forge(&db, &empty, &["doctor", proj.to_str().unwrap()]);
    let findings = doctor_dw_findings(&out);
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-config")
        .expect("config finding present");
    assert_eq!(dw["status"], "warn");
    assert!(dw["evidence"]
        .to_string()
        .contains("no driftwatch configuration"));
}

#[test]
fn plain_checker_run_never_contacts_the_policy_binary() {
    // Feedback-loop guard survives the alignment: `forge check` without
    // --include-policy must not invoke DriftWatch at all.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    let log = tmp.path().join("stub.log");
    write_bin(&bins, "driftwatchdog", &checker_stub("0.4.2", "SEC-1"));
    let proj = rust_proj(tmp.path());
    let mut reg = Command::new(forge_bin());
    reg.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bins)
        .arg("--registry")
        .arg(&db)
        .args(["register", proj.to_str().unwrap()]);
    let registered = reg.output().expect("forge register");
    assert_eq!(
        registered.status.code(),
        Some(0),
        "{}",
        lossy(&registered.stderr)
    );
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bins)
        .env("DRIFTWATCH_STUB_LOG", &log)
        .arg("--registry")
        .arg(&db)
        .args(["check", "app"]);
    let out = cmd.output().expect("forge check");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        !log.exists(),
        "plain check must not drive DriftWatch: {}",
        fs::read_to_string(&log).unwrap_or_default()
    );
    // The policy plane stays opt-in only.
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bins)
        .env("DRIFTWATCH_STUB_LOG", &log)
        .arg("--registry")
        .arg(&db)
        .args(["check", proj.to_str().unwrap(), "--include-policy"]);
    let out = cmd.output().expect("forge check --include-policy");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let logged = fs::read_to_string(&log).unwrap_or_default();
    assert!(logged.contains("check --dry-run --format json"), "{logged}");
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("driftwatch-sec-scan/SEC-1"), "{stdout}");
}

#[test]
fn provider_probe_consumes_envelopes_and_survives_nonzero_exit() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let fix = tmp.path().join("fix");
    fs::create_dir_all(&fix).unwrap();
    // Parseable document with a failing checker row, exit 2: the round
    // trip succeeded; findings keep their own severities.
    let grumpy = write_bin(
        &fix,
        "grumpy.sh",
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'driftwatchdog 0.4.2'; exit 0; fi\n\
         printf '%s' '{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"checkers\":[{\"name\":\"boom\",\"status\":\"failed\",\"alerts\":[],\"error\":\"checker exited\"}],\"summary\":{\"total\":1,\"ok\":0,\"alerting\":0,\"failed\":1,\"timeout\":0,\"protocol_error\":0,\"alerts\":0}}'\n\
         exit 2\n",
    );
    let out = {
        let mut cmd = Command::new(forge_bin());
        cmd.env_remove("FORGE_REGISTRY")
            .env_remove("FORGE_DRIFTWATCH_BIN")
            .env("PATH", tmp.path().join("emptybin"))
            .arg("--registry")
            .arg(&db)
            .arg("--format")
            .arg("json")
            .args([
                "provider",
                "run",
                "driftwatch-policy",
                "--fixture",
                grumpy.to_str().unwrap(),
            ]);
        fs::create_dir_all(tmp.path().join("emptybin")).unwrap();
        cmd.output().expect("provider run")
    };
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("json: {err}; stderr={}", lossy(&out.stderr)));
    let row = &value["run"];
    assert_eq!(row["status"], "supported", "{row}");
    let evidence = row["evidence"].to_string();
    assert!(
        evidence.contains("contract=driftwatch-checker/0.1.0"),
        "{evidence}"
    );
    assert!(evidence.contains("checkers=1"), "{evidence}");
    assert!(evidence.contains("exit=Some(2)"), "{evidence}");

    // Unparseable stdout stays unavailable.
    let junk = write_bin(
        &fix,
        "junk.sh",
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 1; fi\necho 'not json'\n",
    );
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env("PATH", tmp.path().join("emptybin"))
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args([
            "provider",
            "run",
            "driftwatch-policy",
            "--fixture",
            junk.to_str().unwrap(),
        ]);
    let out = cmd.output().expect("provider run");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["run"]["status"], "unavailable", "{}", value["run"]);
    let detail = value["run"]["evidence"].to_string();
    assert!(detail.contains("not parseable JSON"), "{detail}");
}

#[test]
fn provider_live_probe_scans_the_ordered_names() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bins = tmp.path().join("bins");
    fs::create_dir_all(&bins).unwrap();
    write_bin(
        &bins,
        "driftwatchdog",
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'driftwatchdog 0.4.2'; exit 0; fi\n\
         printf '%s' '{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"checkers\":[],\"summary\":{\"total\":0,\"ok\":0,\"alerting\":0,\"failed\":0,\"timeout\":0,\"protocol_error\":0,\"alerts\":0}}'\n",
    );
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", &bins)
        .env("FORGE_PROVIDER_LIVE", "1")
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args(["provider", "run", "driftwatch-policy", "--live"]);
    let out = cmd.output().expect("provider run --live");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("json: {err}; stderr={}", lossy(&out.stderr)));
    let row = &value["run"];
    assert_eq!(row["status"], "supported", "{row}");
    assert_eq!(row["provenance"]["sandbox"], "live");
    assert!(
        row["provenance"]["source"]
            .to_string()
            .contains("driftwatchdog"),
        "{row}"
    );
}

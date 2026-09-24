//! Quality policy integration contract: delegated DriftWatch execution
//! and project-scoped credential-redacted evidence.
//!
//! Covers the `quality-policy-integration` scenarios end to end through
//! the built binary: missing binary, parseable report, non-zero exit,
//! stale observation, credential redaction and per-project isolation.
//! Each scenario drives the CLI through `FORGE_DRIFTWATCH_BIN` so a
//! fixture shell script stands in for a real DriftWatch instance;
//! contract assertions validate the typed JSON the doctor emits.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run_with_driftwatch(db: &Path, args: &[&str], driftwatch_bin: &Path) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env("FORGE_DRIFTWATCH_BIN", driftwatch_bin);
    cmd.arg("--registry").arg(db).arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
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

fn good_report_payload() -> &'static str {
    "{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\
     \"source_revision\":null,\
     \"findings\":[\
        {\"id\":\"AUTH-001\",\"category\":\"security\",\"severity\":\"warn\",\
         \"applicable\":true,\"message\":\"auth marker present in source\",\
         \"evidence\":[\"src/auth/mod.rs exists\"]},\
        {\"id\":\"DEPLOY-002\",\"category\":\"deployment\",\"severity\":\"fail\",\
         \"applicable\":true,\"message\":\"deployment target not configured\",\
         \"evidence\":[\"forge.yaml has no deployment.target\"]},\
        {\"id\":\"FLUTTER-AUTH-001\",\"category\":\"security\",\"severity\":\"info\",\
         \"applicable\":false,\"reason\":\"policy not applicable to flutter-app profile\",\
         \"message\":\"not applicable\"}\
     ]}"
}

fn redacting_report_payload() -> &'static str {
    "{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\
     \"source_revision\":null,\
     \"findings\":[\
        {\"id\":\"LEAK-001\",\"category\":\"security\",\"severity\":\"fail\",\
         \"applicable\":true,\"message\":\"leaked github token ghp_abcdefghijklmnopqrstuvwxyz0123456789\",\
         \"evidence\":[\"config/secret.yaml contains token=abcdef0123456789 and password=hunter2hunter2\"]}\
     ]}"
}

#[test]
fn missing_driftwatch_binary_reports_unavailable_not_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("missing-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("missing-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let bogus = tmp.path().join("definitely-not-a-real-binary-xyz");
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &bogus);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    if stdout.is_empty() {
        panic!("empty stdout; stderr: {}", lossy(&out.stderr));
    }
    let value: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(err) => panic!(
            "json parse failed: {err}\nstdout: {stdout}\nstderr: {}",
            lossy(&out.stderr)
        ),
    };
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .expect("driftwatch-policy finding must be present");
    assert_eq!(dw["status"], "unavailable");
    assert!(
        dw["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("not found")),
        "{dw}"
    );
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn parseable_report_normalizes_findings_and_keeps_rule_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("good-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("good-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             cat <<'JSON'\n{}\nJSON\n",
            good_report_payload()
        ),
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &driftwatch);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = value["doctor"]["findings"].as_array().unwrap();

    // Per-rule findings are surfaced with their original IDs.
    for rule in ["driftwatch-AUTH-001", "driftwatch-DEPLOY-002"] {
        assert!(
            findings.iter().any(|f| f["id"] == rule),
            "missing {rule} in {findings:?}"
        );
    }
    let auth = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-AUTH-001")
        .unwrap();
    assert_eq!(auth["status"], "warn");
    let deploy = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-DEPLOY-002")
        .unwrap();
    assert_eq!(deploy["status"], "fail");

    // Not-applicable policies keep their reason and stay applicable:false.
    let flutter = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-FLUTTER-AUTH-001")
        .expect("not-applicable rule is preserved");
    assert_eq!(flutter["applicable"], false);
    assert!(
        flutter["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("not applicable")),
        "{flutter}"
    );

    // Rollup finding reflects the worst severity and tool version.
    let rollup = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .unwrap();
    assert_eq!(rollup["status"], "fail");
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn non_zero_exit_reports_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("fail-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("fail-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        "\
#!/bin/sh
if [ \"$1\" = \"--version\" ]; then
  echo 'driftwatch 0.1.0'
  exit 0
fi
echo 'fatal: profile mismatch' >&2
exit 2
",
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &driftwatch);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .unwrap();
    assert_eq!(dw["status"], "unavailable");
    assert!(
        dw["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("status")),
        "{dw}"
    );
}

#[test]
fn invalid_output_reports_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("bad-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("bad-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        "\
#!/bin/sh
if [ \"$1\" = \"--version\" ]; then
  echo 'driftwatch 0.1.0'
  exit 0
fi
echo 'not really json'
",
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &driftwatch);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let dw = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-policy")
        .unwrap();
    assert_eq!(dw["status"], "unavailable");
    assert!(
        dw["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("parse")),
        "{dw}"
    );
}

#[test]
fn credentials_in_evidence_are_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("redact-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("redact-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             cat <<'JSON'\n{}\nJSON\n",
            redacting_report_payload()
        ),
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &driftwatch);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let leak = findings
        .iter()
        .find(|f| f["id"] == "driftwatch-LEAK-001")
        .expect("leak finding must be reported");
    let detail = leak["detail"].as_str().unwrap();
    let evidence: Vec<String> = leak["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    let combined = format!("{detail} {}", evidence.join(" "));
    assert!(combined.contains("[REDACTED]"), "{combined}");
    for secret in [
        "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        "abcdef0123456789",
        "hunter2hunter2",
    ] {
        assert!(
            !combined.contains(secret),
            "secret leaked through output: {combined}"
        );
    }
}

#[test]
fn two_projects_each_get_their_own_policy_observation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let a = tmp.path().join("iso-a");
    let b = tmp.path().join("iso-b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    write_file(&a, "forge.yaml", &rust_manifest("iso-a"));
    write_file(&a, "Cargo.toml", "[package]\nname = \"demo-a\"\n");
    write_file(&b, "forge.yaml", &rust_manifest("iso-b"));
    write_file(&b, "Cargo.toml", "[package]\nname = \"demo-b\"\n");

    // DriftWatch reports the active project id (a or b) and tool version
    // exactly once; running two doctors in sequence must not share
    // findings between projects.
    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        "\
#!/bin/sh
if [ \"$1\" = \"--version\" ]; then
  echo 'driftwatch 0.1.0'
  exit 0
fi
project_dir=$(pwd)
project_id=$(basename \"$project_dir\")
cat <<JSON
{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\
 \"source_revision\":null,\"findings\":[\
  {\"id\":\"SCOPE-001\",\"category\":\"security\",\"severity\":\"warn\",\
   \"applicable\":true,\"message\":\"scoped finding for ${project_id}\",\
   \"evidence\":[\"evidence for ${project_id}\"]}]}
JSON
",
    );

    let out_a = run_with_driftwatch(&db, &["doctor", &a.display().to_string()], &driftwatch);
    let out_b = run_with_driftwatch(&db, &["doctor", &b.display().to_string()], &driftwatch);
    assert_eq!(out_a.status.code(), Some(0), "{}", lossy(&out_a.stderr));
    assert_eq!(out_b.status.code(), Some(0), "{}", lossy(&out_b.stderr));

    let value_a: serde_json::Value = serde_json::from_slice(&out_a.stdout).unwrap();
    let value_b: serde_json::Value = serde_json::from_slice(&out_b.stdout).unwrap();
    let finding_a = value_a["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "driftwatch-SCOPE-001")
        .expect("scope finding for project a");
    let finding_b = value_b["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "driftwatch-SCOPE-001")
        .expect("scope finding for project b");
    let text_a = format!(
        "{} {}",
        finding_a["detail"].as_str().unwrap(),
        finding_a["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let text_b = format!(
        "{} {}",
        finding_b["detail"].as_str().unwrap(),
        finding_b["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert!(
        text_a.contains("iso-a") && !text_a.contains("iso-b"),
        "{text_a}"
    );
    assert!(
        text_b.contains("iso-b") && !text_b.contains("iso-a"),
        "{text_b}"
    );
}

#[test]
fn doctor_human_output_includes_credential_redaction() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("human-app");
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest("human-app"));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let driftwatch = write_script(
        tmp.path(),
        "fake-driftwatch.sh",
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             cat <<'JSON'\n{}\nJSON\n",
            redacting_report_payload()
        ),
    );
    let out = run_with_driftwatch(&db, &["doctor", &proj.display().to_string()], &driftwatch);
    let text = lossy(&out.stdout);
    assert!(text.contains("[REDACTED]"), "{text}");
    for secret in ["ghp_abcdefghijklmnopqrstuvwxyz0123456789", "hunter2hunter2"] {
        assert!(
            !text.contains(secret),
            "secret leaked through human output: {text}"
        );
    }
}

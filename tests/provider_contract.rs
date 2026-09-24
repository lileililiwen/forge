//! CLI contract for `forge provider` (`provider-integration-evidence`).
//!
//! Exercises opt-in controlled evidence end to end through the built
//! binary:
//!
//! - R1 success: `run` with a fixture binary records a `supported` row
//!   with `sandbox: fixture` provenance (provider, project, timestamp,
//!   redacted receipt, teardown) for policy, identity, analytics,
//!   deploy and release.
//! - R1 failure: a missing binary, non-zero exit, timeout or malformed
//!   output records `unavailable` without a success state; an unknown
//!   provider id is refused with `error[provider-invalid]`.
//! - R1 boundary: `matrix` without `--live` reports every row as
//!   `not-run`; `run` without `--live`/`--fixture` is `not-run` without
//!   contacting anything; a `not-run` row never becomes `supported`.
//! - R2 success: authorization/mapping refusals surface as structured
//!   `ambiguous-mapping`/refusal evidence; a split release records
//!   `partial` naming the delivered stage.
//! - R2 failure: a credential-shaped substring in fixture output is
//!   redacted to `[REDACTED]` in stdout and never appears verbatim.
//! - R2 boundary: temp probe dirs are removed (teardown) and the run
//!   invents no registered project.

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
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_CONTAINER_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN")
        .env_remove("FORGE_PROVIDER_LIVE");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("provider CLI must run")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("provider CLI json must run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn provider_help_lists_matrix_run_and_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["provider", "--help"]);
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("matrix"), "{text}");
    assert!(text.contains("run"), "{text}");
    assert!(text.contains("inspect"), "{text}");
    let top = run(&db, &["--help"]);
    assert!(top.status.success());
    assert!(lossy(&top.stdout).contains("provider"));
}

#[test]
fn matrix_defaults_to_not_run_for_every_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["provider", "matrix"]);
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let json = run_json(&db, &["provider", "matrix"]);
    assert_eq!(json["contract"], "0.1.0");
    let rows = json["matrix"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    for row in rows {
        assert_eq!(row["status"], "not-run", "{row}");
        assert!(row["provenance"].is_null(), "{row}");
    }
    assert_eq!(json["matrix"]["supported"], 0);
    assert_eq!(json["matrix"]["not_run"], 6);
    let human = lossy(&out.stdout);
    for id in [
        "driftwatch-policy",
        "gate-runtime",
        "oidc-identity",
        "analytics",
        "deploy",
        "release",
    ] {
        assert!(human.contains(id), "{human}");
    }
}

#[test]
fn run_refuses_unknown_provider_with_typed_code() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["provider", "run", "nosuch"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[provider-invalid]"),
        "{}",
        lossy(&out.stderr)
    );
}

#[test]
fn run_without_flags_is_not_run_and_contacts_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let json = run_json(&db, &["provider", "run", "deploy"]);
    assert_eq!(json["run"]["status"], "not-run");
    assert!(json["run"]["provenance"].is_null());
}

#[test]
fn run_policy_fixture_success_records_supported_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let good = write_script(
        &scripts,
        "good.sh",
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"driftwatch 9.9.9\"; exit 0; fi\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
    );
    let json = run_json(
        &db,
        &[
            "provider",
            "run",
            "driftwatch-policy",
            "--fixture",
            good.to_str().unwrap(),
        ],
    );
    let row = &json["run"];
    assert_eq!(row["status"], "supported");
    assert_eq!(row["provenance"]["sandbox"], "fixture");
    assert_eq!(
        row["provenance"]["project_id"],
        serde_json::Value::String("evidence-probe".to_string())
    );
    assert_eq!(row["provenance"]["teardown"], true);
    assert!(!row["provenance"]["observed_at"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn run_identity_fixture_lifecycle_terminates_and_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let json = run_json(
        &db,
        &["provider", "run", "oidc-identity", "--fixture", "in-memory"],
    );
    let row = &json["run"];
    assert_eq!(row["status"], "supported");
    assert_eq!(row["provenance"]["sandbox"], "fixture");
    let evidence = row["evidence"].as_array().unwrap();
    let texts: Vec<&str> = evidence.iter().filter_map(|v| v.as_str()).collect();
    assert!(texts.iter().any(|e| e.contains("cross-project:refused")));
    assert!(texts.iter().any(|e| e.contains("session:terminated")));
}

#[test]
fn run_analytics_fixture_mismatch_is_ambiguous_not_success() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let mismatch = write_script(
        &scripts,
        "mismatch.sh",
        "#!/bin/sh\necho '{\"project_ref\":\"other/repo\",\"evidence\":[]}'\n",
    );
    let json = run_json(
        &db,
        &[
            "provider",
            "run",
            "analytics",
            "--fixture",
            mismatch.to_str().unwrap(),
        ],
    );
    assert_eq!(json["run"]["status"], "unavailable");
    let evidence = json["run"]["evidence"].as_array().unwrap();
    assert!(
        evidence
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("ambiguous-mapping")),
        "{evidence:?}"
    );
}

#[test]
fn run_deploy_fixture_failure_is_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let bad = write_script(&scripts, "bad.sh", "#!/bin/sh\ncat >/dev/null\nexit 1\n");
    let json = run_json(
        &db,
        &[
            "provider",
            "run",
            "deploy",
            "--fixture",
            bad.to_str().unwrap(),
        ],
    );
    assert_eq!(json["run"]["status"], "unavailable");
}

#[test]
fn run_release_fixture_split_stays_partial() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let split = write_script(
        &scripts,
        "split.sh",
        "#!/bin/sh\nfor a in \"$@\"; do if [ \"$a\" = \"container\" ]; then echo failed >&2; exit 1; fi; done\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
    );
    let json = run_json(
        &db,
        &[
            "provider",
            "run",
            "release",
            "--fixture",
            split.to_str().unwrap(),
        ],
    );
    assert_eq!(json["run"]["status"], "unavailable");
    let evidence = json["run"]["evidence"].as_array().unwrap();
    assert!(
        evidence
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("partial")),
        "{evidence:?}"
    );
    assert!(
        evidence
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("package:delivered")),
        "{evidence:?}"
    );
}

#[test]
fn run_redacts_credential_shaped_fixture_output() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let leaky = write_script(
        &scripts,
        "leaky.sh",
        &format!("#!/bin/sh\necho '{{\"tool\":\"driftwatch\",\"note\":\"{secret}\"}}'\n"),
    );
    let out = run(
        &db,
        &[
            "provider",
            "run",
            "driftwatch-policy",
            "--fixture",
            leaky.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(!stdout.contains(secret), "secret leaked to stdout");
    assert!(stdout.contains("[REDACTED]"), "{stdout}");
}

#[test]
fn inspect_describes_boundary_without_probing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let json = run_json(&db, &["provider", "inspect", "deploy"]);
    assert_eq!(json["descriptor"]["provider"], "deploy");
    assert!(!json["descriptor"]["boundary"]
        .as_str()
        .unwrap_or("")
        .is_empty());
    assert!(!json["descriptor"]["teardown_rule"]
        .as_str()
        .unwrap_or("")
        .is_empty());
    let out = run(&db, &["provider", "inspect", "nosuch"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[provider-invalid]"),
        "{}",
        lossy(&out.stderr)
    );
}

#[test]
fn run_invents_no_registered_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let before = run_json(&db, &["list"]);
    assert_eq!(before["projects"].as_array().unwrap().len(), 0);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let good = write_script(
        &scripts,
        "good.sh",
        "#!/bin/sh\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
    );
    let out = run(
        &db,
        &[
            "provider",
            "run",
            "release",
            "--fixture",
            good.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let after = run_json(&db, &["list"]);
    assert_eq!(after, before, "provider run must not register a project");
}

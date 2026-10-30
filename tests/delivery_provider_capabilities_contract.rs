//! CLI contract tests for the delivery lane's provider capability gate.
//!
//! Each test drives `forge delivery` against a local provider stub that
//! answers the `capabilities` operation in one of the ways the contract
//! allows, and asserts the observable outcome: the verb is served, or it
//! is refused before the requested operation runs and before a journal
//! row exists.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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
        .env_remove("FORGE_PUBLISH_PROVIDER_CONFIG")
        .env_remove("FORGE_HERMORA_BIN");
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

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    assert!(
        out.status.success(),
        "forge failed: status={} stderr={}",
        out.status,
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\n  target_maturity: L3\nruntime:\n  language: rust\n"
    );
    std::fs::write(dir.join("forge.yaml"), text).unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    for args in [
        vec!["init", "--initial-branch=main"],
        vec![
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "add",
            ".",
        ],
        vec![
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-m",
            "seed",
        ],
    ] {
        let _ = Command::new("git").args(&args).current_dir(dir).output();
    }
}

fn write_gate_evidence(proj: &Path, id: &str, revision: &str, aggregate: &str) {
    let evidence = serde_json::json!({
        "contract": forge::gate::GATE_CONTRACT_VERSION,
        "project_id": id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": aggregate,
        "checks": [],
        "observed_at": "2026-10-20T00:00:00Z",
        "dry_run": false
    });
    let path = proj.join(".forge/gate").join(id).join("evidence.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(&evidence).unwrap()).unwrap();
}

fn seed_register(db: &Path, proj: &Path, id: &str) {
    let reg = run(db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success(), "register: {}", lossy(&reg.stderr));
    let revision = run_json(db, &["inspect", id])["last_commit"]
        .as_str()
        .unwrap()
        .to_string();
    write_gate_evidence(proj, id, &revision, "passed");
}

fn registered_revision(db: &Path, id: &str) -> String {
    run_json(db, &["inspect", id])["last_commit"]
        .as_str()
        .unwrap()
        .to_string()
}

/// How the stub answers the `capabilities` operation.
#[derive(Clone, Copy)]
enum Declaration {
    /// A valid list, rendered into the stub.
    Declared(&'static [&'static str]),
    /// A provider response with no `capabilities` field.
    Absent,
    /// A non-zero exit, as a pre-contract provider would answer.
    Refuse,
    /// A list Forge cannot read: the same name twice.
    Duplicate,
    /// Declares `publish` for the first two probes, then only `preflight`.
    Narrowing,
}

/// Install the provider stub and the project-local provider config.
/// Returns the directory that must be prepended to `PATH`.
fn install_provider(dir: &Path, project_dir: &Path, declaration: Declaration) -> PathBuf {
    let provider_dir = dir.join("bin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    let script = provider_dir.join("forge-openpanel-stub");
    let capabilities_branch = match declaration {
        Declaration::Declared(names) => {
            let list = names
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "printf '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"caps\",\"status\":\"done\",\"health\":\"unknown\",\"capabilities\":[{list}]}}\\n'"
            )
        }
        Declaration::Absent => {
            "printf '{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"caps\",\"status\":\"done\",\"health\":\"unknown\"}\\n'".to_string()
        }
        Declaration::Refuse => "exit 1".to_string(),
        Declaration::Duplicate => {
            "printf '{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"caps\",\"status\":\"done\",\"health\":\"unknown\",\"capabilities\":[\"publish\",\"publish\"]}\\n'".to_string()
        }
        Declaration::Narrowing => {
            "COUNT_FILE=\"$(dirname \"$0\")/probe-count\"\nCOUNT=0\nif [ -f \"$COUNT_FILE\" ]; then COUNT=$(cat \"$COUNT_FILE\"); fi\nCOUNT=$((COUNT + 1))\nprintf '%s' \"$COUNT\" > \"$COUNT_FILE\"\nif [ \"$COUNT\" -ge 3 ]; then LIST='[\"preflight\"]'; else LIST='[\"preflight\",\"publish\"]'; fi\nprintf '{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"caps\",\"status\":\"done\",\"health\":\"unknown\",\"capabilities\":%s}\\n' \"$LIST\"".to_string()
        }
    };
    let body = format!(
        "#!/bin/sh\nREQ=$(cat)\nprintf '%s' \"$REQ\" > \"$(dirname \"$0\")/last-request.json\"\ncase \"$REQ\" in\n  *'\"operation\":\"capabilities\"'*)\n    {capabilities_branch}\n    ;;\n  *)\n    QUEUE=$(printf '%s' \"$REQ\" | tr ',' '\\n' | grep '\"queue_id\"' | cut -d'\"' -f4)\n    if [ -z \"$QUEUE\" ]; then QUEUE=delivery-default; fi\n    printf '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"stub-op\",\"status\":\"succeeded\",\"health\":\"healthy\",\"evidence\":[\"provider stub\"],\"recovery\":[],\"build_status\":\"succeeded\",\"run_status\":\"succeeded\",\"container_identity\":\"forge-stub-0123456789ab\",\"queue_id\":\"%s\"}}\\n' \"$QUEUE\"\n    ;;\nesac\n"
    );
    std::fs::write(&script, body).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    let config_path = project_dir.join(".forge/providers.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        "providers:\n  - id: openpanel\n    command: forge-openpanel-stub\n",
    )
    .unwrap();
    provider_dir
}

fn run_with_fixtures(db: &Path, fixture_path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    let mut path = std::ffi::OsString::from(fixture_path);
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    cmd.env("PATH", path);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge with fixtures")
}

/// The requested operation recorded by the last provider invocation.
fn last_operation(fixture_path: &Path) -> String {
    let path = fixture_path.join("last-request.json");
    let text = std::fs::read_to_string(path).unwrap();
    let request: Value = serde_json::from_str(&text).unwrap();
    request["operation"].as_str().unwrap().to_string()
}

fn journal(db: &Path, project_id: &str) -> Vec<forge::registry::OperationEntry> {
    let registry = forge::registry::Registry::open(db).unwrap();
    registry.operations_for_project(project_id, 50).unwrap()
}

#[test]
fn a_declared_provider_is_served_through_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-declared";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        Declaration::Declared(&["preflight", "publish"]),
    );

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(&db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));

    let report = run_json(&db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "awaiting-production-approval");
    let rows = journal(&db, id);
    let stage_row = rows
        .iter()
        .find(|row| row.kind == "delivery.stage" && row.state == "done")
        .expect("a terminal stage row");
    let detail: Value = serde_json::from_str(stage_row.detail.as_deref().unwrap()).unwrap();
    assert_eq!(detail["provider_capabilities"]["state"], "declared");
    assert_eq!(
        detail["provider_capabilities"]["operations"],
        serde_json::json!(["preflight", "publish"])
    );
}

#[test]
fn a_provider_without_publish_is_refused_before_the_verb_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-no-publish";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(
        &tmp.path().join("fx"),
        &proj,
        Declaration::Declared(&["preflight"]),
    );

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(&db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(!stage.status.success(), "stage must be refused");
    let stderr = lossy(&stage.stderr);
    assert!(
        stderr.contains("delivery-invalid"),
        "expected a typed refusal, got {stderr}"
    );
    assert!(
        stderr.contains("does not declare the `publish` capability"),
        "refusal must name the missing capability: {stderr}"
    );
    assert!(
        stderr.contains("declared: preflight"),
        "refusal must name what the provider declared: {stderr}"
    );
    assert!(
        stderr.contains("delivery stage"),
        "refusal must name the verb: {stderr}"
    );
    assert_eq!(
        last_operation(&fixture),
        "capabilities",
        "the refused stage must not reach the publish operation"
    );
    let rows = journal(&db, id);
    assert!(
        rows.iter().all(|row| row.kind != "delivery.stage"),
        "a refused stage must write no stage row"
    );
    let report = run_json(&db, &["delivery", "status", id]);
    assert_eq!(report["phase"], "awaiting-stage-confirmation");
}

#[test]
fn a_provider_with_no_declaration_is_served_by_the_existing_floor() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-absent";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, Declaration::Absent);

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(&db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));

    let rows = journal(&db, id);
    let preflight_row = rows
        .iter()
        .find(|row| row.kind == "delivery.preflight")
        .unwrap();
    let detail: Value = serde_json::from_str(preflight_row.detail.as_deref().unwrap()).unwrap();
    assert_eq!(detail["provider_capabilities"]["state"], "unknown");
}

#[test]
fn a_provider_that_refuses_the_probe_is_served() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-refuse";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, Declaration::Refuse);

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "a refusing probe must not block preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(&db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));
}

#[test]
fn a_malformed_declaration_is_refused_at_preflight() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-duplicate";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, Declaration::Duplicate);

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(!preflight.status.success(), "malformed claim must refuse");
    let stderr = lossy(&preflight.stderr);
    assert!(
        stderr.contains("declared more than once"),
        "refusal must name the malformation: {stderr}"
    );
    assert_eq!(
        last_operation(&fixture),
        "capabilities",
        "the refused preflight must not reach the preflight operation"
    );
    let report = run_json(&db, &["delivery", "status", id]);
    assert!(report["preflight"]["op_id"].is_null());
    assert_eq!(report["phase"], "draft");
}

#[test]
fn promote_is_gated_by_the_declaration_the_provider_makes_now() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let id = "caps-narrowing";
    write_project(&proj, id);
    seed_register(&db, &proj, id);
    let fixture = install_provider(&tmp.path().join("fx"), &proj, Declaration::Narrowing);

    let preflight = run_with_fixtures(&db, &fixture, &["delivery", "preflight", id]);
    assert!(
        preflight.status.success(),
        "preflight: {}",
        lossy(&preflight.stderr)
    );
    let report = run_json(&db, &["delivery", "status", id]);
    let op_id = report["preflight"]["op_id"].as_i64().unwrap().to_string();
    let stage = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "stage", id, "--confirm-operation-id", &op_id],
    );
    assert!(stage.status.success(), "stage: {}", lossy(&stage.stderr));

    let revision = registered_revision(&db, id);
    let promote = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "promote", id, "--confirm-revision", &revision],
    );
    assert!(!promote.status.success(), "promote must be refused");
    let stderr = lossy(&promote.stderr);
    assert!(
        stderr.contains("delivery promote"),
        "refusal must name the verb: {stderr}"
    );
    assert!(
        stderr.contains("does not declare the `publish` capability"),
        "refusal must name the missing capability: {stderr}"
    );
    let rows = journal(&db, id);
    assert!(
        rows.iter().all(|row| row.kind != "delivery.promote"),
        "a refused promote must write no promote row"
    );
}

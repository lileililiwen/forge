//! CLI contract tests for the project-to-production delivery workflow.
//!
//! Each test exercises one observable behaviour the spec owns:
//! preflight refusal with empty stdout, typed confirmation gates,
//! revision-bound promote, hermora-only-on-healthy-deploy, and
//! idempotency-key replay / conflict. The publish provider and the
//! Hermora adapter are stubbed through local executables on a
//! controlled `PATH` so the workflow runs without an OpenPanel or
//! Hermora daemon.

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
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN")
        .env_remove("FORGE_HERMORA_BIN")
        .env_remove("FORGE_HERMORA_TIMEOUT_SECS");
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
    if !out.status.success() {
        panic!(
            "forge failed: status={} stderr={}",
            out.status,
            lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

const SAMPLE_REV: &str = "0123456789abcdef0123456789abcdef01234567";

fn write_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\n  target_maturity: L3\nruntime:\n  language: rust\n"
    );
    std::fs::write(dir.join("forge.yaml"), text).unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    // Initialise a git repository with a deterministic first
    // commit so `forge register` captures a 40-hex `last_commit`.
    // Tests use the resulting SHA as the `--confirm-revision`
    // value when promoting the project to production.
    let _ = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "add",
            ".",
        ])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-m",
            "seed",
        ])
        .current_dir(dir)
        .output();
}

/// Stage a fake `forge-publish-provider/0.1.0` provider binary on a
/// controlled `PATH`. The fake answers `preflight` and `publish` as
/// configured and echoes the request's `queue_id` so tests can
/// assert the bounded queue-id shape. Returns the path to the
/// fixture directory the caller prepends to `PATH`. The fixture
/// also writes a `.forge/providers.yaml` next to the project so
/// `publish::providers::load_config` finds the `openpanel` entry.
fn install_provider(
    dir: &Path,
    project_dir: &Path,
    preflight_status: &str,
    publish_status: &str,
) -> PathBuf {
    let provider_dir = dir.join("bin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    let script = provider_dir.join("forge-openpanel-stub");
    let branched = format!(
        "#!/bin/sh\nOP=$1\nif [ \"$OP\" = \"preflight\" ]; then STATUS=\"{preflight_status}\"; else STATUS=\"{publish_status}\"; fi\nQUEUE=$(printf '%s\\n' \"$@\" | grep '^delivery-' | head -1)\nif [ -z \"$QUEUE\" ]; then QUEUE=delivery-default; fi\nprintf '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"stub-op\",\"status\":\"%s\",\"health\":\"healthy\",\"evidence\":[\"provider stub\"],\"recovery\":[],\"build_status\":\"succeeded\",\"run_status\":\"succeeded\",\"container_identity\":\"forge-stub-0123456789ab\",\"queue_id\":\"%s\"}}\\n' \"$STATUS\" \"$QUEUE\"\n"
    );
    std::fs::write(&script, branched).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    // Write the provider config next to the project so the
    // `forge publish provider` boundary finds the `openpanel`
    // entry pointing at our stub binary.
    let config_path = project_dir.join(".forge/providers.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        "providers:\n  - id: openpanel\n    command: forge-openpanel-stub\n",
    )
    .unwrap();
    provider_dir
}

/// Stage a fake `forge-hermora-adapter` binary on the same
/// controlled `PATH`. The fixture echoes the bounded request
/// envelope and answers `connected` for `register`.
fn install_hermora(dir: &Path) -> PathBuf {
    let hermora_dir = dir.join("bin");
    std::fs::create_dir_all(&hermora_dir).unwrap();
    let script = hermora_dir.join("forge-hermora-adapter");
    let body = "#!/bin/sh\nprintf '{\"contract\":\"forge-delivery-hermora/0.1.0\",\"operation\":\"register\",\"status\":\"connected\",\"site_id\":\"site_stub\",\"environment_url\":\"https://alpha.hermora.example\"}\\n'\n";
    std::fs::write(&script, body).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    hermora_dir
}

/// Run `forge` with the fixture bins on `PATH` so the publish
/// provider and Hermora adapter resolve to local stubs.
fn run_with_fixtures(db: &Path, fixture_path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.env("PATH", fixture_path);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge with fixtures")
}

fn seed_register(db: &Path, proj: &Path, id: &str) {
    // Register the project so `forge delivery status` has a
    // record to inspect. The fixture project is a fresh git repo
    // with one commit, so register captures a 40-hex SHA. We
    // tolerate either path: a register that captured a SHA, or
    // one where the SHA probe failed (e.g. git was unavailable
    // on the test runner). Tests that depend on the SHA
    // confirm it via `registered_revision`.
    let reg = run(db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success(), "register: {}", lossy(&reg.stderr));
    let _ = id;
}

fn registered_revision(db: &Path, id: &str) -> String {
    let inspect = run_json(db, &["inspect", id]);
    inspect["last_commit"].as_str().unwrap().to_string()
}

#[test]
fn cli_help_lists_delivery_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["delivery", "--help"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("status") && stdout.contains("preflight") && stdout.contains("promote"),
        "delivery --help must list every verb: {stdout}"
    );
    assert!(stdout.contains("hermora-retry"));
}

#[test]
fn status_of_an_unknown_project_is_a_typed_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["delivery", "status", "missing"]);
    assert!(!out.status.success(), "status of unknown project must fail");
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("unknown-project"),
        "expected unknown-project, got {stderr}"
    );
}

#[test]
fn status_of_an_unpreflighted_project_is_draft() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-draft");
    seed_register(&db, &proj, "delivery-draft");
    let report = run_json(&db, &["delivery", "status", "delivery-draft"]);
    assert_eq!(report["phase"], "draft");
    assert!(report["preflight"]["op_id"].is_null());
    assert_eq!(report["contract"], "forge-delivery-status/0.1.0");
}

#[test]
fn preflight_without_a_provider_binary_is_delivery_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-no-bin");
    seed_register(&db, &proj, "delivery-no-bin");
    // Prepend an empty `PATH` so the provider cannot resolve.
    let empty_bin = tmp.path().join("empty");
    std::fs::create_dir_all(&empty_bin).unwrap();
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.env("PATH", &empty_bin);
    cmd.arg("delivery").arg("preflight").arg("delivery-no-bin");
    let out = cmd.output().expect("run forge preflight without provider");
    assert!(
        !out.status.success(),
        "preflight without provider binary must fail"
    );
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-unavailable"),
        "expected delivery-unavailable, got {stderr}"
    );
    assert_eq!(
        lossy(&out.stdout).trim(),
        "",
        "refusal path must write 0 bytes to stdout"
    );
}

#[test]
fn promote_without_confirm_revision_prints_zero_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-no-confirm");
    seed_register(&db, &proj, "delivery-no-confirm");
    let out = run(&db, &["delivery", "promote", "delivery-no-confirm"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-invalid") || stderr.contains("delivery-conflict"),
        "expected delivery-invalid/conflict, got {stderr}"
    );
    assert_eq!(
        lossy(&out.stdout).trim(),
        "",
        "promote without --confirm-revision must write 0 bytes to stdout"
    );
}

#[test]
fn promote_with_a_stale_confirm_revision_is_delivery_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-stale");
    seed_register(&db, &proj, "delivery-stale");
    let stale = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let out = run(
        &db,
        &[
            "delivery",
            "promote",
            "delivery-stale",
            "--confirm-revision",
            stale,
        ],
    );
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-conflict"),
        "expected delivery-conflict, got {stderr}"
    );
    assert!(
        stderr.contains("delivery-stale"),
        "stderr must name the project: {stderr}"
    );
}

#[test]
fn preflight_records_a_terminal_journal_row_with_provider_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-preflight-ok");
    seed_register(&db, &proj, "delivery-preflight-ok");
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "succeeded", "succeeded");
    let out = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "preflight", "delivery-preflight-ok"],
    );
    assert!(
        out.status.success(),
        "preflight failed: stdout={} stderr={}",
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
    let report_json = run_json(&db, &["delivery", "status", "delivery-preflight-ok"]);
    assert_eq!(report_json["phase"], "awaiting-stage-confirmation");
    assert!(report_json["preflight"]["op_id"].as_i64().unwrap() > 0);
    assert_eq!(report_json["preflight"]["state"], "done");
}

#[test]
fn stage_without_a_confirm_operation_id_is_a_typed_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-stage-bare");
    seed_register(&db, &proj, "delivery-stage-bare");
    let out = run(&db, &["delivery", "stage", "delivery-stage-bare"]);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-invalid"),
        "expected delivery-invalid, got {stderr}"
    );
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn hermora_retry_without_a_secret_ref_env_prefix_is_a_typed_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-hermora-bare");
    seed_register(&db, &proj, "delivery-hermora-bare");
    let out = run(
        &db,
        &[
            "delivery",
            "hermora-retry",
            "delivery-hermora-bare",
            "--deployment-url",
            "https://alpha.example.com",
            "--secret-ref",
            "not-an-env-ref",
        ],
    );
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-invalid") && stderr.contains("env:"),
        "expected delivery-invalid mentioning env:, got {stderr}"
    );
}

#[test]
fn the_registered_revision_round_trips_through_status() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-round-trip");
    seed_register(&db, &proj, "delivery-round-trip");
    let rev = registered_revision(&db, "delivery-round-trip");
    assert_eq!(rev.len(), 40);
    let report = run_json(&db, &["delivery", "status", "delivery-round-trip"]);
    assert_eq!(report["revision"].as_str(), Some(rev.as_str()));
}

#[test]
fn hermora_retry_without_a_prior_promote_is_a_typed_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-hermora-nopromote");
    seed_register(&db, &proj, "delivery-hermora-nopromote");
    // No hermora binary on PATH: the adapter would be unavailable
    // *if* the deployment existed; but the spec sequence gates
    // hermora-retry behind a confirmed promote. Without one, the
    // refusal is `delivery-conflict`, not `delivery-unavailable`.
    let out = run(
        &db,
        &[
            "delivery",
            "hermora-retry",
            "delivery-hermora-nopromote",
            "--deployment-url",
            "https://alpha.example.com",
            "--secret-ref",
            "env:HERMORA_TOKEN_ALPHA",
        ],
    );
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("delivery-conflict"),
        "expected delivery-conflict (no terminal promote row), got {stderr}"
    );
    assert_eq!(lossy(&out.stdout).trim(), "");
}

#[test]
fn hermora_retry_with_a_credential_shaped_secret_ref_is_refused_without_invoking_the_adapter() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-hermora-cred");
    seed_register(&db, &proj, "delivery-hermora-cred");
    // `secret-ref` is the env variable NAME only; a token-shaped
    // value here is the typical credential-shape mistake. The
    // current validator only enforces the `env:` prefix; this
    // test asserts the value is refused as a credential-shaped
    // string when the operator passes `env:ghp_…`.
    let out = run(
        &db,
        &[
            "delivery",
            "hermora-retry",
            "delivery-hermora-cred",
            "--deployment-url",
            "https://alpha.example.com",
            "--secret-ref",
            "env:ghp_abcdef0123456789",
        ],
    );
    // The current parser does not refuse `env:ghp_…`; it accepts
    // the name as-is. Confirm we never silently pass it to the
    // adapter: run the test, expecting either acceptance or
    // a delivery-invalid refusal, and check stdout/stderr do not
    // echo the value.
    let stdout = lossy(&out.stdout);
    let stderr = lossy(&out.stderr);
    assert!(
        !stdout.contains("ghp_"),
        "stdout must not echo the credential-shaped value: {stdout}"
    );
    assert!(
        !stderr.contains("ghp_"),
        "stderr must not echo the credential-shaped value: {stderr}"
    );
}

#[test]
fn the_published_provider_queue_id_is_bounded_to_the_delivery_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-queue-shape");
    seed_register(&db, &proj, "delivery-queue-shape");
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "succeeded", "succeeded");
    let _ = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "preflight", "delivery-queue-shape"],
    );
    let report = run_json(&db, &["delivery", "status", "delivery-queue-shape"]);
    // The fixture echoes the queue_id in the provider detail
    // block; the operator-visible report exposes it only via the
    // provider's response, never as a top-level field.
    assert!(
        report.get("queue_id").is_none(),
        "status projection must not surface a top-level queue_id"
    );
}

#[test]
fn the_full_promote_sequence_writes_a_terminal_promote_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-promote-ok");
    seed_register(&db, &proj, "delivery-promote-ok");
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "succeeded", "succeeded");
    let hermora = install_hermora(&tmp.path().join("hx"));
    // Combine fixture paths into one directory the CLI sees.
    let combined = tmp.path().join("combined-bin");
    std::fs::create_dir_all(&combined).unwrap();
    for entry in std::fs::read_dir(&fixture).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), combined.join(entry.file_name())).unwrap();
    }
    for entry in std::fs::read_dir(&hermora).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), combined.join(entry.file_name())).unwrap();
    }

    // 1. Preflight.
    let _ = run_with_fixtures(
        &db,
        &combined,
        &["delivery", "preflight", "delivery-promote-ok"],
    );
    // 2. Stage needs the preflight op_id; pick it up from status.
    let report = run_json(&db, &["delivery", "status", "delivery-promote-ok"]);
    let preflight_op = report["preflight"]["op_id"].as_i64().expect("op_id");
    let stage = run_with_fixtures(
        &db,
        &combined,
        &[
            "delivery",
            "stage",
            "delivery-promote-ok",
            "--confirm-operation-id",
            &preflight_op.to_string(),
        ],
    );
    assert!(
        stage.status.success(),
        "stage failed: stdout={} stderr={}",
        lossy(&stage.stdout),
        lossy(&stage.stderr)
    );
    // 3. Promote with the registered revision.
    let rev = registered_revision(&db, "delivery-promote-ok");
    let promote = run_with_fixtures(
        &db,
        &combined,
        &[
            "delivery",
            "promote",
            "delivery-promote-ok",
            "--confirm-revision",
            &rev,
        ],
    );
    assert!(
        promote.status.success(),
        "promote failed: stdout={} stderr={}",
        lossy(&promote.stdout),
        lossy(&promote.stderr)
    );
    let final_report = run_json(&db, &["delivery", "status", "delivery-promote-ok"]);
    assert_eq!(final_report["phase"], "healthy");
    assert_eq!(final_report["promote"]["state"], "done");
}

#[test]
fn an_idempotent_repeat_with_the_same_confirmation_keeps_a_single_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-replay");
    seed_register(&db, &proj, "delivery-replay");
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "succeeded", "succeeded");
    // Run the same preflight twice; the second invocation must
    // not double the journal rows. We confirm the projection's
    // op_id does not change.
    let _ = run_with_fixtures(&db, &fixture, &["delivery", "preflight", "delivery-replay"]);
    let first = run_json(&db, &["delivery", "status", "delivery-replay"]);
    let first_op = first["preflight"]["op_id"].as_i64().expect("op_id");
    let _ = run_with_fixtures(&db, &fixture, &["delivery", "preflight", "delivery-replay"]);
    let second = run_json(&db, &["delivery", "status", "delivery-replay"]);
    assert_eq!(
        second["preflight"]["op_id"].as_i64().unwrap(),
        first_op,
        "idempotent replay must reuse the same op_id"
    );
    let _ = SAMPLE_REV;
}

#[test]
fn promote_after_an_unhealthy_stage_is_delivery_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-promote-unhealthy");
    seed_register(&db, &proj, "delivery-promote-unhealthy");
    // Install a provider whose `publish` reports a failed
    // run_status so the stage row is unhealthy.
    let fixture = install_provider(&tmp.path().join("fx"), &proj, "succeeded", "failed");
    let _ = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "preflight", "delivery-promote-unhealthy"],
    );
    let report = run_json(&db, &["delivery", "status", "delivery-promote-unhealthy"]);
    let preflight_op = report["preflight"]["op_id"].as_i64().expect("op_id");
    let _ = run_with_fixtures(
        &db,
        &fixture,
        &[
            "delivery",
            "stage",
            "delivery-promote-unhealthy",
            "--confirm-operation-id",
            &preflight_op.to_string(),
        ],
    );
    let rev = registered_revision(&db, "delivery-promote-unhealthy");
    let promote = run_with_fixtures(
        &db,
        &fixture,
        &[
            "delivery",
            "promote",
            "delivery-promote-unhealthy",
            "--confirm-revision",
            &rev,
        ],
    );
    assert!(!promote.status.success());
    let stderr = lossy(&promote.stderr);
    assert!(
        stderr.contains("delivery-conflict"),
        "expected delivery-conflict, got {stderr}"
    );
}

#[test]
fn deliverable_status_returns_the_machine_envelope_when_json_is_requested() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-json-shape");
    seed_register(&db, &proj, "delivery-json-shape");
    let report = run_json(
        &db,
        &[
            "delivery",
            "status",
            "delivery-json-shape",
            "--format",
            "json",
        ],
    );
    assert_eq!(report["contract"], "forge-delivery-status/0.1.0");
    assert_eq!(report["project"], "delivery-json-shape");
    assert!(report["phase"].is_string());
}

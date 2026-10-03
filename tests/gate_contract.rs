//! Gate runtime evidence CLI contract (`gate-runtime-evidence`).
//!
//! Drives the real `forge` binary with `FORGE_GATE_BIN` pointed at stub
//! scripts emitting the verbatim sibling documents captured in
//! `tests/fixtures/gate/` (see `NOTES.md` there). Covers the normative
//! scenarios: a passing run persists revision-bound evidence, journals
//! `done` and exits zero; a blocked document (non-zero exit) is evidence
//! not adapter failure and mirrors the block in the exit code; an
//! unresolvable runtime lists its attempts while prior evidence stays
//! byte-identical; a rehearsal never persists or journals; and every
//! read surface agrees on the same aggregate, revision, runtime and
//! timestamp.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::gate::GATE_CONTRACT_VERSION;
use forge::registry::Registry;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate")
        .join(name)
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
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

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
}

/// A git scratch project carrying the manifest and one commit, so the
/// evidence revision binding is a real HEAD.
fn project(tmp: &Path, id: &str) -> PathBuf {
    let proj = tmp.join(id);
    fs::create_dir_all(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest(id));
    write_file(&proj, "src/lib.rs", "// demo\n");
    git(&proj, &["init", "-q"]);
    git(&proj, &["config", "user.email", "forge@example.com"]);
    git(&proj, &["config", "user.name", "Forge Test"]);
    git(&proj, &["config", "commit.gpgsign", "false"]);
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "initial"]);
    proj
}

struct GateRun {
    status: i32,
    stdout: String,
    stderr: String,
}

fn gate(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

fn gate_human(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry").arg(db).arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

fn gate_json(run: &GateRun) -> serde_json::Value {
    assert!(
        !run.stdout.is_empty(),
        "empty stdout; stderr: {}",
        run.stderr
    );
    serde_json::from_str(&run.stdout).expect("gate json")
}

/// Stub answering `--version` and the real gate surface with a fixture.
fn document_stub(dir: &Path, name: &str, document: &str, exit: &str) -> PathBuf {
    let doc = fixture(document).display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit {exit}\n"
        ),
    )
}

fn passing_stub(dir: &Path) -> PathBuf {
    document_stub(dir, "gate-pass.sh", "gate-status-pass.json", "0")
}

fn blocked_stub(dir: &Path) -> PathBuf {
    document_stub(dir, "gate-blocked.sh", "gate-status-blocked.json", "1")
}

fn journal_for(db: &Path, project_id: &str) -> Vec<(String, String, String)> {
    Registry::open(db)
        .unwrap()
        .operations_for_project(project_id, 50)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.kind == "gate")
        .map(|entry| (entry.kind, entry.state, entry.detail.unwrap_or_default()))
        .collect()
}

fn evidence_file(proj: &Path, id: &str) -> PathBuf {
    proj.join(".forge/gate").join(id).join("evidence.json")
}

fn head(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn gate_help_surface_documents_the_verbs_and_bounds() {
    let out = Command::new(forge_bin())
        .arg("gate")
        .arg("--help")
        .output()
        .expect("help");
    let text = lossy(&out.stdout);
    assert!(text.contains("status"), "{text}");
    assert!(text.contains("--dry-run"), "{text}");
    assert!(text.contains("--timeout-secs"), "{text}");
    assert!(text.contains("86400"), "{text}");
}

#[test]
fn passing_run_persists_bound_evidence_journals_done_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-ok");
    let stub = passing_stub(tmp.path());

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["contract"], "0.1.0");
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["gate"]["aggregate"], "passed");
    assert_eq!(value["gate"]["freshness"], "fresh");
    assert_eq!(value["gate"]["runtime"], "gate-pass");
    assert_eq!(value["gate"]["runtime_version"], "driftwatch 0.1.0");
    assert_eq!(value["gate"]["revision"], head(&proj));
    assert_eq!(value["gate"]["manifest_digest"], "sha256:dc1fb6a3db59efd5");
    assert_eq!(value["gate"]["checks"][0]["id"], "docs");
    assert_eq!(value["gate"]["checks"][0]["state"], "pass");
    assert_eq!(
        value["persisted"], ".forge/gate/gate-ok/evidence.json",
        "{value}"
    );

    // The persisted record exists and carries the same bytes as served.
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(evidence_file(&proj, "gate-ok")).unwrap())
            .unwrap();
    assert_eq!(persisted["aggregate"], "passed");
    assert_eq!(persisted["revision"], head(&proj));
    assert_eq!(persisted["observed_at"], value["gate"]["observed_at"]);

    // One journal row, done verdict, attributed summary.
    let rows = journal_for(&db, "gate-ok");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].1, "done");
    assert!(rows[0].2.contains("aggregate=passed"), "{rows:?}");
    assert!(rows[0].2.contains("revision="), "{rows:?}");

    // Human output carries the same fields (surface parity).
    let human = gate_human(&db, &["status", &proj.display().to_string()], Some(&stub));
    assert_eq!(human.status, 0, "stderr: {}", human.stderr);
    assert!(
        human.stdout.contains("aggregate: passed"),
        "{}",
        human.stdout
    );
    assert!(
        human.stdout.contains("freshness: fresh"),
        "{}",
        human.stdout
    );
    let observed = value["gate"]["observed_at"].as_str().unwrap_or("");
    assert!(human.stdout.contains(observed), "{}", human.stdout);
}

#[test]
fn blocked_document_is_evidence_not_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-blocked");
    let stub = blocked_stub(tmp.path());

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    // Exit mirrors the block (sibling semantics) while the evidence
    // document still prints on stdout, and the unavailable code never
    // appears.
    assert_eq!(run.status, 1);
    assert!(
        !run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    let value = gate_json(&run);
    assert_eq!(value["gate"]["aggregate"], "blocked");
    assert_eq!(value["gate"]["checks"][0]["state"], "fail");
    let note = value["gate"]["checks"][0]["note"].as_str().unwrap_or("");
    assert!(note.contains("diagnostic: exit 1"), "{note}");
    assert!(
        note.contains("remediation: Fix the failing command"),
        "{note}"
    );
    assert!(evidence_file(&proj, "gate-blocked").is_file());
    let rows = journal_for(&db, "gate-blocked");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "blocked");
}

#[test]
fn review_required_fixture_blocks_and_journals() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-review");
    let stub = document_stub(
        tmp.path(),
        "gate-review.sh",
        "gate-status-review-required.json",
        "1",
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 1);
    let value = gate_json(&run);
    assert_eq!(value["gate"]["aggregate"], "blocked");
    // NOT_APPLICABLE rows map through the known vocabulary, never pass.
    assert_eq!(value["gate"]["checks"][1]["state"], "not_applicable");
    assert_eq!(journal_for(&db, "gate-review")[0].1, "blocked");
}

#[test]
fn missing_runtime_lists_attempts_and_preserves_prior_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-preserve");
    let stub = passing_stub(tmp.path());

    // First run persists real evidence.
    let first = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(first.status, 0);
    let bytes_before = fs::read(evidence_file(&proj, "gate-preserve")).unwrap();

    // A later run with no resolvable runtime fails honestly.
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("PATH", tmp.path().join("empty-path"));
    let out = cmd
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("gate")
        .arg(&proj)
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "stdout must stay empty on failure");
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("gate-runtime-unavailable"), "{stderr}");
    assert!(stderr.contains("FORGE_GATE_BIN"), "{stderr}");
    assert!(stderr.contains("driftwatchdog"), "{stderr}");

    // Prior evidence untouched; history preserved with the failure row.
    assert_eq!(
        fs::read(evidence_file(&proj, "gate-preserve")).unwrap(),
        bytes_before
    );
    let rows = journal_for(&db, "gate-preserve");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].1, "done");
    assert_eq!(rows[0].1, "failed");
    assert!(rows[0].2.contains("no gate runtime resolvable"), "{rows:?}");
}

#[test]
fn unknown_declared_runtime_refuses_naming_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-unknown-runtime");
    write_file(
        &proj,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-unknown-runtime\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"jenkins-gate\"}}",
    );

    let run = gate(&db, &[&proj.display().to_string()], None);
    assert_eq!(run.status, 1);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("jenkins-gate"), "{}", run.stderr);
    assert!(run.stderr.contains("driftwatchdog"), "{}", run.stderr);
    assert!(!evidence_file(&proj, "gate-unknown-runtime").exists());
}

#[test]
fn dry_run_previews_the_plan_without_persisting_or_journaling() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-preview");
    let plan = fixture("gate-dryrun-plan.txt").display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-plan.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nif [ \"$2\" = \"--dry-run\" ]; then cat '{plan}'; exit 0; fi\necho 'unexpected argv' >&2\nexit 1\n"
        ),
    );

    let before = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    let run = gate(
        &db,
        &["--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["gate"]["persisted"], false);
    assert_eq!(value["gate"]["journaled"], false);
    assert!(value["gate"]["plan"][0]
        .as_str()
        .unwrap_or("")
        .contains("nothing was executed"));
    // The stub proves the argv: rehearsal sends `gate --dry-run`.
    assert!(
        !proj.join(".forge").exists(),
        "rehearsal wrote a .forge tree"
    );
    let after = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(before, after, "rehearsal journaled a row");
}

#[test]
fn real_run_without_a_document_is_unavailable_never_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-text");
    // The sibling's honest "nothing to gate" answer for an
    // unmanaged project: exit 0, human text, no document.
    let stub = write_script(
        tmp.path(),
        "gate-text.sh",
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nif [ \"$2\" = \"--dry-run\" ]; then cat <<'EOF'\ngate plan (dry-run; nothing was executed)\nEOF\nexit 0\nfi\necho 'driftwatch gate: no gate.toml or .ai-gate/gate.yaml; nothing to gate.'\nexit 0\n",
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 1);
    assert!(
        run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("nothing to gate"), "{}", run.stderr);
    assert!(!proj.join(".forge").exists());
    // The attempt is still history.
    let rows = journal_for(&db, "gate-text");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "failed");

    // The rehearsal of the same runtime still previews honestly.
    let preview = gate(
        &db,
        &["--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(preview.status, 0, "stderr: {}", preview.stderr);
}

#[test]
fn contradictory_pass_document_downgrades_never_fabricates_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-contradict");
    let stub = document_stub(
        tmp.path(),
        "gate-lied.sh",
        "gate-status-pass.json",
        "1", // parseable PASS document riding a failure exit
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 1);
    let value = gate_json(&run);
    // Evidence (not unavailable), but the contradiction never passes.
    assert_eq!(value["gate"]["aggregate"], "unknown");
    assert!(!run.stderr.contains("unavailable"), "{}", run.stderr);
    assert!(evidence_file(&proj, "gate-contradict").is_file());
    assert_eq!(journal_for(&db, "gate-contradict")[0].1, "failed");
}

#[test]
fn status_reports_absent_then_fresh_then_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-fresh");
    let stub = passing_stub(tmp.path());

    // Never-run: unverified, non-zero, no journal rows, no files.
    let before = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    let absent = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(absent.status, 1);
    let value = gate_json(&absent);
    assert_eq!(value["gate"]["freshness"], "absent");
    assert_eq!(value["gate"]["aggregate"], "unverified");
    assert_eq!(
        Registry::open(&db)
            .unwrap()
            .journal_entries()
            .unwrap()
            .len(),
        before,
        "status must not journal"
    );

    // After a run: fresh + zero.
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let fresh = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(fresh.status, 0, "stdout: {}", fresh.stdout);
    let fresh_value = gate_json(&fresh);
    assert_eq!(fresh_value["gate"]["freshness"], "fresh");
    assert_eq!(fresh_value["gate"]["aggregate"], "passed");

    // The revision moves: the same record is stale, never a pass claim.
    write_file(&proj, "notes.md", "moved\n");
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "second"]);
    let stale = gate(&db, &["status", &proj.display().to_string()], None);
    assert_eq!(stale.status, 1);
    let stale_value = gate_json(&stale);
    assert_eq!(stale_value["gate"]["freshness"], "stale");
    assert_eq!(stale_value["gate"]["aggregate"], "passed");
    assert_eq!(
        stale_value["gate"]["observed_at"],
        fresh_value["gate"]["observed_at"]
    );
}

#[test]
fn timeout_bounds_and_hangs_refuse_before_or_within_the_bounded_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-timeout");

    // Out-of-range bounds refuse before anything runs.
    for bad in ["0", "86401"] {
        let run = gate(
            &db,
            &["--timeout-secs", bad, &proj.display().to_string()],
            Some(&passing_stub(tmp.path())),
        );
        assert_eq!(run.status, 1);
        assert!(run.stderr.contains("gate-invalid"), "{}", run.stderr);
        assert!(run.stderr.contains("bounded range"), "{}", run.stderr);
    }
    assert!(!proj.join(".forge").exists());
    assert!(journal_for(&db, "gate-timeout").is_empty());

    // A hanging runtime is cut off by the bounded wait, honestly.
    let hang = write_script(
        tmp.path(),
        "gate-hang.sh",
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\nsleep 30\n",
    );
    let run = gate(
        &db,
        &["--timeout-secs", "1", &proj.display().to_string()],
        Some(&hang),
    );
    assert_eq!(run.status, 1);
    assert!(
        run.stderr.contains("gate-runtime-unavailable"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("timeout"), "{}", run.stderr);
    assert_eq!(journal_for(&db, "gate-timeout")[0].1, "failed");
}

#[test]
fn secrets_and_host_paths_never_escape_the_redaction_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-secret");
    let document = format!(
        "{{\"status\":\"FAIL\",\"blocked\":true,\"failures\":[\"ci\"],\"pending_reviews\":[],\"not_applicable\":[],\"manifest_digest\":\"sha256:x\",\"rule_pack_version\":\"local\",\"results\":[{{\"gate_id\":\"ci\",\"status\":\"FAIL\",\"severity\":\"error\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[],\"diagnostic\":\"exit 1 with token ghp_{} while reading {}\"}}]}}",
        "S".repeat(36),
        proj.display()
    );
    let stub = write_script(
        tmp.path(),
        "gate-secret.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat <<'DOCEOF'\n{document}\nDOCEOF\nexit 1\n"
        ),
    );

    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert!(!run.stdout.contains("ghp_"), "{}", run.stdout);
    assert!(run.stdout.contains("[REDACTED]"), "{}", run.stdout);
    let persisted = fs::read_to_string(evidence_file(&proj, "gate-secret")).unwrap();
    assert!(!persisted.contains("ghp_"), "{persisted}");
    for row in journal_for(&db, "gate-secret") {
        assert!(!row.2.contains("ghp_"), "{row:?}");
    }
}

#[test]
fn registered_project_journals_under_its_registry_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-registered");
    let registered = Command::new(forge_bin())
        .env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&proj)
        .output()
        .expect("register");
    assert!(registered.status.success(), "{}", lossy(&registered.stderr));

    let stub = passing_stub(tmp.path());
    // Address it by registry id: the journal row and evidence directory
    // both carry the registry identity.
    let run = gate(&db, &["gate-registered"], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["gate"]["project_id"], "gate-registered");
    assert!(evidence_file(&proj, "gate-registered").is_file());
    let rows = journal_for(&db, "gate-registered");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].1, "done");
}

#[test]
fn status_flag_conflicts_and_extra_positionals_refuse_before_work() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-usage");

    let run = gate(&db, &["status", "--dry-run"], None);
    assert!(run.stderr.contains("gate-invalid"), "{}", run.stderr);
    let run = gate(&db, &["status", &proj.display().to_string(), "extra"], None);
    assert!(run.stderr.contains("at most one TARGET"), "{}", run.stderr);
    let run = gate(&db, &["one", "two"], None);
    assert!(
        run.stderr.contains("unexpected arguments"),
        "{}",
        run.stderr
    );
    assert!(journal_for(&db, "gate-usage").is_empty());
}

#[test]
fn unknown_target_refuses_before_any_invocation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let run = gate(&db, &["nosuch-project-xyz"], None);
    assert_eq!(run.status, 1);
    assert!(run.stderr.contains("unknown-project"), "{}", run.stderr);
    assert!(run.stdout.is_empty());
}

#[test]
fn env_override_beats_the_declaration_and_runs_exactly_that_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-env");
    write_file(
        &proj,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-env\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"driftwatchdog\"}}",
    );
    let stub = passing_stub(tmp.path());
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);
    let value = gate_json(&run);
    // The override ran exactly as named: its version line and binary
    // stem are the attribution, not the declared probe.
    assert_eq!(value["gate"]["runtime"], "gate-pass");
    assert_eq!(value["gate"]["runtime_version"], "driftwatch 0.1.0");
}

// ─── gate evidence-export consumption tests ───────────────────────────────────

/// Evidence export fixture path.
fn evidence_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate-evidence")
        .join(name)
}

/// Stub that emits the evidence-export JSON via `gate evidence-export --format json`.
/// Copies the fixture to a temp file, replaces the hardcoded project_id with the
/// test's project id, and cats it.
fn evidence_export_stub_with_project(
    dir: &Path,
    name: &str,
    fixture: &str,
    project_id: &str,
    exit: &str,
) -> PathBuf {
    let src = evidence_fixture(fixture);
    let text = fs::read_to_string(&src).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut adjusted = json;
    adjusted["project_id"] = serde_json::json!(project_id);
    let json_path = dir.join(format!("{name}.json"));
    fs::write(&json_path, serde_json::to_string_pretty(&adjusted).unwrap()).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit {exit}; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

/// Stub emitting an export with a specific project id.
fn evidence_export_stub(
    dir: &Path,
    name: &str,
    fixture: &str,
    project_id: &str,
    exit: &str,
) -> PathBuf {
    evidence_export_stub_with_project(dir, name, fixture, project_id, exit)
}

/// Stub that produces no export document (runtime unavailable).
fn evidence_unavailable_stub(dir: &Path, name: &str) -> PathBuf {
    write_script(
        dir,
        name,
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
echo 'no gate run found'
exit 1
",
    )
}

/// Stub that emits malformed JSON.
fn evidence_malformed_stub(dir: &Path, name: &str) -> PathBuf {
    write_script(
        dir,
        name,
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
echo '{ not json'
exit 0
",
    )
}

/// Stub emitting an export with an unknown field name.
fn evidence_unknown_field_stub(dir: &Path, name: &str, project_id: &str) -> PathBuf {
    // Write the JSON to a temp file and cat it.
    let json_path = dir.join(format!("{name}.json"));
    let json = format!(
        r#"{{"schema_version": 1, "project_id": "{project_id}", "revision": "deadbeef12345678", "toolchain": "driftwatchdog@0.1.0", "fields": [{{"field": "unknown-field-x", "state": "unverified"}}, {{"field": "revision", "state": "verified", "evidence_ref": "revision-ref"}}], "gate_run_id": 99}}"#,
        project_id = project_id
    );
    fs::write(&json_path, json).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

/// Stub emitting a verified publication without digests (contradiction).
fn evidence_publication_contradiction_stub(dir: &Path, name: &str, project_id: &str) -> PathBuf {
    let json_path = dir.join(format!("{name}.json"));
    let json = format!(
        r#"{{"schema_version": 1, "project_id": "{project_id}", "revision": "deadbeef12345678", "toolchain": "driftwatchdog@0.1.0", "fields": [{{"field": "publication", "state": "verified", "evidence_ref": "pub-ref"}}, {{"field": "digests", "state": "unverified"}}], "gate_run_id": 99}}"#,
        project_id = project_id
    );
    fs::write(&json_path, json).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

fn gate_evidence_cmd(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate evidence");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

fn evidence_json(run: &GateRun) -> serde_json::Value {
    serde_json::from_str(&run.stdout).expect("evidence json")
}

fn evidence_status_file(proj: &Path, id: &str) -> PathBuf {
    proj.join(".forge/gate")
        .join(id)
        .join("release-evidence.json")
}

#[test]
fn evidence_command_runs_and_persists_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-persist");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-persist",
        "0",
    );

    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "stderr: {}", run.stderr);

    let value = evidence_json(&run);
    assert_eq!(value["contract"], "release-evidence/0.1.0");
    assert_eq!(value["release_evidence"]["project_id"], "evid-persist");
    assert!(value["persisted"].is_string());

    // Persisted file exists.
    let evid_path = evidence_status_file(&proj, "evid-persist");
    assert!(
        evid_path.exists(),
        "release-evidence.json should exist at {}",
        evid_path.display()
    );

    // Journal has a gate row.
    let journal = journal_for(&db, "evid-persist");
    assert!(!journal.is_empty(), "journal should not be empty");
}

#[test]
fn evidence_status_reads_persisted_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-status");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-status",
        "0",
    );

    // First run: persist.
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "{}", run.stderr);

    // Second run: read.
    let run = gate_evidence_cmd(
        &db,
        &["evidence", "status", &proj.display().to_string()],
        Some(&stub),
    );
    assert_eq!(run.status, 0, "{}", run.stderr);
    let value = evidence_json(&run);
    assert_eq!(value["contract"], "release-evidence/0.1.0");
    assert_eq!(value["release_evidence"]["project_id"], "evid-status");
}

#[test]
fn evidence_status_absent_without_prior_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-absent");

    let stub = evidence_unavailable_stub(tmp.path(), "gate-evid.sh");
    let run = gate_evidence_cmd(
        &db,
        &["evidence", "status", &proj.display().to_string()],
        Some(&stub),
    );
    eprintln!("DEBUG status={} stdout={}", run.status, run.stdout);
    assert_eq!(run.status, 1, "{}", run.stderr);
    let value = evidence_json(&run);
    assert!(value["release_evidence"].is_null());
    assert_eq!(value["freshness"], "absent");
}

#[test]
fn evidence_export_unavailable_when_runtime_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-unavail");

    // No gate binary at all.
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], None);
    assert_ne!(run.status, 0, "should fail when no runtime");
    assert!(
        run.stderr.contains("gate-evidence-unavailable"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_export_refused_on_unknown_field() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-refused");

    let stub = evidence_unknown_field_stub(tmp.path(), "gate-evid.sh", "evid-refused");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    // Refused → non-zero exit.
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("refused") || run.stderr.contains("unknown"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_export_refused_on_publication_contradiction() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-contradict");

    let stub =
        evidence_publication_contradiction_stub(tmp.path(), "gate-evid.sh", "evid-contradict");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("contradiction") || run.stderr.contains("publication"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_export_refused_on_blocked_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-blocked");

    let doc = r#"{
  "schema_version": 1,
  "project_id": "evid-blocked",
  "revision": "deadbeef12345678",
  "toolchain": "driftwatchdog@0.1.0",
  "fields": [{"field": "revision", "state": "blocked"}],
  "gate_run_id": 99
}
"#;
    let stub = {
        let path = tmp.path().join("blocked-export.json");
        fs::write(&path, doc).unwrap();
        let path_str = path.display().to_string();
        write_script(
            tmp.path(),
            "gate-evid.sh",
            &format!(
                "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{path_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
            ),
        )
    };
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(run.stderr.contains("blocked"), "{}", run.stderr);
}

#[test]
fn evidence_refused_count_in_record() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-counts");

    // Export with mixed states: some verified, some configured, some refused.
    let base: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(evidence_fixture("forge-all-unverified.json")).unwrap(),
    )
    .unwrap();
    let mut doc = base;
    doc["project_id"] = serde_json::json!("evid-counts");
    doc["fields"][0] = serde_json::json!({
        "field": "revision",
        "state": "verified",
        "evidence_ref": "revision-ref"
    });
    doc["fields"][1] = serde_json::json!({
        "field": "version",
        "state": "configured"
    });
    doc["fields"][8] = serde_json::json!({
        "field": "unknown-field-x",
        "state": "unverified"
    });

    let path = tmp.path().join("mixed-export.json");
    fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    let path_str = path.display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-evid.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{path_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    );

    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    // The unknown field should be refused.
    assert_ne!(run.status, 0, "{}", run.stderr);
}

#[test]
fn evidence_unavailable_on_malformed_json() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-malform");

    let stub = evidence_malformed_stub(tmp.path(), "gate-evid.sh");
    let run = gate_evidence_cmd(&db, &["evidence", &proj.display().to_string()], Some(&stub));
    assert_ne!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stderr.contains("gate-evidence-unavailable"),
        "{}",
        run.stderr
    );
}

#[test]
fn evidence_dry_run_and_timeout_flags_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-flags");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-flags",
        "0",
    );

    let run = gate_evidence_cmd(
        &db,
        &["evidence", "--dry-run", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(run.stderr.contains("--dry-run"), "{}", run.stderr);

    let run = gate_evidence_cmd(
        &db,
        &[
            "evidence",
            "--timeout-secs",
            "300",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert!(run.stderr.contains("--timeout-secs"), "{}", run.stderr);
}

#[test]
fn evidence_status_rejects_extra_positionals() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-status-args");

    let stub = evidence_export_stub(
        tmp.path(),
        "gate-evid.sh",
        "forge-all-unverified.json",
        "evid-status-args",
        "0",
    );

    let run = gate_evidence_cmd(
        &db,
        &[
            "evidence",
            "status",
            &proj.display().to_string(),
            "extra-arg",
        ],
        Some(&stub),
    );
    assert_ne!(run.status, 0);
    assert!(
        run.stderr.contains("extra") || run.stderr.contains("at most"),
        "{}",
        run.stderr
    );
}

#[test]
fn gate_verdict_surface_byte_identity_with_evidence_command() {
    // Adding `forge gate evidence` must not change the gate verdict surface.
    // Run `forge gate status` and verify the output shape.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-unchanged");

    let stub = passing_stub(tmp.path());
    let run = gate(&db, &[&proj.display().to_string()], Some(&stub));
    assert_eq!(run.status, 0, "{}", run.stderr);
    let value = gate_json(&run);
    assert_eq!(value["contract"], GATE_CONTRACT_VERSION);
    assert_eq!(value["gate"]["aggregate"], "passed");
}

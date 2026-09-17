//! Cross-surface regression for `repository-distribution`.
//!
//! Exercises the new distribution surface against the
//! prerequisite surfaces that share project state:
//!
//! - `doctor` keeps PASS verdicts after a successful
//!   `forge mirror` run (the contract is read-only over
//!   files; the mirror must not leave the project in a
//!   state the doctor considers unhealthy).
//! - `commit` + `mirror` are sequenced: a `forge commit` is
//!   the natural pre-condition for `forge mirror`, and the
//!   mirror state file must survive the commit and the next
//!   mirror run.
//! - The `mirror` operation is journaled in the registry's
//!   `operations` table under the `mirror` kind, with a
//!   per-run summary that records primary + mirror outcomes.
//! - `agent` sessions are independent of distribution: an
//!   agent session started on a project that has a
//!   distribution section is not affected by the mirror
//!   surface, and vice versa.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stdout={} stderr={}",
        args,
        out.status,
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
}

fn bare_repo(tmp: &Path, label: &str) -> PathBuf {
    let path = tmp.join(format!("{label}.git"));
    fs::create_dir_all(&path).unwrap();
    let mut cmd = Command::new("git");
    cmd.arg("init").arg("--bare").arg("-q").arg(&path);
    let out = cmd.output().expect("git init --bare");
    assert!(
        out.status.success(),
        "git init --bare failed: {}",
        lossy(&out.stderr)
    );
    let mut head_cmd = Command::new("git");
    head_cmd
        .arg("symbolic-ref")
        .arg("HEAD")
        .arg("refs/heads/main")
        .current_dir(&path);
    let head_out = head_cmd.output().expect("git symbolic-ref HEAD");
    assert!(
        head_out.status.success(),
        "git symbolic-ref HEAD failed: {}",
        lossy(&head_out.stderr)
    );
    path
}

fn write_distribution_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndistribution:\n  primary: github\n  mirrors:\n    - gitee\n"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "--", "forge.yaml", "README.md"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

#[test]
fn doctor_remains_healthy_after_successful_mirror() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "doc-mirror");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    // Doctor before mirror: project is fresh; the
    // distribution mirror paths are in the repo as
    // configured remotes and should not change the verdict.
    let before = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    // Successful mirror.
    let _ = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    // Doctor after mirror: the verdict must be the same
    // as before. The mirror contract is read-only over
    // project files; nothing it writes should change the
    // doctor finding set.
    let after = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let before_verdict = before["doctor"]["verdict"].as_str();
    let after_verdict = after["doctor"]["verdict"].as_str();
    assert_eq!(
        before_verdict, after_verdict,
        "doctor verdict changed after mirror: before={before_verdict:?} after={after_verdict:?}"
    );
}

#[test]
fn mirror_journals_run_under_mirror_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "journal-mirror");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    // Register the project so journal rows carry a valid
    // project id.
    let _ = run_json(&db, &["register", proj.to_str().unwrap()]);
    // Successful mirror run; the journal row records the
    // per-remote summary and the verdict.
    let _ = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    // Use the doctor summary, which exposes the journal as
    // `operations`, to assert the journal entries.
    let doctor = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    // The journal itself is not exposed through the doctor
    // output, but the registry's `record_operation` is the
    // only path that records mirror runs. The test
    // therefore inspects the registry directly.
    let registry = forge::registry::Registry::open(&db).expect("registry");
    let entries = registry.journal_entries().expect("journal");
    let mirror_entries: Vec<_> = entries.iter().filter(|e| e.kind == "mirror").collect();
    assert!(
        !mirror_entries.is_empty(),
        "mirror run must be journaled; entries={entries:?}"
    );
    // The last mirror entry must reference the project
    // id and report the verdict.
    let last = mirror_entries.last().unwrap();
    assert_eq!(last.project_id, "journal-mirror");
    let detail = last.detail.clone().unwrap_or_default();
    assert!(detail.contains("primary=github"), "detail={detail}");
    assert!(detail.contains("healthy=true"), "detail={detail}");
    let _ = doctor;
}

#[test]
fn commit_then_mirror_preserves_state_and_runs_again() {
    // The natural pre-condition for a mirror run is a
    // commit. The contract must round-trip through:
    // (1) `forge commit` to record a new SHA on the local
    //     branch
    // (2) `forge mirror` to push the branch to the primary
    //     and the mirror
    // (3) a second `forge commit` to add more state
    // (4) `forge mirror --retry-failed` to verify the
    //     state file correctly tracks which refs were
    //     delivered
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "commit-mirror");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    // (1) Initial commit: the project is already at a
    // clean state; commit a new edit to give the
    // mirror something to push.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    let commit = run_json(
        &db,
        &[
            "commit",
            proj.to_str().unwrap(),
            "--path",
            "README.md",
            "--message",
            "bump readme",
        ],
    );
    let commit_sha = commit["commit"]["commit_sha"]
        .as_str()
        .expect("commit sha")
        .to_string();
    // (2) First mirror run.
    let first = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let first_primary = first["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .unwrap();
    let first_mirror = first["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .unwrap();
    assert_eq!(first_primary["status"], "delivered");
    assert_eq!(first_mirror["status"], "delivered");
    assert_eq!(
        first_primary["commit_sha"].as_str(),
        Some(commit_sha.as_str())
    );
    // (3) A second commit advances the local SHA. The
    // state file still records the previous SHA; a retry
    // must NOT skip the new commit because the stored
    // SHA does not match the current HEAD.
    fs::write(proj.join("README.md"), "v3\n").unwrap();
    let _ = run_json(
        &db,
        &[
            "commit",
            proj.to_str().unwrap(),
            "--path",
            "README.md",
            "--message",
            "second readme",
        ],
    );
    // (4) Retry after the second commit. The retry's
    // contract re-reads the live git state and pushes the
    // current SHA; the previous SHA in the state file is
    // superseded by the new commit.
    let retry = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
            "--retry-failed",
        ],
    );
    let retry_primary = retry["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "primary")
        .unwrap();
    let retry_mirror = retry["mirror"]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "mirror")
        .unwrap();
    // After the second commit, the retry must re-push
    // because the state file does not match the new
    // HEAD; the status is therefore `delivered`, not
    // `skipped`.
    assert_eq!(retry_primary["status"], "delivered");
    assert_eq!(retry_mirror["status"], "delivered");
    assert_ne!(
        retry_primary["commit_sha"].as_str(),
        Some(commit_sha.as_str()),
        "retry must push the new SHA, not the previously delivered one"
    );
    // State file records the new SHAs.
    let state_path = proj.join(".forge/distribution/commit-mirror/state.json");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).expect("state")).expect("state json");
    assert!(state["primary_delivered"]["main"].is_string());
    assert!(state["mirrors_delivered"]["gitee"]["main"].is_string());
}

#[test]
fn agent_session_does_not_block_mirror_or_vice_versa() {
    // The agent and distribution surfaces are independent:
    // an active agent session on a project does not block
    // a `forge mirror` run, and a `forge mirror` run does
    // not change the agent session state.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "agent-mirror");
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );
    // Start an agent session.
    let start = run_json(
        &db,
        &[
            "agent",
            "start",
            proj.to_str().unwrap(),
            "--session",
            "sess-1",
            "--provider",
            "opencode",
        ],
    );
    let session_id = start["transition"]["session_id"].as_str().unwrap();
    assert_eq!(session_id, "sess-1");
    // Run a mirror. The agent session must remain
    // `active` (the mirror contract is not aware of the
    // agent runtime).
    let _ = run_json(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    let status = run_json(
        &db,
        &[
            "agent",
            "status",
            proj.to_str().unwrap(),
            "--session",
            "sess-1",
        ],
    );
    let state = status["state"].as_str().unwrap();
    assert_eq!(state, "active");
    // List sessions to ensure the session is still
    // discoverable after the mirror run.
    let list = run_json(&db, &["agent", "list", proj.to_str().unwrap()]);
    let sessions = list["sessions"].as_array().expect("sessions");
    assert!(sessions.iter().any(|s| s["session_id"] == "sess-1"));
}

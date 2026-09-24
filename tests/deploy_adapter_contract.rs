//! Reference Jenkins adapter round trips (`jenkins-deploy-adapter-consumption`).
//!
//! Exercises the bundled `adapters/jenkins/forge-deployer-jenkins`
//! executable against stubbed jenkins-local trees (the sandbox the
//! design calls a "disposable Jenkins-less host"):
//!
//! - the dry-run rehearsal reaches `delivered` through Forge with
//!   no side effect and never writes persisted state;
//! - a real trigger delivers with the Jenkins guidance evidence;
//!   blocked `project.sh` exit codes surface as failed stages with
//!   the runtime's own recovery lines and preserve prior state;
//! - observe uses the read-only status verb, maps the explicit
//!   health vocabulary (running/stopped/everything-else), never
//!   claims healthy outside `running`, and an unknown status
//!   preserves the last good observation;
//! - a missing or incomplete executor stays `unavailable` with the
//!   persisted record byte-identical;
//! - the provider matrix `deploy` row reaches `supported` against
//!   the adapter with `--dry-run` only (sandbox `fixture`) and
//!   names the adapter and revision; without the executor
//!   configured the row is `unavailable`, never healthy.
//!
//! Production Jenkins evidence is explicitly deferred to the
//! jenkins-local adoption checklist in
//! `adapters/jenkins/jenkins-adapter.md`; nothing here claims it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ADAPTER: &str = "adapters/jenkins/forge-deployer-jenkins";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn adapter_path() -> PathBuf {
    repo_root().join(ADAPTER)
}

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_JENKINS_LOCAL_DIR")
        .env_remove("FORGE_JENKINS_STATUS_CMD")
        .env_remove("FORGE_JENKINS_REVISION")
        .env_remove("TRACE_DIR")
        .env_remove("STATUS_TOKEN")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    cmd
}

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
}

fn write_deploy_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndeployment:\n  artifact: docker-compose.yml\n  default: home\n  targets:\n    - name: home\n      kind: docker-compose\n      service: app\n  health:\n    kind: docker\n    service: app\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("docker-compose.yml"),
        "services:\n  app:\n    image: app:0.1.0\n",
    )
    .unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "--", "forge.yaml", "docker-compose.yml"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

fn make_exec(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

/// Stubbed jenkins-local checkout: `project.sh` traces every real
/// (non-dry-run) invocation into `$TRACE_DIR/trace.log`, maps one
/// project name onto the blocked exit 3 path and leaks a
/// credential-shaped line plus an absolute host path that the
/// adapter must scrub. `project-action.sh status` answers with a
/// `project-status.sh`-shaped row driven by `$STATUS_TOKEN`.
fn stub_jenkins_local(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    make_exec(
        &dir.join("project.sh"),
        r#"#!/bin/sh
project=$1
action=${2:-deploy}
shift 2 || true
for a in "$@"; do
  if [ "$a" = "--dry-run" ]; then
    echo "Project: $project"
    echo "Action: $action"
    echo "Would sync, configure shared PostgreSQL when needed, register the Jenkins job, and trigger it."
    exit 0
  fi
done
[ -n "$TRACE_DIR" ] && echo "apply $project $action" >> "$TRACE_DIR/trace.log"
case "$project" in
  blocked-demo)
    echo "Project configuration was created." >&2
    echo "Add the project's production values, then rerun: project.sh $project deploy" >&2
    exit 3
    ;;
esac
echo "Project: $project"
echo "Action: $action"
echo "Triggered Jenkins: Production/$project ($action)"
echo "synced /Users/operator/jenkins/projects/$project with token ghp_abcdefghijklmnopqrstuvwxyz0123456789" >&2
exit 0
"#,
    );
    make_exec(
        &dir.join("project-action.sh"),
        r#"#!/bin/sh
project=$1
action=${2:-status}
[ "$action" = status ] || { echo "bad action $action" >&2; exit 2; }
[ -n "$TRACE_DIR" ] && echo "observe $project $action" >> "$TRACE_DIR/trace.log"
echo "PROJECT                  CONFIG       CONTAINERS   URL"
echo "------------------------ ------------ ------------ ---"
printf '%-24s %-12s %-12s %s\n' "$project" allocated "$STATUS_TOKEN" "https://$project.example"
exit 0
"#,
    );
}

fn deploy_cli(
    db: &Path,
    proj: &Path,
    args: &[&str],
    jenkins_dir: Option<&Path>,
    trace: &Path,
    status_token: &str,
) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .env("FORGE_DEPLOYER_BIN", adapter_path())
        .env("TRACE_DIR", trace)
        .env("STATUS_TOKEN", status_token);
    if let Some(dir) = jenkins_dir {
        cmd.env("FORGE_JENKINS_LOCAL_DIR", dir);
    }
    cmd.args(["deploy"]);
    cmd.args(args);
    cmd.args([proj.to_str().unwrap()]);
    cmd.output().expect("forge deploy")
}

fn provider_run(db: &Path, jenkins_dir: Option<&Path>, trace: &Path) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .env("TRACE_DIR", trace)
        .env("STATUS_TOKEN", "running")
        .env("FORGE_JENKINS_REVISION", "cannedrev");
    if let Some(dir) = jenkins_dir {
        cmd.env("FORGE_JENKINS_LOCAL_DIR", dir);
    }
    cmd.args(["provider", "run", "deploy", "--fixture"]);
    cmd.arg(adapter_path());
    cmd.output().expect("forge provider run")
}

fn setup(name: &str) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let jenkins = tmp.path().join("jenkins-local");
    let trace = tmp.path().join("trace");
    fs::create_dir_all(&trace).unwrap();
    write_deploy_project(&proj, name);
    stub_jenkins_local(&jenkins);
    (tmp, db, proj, jenkins, trace)
}

fn trace_lines(trace: &Path) -> Vec<String> {
    let text = fs::read_to_string(trace.join("trace.log")).unwrap_or_default();
    text.lines().map(|s| s.to_string()).collect()
}

#[test]
fn adapter_version_grammar_is_probeable() {
    let out = Command::new(adapter_path())
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("adapter --version");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(
        text.starts_with("forge-deployer-jenkins/0.1.0 (contract forge-deploy-executor/0.1.0)"),
        "version line must name adapter and contract: {text}"
    );
}

#[test]
fn dry_run_rehearsal_delivers_without_side_effect_or_state() {
    let (tmp, db, proj, jenkins, trace) = setup("jenkins-dryrun");
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm", "--dry-run"],
        Some(&jenkins),
        &trace,
        "running",
    );
    let stdout = lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("stdout: {stdout}"));
    let report = &json["deploy"];
    assert_eq!(report["dry_run"], true);
    let apply = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "delivered");
    assert!(
        apply["note"]
            .as_str()
            .unwrap_or("")
            .contains("dry-run rehearsal"),
        "{apply:?}"
    );
    assert!(apply["evidence"].as_array().unwrap().iter().any(|e| e
        .as_str()
        .unwrap_or("")
        .starts_with("executor=forge-deployer-jenkins/0.1.0@")));
    // No side effect ran and no persisted state exists for a rehearsal.
    assert!(
        trace_lines(&trace).is_empty(),
        "trace: {:?}",
        trace_lines(&trace)
    );
    assert!(
        !proj.join(".forge/deploy").exists(),
        "dry-run must never persist deploy state"
    );
    let _ = tmp;
}

#[test]
fn real_apply_delivers_then_observe_running_and_unknown_preserves_history() {
    let (tmp, db, proj, jenkins, trace) = setup("jenkins-apply-observe");
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    let stdout = lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("stdout: {stdout}"));
    let report = &json["deploy"];
    let apply = report["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "delivered");
    // A triggered-but-unobserved pipeline is honest `unknown`, not
    // healthy: the CLI exits with the health failure boundary.
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("deploy-health-failed"));
    assert_eq!(
        trace_lines(&trace),
        vec!["apply jenkins-apply-observe deploy"]
    );

    // observe with a running row: healthy through the read-only verb.
    let out = deploy_cli(
        &db,
        &proj,
        &["observe", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(json["deploy"]["stages"][0]["status"], "running");
    // Observe never re-triggered the deployment.
    assert_eq!(
        trace_lines(&trace),
        vec![
            "apply jenkins-apply-observe deploy".to_string(),
            "observe jenkins-apply-observe status".to_string(),
        ]
    );

    // Unrecognized status: unknown now, previous good observation
    // remains the recorded history entry.
    let out = deploy_cli(
        &db,
        &proj,
        &["observe", "--confirm"],
        Some(&jenkins),
        &trace,
        "partial",
    );
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(json["deploy"]["observation"]["status"], "unknown");
    let list = {
        let mut cmd = clean_cmd();
        cmd.arg("--registry")
            .arg(&db)
            .arg("--format")
            .arg("json")
            .args(["deploy", "list"])
            .arg(&proj);
        cmd.output().expect("list")
    };
    let list: serde_json::Value = serde_json::from_slice(&list.stdout).expect("list json");
    let deploy_id = list["deploys"][0]["deploy_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(list["deploys"][0]["current_state"], "unknown");
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .args(["deploy", "inspect"])
        .arg(&deploy_id)
        .arg(&proj);
    let out = cmd.output().expect("inspect");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("inspect json");
    assert_eq!(json["deploy"]["current_state"], "unknown");
    assert_eq!(json["deploy"]["last_observed_running"]["status"], "running");
    let _ = tmp;
}

#[test]
fn stopped_status_observes_down_as_failed() {
    let (tmp, db, proj, jenkins, trace) = setup("jenkins-stopped");
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let out = deploy_cli(
        &db,
        &proj,
        &["observe", "--confirm"],
        Some(&jenkins),
        &trace,
        "stopped",
    );
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(json["deploy"]["observation"]["status"], "failed");
    let _ = tmp;
}

#[test]
fn blocked_project_surfaces_failed_stage_with_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let jenkins = tmp.path().join("jenkins-local");
    let trace = tmp.path().join("trace");
    fs::create_dir_all(&trace).unwrap();
    // The blocked stub is keyed on the jenkins-local project name
    // `blocked-demo`; the adapter forwards Forge's project id, so
    // register the manifest under that id.
    write_deploy_project(&proj, "blocked-demo");
    stub_jenkins_local(&jenkins);
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    let stdout = lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("stdout: {stdout}"));
    let apply = json["deploy"]["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["stage"] == "apply")
        .expect("apply stage");
    assert_eq!(apply["status"], "failed");
    let recovery = apply["recovery"].as_array().expect("recovery");
    assert!(
        recovery
            .iter()
            .any(|r| r.as_str().unwrap_or("").contains("rerun the apply")),
        "recovery must name the operator step: {recovery:?}"
    );
    let evidence = apply["evidence"].as_array().expect("evidence");
    assert!(
        evidence
            .iter()
            .any(|e| e.as_str().unwrap_or("").contains("production values")),
        "the runtime's own guidance must survive as evidence: {evidence:?}"
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !proj.join(".forge/deploy").exists(),
        "a blocked trigger must never persist state"
    );
    let _ = tmp;
}

#[test]
fn evidence_is_scrubbed_of_secrets_and_host_paths() {
    let (tmp, db, proj, jenkins, trace) = setup("jenkins-scrub");
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    let stdout = lossy(&out.stdout);
    assert!(
        !stdout.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "credential-shaped output must never reach stdout"
    );
    assert!(
        !stdout.contains("/Users/operator"),
        "absolute host paths must never reach stdout"
    );
    assert!(stdout.contains("[REDACTED]"), "{stdout}");
    assert!(stdout.contains("<host-path>"), "{stdout}");
    // And never reaches the persisted record either.
    let state_files: Vec<PathBuf> = walkstate(&proj.join(".forge/deploy"));
    assert_eq!(state_files.len(), 1, "{state_files:?}");
    let state = fs::read_to_string(&state_files[0]).unwrap();
    assert!(!state.contains("ghp_"), "persisted record leaked a secret");
    assert!(
        !state.contains("/Users/operator"),
        "persisted record leaked a path"
    );
    let _ = tmp;
}

fn walkstate(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !dir.exists() {
        return out;
    }
    let entries = fs::read_dir(dir).expect("read_dir");
    for entry in entries {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            out.extend(walkstate(&path));
        } else if path.file_name().map(|n| n == "state.json").unwrap_or(false) {
            out.push(path);
        }
    }
    out
}

#[test]
fn missing_executor_configuration_is_unavailable_not_failed() {
    let (tmp, db, proj, _jenkins, trace) = setup("jenkins-absent");
    let out = deploy_cli(&db, &proj, &["apply", "--confirm"], None, &trace, "running");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("deploy-target-unavailable"), "{stderr}");
    assert!(stderr.contains("FORGE_JENKINS_LOCAL_DIR"), "{stderr}");
    assert!(
        !proj.join(".forge/deploy").exists(),
        "unavailable must not invent a state record"
    );
    let _ = tmp;
}

#[test]
fn incomplete_jenkins_tree_leaves_prior_state_byte_identical() {
    let (tmp, db, proj, jenkins, trace) = setup("jenkins-partial-tree");
    // First apply delivers and persists.
    let out = deploy_cli(
        &db,
        &proj,
        &["apply", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    assert_eq!(out.status.code(), Some(1)); // delivered + unobserved
    let state_files = walkstate(&proj.join(".forge/deploy"));
    assert_eq!(state_files.len(), 1);
    let before = fs::read(&state_files[0]).unwrap();
    // Remove the status verb: the executor is now incomplete. A
    // re-observation must be unavailable and change nothing.
    fs::remove_file(jenkins.join("project-action.sh")).unwrap();
    let out = deploy_cli(
        &db,
        &proj,
        &["observe", "--confirm"],
        Some(&jenkins),
        &trace,
        "running",
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("deploy-target-unavailable"), "{stderr}");
    let after = fs::read(&state_files[0]).unwrap();
    assert_eq!(
        before, after,
        "an unavailable executor must not touch the record"
    );
    let _ = tmp;
}

#[test]
fn provider_deploy_row_reaches_supported_dry_run_only() {
    let (tmp, db, _proj, jenkins, trace) = setup("provider-row");
    let out = provider_run(&db, Some(&jenkins), &trace);
    let stdout = lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("stdout: {stdout}"));
    let row = &json["run"];
    assert_eq!(row["status"], "supported", "{row}");
    assert_eq!(row["provenance"]["sandbox"], "fixture");
    let evidence: Vec<&str> = row["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap_or(""))
        .collect();
    assert!(
        evidence.contains(&"adapter_source=forge-deployer-jenkins/0.1.0@cannedrev"),
        "row must name the adapter and its revision: {evidence:?}"
    );
    assert!(
        evidence.contains(&"adapter_status=delivered"),
        "{evidence:?}"
    );
    // The dry-run probe performed no side effect at all.
    assert!(
        trace_lines(&trace).is_empty(),
        "trace: {:?}",
        trace_lines(&trace)
    );
    let _ = tmp;
}

#[test]
fn provider_deploy_row_without_executor_is_unavailable_not_healthy() {
    let (tmp, db, _proj, _jenkins, trace) = setup("provider-row-absent");
    let out = provider_run(&db, None, &trace);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let row = &json["run"];
    assert_eq!(row["status"], "unavailable", "{row}");
    let receipt: Vec<&str> = row["provenance"]["receipt"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap_or(""))
        .collect();
    assert!(
        receipt
            .iter()
            .any(|e| e.contains("FORGE_JENKINS_LOCAL_DIR")),
        "the refusal must name the missing configuration: {receipt:?}"
    );
    assert!(trace_lines(&trace).is_empty());
    let _ = tmp;
}

//! Reference SSH adapter round trips (`deploy-target-ssh`).
//!
//! Exercises the bundled `adapters/ssh/forge-deployer-ssh`
//! executable against a recording `ssh`/`scp` pair, so the
//! transfer's argv, its order, the staged name, the rename, the
//! health vocabulary, an unreachable host and the dry-run
//! rehearsal are all checkable without a network and without a
//! host.
//!
//! Production SSH evidence is explicitly deferred to the host
//! owner's adoption checklist in `adapters/ssh/ssh-adapter.md`;
//! nothing here claims it.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

const ADAPTER: &str = "adapters/ssh/forge-deployer-ssh";

fn adapter_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ADAPTER)
}

/// What the recording programs answer.
#[derive(Clone)]
struct Answers {
    ssh_exit: i32,
    scp_exit: i32,
    ssh_stdout: String,
}

impl Default for Answers {
    fn default() -> Self {
        Answers {
            ssh_exit: 0,
            scp_exit: 0,
            ssh_stdout: String::new(),
        }
    }
}

/// The result of one adapter invocation.
struct Outcome {
    code: i32,
    envelope: Value,
}

/// A recording `ssh`/`scp` pair and the trace they write.
struct Recorder {
    dir: TempDir,
    trace: PathBuf,
}

impl Recorder {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a temp dir");
        let trace = dir.path().join("trace.log");
        let ssh = dir.path().join("fake-ssh");
        let scp = dir.path().join("fake-scp");
        fs::write(
            &ssh,
            "#!/bin/sh\nprintf 'ssh %s\\n' \"$*\" >> \"$TRACE\"\nif [ -n \"$FAKE_SSH_STDOUT\" ]; then printf '%s\\n' \"$FAKE_SSH_STDOUT\"; fi\nexit \"${FAKE_SSH_EXIT:-0}\"\n",
        )
        .expect("the ssh recorder");
        fs::write(
            &scp,
            "#!/bin/sh\nprintf 'scp %s\\n' \"$*\" >> \"$TRACE\"\nexit \"${FAKE_SCP_EXIT:-0}\"\n",
        )
        .expect("the scp recorder");
        for script in [&ssh, &scp] {
            let mut perms = fs::metadata(script).expect("the recorder").permissions();
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
            fs::set_permissions(script, perms).expect("the recorder is executable");
        }
        Recorder { dir, trace }
    }

    /// The recorded invocations, in order.
    fn calls(&self) -> Vec<String> {
        match fs::read_to_string(&self.trace) {
            Ok(text) => text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.to_string())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Run the adapter once, with the recording programs in place.
    fn run(&self, args: &[&str], payload: &Value, answers: &Answers) -> Outcome {
        let mut child = Command::new(adapter_path())
            .args(args)
            .env("FORGE_SSH_BIN", self.dir.path().join("fake-ssh"))
            .env("FORGE_SCP_BIN", self.dir.path().join("fake-scp"))
            .env("TRACE", &self.trace)
            .env("FAKE_SSH_EXIT", answers.ssh_exit.to_string())
            .env("FAKE_SCP_EXIT", answers.scp_exit.to_string())
            .env("FAKE_SSH_STDOUT", &answers.ssh_stdout)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the adapter starts");
        let payload = serde_json::to_vec(payload).expect("the payload serializes");
        {
            use std::io::Write;
            let mut stdin = child.stdin.take().expect("the adapter's stdin");
            stdin.write_all(&payload).expect("the payload is delivered");
        }
        let output = child.wait_with_output().expect("the adapter finishes");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let envelope: Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|err| panic!("the adapter prints one JSON envelope: {err}: {stdout}"));
        Outcome {
            code: output.status.code().unwrap_or(-1),
            envelope,
        }
    }
}

/// The payload of a deploy against an `ssh` target.
fn payload(host: Option<&str>, path: Option<&str>) -> Value {
    json!({
        "contract": "forge-deploy-executor/0.1.0",
        "project_id": "app",
        "deploy_id": "app-vps-0123456789ab",
        "target": {
            "name": "vps",
            "kind": "ssh",
            "host": host,
            "user": "deploy",
            "path": path,
            "service": null,
            "note": null
        },
        "artifact": {
            "path": "docker-compose.yml",
            "content_hash": "abc",
            "byte_size": 42
        },
        "health": {
            "kind": "docker",
            "service": "app",
            "url": null,
            "process": null,
            "interval_seconds": null
        },
        "revision": "deadbeef",
        "dry_run": false
    })
}

/// The payload of a deploy against an `ssh` target with no health
/// check declared.
fn payload_without_health(host: &str, path: &str) -> Value {
    let mut value = payload(Some(host), Some(path));
    value["health"] = Value::Null;
    value
}

fn apply_args() -> Vec<&'static str> {
    vec![
        "apply",
        "--target",
        "vps",
        "--kind",
        "ssh",
        "--project",
        "app",
        "--revision",
        "deadbeef",
        "--artifact",
        "docker-compose.yml",
    ]
}

fn observe_args() -> Vec<&'static str> {
    vec![
        "observe",
        "--target",
        "vps",
        "--project",
        "app",
        "--deploy-id",
        "app-vps-0123456789ab",
    ]
}

fn evidence(envelope: &Value, key: &str) -> Vec<String> {
    envelope
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|text| text.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_apply_transfers_the_artifact_in_three_commands() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &apply_args(),
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_eq!(outcome.code, 0, "a delivered transfer exits zero");
    assert_eq!(outcome.envelope["apply_status"], "delivered");
    assert_eq!(
        recorder.calls(),
        vec![
            "ssh deploy@home-server-01 /bin/mkdir -p /srv/app".to_string(),
            "scp docker-compose.yml deploy@home-server-01:/srv/app/docker-compose.yml.forge-new"
                .to_string(),
            "ssh deploy@home-server-01 /bin/mv -f /srv/app/docker-compose.yml.forge-new /srv/app/docker-compose.yml"
                .to_string(),
        ],
        "the three commands run in this order, with the staged name in between"
    );
}

#[test]
fn the_adapter_refuses_a_kind_it_does_not_serve() {
    let recorder = Recorder::new();
    let mut args = apply_args();
    let kind = args.iter().position(|word| *word == "--kind").expect("the kind flag");
    args[kind + 1] = "docker-compose";
    let outcome = recorder.run(
        &args,
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_eq!(outcome.code, 1, "a refused kind exits non-zero");
    assert_eq!(outcome.envelope["apply_status"], "failed");
    assert!(
        outcome.envelope["apply_note"]
            .as_str()
            .unwrap_or_default()
            .contains("not wired through the SSH executor"),
        "the refusal names the reason: {}",
        outcome.envelope["apply_note"]
    );
    assert!(
        recorder.calls().is_empty(),
        "a refused kind contacts no host"
    );
}

#[test]
fn the_remote_path_never_reaches_the_evidence() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &apply_args(),
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    let lines = evidence(&outcome.envelope, "apply_evidence");
    assert!(!lines.is_empty(), "the transfer names what it did");
    for line in &lines {
        assert!(
            !line.contains("/srv/app"),
            "the remote path never appears in the evidence: {line}"
        );
    }
    assert!(
        lines
            .iter()
            .any(|line| line.contains("<host-path>.forge-new")),
        "the staged name is reported: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("scp docker-compose.yml ")),
        "the local artifact is reported by its project-relative name: {lines:?}"
    );
}

#[test]
fn a_failed_transfer_is_a_failed_stage_with_a_non_zero_exit() {
    let recorder = Recorder::new();
    let answers = Answers {
        scp_exit: 1,
        ..Answers::default()
    };
    let outcome = recorder.run(
        &apply_args(),
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &answers,
    );
    assert_ne!(outcome.code, 0, "a failed transfer exits non-zero");
    assert_eq!(outcome.envelope["apply_status"], "failed");
    assert!(
        !evidence(&outcome.envelope, "apply_evidence").is_empty(),
        "a failed stage keeps the evidence of what it tried"
    );
    assert!(
        !evidence(&outcome.envelope, "recovery").is_empty(),
        "a failed stage names the operator's next steps"
    );
    assert_eq!(
        recorder.calls().len(),
        2,
        "the transfer stops at the failing step"
    );
}

#[test]
fn the_dry_run_contacts_no_host() {
    let recorder = Recorder::new();
    let mut args = apply_args();
    args.push("--dry-run");
    let outcome = recorder.run(
        &args,
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.envelope["apply_status"], "delivered");
    assert!(
        outcome.envelope["apply_note"]
            .as_str()
            .unwrap_or_default()
            .contains("dry-run"),
        "the note says it was a rehearsal: {}",
        outcome.envelope["apply_note"]
    );
    assert!(
        recorder.calls().is_empty(),
        "a dry run contacts no host: {:?}",
        recorder.calls()
    );
    let lines = evidence(&outcome.envelope, "apply_evidence");
    assert!(
        lines.iter().any(|line| line.contains(".forge-new")),
        "the evidence names the transfer it would have made: {lines:?}"
    );
}

#[test]
fn a_target_without_a_host_is_refused() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &apply_args(),
        &payload(None, Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_ne!(outcome.code, 0);
    assert_eq!(outcome.envelope["apply_status"], "failed");
    assert!(
        outcome.envelope["apply_note"]
            .as_str()
            .unwrap_or_default()
            .contains("host"),
        "the refusal names what is missing: {}",
        outcome.envelope["apply_note"]
    );
    assert!(recorder.calls().is_empty());
}

#[test]
fn a_remote_path_that_could_become_a_command_is_refused() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &apply_args(),
        &payload(
            Some("home-server-01"),
            Some("/srv/app; rm -rf /srv/app/docker-compose.yml"),
        ),
        &Answers::default(),
    );
    assert_ne!(outcome.code, 0);
    assert_eq!(outcome.envelope["apply_status"], "failed");
    assert!(
        recorder.calls().is_empty(),
        "a refused path reaches no command line"
    );
}

#[test]
fn the_health_vocabulary_is_explicit() {
    let cases: Vec<(&str, &str, i32, &str, &str)> = vec![
        ("docker", "app", 0, "Up 4 minutes", "running"),
        ("docker", "app", 0, "Restarting (1) 2 seconds ago", "failed"),
        ("http", "http://127.0.0.1:8080/health", 0, "", "running"),
        ("http", "http://127.0.0.1:8080/health", 22, "", "failed"),
        ("process", "forge-agent", 0, "", "running"),
        ("process", "forge-agent", 1, "", "failed"),
    ];
    for (kind, subject, exit, stdout, expected) in cases {
        let recorder = Recorder::new();
        let answers = Answers {
            ssh_exit: exit,
            ssh_stdout: stdout.to_string(),
            ..Answers::default()
        };
        let mut args = observe_args();
        args.push("--health-kind");
        args.push(kind);
        match kind {
            "docker" => {
                args.push("--health-service");
                args.push(subject);
            }
            "http" => {
                args.push("--health-url");
                args.push(subject);
            }
            _ => {
                args.push("--health-process");
                args.push(subject);
            }
        }
        let outcome = recorder.run(
            &args,
            &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
            &answers,
        );
        assert_eq!(outcome.code, 0, "an observation is not a failed stage");
        assert_eq!(
            outcome.envelope["observation_status"], expected,
            "the `{kind}` check with exit {exit} and output `{stdout}`"
        );
    }
}

#[test]
fn a_service_that_is_not_visible_is_unknown() {
    let recorder = Recorder::new();
    let mut args = observe_args();
    args.push("--health-kind");
    args.push("docker");
    args.push("--health-service");
    args.push("app");
    let outcome = recorder.run(
        &args,
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_eq!(outcome.envelope["observation_status"], "unknown");
    assert!(
        outcome.envelope["observation_detail"]
            .as_str()
            .unwrap_or_default()
            .contains("not visible"),
        "the detail says the service is not visible: {}",
        outcome.envelope["observation_detail"]
    );
}

#[test]
fn a_target_without_a_health_check_is_unknown() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &observe_args(),
        &payload_without_health("home-server-01", "/srv/app/docker-compose.yml"),
        &Answers::default(),
    );
    assert_eq!(outcome.envelope["observation_status"], "unknown");
    assert_eq!(
        outcome.envelope["observation_detail"],
        "no health check declared for this target"
    );
}

#[test]
fn an_unreachable_host_is_unknown_not_failed() {
    let recorder = Recorder::new();
    let answers = Answers {
        ssh_exit: 255,
        ssh_stdout: "ssh: connect to host home-server-01 port 22: Connection refused".to_string(),
        ..Answers::default()
    };
    let mut args = observe_args();
    args.push("--health-kind");
    args.push("docker");
    args.push("--health-service");
    args.push("app");
    let outcome = recorder.run(
        &args,
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &answers,
    );
    assert_eq!(outcome.code, 0);
    assert_eq!(
        outcome.envelope["observation_status"], "unknown",
        "disconnected means unknown, not offline proof"
    );
    assert!(
        outcome.envelope["observation_detail"]
            .as_str()
            .unwrap_or_default()
            .contains("could not be reached"),
        "the detail says the host could not be reached: {}",
        outcome.envelope["observation_detail"]
    );
}

#[test]
fn the_adapter_names_its_contract_and_its_source() {
    let recorder = Recorder::new();
    let outcome = recorder.run(
        &observe_args(),
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &Answers::default(),
    );
    assert_eq!(outcome.envelope["contract"], "forge-deploy-executor/0.1.0");
    assert_eq!(outcome.envelope["source"], "forge-deployer-ssh/0.1.0");
    assert_eq!(outcome.envelope["source_revision"], "unknown");
}

#[test]
fn credential_shaped_output_is_scrubbed() {
    let recorder = Recorder::new();
    let answers = Answers {
        ssh_exit: 1,
        ssh_stdout: "token=ghp_abcdefghijklmnopqrstuvwxyz012345".to_string(),
        ..Answers::default()
    };
    let mut args = observe_args();
    args.push("--health-kind");
    args.push("docker");
    args.push("--health-service");
    args.push("app");
    let outcome = recorder.run(
        &args,
        &payload(Some("home-server-01"), Some("/srv/app/docker-compose.yml")),
        &answers,
    );
    let lines = evidence(&outcome.envelope, "observation_evidence");
    assert!(!lines.is_empty());
    for line in &lines {
        assert!(
            !line.contains("ghp_abcdefghijklmnopqrstuvwxyz012345"),
            "a credential never leaves the adapter: {line}"
        );
    }
}

//! Operator login credentials contract: `forge identity status`,
//! `--password-stdin`, and `scripts/web.sh start` / `reset-password`.
//!
//! The store keeps only an Argon2id password hash, so an existing password
//! can never be recovered. These tests pin the behavior that makes the
//! operator's login reachable: a readable status, a non-interactive input
//! path that never puts the secret in argv, rotation that revokes sessions,
//! and a start/reset script that prints the login exactly once without
//! persisting it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

const EMAIL: &str = "operator@example.test";
const DEFAULT_EMAIL: &str = "operator@example.com";
const PASSWORD: &str = "a-strong-test-password";
const NEW_PASSWORD: &str = "a-rotated-strong-password";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/web.sh")
}

/// Run the real binary with an isolated registry, no ambient
/// `FORGE_REGISTRY`, no stdin.
fn run(dir: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(dir);
    cmd.output().expect("run forge")
}

/// Run the real binary with a password piped on stdin.
fn run_with_stdin(dir: &Path, args: &[&str], stdin: &str) -> Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(dir);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait forge")
}

fn run_json(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json output")
}

/// Run the real script with an isolated registry and run/log/web-root dirs.
fn run_script(root: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new("sh");
    cmd.arg(script());
    for arg in args {
        cmd.arg(arg);
    }
    cmd.env("FORGE_BIN", forge_bin());
    cmd.env("FORGE_REGISTRY", root.join("registry.db"));
    cmd.env("FORGE_WEB_RUN_DIR", root.join("run"));
    cmd.env("FORGE_WEB_LOG_DIR", root.join("log"));
    cmd.env("FORGE_WEB_ROOT_DIR", root.join("web-root"));
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.output().expect("run scripts/web.sh")
}

/// The banner line `web: account:    <email>`.
fn banner_email(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("web: account:"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The banner line `web: password:   <value>`; `None` for the explicit
/// "(unchanged; …)" line so callers can tell "no password printed".
fn banner_password(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("web: password:"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.starts_with('('))
}

/// Reserve a free loopback port by binding to `0` and releasing it.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

#[test]
fn status_reports_fresh_then_configured_with_email() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let fresh = run_json(dir, &["identity", "status", "--format", "json"]);
    assert_eq!(fresh["configured"], false);
    assert!(fresh["email"].is_null(), "fresh status must carry no email");

    // The human form is trivially parseable and never leaks a secret.
    let human = run(dir, &["identity", "status"]);
    assert!(human.status.success());
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("configured: false"), "{text}");

    let setup = run_with_stdin(
        dir,
        &["identity", "setup", "--email", EMAIL, "--password-stdin"],
        &format!("{PASSWORD}\n"),
    );
    assert!(
        setup.status.success(),
        "setup failed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let configured = run_json(dir, &["identity", "status", "--format", "json"]);
    assert_eq!(configured["configured"], true);
    assert_eq!(configured["email"], EMAIL);

    let human = run(dir, &["identity", "status"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("configured: true"), "{text}");
    assert!(text.contains(&format!("email: {EMAIL}")), "{text}");
    assert!(!text.contains(PASSWORD), "status must never print a secret");
}

#[test]
fn setup_password_stdin_works_without_the_secret_in_argv() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let setup = run_with_stdin(
        dir,
        &["identity", "setup", "--email", EMAIL, "--password-stdin"],
        &format!("{PASSWORD}\n"),
    );
    assert!(
        setup.status.success(),
        "setup failed: {}",
        String::from_utf8_lossy(&setup.stderr)
    );

    // The password on stdin authenticated, so it really became the credential.
    let token =
        forge::identity::global::authenticate(dir.join("registry.db").as_path(), EMAIL, PASSWORD)
            .expect("authenticate")
            .expect("the piped password must be the stored credential");
    assert_eq!(token.len(), 64);

    // There is no argv password flag: passing the value as an argument is an
    // unknown argument, and `--help` advertises only `--password-stdin`.
    let argv_attempt = run(
        dir,
        &[
            "identity",
            "setup",
            "--email",
            EMAIL,
            "--password",
            PASSWORD,
        ],
    );
    assert!(
        !argv_attempt.status.success(),
        "a --password flag must not exist"
    );
    let help = run(dir, &["identity", "setup", "--help"]);
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("--password-stdin"), "{help_text}");
    assert!(
        !help_text.contains("--password <"),
        "no flag may accept the password as a value: {help_text}"
    );
}

#[test]
fn change_password_stdin_rotates_and_invalidates_a_prior_session() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let db = dir.join("registry.db");

    let setup = run_with_stdin(
        dir,
        &["identity", "setup", "--email", EMAIL, "--password-stdin"],
        &format!("{PASSWORD}\n"),
    );
    assert!(setup.status.success());
    let token = forge::identity::global::authenticate(&db, EMAIL, PASSWORD)
        .unwrap()
        .expect("old password works");
    assert!(forge::identity::global::session_valid(&db, &token).unwrap());

    let rotate = run_with_stdin(
        dir,
        &["identity", "change-password", "--password-stdin"],
        &format!("{NEW_PASSWORD}\n"),
    );
    assert!(
        rotate.status.success(),
        "rotate failed: {}",
        String::from_utf8_lossy(&rotate.stderr)
    );

    assert!(
        forge::identity::global::authenticate(&db, EMAIL, PASSWORD)
            .unwrap()
            .is_none(),
        "the old password must stop working"
    );
    assert!(
        forge::identity::global::authenticate(&db, EMAIL, NEW_PASSWORD)
            .unwrap()
            .is_some(),
        "the rotated password must work"
    );
    assert!(
        !forge::identity::global::session_valid(&db, &token).unwrap(),
        "a session minted before the rotation must be revoked"
    );
}

#[test]
fn a_short_password_on_stdin_is_refused() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let db = dir.join("registry.db");

    let weak_setup = run_with_stdin(
        dir,
        &["identity", "setup", "--email", EMAIL, "--password-stdin"],
        "short12\n",
    );
    assert!(!weak_setup.status.success());
    assert!(
        String::from_utf8_lossy(&weak_setup.stderr).contains("12"),
        "{}",
        String::from_utf8_lossy(&weak_setup.stderr)
    );
    assert!(!forge::identity::global::is_configured(&db).unwrap());

    // Once an administrator exists, a weak rotation leaves the credential
    // usable rather than locking the operator out.
    let setup = run_with_stdin(
        dir,
        &["identity", "setup", "--email", EMAIL, "--password-stdin"],
        &format!("{PASSWORD}\n"),
    );
    assert!(setup.status.success());
    let weak_rotate = run_with_stdin(
        dir,
        &["identity", "change-password", "--password-stdin"],
        "short12\n",
    );
    assert!(!weak_rotate.status.success());
    assert!(forge::identity::global::authenticate(&db, EMAIL, PASSWORD)
        .unwrap()
        .is_some());
}

#[test]
fn reset_password_prints_once_and_never_writes_the_password_to_state() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let out = run_script(root, &["reset-password"]);
    assert!(
        out.status.success(),
        "reset-password failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    let email = banner_email(&stdout).expect("the banner must name the account");
    assert_eq!(email, DEFAULT_EMAIL);
    let password = banner_password(&stdout).expect("a fresh password must be printed once");

    // The printed password really is the credential.
    let db = root.join("registry.db");
    assert!(
        forge::identity::global::authenticate(&db, &email, &password)
            .unwrap()
            .is_some(),
        "the printed password must authenticate"
    );

    // The password is stdout-only: not in the run-state file.
    let state = root.join("run/state");
    if state.exists() {
        let state_text = std::fs::read_to_string(&state).unwrap();
        assert!(
            !state_text.contains(&password),
            "the password must never reach .forge/run/state"
        );
    }
}

#[test]
fn start_creates_then_recognizes_the_account_without_inventing_a_password() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let api_port = free_port();
    let web_port = free_port();
    let api = api_port.to_string();
    let web = web_port.to_string();

    // First start: no administrator, so a password is created and printed.
    let first = run_script(
        root,
        &[
            "start",
            "--api-port",
            &api,
            "--web-port",
            &web,
            "--admin-email",
            EMAIL,
        ],
    );
    let _ = run_script(root, &["stop"]);
    assert!(
        first.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert_eq!(banner_email(&first_stdout).as_deref(), Some(EMAIL));
    assert!(
        first_stdout.contains(&format!("http://127.0.0.1:{web}/")),
        "the login URL must be printed: {first_stdout}"
    );
    let password =
        banner_password(&first_stdout).expect("the first start must print the created password");
    let db = root.join("registry.db");
    assert!(forge::identity::global::authenticate(&db, EMAIL, &password)
        .unwrap()
        .is_some());

    // Second start: the account exists, so the script prints the email and
    // explicitly prints no password instead of inventing one.
    let second = run_script(
        root,
        &[
            "start",
            "--api-port",
            &api,
            "--web-port",
            &web,
            "--admin-email",
            EMAIL,
        ],
    );
    let _ = run_script(root, &["stop"]);
    assert!(
        second.status.success(),
        "second start failed: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert_eq!(banner_email(&second_stdout).as_deref(), Some(EMAIL));
    assert!(
        second_stdout.contains(&format!("http://127.0.0.1:{web}/")),
        "the login URL must be printed: {second_stdout}"
    );
    assert!(
        banner_password(&second_stdout).is_none(),
        "an existing account must not print or invent a password: {second_stdout}"
    );
    assert!(
        second_stdout.contains("reset-password"),
        "the operator must be told how to reset: {second_stdout}"
    );
    assert!(!second_stdout.contains(&password));
}

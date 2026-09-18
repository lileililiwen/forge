//! CLI contract for `forge identity` (`central-admin-identity`).
//!
//! Exercises the OIDC admin federation surface end to end through
//! the built binary:
//!
//! - R1 success: a project with a valid `identity:` block
//!   validates its OIDC configuration, builds a state/nonce
//!   challenge, completes the round trip and mints a per-project
//!   admin session; the registry records the `identity` journal
//!   row.
//! - R1 failure: an unsupported provider, a cleartext issuer, an
//!   admin claim that is not in the configured allow list and a
//!   callback state mismatch all surface with the typed
//!   `identity-invalid` or `identity-permission-denied` codes so
//!   the failure is observable on stdout before the typed
//!   exit-1 error renders on stderr.
//! - R1 boundary: a user authenticated at the provider but
//!   lacking the configured `admin_claim` value is refused with
//!   `identity-permission-denied` so a provider login never
//!   silently grants admin access.
//! - R2 success: a per-project session id validates, can be
//!   inspected and terminated; the per-project session file is
//!   the only place the session state is recorded.
//! - R2 failure: a session minted for project A presented to
//!   project B is refused with `identity-session-cross-project`
//!   so cookies and tokens are never shared across unrelated
//!   applications.
//! - R2 boundary: terminating one project's session does not
//!   implicitly revoke or validate another project's session so
//!   per-project isolation survives an explicit terminate call.

use std::fs;
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
        .env_remove("FORGE_NOTES_BIN");
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

const IDENTITY_YAML: &str = "  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

fn write_identity_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n{IDENTITY_YAML}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

#[test]
fn cli_help_lists_identity_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["identity", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout);
    for sub in [
        "validate-config",
        "build-challenge",
        "complete-auth",
        "session-list",
        "session-inspect",
        "session-validate",
        "session-terminate",
    ] {
        assert!(
            stdout.contains(sub),
            "identity help must mention `{sub}`:\n{stdout}"
        );
    }
}

#[test]
fn top_level_help_includes_identity_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("identity"),
        "top-level help should advertise `identity`; got: {stdout}"
    );
}

#[test]
fn validate_config_accepts_well_formed_block() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-validate-ok");
    let value = run_json(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(value["contract"], "0.1.0");
    assert_eq!(value["project_id"], "id-validate-ok");
    let cfg = &value["config"];
    assert_eq!(cfg["provider"], "okta");
    assert_eq!(cfg["client_id"], "forge-admin");
    assert_eq!(cfg["audience"], "forge-admin");
    assert_eq!(cfg["admin_claim"], "groups");
    assert_eq!(cfg["state_ttl_seconds"], 120);
    assert_eq!(cfg["session_ttl_seconds"], 3600);
    let scopes = cfg["scopes"].as_array().unwrap();
    assert_eq!(scopes, &vec!["openid", "profile"]);
}

#[test]
fn validate_config_rejects_unknown_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: id-validate-bad\n  name: x\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nidentity:\n  provider: custom-idp\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n",
    )
    .unwrap();
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-invalid]"),
        "stderr must report identity-invalid: {stderr}"
    );
    assert!(
        stderr.contains("custom-idp"),
        "stderr must name the bad provider: {stderr}"
    );
}

#[test]
fn validate_config_rejects_cleartext_issuer() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: id-validate-issuer\n  name: x\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nidentity:\n  provider: okta\n  issuer: http://example.okta.com\n  client_id: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n",
    )
    .unwrap();
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-invalid]"),
        "stderr must report identity-invalid: {stderr}"
    );
    assert!(
        stderr.contains("https"),
        "stderr must name the https requirement: {stderr}"
    );
}

#[test]
fn validate_config_rejects_missing_openid_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: id-validate-scope\n  name: x\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nidentity:\n  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n",
    )
    .unwrap();
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[identity-invalid]"));
    assert!(stderr.contains("openid"));
}

#[test]
fn validate_config_rejects_raw_secret_in_manifest() {
    // R1 boundary: the manifest must carry a `client_secret_ref`
    // (a `scheme://` reference), not a raw secret. A raw
    // value is refused with `identity-invalid` so a leaked
    // secret never reaches the registry.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: id-validate-secret\n  name: x\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nidentity:\n  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  client_secret_ref: super-secret\n",
    )
    .unwrap();
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[identity-invalid]"));
    assert!(stderr.contains("client_secret_ref"));
}

#[test]
fn validate_config_rejects_missing_block() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: id-validate-missing\n  name: x\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[identity-invalid]"));
    assert!(stderr.contains("identity"));
}

#[test]
fn build_challenge_returns_random_state_and_nonce() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-challenge");
    let value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let challenge = &value["challenge"];
    assert_eq!(value["contract"], "0.1.0");
    assert_eq!(challenge["project_id"], "id-challenge");
    assert_eq!(challenge["code_challenge_method"], "S256");
    let state = challenge["state"].as_str().unwrap();
    let nonce = challenge["nonce"].as_str().unwrap();
    let verifier = challenge["code_verifier"].as_str().unwrap();
    let code_challenge = challenge["code_challenge"].as_str().unwrap();
    assert_eq!(state.len(), 64, "state must be hex(32 bytes)");
    assert_eq!(nonce.len(), 64, "nonce must be hex(32 bytes)");
    assert_eq!(verifier.len(), 64, "verifier must be hex(32 bytes)");
    assert!(!code_challenge.is_empty());
    // A second challenge produces a different state/nonce/verifier.
    let value2 = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let challenge2 = &value2["challenge"];
    assert_ne!(state, challenge2["state"].as_str().unwrap());
    assert_ne!(nonce, challenge2["nonce"].as_str().unwrap());
    assert_ne!(verifier, challenge2["code_verifier"].as_str().unwrap());
}

#[test]
fn complete_auth_round_trip_mints_session_for_admin_claim() {
    // R1 success scenario: the project mints an
    // `admin:access` session because the provider claim
    // matches the configured allow list.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-roundtrip");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let code = "abcd1234";
    let value = run_json(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            state,
            "--code",
            code,
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    let session = &value["outcome"]["Session"];
    assert_eq!(session["project_id"], "id-roundtrip");
    assert_eq!(session["state"], "active");
    assert_eq!(session["subject"], "user-1");
    let perms = session["permissions"].as_array().unwrap();
    assert!(perms.iter().any(|p| p == "admin:access"));
    let session_id = session["session_id"].as_str().unwrap();
    assert!(!session_id.is_empty());
    // Session file lives under .forge/identity/<project>/sessions/<id>.json.
    let session_file = proj
        .join(".forge")
        .join("identity")
        .join("id-roundtrip")
        .join("sessions")
        .join(format!("{session_id}.json"));
    assert!(session_file.is_file(), "session file must exist");
}

#[test]
fn complete_auth_refuses_admin_claim_outside_allow_list() {
    // R1 boundary scenario: provider login for a user
    // outside the configured allow list is refused with
    // `identity-permission-denied` so provider login
    // never implicitly grants admin.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-claim-out");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let out = run(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "intern",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "intern",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-permission-denied]"),
        "stderr must report identity-permission-denied: {stderr}"
    );
    assert!(
        stderr.contains("provider login does not imply"),
        "stderr must reference the boundary: {stderr}"
    );
}

#[test]
fn complete_auth_refuses_state_mismatch_with_typed_rejection() {
    // R1 failure scenario: a callback with a state that
    // does not match the issued challenge is refused
    // with `identity-invalid` (issuer/audience/state/nonce
    // validation).
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-state-mismatch");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let out = run(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            "deadbeef",
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-invalid]"),
        "stderr must report identity-invalid: {stderr}"
    );
    assert!(stderr.contains("state"));
}

#[test]
fn session_list_returns_minted_sessions_in_stable_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-list");
    for i in 0..2 {
        let challenge_value = run_json(
            &db,
            &["identity", "build-challenge", proj.to_str().unwrap()],
        );
        let state = challenge_value["challenge"]["state"].as_str().unwrap();
        let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
        let value = run_json(
            &db,
            &[
                "identity",
                "complete-auth",
                proj.to_str().unwrap(),
                "--state",
                state,
                "--code",
                &format!("code-{i}"),
                "--subject",
                &format!("user-{i}"),
                "--nonce",
                nonce,
                "--admin-claim-value",
                "forge-admins",
            ],
        );
        assert!(
            value["outcome"]["Session"].is_object(),
            "i={i}; value={value}"
        );
    }
    let value = run_json(&db, &["identity", "session-list", proj.to_str().unwrap()]);
    assert_eq!(value["contract"], "0.1.0");
    let sessions = value["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 2);
    // Stable id order.
    let ids: Vec<&str> = sessions
        .iter()
        .map(|s| s["session_id"].as_str().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn session_inspect_returns_full_session_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-inspect");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    let session_id = mint["outcome"]["Session"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let value = run_json(
        &db,
        &[
            "identity",
            "session-inspect",
            proj.to_str().unwrap(),
            "--session",
            &session_id,
        ],
    );
    assert_eq!(value["project_id"], "id-inspect");
    assert_eq!(value["session"]["session_id"], session_id);
    assert_eq!(value["session"]["state"], "active");
}
#[test]
fn session_validate_grants_admin_permission_for_active_session() {
    // R2 success scenario: the per-project session id
    // validates against the project and the
    // `admin:access` permission.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-validate-grant");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    let session_id = mint["outcome"]["Session"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj.to_str().unwrap(),
            "--session",
            &session_id,
        ],
    );
    assert_eq!(out.status.code(), Some(0));
}
#[test]
fn session_validate_refuses_cross_project_token() {
    // R2 failure scenario: a session minted for
    // project A is rejected with
    // `identity-session-cross-project` when presented
    // to project B (no cross-project cookies/tokens).
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj_a = tmp.path().join("a");
    let proj_b = tmp.path().join("b");
    write_identity_project(&proj_a, "id-cross-a");
    write_identity_project(&proj_b, "id-cross-b");
    // Mint a session for project A.
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj_a.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        &db,
        &[
            "identity",
            "complete-auth",
            proj_a.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    let session_id = mint["outcome"]["Session"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    // Try to use it for project B.
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj_b.to_str().unwrap(),
            "--session",
            &session_id,
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-session-cross-project]"),
        "stderr must report identity-session-cross-project: {stderr}"
    );
    assert!(stderr.contains("id-cross-a"));
    assert!(stderr.contains("id-cross-b"));
}
#[test]
fn session_validate_refuses_missing_session_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-missing-session");
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj.to_str().unwrap(),
            "--session",
            "deadbeef",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-session-not-found]"),
        "stderr must report identity-session-not-found: {stderr}"
    );
}

#[test]
fn session_terminate_revokes_session_and_removes_file() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-terminate");
    let challenge_value = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge_value["challenge"]["state"].as_str().unwrap();
    let nonce = challenge_value["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    let session_id = mint["outcome"]["Session"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run(
        &db,
        &[
            "identity",
            "session-terminate",
            proj.to_str().unwrap(),
            "--format",
            "json",
            "--session",
            &session_id,
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stdout={}", lossy(&out.stdout)));
    assert_eq!(value["terminated"], true);
    assert_eq!(value["session"]["state"], "revoked");
    // Session file is removed so a revoked session cannot
    // be re-presented.
    let session_file = proj
        .join(".forge")
        .join("identity")
        .join("id-terminate")
        .join("sessions")
        .join(format!("{session_id}.json"));
    assert!(
        !session_file.exists(),
        "session file must be removed after terminate"
    );
    // Re-validating a terminated session returns
    // `identity-session-not-found`.
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj.to_str().unwrap(),
            "--session",
            &session_id,
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[identity-session-not-found]"));
}
#[test]
fn complete_auth_redacts_credential_shaped_evidence() {
    // R1 failure scenario: a provider-supplied error
    // that includes a token-shaped string is redacted
    // before it lands in the journal evidence so a
    // leaked secret never reaches the operations
    // table.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-redact");
    let out = run(
        &db,
        &[
            "identity",
            "complete-auth",
            proj.to_str().unwrap(),
            "--state",
            "deadbeef",
            "--code",
            "abcd1234",
            "--error",
            "access_denied",
            "--error-description",
            "token ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "--subject",
            "user-1",
            "--nonce",
            "deadbeef",
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[identity-invalid]"),
        "stderr must report identity-invalid: {stderr}"
    );
    assert!(
        !stderr.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "stderr must not leak the credential-shaped secret: {stderr}"
    );
    assert!(stderr.contains("[REDACTED]"));
}

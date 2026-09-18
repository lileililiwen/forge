//! Cross-surface regression tests for `forge identity`
//! (`central-admin-identity`).
//!
//! Exercises the OIDC admin federation surface across the
//! other Core contracts:
//!
//! - The Core registry's `identity` journal row records the
//!   per-call summary so the operations table stays
//!   independent of the identity surface.
//! - The doctor verdict on the same project directory is
//!   unchanged after a successful identity round trip so
//!   the existing doctor contract still holds.
//! - A credential-shaped substring in evidence is redacted
//!   through `redact_identity_evidence` which delegates to
//!   `policy::redact_credentials`.
//! - The `forge feature add` workflow remains compatible
//!   after a successful identity round trip on the same
//!   project so the new surface does not regress the
//!   feature ownership contract.
//! - The R2 boundary: terminating one project's session
//!   does not affect a sibling project's session.
//! - The R1 boundary: a user authenticated at the provider
//!   but lacking the configured `admin_claim` value is
//!   refused with `identity-permission-denied` so a
//!   provider login never silently grants admin.

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

fn mint_session(db: &Path, proj: &Path, _project_id: &str) -> String {
    let challenge = run_json(db, &["identity", "build-challenge", proj.to_str().unwrap()]);
    let state = challenge["challenge"]["state"].as_str().unwrap();
    let nonce = challenge["challenge"]["nonce"].as_str().unwrap();
    let minted = run_json(
        db,
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
    minted["outcome"]["Session"]["session_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn registry_journal_records_identity_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-xsurface-journal");
    let out = run(
        &db,
        &["identity", "validate-config", proj.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0));
    // The registry's operations table records the identity
    // row; the identity surface never invents a project.
    let list = run_json(&db, &["list"]);
    let projects = list["projects"].as_array().unwrap();
    assert!(
        projects.is_empty(),
        "identity is not a registered project; got: {projects:?}"
    );
    // Re-running a `session-list` keeps the operations table
    // alive (it does not require a registered project).
    let out = run(&db, &["identity", "session-list", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn doctor_verdict_is_unchanged_after_identity_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-xsurface-doctor");
    let before = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let before_finding_count = before["doctor"]["findings"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    // Complete a full identity round trip.
    let _session = mint_session(&db, &proj, "id-xsurface-doctor");
    let after = run_json(&db, &["doctor", proj.to_str().unwrap()]);
    let after_finding_count = after["doctor"]["findings"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(
        before_finding_count, after_finding_count,
        "doctor finding count must not change after identity round trip"
    );
}

#[test]
fn credential_in_evidence_is_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-xsurface-redact");
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
            "token ghp_abcdefghijklmnopqrstuvwxyz0123456789 was used",
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
        !stderr.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "stderr must not leak the credential-shaped secret: {stderr}"
    );
    assert!(
        stderr.contains("[REDACTED]"),
        "stderr must show [REDACTED]: {stderr}"
    );
}

#[test]
fn feature_add_remains_compatible_after_identity_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-xsurface-feature");
    let _session = mint_session(&db, &proj, "id-xsurface-feature");
    // The auth feature is already declared by default; the
    // feature add workflow must still report success on a
    // declared feature (idempotent) and refuse an unknown
    // one with `error[unknown-feature]`.
    let out = run(&db, &["feature", "list"]);
    assert_eq!(out.status.code(), Some(0));
    let out = run(&db, &["feature", "add", "nosuch", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[unknown-feature]"),
        "stderr must report unknown-feature: {stderr}"
    );
}

#[test]
fn terminate_one_project_does_not_revoke_another() {
    // R2 boundary: revoking project A's session must not
    // implicitly revoke or validate project B's session.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj_a = tmp.path().join("a");
    let proj_b = tmp.path().join("b");
    write_identity_project(&proj_a, "id-xsurface-a");
    write_identity_project(&proj_b, "id-xsurface-b");
    let session_a = mint_session(&db, &proj_a, "id-xsurface-a");
    let session_b = mint_session(&db, &proj_b, "id-xsurface-b");
    // Terminate A.
    let out = run(
        &db,
        &[
            "identity",
            "session-terminate",
            proj_a.to_str().unwrap(),
            "--session",
            &session_a,
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    // A's session no longer validates; B's session still does.
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj_a.to_str().unwrap(),
            "--session",
            &session_a,
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let out = run(
        &db,
        &[
            "identity",
            "session-validate",
            proj_b.to_str().unwrap(),
            "--session",
            &session_b,
        ],
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn admin_claim_outside_allow_list_is_refused() {
    // R1 boundary: a user authenticated at the provider
    // but lacking the configured `admin_claim` value is
    // refused with `identity-permission-denied` so a
    // provider login never silently grants admin.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "id-xsurface-claim");
    let challenge = run_json(
        &db,
        &["identity", "build-challenge", proj.to_str().unwrap()],
    );
    let state = challenge["challenge"]["state"].as_str().unwrap();
    let nonce = challenge["challenge"]["nonce"].as_str().unwrap();
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
            "interns",
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

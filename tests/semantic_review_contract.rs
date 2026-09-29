//! Semantic project description and classification review
//! (`project-semantic-description-review`).
//!
//! Exercises the versioned `forge-semantic-proposal/0.1.0` surface
//! through the built binary: every requirement named in
//! `openspec/changes/project-semantic-description-review/specs/`
//! has at least one named test here. The contract answers the
//! questions the capability names:
//!
//! - A suggestion carries its evidence sources, revisions,
//!   confidence label and provider identity; a generated value is
//!   never the project's declared value.
//! - An operator decision (`approve` / `reject`) requires an
//!   explicit `--confirm`; a stale or already-decided proposal
//!   refuses.
//! - Conflicting evidence is recorded as a `Conflicted` proposal
//!   with each source retained; no approval is offered on a
//!   conflict.
//! - An unavailable provider short-circuits to a typed
//!   `unavailable` state and prior proposals stay untouched.
//! - The suggest path is read-only against the project, the
//!   provider and the registry: nothing is auto-applied.
//!
//! Every case runs through the built `forge` binary. No live
//! network call is made; the closed `Operator`/`Local` provider
//! set is the only one the surface accepts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-semantic-proposal/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN");
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_rust_l1(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let body = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), body).unwrap();
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> (Value, Option<i32>) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let code = out.status.code();
    let value: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stderr={}", lossy(&out.stderr)));
    (value, code)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path
            .strip_prefix(root)
            .unwrap()
            .starts_with(".forge/semantic")
        {
            out.push(path);
        }
    }
}

fn semantic_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

/// Reconstruct `<kind>-<hash>` from a JSON `proposal.id` object.
fn dir_from_id(value: &Value) -> String {
    format!(
        "{}-{}",
        value["kind"].as_str().expect("kind"),
        value["hash"].as_str().expect("hash")
    )
}

#[test]
fn help_advertises_describe_and_classify_with_their_five_subcommands() {
    for verb in ["describe", "classify"] {
        let out = Command::new(forge_bin())
            .args([verb, "--help"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}: {}", verb, lossy(&out.stderr));
        let text = lossy(&out.stdout);
        for sub in ["suggest", "list", "show", "approve", "reject"] {
            assert!(text.contains(sub), "{verb}: missing `{sub}`: {text}");
        }
    }
}

#[test]
fn describe_suggest_writes_a_traceable_proposal_with_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_rust_l1(&proj, "demo");

    let (value, code) = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "Traceable catalog of demo projects",
            "--current-value",
            "(missing)",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--confidence",
            "high",
            "--provider",
            "operator",
            "--note",
            "captured from the README",
        ],
    );
    assert_eq!(code, Some(0));
    assert_eq!(value["status"], "generated");
    let proposal = &value["proposal"];
    assert_eq!(proposal["contract"], CONTRACT);
    assert_eq!(proposal["state"], "suggested");
    assert_eq!(proposal["project_id"], "demo");
    assert_eq!(proposal["kind"], "description");
    assert_eq!(proposal["confidence"], "high");
    assert_eq!(proposal["provider"], "operator");
    assert_eq!(
        proposal["suggested_value"],
        "Traceable catalog of demo projects"
    );
    let evidence = proposal["evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0]["path"], "README.md");
    assert_eq!(evidence[0]["revision"], "rev-1");
    let files = value["files_written"].as_array().unwrap();
    let file_strs: Vec<&str> = files.iter().map(|f| f.as_str().unwrap()).collect();
    let dir = dir_from_id(&proposal["id"]);
    let expected_manifest = format!(".forge/semantic/demo/{dir}/manifest.json");
    let expected_proposal = format!(".forge/semantic/demo/{dir}/proposal.md");
    assert!(file_strs.contains(&expected_manifest.as_str()));
    assert!(file_strs.contains(&expected_proposal.as_str()));
    let manifest_text = fs::read_to_string(proj.join(&expected_manifest)).unwrap();
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["project_id"], "demo");
}

#[test]
fn describe_suggest_does_not_apply_value_to_project_or_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("apply-app");
    write_rust_l1(&proj, "apply-app");
    let before = fs::read_to_string(proj.join("forge.yaml")).unwrap();

    let _ = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "A demo Rust project",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--provider",
            "operator",
        ],
    );

    // Project file is byte-identical.
    assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), before);
    // No registry file is created by a read-only path.
    assert!(!db.exists());
}

#[test]
fn describe_suggest_is_idempotent_on_same_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("idem-app");
    write_rust_l1(&proj, "idem-app");

    let first = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(first.0["status"], "generated");
    let dir = dir_from_id(&first.0["proposal"]["id"]);
    let manifest_path = proj
        .join(".forge/semantic/idem-app")
        .join(&dir)
        .join("manifest.json");
    let mtime = fs::metadata(&manifest_path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let second = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(second.0["status"], "existing");
    let second_files = second.0["files_written"].as_array().unwrap();
    assert!(
        second_files.is_empty(),
        "no files should be rewritten on a re-run"
    );
    let mtime2 = fs::metadata(&manifest_path).unwrap().modified().unwrap();
    assert_eq!(
        mtime, mtime2,
        "manifest mtime must be unchanged on a re-run"
    );
}

#[test]
fn describe_suggest_credential_shaped_value_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("cred-app");
    write_rust_l1(&proj, "cred-app");
    let before = semantic_files(&proj);
    let out = run(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    assert_eq!(semantic_files(&proj), before, "no proposal must be written");
}

#[test]
fn describe_suggest_unknown_provider_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("prov-app");
    write_rust_l1(&proj, "prov-app");
    let out = run(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--provider",
            "gpt-4",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    assert!(stderr.contains("unknown semantic provider"), "{stderr}");
}

#[test]
fn describe_suggest_conflicting_value_for_same_revision_records_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("conf-app");
    write_rust_l1(&proj, "conf-app");

    let first = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "first value",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(first.0["status"], "generated");

    let second = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "second value",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(second.0["status"], "conflicted");
    let proposal = &second.0["proposal"];
    assert_eq!(proposal["state"], "conflicted");
    let conflict = &proposal["conflict"];
    assert_eq!(conflict["first_value"], "first value");
    assert_eq!(conflict["second_value"], "second value");
    assert!(conflict["first_evidence"]["path"].is_string());
    assert!(conflict["second_evidence"]["path"].is_string());
}

#[test]
fn describe_suggest_new_revision_supersedes_prior_open_proposal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("sup-app");
    write_rust_l1(&proj, "sup-app");

    let first = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "first",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(first.0["status"], "generated");
    let first_id = dir_from_id(&first.0["proposal"]["id"]);

    let second = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "second",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-2",
        ],
    );
    assert_eq!(second.0["status"], "superseded");
    let first_manifest = proj
        .join(".forge/semantic/sup-app")
        .join(&first_id)
        .join("manifest.json");
    let first_text = fs::read_to_string(&first_manifest).unwrap();
    let first_value: Value = serde_json::from_str(&first_text).unwrap();
    assert_eq!(first_value["state"], "superseded");
}

#[test]
fn describe_list_and_show_round_trip_a_proposal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("round-app");
    write_rust_l1(&proj, "round-app");
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "Round-trip proposal",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let dir = dir_from_id(&suggest.0["proposal"]["id"]);
    let list = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    let entries = list.0["proposals"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["dir_name"], dir);
    assert_eq!(entries[0]["state"], "suggested");
    let show = run_json(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    assert_eq!(show.0["proposal"]["state"], "suggested");
    assert_eq!(show.0["proposal"]["suggested_value"], "Round-trip proposal");
}

#[test]
fn describe_approve_requires_explicit_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("appr-app");
    write_rust_l1(&proj, "appr-app");
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let dir = dir_from_id(&suggest.0["proposal"]["id"]);

    let no_confirm = run(&db, &["describe", "approve", &dir, proj.to_str().unwrap()]);
    assert_eq!(no_confirm.status.code(), Some(1));
    let stderr = lossy(&no_confirm.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");

    let confirmed = run_json(
        &db,
        &[
            "describe",
            "approve",
            &dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(confirmed.0["state"], "approved");
    let manifest_path = proj
        .join(".forge/semantic/appr-app")
        .join(&dir)
        .join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["state"], "approved");
    assert!(manifest["decided_at"].is_string());
}

#[test]
fn describe_approve_stale_proposal_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("stale-app");
    write_rust_l1(&proj, "stale-app");
    let first = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "first",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let first_id = dir_from_id(&first.0["proposal"]["id"]);
    let _ = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "second",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-2",
        ],
    );
    let out = run(
        &db,
        &[
            "describe",
            "approve",
            &first_id,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-conflict]"), "{stderr}");
}

#[test]
fn describe_approve_conflicted_proposal_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("conf-app");
    write_rust_l1(&proj, "conf-app");
    let _ = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "first",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let conflicted = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "second",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let dir = dir_from_id(&conflicted.0["proposal"]["id"]);
    let out = run(
        &db,
        &[
            "describe",
            "approve",
            &dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-conflict]"), "{stderr}");
}

#[test]
fn describe_reject_moves_to_rejected_without_applying() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("rej-app");
    write_rust_l1(&proj, "rej-app");
    let before = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "rejected value",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    let dir = dir_from_id(&suggest.0["proposal"]["id"]);
    let outcome = run_json(
        &db,
        &[
            "describe",
            "reject",
            &dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(outcome.0["state"], "rejected");
    assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), before);
}

#[test]
fn classify_suggest_persists_a_classification_proposal() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("classify-app");
    write_rust_l1(&proj, "classify-app");
    let (value, code) = run_json(
        &db,
        &[
            "classify",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "tools",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--provider",
            "operator",
        ],
    );
    assert_eq!(code, Some(0));
    assert_eq!(value["status"], "generated");
    let proposal = &value["proposal"];
    assert_eq!(proposal["kind"], "domain");
    assert_eq!(proposal["state"], "suggested");
    assert_eq!(proposal["provider"], "operator");
}

#[test]
fn describe_operations_do_not_create_a_registry_database() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("nodb-app");
    write_rust_l1(&proj, "nodb-app");
    let _ = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert!(!db.exists(), "suggest must be a read-only path");
    let _ = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    assert!(!db.exists(), "list must be a read-only path");
    // `show` for an absent proposal is a typed refusal; the command
    // exits 1 and writes the error to stderr, so it is run with `run`
    // and checked for a typed error rather than parsed as JSON.
    let show_absent = run(
        &db,
        &[
            "describe",
            "show",
            "description-deadbeefdead",
            proj.to_str().unwrap(),
        ],
    );
    assert_eq!(show_absent.status.code(), Some(1));
    let stderr = lossy(&show_absent.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    assert!(!db.exists(), "show must be a read-only path");
}

#[test]
fn describe_suggest_refuses_when_no_evidence_path_is_supplied() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("no-ev-app");
    write_rust_l1(&proj, "no-ev-app");
    let out = run(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    assert!(stderr.contains("at least one --evidence-path"), "{stderr}");
}

#[test]
fn describe_suggest_refuses_when_evidence_revisions_count_mismatches() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("rev-mismatch-app");
    write_rust_l1(&proj, "rev-mismatch-app");
    let out = run(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--evidence-revision",
            "rev-2",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
}

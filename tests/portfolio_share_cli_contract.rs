//! Portfolio share CLI contract (`portfolio-share-publish`).
//!
//! Drives `forge portfolio share …` end to end against a local registry
//! and a local file target. It covers the verification oracle in the
//! change design:
//!
//! - **Allowlist only**: a registered project with no share record is
//!   absent from the manifest, and a `/admin/settings` surface is
//!   refused with the reason persisted.
//! - **Exact approval**: an edit after approval changes the manifest
//!   hash, so the stale approval no longer publishes.
//! - **Determinism and idempotency**: the same records produce the
//!   same canonical bytes and SHA-256, and a retried operation key
//!   never creates a second publication.
//! - **No private data in the public artifact**: a credential-shaped
//!   value is refused and never echoed.
//!
//! Everything runs against a local registry and a local file target.
//! No GitHub, Cloudflare or other external host is contacted, and no
//! remote or multi-user readiness is claimed.

#[path = "support/share.rs"]
mod support;

use std::fs;

use serde_json::Value;
use support::{
    clean_cmd, expect_refusal, fleet, lossy, registered, run, run_json, share_set, share_set_args,
    shell_stub,
};

#[test]
fn share_help_advertises_the_whole_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _dir) = registered(tmp.path(), "alethefy");
    let help = lossy(&run(&db, &["portfolio", "share", "--help"]).stdout);
    for sub in [
        "set",
        "remove",
        "show",
        "list",
        "preview",
        "approve",
        "publish",
        "reconcile",
        "audit",
    ] {
        assert!(
            help.contains(sub),
            "`{sub}` missing from share --help:\n{help}"
        );
    }
    let publish = lossy(&run(&db, &["portfolio", "share", "publish", "--help"]).stdout);
    for flag in ["--target", "--operation-key", "--actor", "--adapter"] {
        assert!(
            publish.contains(flag),
            "`{flag}` missing from publish --help:\n{publish}"
        );
    }
    let set = lossy(&run(&db, &["portfolio", "share", "set", "--help"]).stdout);
    for flag in [
        "--title",
        "--summary",
        "--category",
        "--source-url",
        "--demo-url",
        "--visibility",
        "--featured",
        "--status",
        "--surface",
    ] {
        assert!(
            set.contains(flag),
            "`{flag}` missing from set --help:\n{set}"
        );
    }
}

#[test]
fn a_private_project_is_never_listed() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    assert_eq!(preview["preview"]["project_count"], 1);
    let body = preview["preview"]["canonical_body"].as_str().unwrap();
    assert!(body.contains("alethefy"));
    assert!(!body.contains("\"forge\""));
    let listed = run_json(&db, &["portfolio", "share", "list"]);
    assert_eq!(listed["records"].as_array().unwrap().len(), 1);
}

#[test]
fn an_admin_surface_is_rejected_and_the_reason_is_persisted() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    let mut args = share_set_args("alethefy", "Alethefy");
    let surface = args.len() - 1;
    args[surface] = "Admin=https://example.com/alethefy/admin/settings".to_string();
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    expect_refusal(&db, &borrowed, "portfolio-share-invalid");
    let shown = run_json(&db, &["portfolio", "share", "show", "alethefy"]);
    assert_eq!(shown["shared"], false);
    let findings = shown["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["code"], "share-write-refused");
    assert!(findings[0]["detail"]
        .as_str()
        .unwrap()
        .contains("private or administrative"));
    // The refused path itself never reaches the audit trail.
    assert!(!findings[0]["detail"].as_str().unwrap().contains("/admin"));
}

#[test]
fn a_credential_shaped_value_is_rejected_without_being_echoed() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    let mut args = share_set_args("alethefy", "Alethefy");
    let summary = args
        .iter()
        .position(|a| a == "--summary")
        .expect("summary flag");
    args[summary + 1] = "Deploy with password=hunter2000 before review.".to_string();
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run(&db, &borrowed);
    assert!(!out.status.success());
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[portfolio-share-invalid]"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("hunter2000"),
        "the credential must never be echoed: {stderr}"
    );
    assert!(stderr.contains("no share state was changed"), "{stderr}");
    let shown = run_json(&db, &["portfolio", "share", "show", "alethefy"]);
    assert_eq!(shown["shared"], false);
    assert!(
        !run_json(&db, &["portfolio", "share", "preview"])["preview"]["canonical_body"]
            .as_str()
            .unwrap()
            .contains("hunter2000")
    );
}

#[test]
fn a_non_https_or_internal_url_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    for url in [
        "http://example.com/alethefy",
        "https://localhost/alethefy",
        "https://forge.internal/alethefy",
        "https://10.0.0.4/alethefy",
        "https://alethefy.example.com/x?token=abcdef123456",
    ] {
        let mut args = share_set_args("alethefy", "Alethefy");
        let at = args.iter().position(|a| a == "--source-url").unwrap();
        args[at + 1] = url.to_string();
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        expect_refusal(&db, &borrowed, "portfolio-share-invalid");
    }
    assert_eq!(
        run_json(&db, &["portfolio", "share", "list"])["records"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn an_out_of_vocabulary_status_is_a_typed_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    let mut args = share_set_args("alethefy", "Alethefy");
    let at = args.iter().position(|a| a == "--status").unwrap();
    args[at + 1] = "online".to_string();
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    expect_refusal(&db, &borrowed, "portfolio-share-invalid");
}

#[test]
fn the_manifest_is_deterministic_across_record_order() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    share_set(&db, "forge", "Forge");
    let first = run_json(&db, &["portfolio", "share", "preview"]);
    let body_first = first["preview"]["canonical_body"]
        .as_str()
        .unwrap()
        .to_string();
    let hash_first = first["preview"]["manifest_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    // A fresh registry built in the opposite order must produce the
    // same canonical bytes: projects sort by stable id, surfaces by
    // (label, url).
    let other = tempfile::tempdir().unwrap();
    let (db2, _token2) = fleet(other.path());
    share_set(&db2, "forge", "Forge");
    share_set(&db2, "alethefy", "Alethefy");
    let second = run_json(&db2, &["portfolio", "share", "preview"]);
    assert_eq!(
        second["preview"]["canonical_body"].as_str().unwrap(),
        body_first
    );
    assert_eq!(
        second["preview"]["manifest_sha256"].as_str().unwrap(),
        hash_first
    );
    assert_eq!(hash_first.len(), 64);
    assert!(hash_first.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn publication_requires_the_exact_approval() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    // Nothing is approved yet.
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            "/tmp/never-written.json",
            "--operation-key",
            "op-early",
        ],
        "portfolio-share-conflict",
    );
    // A stale hash is refused and records no approval.
    expect_refusal(
        &db,
        &["portfolio", "share", "approve", "--hash", &"0".repeat(64)],
        "portfolio-share-conflict",
    );
    assert!(run_json(&db, &["portfolio", "share", "audit"])["approvals"]
        .as_array()
        .unwrap()
        .is_empty());

    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    let hash = preview["preview"]["manifest_sha256"].as_str().unwrap();
    let approved = run_json(&db, &["portfolio", "share", "approve", "--hash", hash]);
    assert_eq!(approved["approval"]["revision"], 1);
    assert_eq!(approved["approval"]["state"], "approved");
    assert_eq!(approved["approval"]["actor"], "local-admin");

    // An edit after approval changes the hash, so the stale approval
    // no longer publishes.
    share_set(&db, "alethefy", "Alethefy renamed");
    let target = tmp.path().join("catalog.json");
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-stale",
        ],
        "portfolio-share-conflict",
    );
    assert!(!target.exists(), "a refused publication writes nothing");
    assert!(
        run_json(&db, &["portfolio", "share", "audit"])["publications"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// --- idempotency and reconciliation --------------------------------------

#[test]
fn publishing_writes_only_public_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    let hash = preview["preview"]["manifest_sha256"].as_str().unwrap();
    run_json(&db, &["portfolio", "share", "approve", "--hash", hash]);
    let target = tmp.path().join("public/portfolio-manifest.json");
    let report = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-1",
        ],
    );
    assert_eq!(report["publication"]["status"], "published");
    assert_eq!(report["publication"]["project_count"], 1);
    assert_eq!(report["publication"]["manifest_sha256"], hash);

    let document: Value =
        serde_json::from_str(&fs::read_to_string(&target).unwrap()).expect("manifest document");
    assert_eq!(document["schema_family"], "public-portfolio-manifest");
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["manifest_sha256"], hash);
    assert_eq!(document["projects"][0]["id"], "alethefy");
    assert_eq!(document["projects"][0]["title"], "Alethefy");
    assert_eq!(document["projects"][0]["showcase_status"], "demo");
    // No private field survives into the public artifact.
    let serialized = document.to_string();
    for forbidden in [
        "/tmp",
        "next_action",
        "blocker",
        "confidence",
        "local_path",
        "password",
        "evidence_snapshots",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "`{forbidden}` leaked into the public manifest: {serialized}"
        );
    }
    let record = run_json(&db, &["portfolio", "share", "show", "alethefy"]);
    assert_eq!(record["share"]["state"], "published");
}

#[test]
fn an_empty_catalog_is_valid_and_publishable() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    assert_eq!(preview["preview"]["project_count"], 0);
    assert_eq!(preview["preview"]["approvable"], true);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    let target = tmp.path().join("empty.json");
    let report = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-empty",
        ],
    );
    assert_eq!(report["publication"]["status"], "published");
    let document: Value =
        serde_json::from_str(&fs::read_to_string(&target).unwrap()).expect("document");
    assert_eq!(document["projects"].as_array().unwrap().len(), 0);
}

#[test]
fn retrying_an_operation_key_never_creates_a_second_publication() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    let target = tmp.path().join("catalog.json");
    let publish = [
        "portfolio".to_string(),
        "share".to_string(),
        "publish".to_string(),
        "--target".to_string(),
        target.to_str().unwrap().to_string(),
        "--operation-key".to_string(),
        "op-retry".to_string(),
    ];
    let borrowed: Vec<&str> = publish.iter().map(String::as_str).collect();
    let first = run_json(&db, &borrowed);
    let bytes_after_first = fs::read(&target).unwrap();
    let second = run_json(&db, &borrowed);
    assert_eq!(
        first["publication"]["publication_id"],
        second["publication"]["publication_id"]
    );
    assert_eq!(second["publication"]["status"], "published");
    let audit = run_json(&db, &["portfolio", "share", "audit"]);
    assert_eq!(audit["publications"].as_array().unwrap().len(), 1);
    assert_eq!(audit["unreconciled"], Value::Null);
    // The artifact is byte-identical: a retry reconciled rather than
    // republishing.
    assert_eq!(fs::read(&target).unwrap(), bytes_after_first);
}

#[test]
fn one_operation_key_refuses_a_different_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("a.json").to_str().unwrap(),
            "--operation-key",
            "op-shared",
        ],
    );
    // A different manifest under the same key is a conflict, never a
    // silent re-interpretation of the key.
    share_set(&db, "forge", "Forge");
    let second = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            second["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("b.json").to_str().unwrap(),
            "--operation-key",
            "op-shared",
        ],
        "portfolio-share-conflict",
    );
    assert!(!tmp.path().join("b.json").exists());
}

#[test]
#[cfg(unix)]
fn a_failing_adapter_does_not_create_a_false_success_and_retry_reconciles() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );

    // An adapter that writes the manifest and then hangs is the
    // partial-publication case: Forge times out, records the attempt
    // as failed, and keeps the approval.
    let adapter = tmp.path().join("partial-adapter.sh");
    shell_stub(
        &adapter,
        "#!/bin/sh\ncat > /dev/null\nwhile true; do sleep 1; done\n",
    );
    let target = tmp.path().join("catalog.json");
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .args([
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-partial",
            "--adapter",
            adapter.to_str().unwrap(),
        ])
        .env("FORGE_PORTFOLIO_SHARE_TIMEOUT_SECS", "1");
    let out = cmd.output().expect("run forge");
    assert!(
        !out.status.success(),
        "a timed-out adapter is not a success"
    );
    assert!(lossy(&out.stderr).contains("error[portfolio-share-invalid]"));
    let audit = run_json(&db, &["portfolio", "share", "audit"]);
    let attempts = audit["publications"].as_array().unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["status"], "failed");
    assert_eq!(attempts[0]["error_code"], "portfolio-share-invalid");
    // The approval is retained, so the same operation key retries.
    assert_eq!(audit["approvals"][0]["state"], "approved");

    let retry = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-partial",
        ],
    );
    assert_eq!(retry["publication"]["status"], "published");
    assert_eq!(
        retry["publication"]["publication_id"],
        attempts[0]["publication_id"]
    );
    assert_eq!(
        run_json(&db, &["portfolio", "share", "audit"])["publications"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(target.exists());
}

#[test]
fn a_manifest_that_cannot_be_approved_reports_its_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    // Plant a record that an older build could have written: the
    // second gate catches a private surface at preview time.
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO portfolio_share_records
                    (project_id, title, summary, category, source_url, visibility,
                     featured, showcase_status, state, revision, created_at, updated_at)
                 VALUES ('forge', 'Forge', 's', 'platform', 'https://example.com/forge',
                         'public', 0, 'demo', 'validated', 1, '2026-09-29T00:00:00Z',
                         '2026-09-29T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
                "INSERT INTO portfolio_share_surfaces (project_id, label, url, created_at)
                 VALUES ('forge', 'Admin', 'https://example.com/forge/admin', '2026-09-29T00:00:00Z')",
                [],
            )
            .unwrap();
    }
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    assert_eq!(preview["preview"]["project_count"], 0);
    assert_eq!(preview["preview"]["approvable"], false);
    let findings = preview["preview"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["project_id"], "forge");
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
        "portfolio-share-conflict",
    );
}

#[test]
fn withdrawing_a_record_removes_it_from_the_catalog() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    share_set(&db, "forge", "Forge");
    assert_eq!(
        run_json(&db, &["portfolio", "share", "preview"])["preview"]["project_count"],
        2
    );
    let removed = run_json(&db, &["portfolio", "share", "remove", "forge"]);
    assert_eq!(removed["removed"], true);
    assert_eq!(
        run_json(&db, &["portfolio", "share", "preview"])["preview"]["project_count"],
        1
    );
    let again = run_json(&db, &["portfolio", "share", "remove", "forge"]);
    assert_eq!(again["removed"], false);
}

#[test]
fn the_audit_trail_explains_what_was_public_when_by_whom() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    let hash = preview["preview"]["manifest_sha256"].as_str().unwrap();
    let approved = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            hash,
            "--actor",
            "release-bot",
        ],
    );
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("catalog.json").to_str().unwrap(),
            "--operation-key",
            "op-audit",
            "--actor",
            "release-bot",
        ],
    );
    let audit = run_json(&db, &["portfolio", "share", "audit"]);
    let approval = &audit["approvals"][0];
    assert_eq!(approval["manifest_sha256"], hash);
    assert_eq!(approval["actor"], "release-bot");
    assert_eq!(approval["state"], "published");
    assert_eq!(approval["revision"], approved["approval"]["revision"]);
    let publication = &audit["publications"][0];
    assert_eq!(publication["manifest_sha256"], hash);
    assert_eq!(publication["operation_key"], "op-audit");
    assert_eq!(publication["actor"], "release-bot");
    assert_eq!(publication["status"], "published");
    assert_eq!(
        publication["manifest_revision"],
        approved["approval"]["revision"]
    );
    assert!(!publication["attempted_at"].as_str().unwrap().is_empty());
    assert!(!publication["finished_at"].as_str().unwrap().is_empty());
}

#[test]
#[cfg(unix)]
fn an_unreconciled_publication_blocks_the_next_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    // An adapter that reports an indeterminate outcome.
    let adapter = tmp.path().join("unknown-adapter.sh");
    shell_stub(
        &adapter,
        "#!/bin/sh\ncat > /dev/null\nprintf '%s' '{\"contract\":\"forge-portfolio-share-adapter/0.1.0\",\"status\":\"unknown\"}'\n",
    );
    let report = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("catalog.json").to_str().unwrap(),
            "--operation-key",
            "op-unknown",
            "--adapter",
            adapter.to_str().unwrap(),
        ],
    );
    assert_eq!(report["publication"]["status"], "unknown");
    assert_eq!(
        run_json(&db, &["portfolio", "share", "audit"])["unreconciled"]["operation_key"],
        "op-unknown"
    );

    // A new revision cannot be published until it is reconciled.
    share_set(&db, "forge", "Forge");
    let second = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            second["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("second.json").to_str().unwrap(),
            "--operation-key",
            "op-next",
        ],
        "portfolio-share-conflict",
    );

    let reconciled = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "reconcile",
            "--publication",
            report["publication"]["publication_id"].to_string().as_str(),
            "--result",
            "failed",
            "--actor",
            "ops-admin",
        ],
    );
    assert_eq!(reconciled["publication"]["status"], "failed");
    assert_eq!(reconciled["publication"]["actor"], "ops-admin");
    assert_eq!(
        run_json(&db, &["portfolio", "share", "audit"])["unreconciled"],
        Value::Null
    );
    let resumed = run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("second.json").to_str().unwrap(),
            "--operation-key",
            "op-next",
        ],
    );
    assert_eq!(resumed["publication"]["status"], "published");
}

#[test]
#[cfg(unix)]
fn an_adapter_answering_the_wrong_contract_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    let adapter = tmp.path().join("wrong-contract.sh");
    shell_stub(
        &adapter,
        "#!/bin/sh\ncat > /dev/null\nprintf '%s' '{\"contract\":\"something-else/9.9.9\",\"status\":\"published\"}'\n",
    );
    expect_refusal(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("catalog.json").to_str().unwrap(),
            "--operation-key",
            "op-wrong",
            "--adapter",
            adapter.to_str().unwrap(),
        ],
        "portfolio-share-invalid",
    );
    let audit = run_json(&db, &["portfolio", "share", "audit"]);
    assert_eq!(audit["publications"][0]["status"], "failed");
    // A failed adapter never leaves a partial artifact behind.
    assert!(!tmp.path().join("catalog.json").exists());
}

#[test]
#[cfg(unix)]
fn an_adapter_never_receives_a_credential_or_a_registry_query() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "approve",
            "--hash",
            preview["preview"]["manifest_sha256"].as_str().unwrap(),
        ],
    );
    let captured = tmp.path().join("captured.json");
    let adapter = tmp.path().join("capture-adapter.sh");
    shell_stub(
        &adapter,
        &format!(
            "#!/bin/sh\ncat > {}\nprintf '%s' '{{\"contract\":\"forge-portfolio-share-adapter/0.1.0\",\"status\":\"published\"}}'\n",
            captured.display()
        ),
    );
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            tmp.path().join("catalog.json").to_str().unwrap(),
            "--operation-key",
            "op-capture",
            "--adapter",
            adapter.to_str().unwrap(),
        ],
    );
    let request: Value =
        serde_json::from_str(&fs::read_to_string(&captured).unwrap()).expect("adapter request");
    assert_eq!(request["contract"], "forge-portfolio-share-adapter/0.1.0");
    assert_eq!(request["operation_key"], "op-capture");
    // `serde_json::Value` objects are sorted maps, so this asserts the
    // set of keys the adapter receives.
    let mut keys: Vec<String> = request.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "contract",
            "document",
            "manifest_revision",
            "manifest_sha256",
            "operation_key",
            "target",
        ]
    );
    // The document handed over carries exactly the approved public
    // manifest, with no session, token, credential or registry path.
    let handed_over: Value =
        serde_json::from_str(request["document"].as_str().expect("document string"))
            .expect("handed-over manifest");
    let canonical: Value = serde_json::from_str(
        preview["preview"]["canonical_body"]
            .as_str()
            .expect("canonical body"),
    )
    .expect("canonical body");
    assert_eq!(
        handed_over["manifest_sha256"],
        preview["preview"]["manifest_sha256"]
    );
    assert_eq!(handed_over["projects"], canonical["projects"]);
    assert_eq!(handed_over["schema_family"], canonical["schema_family"]);
    let serialized = request.to_string();
    assert!(!serialized.contains("session"));
    assert!(!serialized.contains(db.to_str().unwrap()));
}

// --- HTTP contract ------------------------------------------------------

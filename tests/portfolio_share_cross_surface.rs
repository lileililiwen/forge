//! Portfolio share cross-surface regression (`portfolio-share-publish`).
//!
//! The share domain is additive. These tests exist to prove it stayed
//! that way: publishing a manifest must not touch the operations
//! journal, the projects table, the portfolio metadata projection or
//! the inventory surface.

#[path = "support/share.rs"]
mod support;

use chrono::Utc;

use support::{fleet, run_json, share_set};

#[test]
fn existing_publish_and_inventory_surfaces_are_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    // Publishing a share record must not touch the operations
    // journal, the projects table or the portfolio metadata domain.
    let before_operations = forge::registry::Registry::open(&db)
        .unwrap()
        .recent_operations(100)
        .unwrap()
        .len();
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
            tmp.path().join("catalog.json").to_str().unwrap(),
            "--operation-key",
            "op-journal",
        ],
    );
    let registry = forge::registry::Registry::open(&db).unwrap();
    assert_eq!(
        registry.recent_operations(100).unwrap().len(),
        before_operations,
        "the share domain writes no operations journal row"
    );
    assert_eq!(registry.list().unwrap().len(), 2);
    // The portfolio metadata domain is untouched by a share write.
    let portfolio = registry
        .portfolio_project_view("alethefy", Utc::now())
        .unwrap();
    assert_eq!(portfolio.profile.project_id, "alethefy");
    assert!(portfolio.profile.lifecycle.is_none());
    assert!(portfolio.tags.is_empty());
    // The inventory surface still answers.
    assert!(run_json(&db, &["list"])["projects"].is_array());
}

#[test]
fn share_records_never_leak_into_the_portfolio_projection() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _token) = fleet(tmp.path());
    share_set(&db, "alethefy", "Alethefy");
    let portfolio = run_json(&db, &["portfolio", "show", "alethefy"]);
    let serialized = portfolio.to_string();
    assert!(
        !serialized.contains("github.com/lileililiwen/alethefy"),
        "the private portfolio projection must not carry the share record: {serialized}"
    );
}

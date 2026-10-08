//! Review-required fixtures.

use super::*;

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

//! Unit tests for the shared delivery rule.
//!
//! Each test pins one condition the rule owns: the evidence must use
//! a supported contract, come from the full profile, not be a
//! rehearsal record, belong to the project, be fresh for the revision
//! and have a passed aggregate.

#[cfg(test)]
pub(super) mod tests {
    use crate::gate::contract::GATE_CONTRACT_VERSION;
    use crate::gate::ship_gate::evaluate_ship_gate;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    const PROJECT_ID: &str = "app";
    const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A project directory with no Gate evidence at all.
    fn project_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        tmp
    }

    /// Write one evidence record for `id` and return the project root.
    fn write_evidence(dir: &Path, id: &str, record: serde_json::Value) {
        let path = dir.join(".forge/gate").join(id).join("evidence.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
    }

    /// A record that satisfies every condition of the rule.
    fn passing_record() -> serde_json::Value {
        serde_json::json!({
            "contract": GATE_CONTRACT_VERSION,
            "project_id": PROJECT_ID,
            "runtime": "forge",
            "profile": "full",
            "revision": REVISION,
            "aggregate": "passed",
            "checks": [],
            "observed_at": "2026-10-18T00:00:00Z",
            "dry_run": false
        })
    }

    #[test]
    fn absent_evidence_is_refused() {
        let tmp = project_dir();
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(!status.satisfied, "no evidence cannot satisfy the rule");
        assert_eq!(
            status.reason,
            "Forge Gate evidence is absent for `app`; run `forge gate --profile full` from the project directory and retry"
        );
    }

    #[test]
    fn unreadable_evidence_counts_as_absent() {
        let tmp = project_dir();
        let path = tmp
            .path()
            .join(".forge/gate")
            .join(PROJECT_ID)
            .join("evidence.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{not json").unwrap();
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "a corrupt record cannot satisfy the rule"
        );
        assert!(
            status.reason.contains("is absent for `app`"),
            "a corrupt record reads as absent: {}",
            status.reason
        );
    }

    #[test]
    fn fresh_passing_full_profile_evidence_satisfies_the_rule() {
        let tmp = project_dir();
        write_evidence(tmp.path(), PROJECT_ID, passing_record());
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            status.satisfied,
            "the rule must admit the record: {}",
            status.reason
        );
        assert!(
            status.reason.is_empty(),
            "a satisfied verdict has no reason"
        );
    }

    #[test]
    fn a_rehearsal_record_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["dry_run"] = serde_json::json!(true);
        write_evidence(tmp.path(), PROJECT_ID, record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "a rehearsal record is not a verification"
        );
        assert!(status.reason.contains("must use a supported contract"));
    }

    #[test]
    fn stale_evidence_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["revision"] = serde_json::json!("fedcba9876543210fedcba9876543210fedcba98");
        write_evidence(tmp.path(), PROJECT_ID, record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "evidence bound to another revision is stale"
        );
        assert!(
            status.reason.contains("freshness=stale"),
            "the refusal names the observed freshness: {}",
            status.reason
        );
    }

    #[test]
    fn failed_evidence_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["aggregate"] = serde_json::json!("failed");
        write_evidence(tmp.path(), PROJECT_ID, record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(!status.satisfied, "a failed run is not a verification");
        assert!(
            status.reason.contains("aggregate=failed"),
            "the refusal names the observed aggregate: {}",
            status.reason
        );
    }

    #[test]
    fn another_projects_evidence_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["project_id"] = serde_json::json!("other");
        write_evidence(tmp.path(), "other", record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "another project's evidence is not this project's"
        );
        assert!(
            status.reason.contains("project=other"),
            "the refusal names the observed project: {}",
            status.reason
        );
    }

    #[test]
    fn a_profile_other_than_full_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["profile"] = serde_json::json!("fast");
        write_evidence(tmp.path(), PROJECT_ID, record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "a fast-profile run is not a full verification"
        );
        assert!(
            status.reason.contains("profile=fast"),
            "the refusal names the observed profile: {}",
            status.reason
        );
    }

    #[test]
    fn an_unsupported_contract_is_refused() {
        let tmp = project_dir();
        let mut record = passing_record();
        record["contract"] = serde_json::json!("9.9.9");
        write_evidence(tmp.path(), PROJECT_ID, record);
        let status = evaluate_ship_gate(tmp.path(), PROJECT_ID, REVISION);
        assert!(
            !status.satisfied,
            "an unknown contract is not a verification"
        );
        assert!(
            status.reason.contains("contract=9.9.9"),
            "the refusal names the observed contract: {}",
            status.reason
        );
    }
}

//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    use chrono::{DateTime, Utc};

    use crate::core::manifest::CANONICAL_MANIFEST;
    use crate::doctor::{FindingStatus, Remediation};
    use crate::spec::contract::MAX_FINDINGS_PER_SPEC;
    use crate::spec::lifecycle::{
        apply_routing, build_spec_draft, generate_spec, list_specs, route_finding, spec_id_for,
        validate_request,
    };
    use crate::spec::model::{
        DoctorFindingInput, FindingSource, RoutingStatus, SpecRequest, SpecRoute, SpecStatus,
    };
    use crate::upgrade::SemanticConflict;

    fn write_manifest(dir: &Path, id: &str) {
        let body = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join(CANONICAL_MANIFEST), body).unwrap();
    }

    #[test]
    fn spec_id_is_stable_for_same_finding_set() {
        let a = spec_id_for("proj", &["a".to_string(), "b".to_string()]);
        let b = spec_id_for("proj", &["b".to_string(), "a".to_string()]);
        assert_eq!(a, b);
    }

    #[test]
    fn spec_id_differs_for_different_finding_sets() {
        let a = spec_id_for("proj", &["a".to_string()]);
        let b = spec_id_for("proj", &["b".to_string()]);
        assert_ne!(a, b);
    }

    #[test]
    fn validate_request_rejects_oversized_set() {
        let mut ids: Vec<String> = (0..MAX_FINDINGS_PER_SPEC + 1)
            .map(|i| format!("f{i}"))
            .collect();
        let request = SpecRequest {
            project_path: PathBuf::from("/nonexistent"),
            finding_ids: ids.clone(),
            reason: None,
        };
        let err = validate_request(&request).unwrap_err();
        assert!(err.to_string().contains("at most"), "{}", err);
        ids.truncate(2);
        let small = SpecRequest {
            project_path: PathBuf::from("/nonexistent"),
            finding_ids: ids,
            reason: None,
        };
        assert!(validate_request(&small).is_ok());
    }

    #[test]
    fn build_spec_draft_captures_provenance_and_scenarios() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "demo");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["a".to_string(), "b".to_string()],
            reason: Some("manual override".to_string()),
        };
        let sources = vec![FindingSource::Doctor(DoctorFindingInput {
            id: "a".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Ai,
            category: "spec".to_string(),
            detail: "manual detail".to_string(),
        })];
        let draft = build_spec_draft(
            &request,
            &sources,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(draft.provenance.project_id, "demo");
        assert_eq!(draft.provenance.profile, "rust-web");
        assert!(draft.findings.contains(&"a".to_string()));
        assert!(draft.acceptance.iter().any(|s| s.source_finding == "a"));
        assert!(draft.why.contains("manual override"));
    }

    #[test]
    fn generate_spec_is_idempotent_on_unchanged_finding_set() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "idem");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["only".to_string()],
            reason: None,
        };
        let first = generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "only".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(first.status, SpecStatus::Generated);
        assert_eq!(first.files_written.len(), 4);
        let second = generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "only".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(second.status, SpecStatus::Existing);
        assert!(second.files_written.is_empty());
        assert!(second.note.contains("already exists"));
    }

    #[test]
    fn router_routes_manual_finding_as_manual() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "manual");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["judgment".to_string()],
            reason: None,
        };
        let source = FindingSource::Doctor(DoctorFindingInput {
            id: "judgment".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Manual,
            category: "manual".to_string(),
            detail: "needs a human".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Manual);
        let outcome = apply_routing(
            &request,
            &source,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.status, RoutingStatus::Manual);
        assert!(outcome.files_changed.is_empty());
        assert!(outcome.note.contains("manual boundary"), "{}", outcome.note);
        assert!(
            outcome.note.contains("no project changes"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn router_routes_feature_compatibility_as_deterministic() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "feat");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["features-compatible".to_string()],
            reason: None,
        };
        let source = FindingSource::Doctor(DoctorFindingInput {
            id: "features-compatible".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Manual,
            category: "automatic".to_string(),
            detail: "incompatible".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Deterministic);
        assert_eq!(decision.action.as_deref(), Some("forge upgrade"));
    }

    #[test]
    fn router_routes_semantic_conflict_as_semantic() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "conflict");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["semantic-auth".to_string()],
            reason: None,
        };
        let source = FindingSource::Conflict(SemanticConflict {
            project_id: "conflict".to_string(),
            feature: "auth".to_string(),
            owned_file: ".forge/features/auth.receipt".to_string(),
            reason: "drifted".to_string(),
            suggested_spec: "forge spec generate".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Semantic);
        let outcome = apply_routing(
            &request,
            &source,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.status, RoutingStatus::SpecGenerated);
        assert!(!outcome.files_changed.is_empty());
    }

    #[test]
    fn list_specs_returns_generated_entries() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "list");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["f".to_string()],
            reason: None,
        };
        generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "f".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        let entries = list_specs(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].project_id, "list");
    }
}

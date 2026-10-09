//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::component::catalog::{
        component_catalog, descriptor, epoch_record_time, inspect_component, is_primitive_id,
        latest_verified_timestamp, validate_descriptor,
    };
    use crate::component::contract::{CERTIFIED_TEST_COVERAGE, DEFAULT_INSTALL_STRATEGY};
    use crate::component::engine::{
        qualify_component, record_qualification, render_outcome_human, render_plan_human,
        resolve_outcome, select_strongest, validate_request,
    };
    use crate::component::model::{
        ComponentContract, ComponentDescriptor, ComponentEvidence, ComponentPort,
        ComponentQualifyEvidence, ComponentQualifyRequest, ComponentQuality, ComponentRequest,
    };
    use tempfile::TempDir;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn request(profile: &str, ids: &[&str]) -> ComponentRequest {
        ComponentRequest {
            profile: profile.to_string(),
            component_ids: strings(ids),
        }
    }

    #[test]
    fn catalog_lists_components_in_stable_id_order() {
        let catalog = component_catalog();
        let ids: Vec<&str> = catalog.iter().map(|c| c.id.as_str()).collect();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "ids must be unique");
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "catalog must be in stable id order");
        // Spot check the brief's required ids (R1 success scenario).
        for id in [
            "paginated-query",
            "idempotency-guard",
            "validated-form",
            "audit-action",
            "soft-delete",
            "retry-external-call",
            "require-permission",
            "api-mutation",
            "loading-state",
            "error-boundary",
            "confirm-dialog",
            "empty-state",
            "toast",
            "file-picker",
            "webhook-receiver",
        ] {
            assert!(ids.contains(&id), "missing catalog id {id}");
        }
        for descriptor in &catalog {
            assert!(!descriptor.contract.inputs.is_empty(), "{}", descriptor.id);
            assert!(!descriptor.contract.outputs.is_empty(), "{}", descriptor.id);
            assert!(!descriptor.profiles.is_empty(), "{}", descriptor.id);
            assert!(
                !descriptor.install_strategy.trim().is_empty(),
                "{}",
                descriptor.id
            );
            assert!(!descriptor.tests.trim().is_empty(), "{}", descriptor.id);
            assert!(
                !descriptor.documentation.trim().is_empty(),
                "{}",
                descriptor.id
            );
        }
    }

    #[test]
    fn catalog_refuses_programming_primitives() {
        for primitive in [
            "if",
            "loop",
            "for",
            "while",
            "try",
            "catch",
            "string-concat",
            "addition",
        ] {
            assert!(is_primitive_id(primitive), "{primitive}");
        }
        assert!(!is_primitive_id("paginated-query"));
        assert!(is_primitive_id("IF"));
        assert!(is_primitive_id("string-concat"));
    }

    #[test]
    fn validate_descriptor_rejects_missing_contract() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.contract.inputs.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("no inputs"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_primitive_id() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.id = "if".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_empty_profiles() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.profiles.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("no tested profile"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_unnamed_port() {
        let mut descriptor = inspect_component("paginated-query").unwrap();
        descriptor.contract.inputs[0].name.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("unnamed port"), "{text}");
    }

    #[test]
    fn resolve_known_components_succeeds_with_certified_evidence() {
        let outcome = resolve_outcome(&request(
            "rust-web",
            &["paginated-query", "idempotency-guard"],
        ))
        .unwrap();
        assert!(outcome.plan.rejections.is_empty());
        let ids: Vec<&str> = outcome.plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"paginated-query"));
        assert!(ids.contains(&"idempotency-guard"));
        for entry in &outcome.evidence_summary {
            assert_eq!(entry.quality, ComponentQuality::Certified);
            assert!(entry.evidence.test_coverage >= CERTIFIED_TEST_COVERAGE);
            assert!(entry.evidence.security_review);
        }
    }

    #[test]
    fn resolve_profile_incompatibility_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("flutter-app", &["paginated-query"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(outcome.plan.rejections[0].code, "component-invalid");
        assert!(outcome.plan.rejections[0].reason.contains("flutter-app"));
    }

    #[test]
    fn resolve_unknown_component_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("rust-web", &["made-up"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections[0].code, "component-invalid");
        assert!(outcome.plan.rejections[0]
            .reason
            .contains("not in the catalog"));
    }

    #[test]
    fn resolve_only_deprecated_reports_quality_conflict() {
        // webhook-receiver is the only deprecated catalog entry; its
        // profile set is rust-web / python-service.
        let outcome = resolve_outcome(&request("rust-web", &["webhook-receiver"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "component-quality-conflict"
        );
        assert!(outcome.plan.rejections[0].reason.contains("only candidate"));
    }

    #[test]
    fn resolve_prefers_certified_over_other_qualities() {
        // Construct a synthetic catalog where two descriptors share an
        // id; the certified one must be selected.
        let now = latest_verified_timestamp();
        let certified = ComponentDescriptor {
            id: "demo-shared".to_string(),
            version: "0.1.0".to_string(),
            purpose: "shared component with two quality levels".to_string(),
            contract: ComponentContract {
                inputs: vec![ComponentPort {
                    name: "in".to_string(),
                    description: "demo input".to_string(),
                }],
                outputs: vec![ComponentPort {
                    name: "out".to_string(),
                    description: "demo output".to_string(),
                }],
            },
            depends_on: Vec::new(),
            profiles: vec!["rust-web".to_string()],
            install_strategy: DEFAULT_INSTALL_STRATEGY.to_string(),
            validation: Vec::new(),
            documentation: "demo".to_string(),
            tests: "demo".to_string(),
            quality: ComponentQuality::Certified,
            evidence: ComponentEvidence {
                usage_count: 5,
                test_coverage: 0.95,
                last_verified: now,
                known_issues: Vec::new(),
                security_review: true,
            },
        };
        let experimental = ComponentDescriptor {
            quality: ComponentQuality::Experimental,
            evidence: ComponentEvidence {
                usage_count: 1,
                test_coverage: 0.40,
                last_verified: now,
                known_issues: vec!["draft".to_string()],
                security_review: false,
            },
            ..certified.clone()
        };
        // The plan resolver uses the built-in catalog; emulate the
        // candidate selection by reusing `select_strongest` directly
        // so the test exercises the same ranking logic.
        let candidates: Vec<&ComponentDescriptor> = vec![&experimental, &certified];
        let selected = select_strongest(&candidates).unwrap();
        assert_eq!(selected.quality, ComponentQuality::Certified);
    }

    #[test]
    fn validate_request_rejects_primitive_id() {
        let err = validate_request(&request("rust-web", &["if"])).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_request_rejects_duplicate_id() {
        let err = validate_request(&request(
            "rust-web",
            &["paginated-query", "paginated-query"],
        ))
        .unwrap_err();
        assert_eq!(err.code(), "component-invalid");
        let text = err.to_string();
        assert!(text.contains("duplicated"), "{text}");
    }

    #[test]
    fn validate_request_rejects_empty_id_list() {
        let err = validate_request(&request("rust-web", &[])).unwrap_err();
        assert_eq!(err.code(), "component-invalid");
    }

    #[test]
    fn qualify_to_certified_requires_security_review() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: false,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert_eq!(outcome.prior_quality, ComponentQuality::Verified);
        assert!(
            outcome.note.contains("missing security review"),
            "{}",
            outcome.note
        );
        assert!(
            outcome.note.contains("remains at quality 'verified'"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_requires_test_coverage() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.50,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("test coverage 0.5"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_requires_fresh_verification() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: epoch_record_time(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("freshness window"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn qualify_to_certified_succeeds_with_complete_evidence() {
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "production ready".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert_eq!(outcome.prior_quality, ComponentQuality::Verified);
        assert_eq!(outcome.target_quality, ComponentQuality::Certified);
    }

    #[test]
    fn qualify_to_deprecated_never_writes_a_receipt() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Deprecated,
            reason: "superseded by toast-v2".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert!(outcome.files_written.is_empty());
        assert!(!tmp.path().join(".forge").exists());
    }

    #[test]
    fn qualify_already_at_target_is_a_noop() {
        let request = ComponentQualifyRequest {
            component_id: "idempotency-guard".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "noop".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = qualify_component(&request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(
            outcome.note.contains("already at quality"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn refused_qualification_does_not_write_a_receipt() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "promote for production".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.50,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(!outcome.promoted);
        assert!(outcome.files_written.is_empty());
        assert!(!tmp.path().join(".forge/components").exists());
    }

    #[test]
    fn record_qualification_writes_receipt_for_accepted_promotion() {
        let tmp = TempDir::new().unwrap();
        let request = ComponentQualifyRequest {
            component_id: "toast".to_string(),
            target_quality: ComponentQuality::Certified,
            reason: "production ready".to_string(),
        };
        let evidence = ComponentQualifyEvidence {
            test_coverage: 0.95,
            last_verified: latest_verified_timestamp(),
            known_issues: Vec::new(),
            security_review: true,
        };
        let outcome = record_qualification(tmp.path(), &request, &evidence).unwrap();
        assert!(outcome.promoted);
        assert_eq!(
            outcome.files_written,
            vec![".forge/components/toast/qualify.json".to_string()]
        );
        let body = std::fs::read_to_string(tmp.path().join(".forge/components/toast/qualify.json"))
            .unwrap();
        assert!(body.contains("\"security_review\": true"));
        assert!(body.contains("\"target_quality\": \"certified\""));
        assert!(body.contains("\"component_id\": \"toast\""));
    }

    #[test]
    fn component_helpers_render_human_output() {
        let outcome =
            resolve_outcome(&request("rust-web", &["paginated-query", "made-up"])).unwrap();
        let human = render_outcome_human(&outcome);
        assert!(human.contains("component resolve:"));
        assert!(human.contains("paginated-query"));
        assert!(human.contains("rejections:"));
        assert!(human.contains("made-up"));
        let plan_human = render_plan_human(&outcome.plan);
        assert!(plan_human.contains("plan for profile 'rust-web'"));
    }
}

//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::ui_pattern::constants::{
        UI_PATTERN_CERTIFIED_TEST_COVERAGE, UI_PATTERN_REQUIRED_STATES,
    };
    use crate::ui_pattern::install::{install_pattern, render_outcome_human, render_plan_human};
    use crate::ui_pattern::model::{UiPatternInstallRequest, UiPatternQuality, UiPatternRequest};
    use crate::ui_pattern::patterns::ui_pattern_catalog;
    use crate::ui_pattern::resolve::{inspect_ui_pattern, resolve_outcome, validate_request};
    use crate::ui_pattern::validate::{is_primitive_id, validate_descriptor};
    use std::fs;
    use tempfile::TempDir;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    fn request(profile: &str, ids: &[&str]) -> UiPatternRequest {
        UiPatternRequest {
            profile: profile.to_string(),
            pattern_ids: strings(ids),
        }
    }

    #[test]
    fn catalog_lists_patterns_in_stable_id_order() {
        let catalog = ui_pattern_catalog();
        let ids: Vec<&str> = catalog.iter().map(|c| c.id.as_str()).collect();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "ids must be unique");
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "catalog must be in stable id order");
        for id in [
            "login",
            "register",
            "forgot-password",
            "dashboard",
            "crud-table",
            "filter-bar",
            "form",
            "settings",
            "profile",
            "billing",
            "empty-state",
            "success-page",
            "error-page",
            "modal",
            "confirm-dialog",
            "file-upload",
            "navigation",
        ] {
            assert!(ids.contains(&id), "missing catalog id {id}");
        }
        for descriptor in &catalog {
            assert!(!descriptor.states.is_empty(), "{}", descriptor.id);
            for required in UI_PATTERN_REQUIRED_STATES {
                assert!(
                    descriptor.states.iter().any(|s| s.name == *required),
                    "{} missing required state {required}",
                    descriptor.id
                );
            }
            assert!(!descriptor.adapters.is_empty(), "{}", descriptor.id);
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
    fn catalog_refuses_programming_primitives_and_placeholders() {
        for primitive in [
            "if",
            "loop",
            "for",
            "while",
            "try",
            "catch",
            "string-concat",
            "addition",
            "screenshot",
            "html-fragment",
            "copy-paste",
            "lorem-ipsum",
        ] {
            assert!(is_primitive_id(primitive), "{primitive}");
        }
        assert!(!is_primitive_id("login"));
        assert!(!is_primitive_id("crud-table"));
    }

    #[test]
    fn validate_descriptor_rejects_missing_state_contract() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.states.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("no state contract"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_missing_required_state() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.states.retain(|s| s.name != "error");
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("missing required state 'error'"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_unknown_intent() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.intent = "scratchpad".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("unknown intent"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_primitive_id() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.id = "if".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_descriptor_rejects_placeholder_artifact() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.adapters[0].artifact_source = "<html><body>login</body></html>".to_string();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(
            text.contains("copied screenshot or HTML fragment"),
            "{text}"
        );
    }

    #[test]
    fn validate_descriptor_rejects_empty_adapters() {
        let mut descriptor = inspect_ui_pattern("login").unwrap();
        descriptor.adapters.clear();
        let err = validate_descriptor(&descriptor).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("no tested platform adapter"), "{text}");
    }

    #[test]
    fn resolve_known_patterns_succeeds_with_certified_evidence() {
        let outcome = resolve_outcome(&request("react-web", &["login", "form"])).unwrap();
        assert!(outcome.plan.rejections.is_empty());
        let ids: Vec<&str> = outcome.plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"login"));
        assert!(ids.contains(&"form"));
        for entry in &outcome.evidence_summary {
            assert_eq!(entry.quality, UiPatternQuality::Certified);
            assert!(entry.evidence.test_coverage >= UI_PATTERN_CERTIFIED_TEST_COVERAGE);
            assert!(entry.evidence.security_review);
        }
    }

    #[test]
    fn resolve_profile_incompatibility_surfaces_typed_rejection() {
        // `form` ships for react-web/nextjs-web/flutter-app, so test a
        // pattern that only has web adapters; `billing` ships only for
        // react-web and nextjs-web.
        let outcome = resolve_outcome(&request("flutter-app", &["billing"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "ui-pattern-unsupported-platform"
        );
        assert!(outcome.plan.rejections[0].reason.contains("flutter-app"));
    }

    #[test]
    fn resolve_unknown_pattern_surfaces_typed_rejection() {
        let outcome = resolve_outcome(&request("react-web", &["made-up"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(outcome.plan.rejections[0].code, "ui-pattern-invalid");
        assert!(outcome.plan.rejections[0]
            .reason
            .contains("not in the catalog"));
    }

    #[test]
    fn resolve_only_deprecated_reports_quality_conflict() {
        // The deprecated test entry reuses the `form` id with a
        // single nextjs-web adapter, so a resolve for `form` on
        // nextjs-web surfaces the deprecated-only branch via
        // `webhook-receiver` (the deprecated id) on its tested
        // adapter (nextjs-web).
        let outcome = resolve_outcome(&request("nextjs-web", &["webhook-receiver"])).unwrap();
        assert!(outcome.plan.steps.is_empty());
        assert_eq!(outcome.plan.rejections.len(), 1);
        assert_eq!(
            outcome.plan.rejections[0].code,
            "ui-pattern-quality-conflict"
        );
        assert!(outcome.plan.rejections[0].reason.contains("only candidate"));
    }

    #[test]
    fn validate_request_rejects_primitive_id() {
        let err = validate_request(&request("react-web", &["if"])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("programming primitive"), "{text}");
    }

    #[test]
    fn validate_request_rejects_duplicate_id() {
        let err = validate_request(&request("react-web", &["login", "login"])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
        let text = err.to_string();
        assert!(text.contains("duplicated"), "{text}");
    }

    #[test]
    fn validate_request_rejects_empty_id_list() {
        let err = validate_request(&request("react-web", &[])).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
    }

    #[test]
    fn install_pattern_writes_artifact_and_receipt_for_supported_profile() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "studio needs the standard form".to_string(),
        };
        let outcome = install_pattern(tmp.path(), &request).unwrap();
        assert!(outcome.installed);
        assert_eq!(outcome.pattern_id, "form");
        assert!(outcome.files_written.iter().any(|p| p == "src/ui/form.tsx"));
        let artifact = std::fs::read_to_string(tmp.path().join("src/ui/form.tsx")).unwrap();
        assert!(artifact.contains("function Form("));
        assert!(artifact.contains("data-state="));
        let receipt =
            std::fs::read_to_string(tmp.path().join(".forge/ui-patterns/form/install.json"))
                .unwrap();
        assert!(receipt.contains("\"pattern_id\": \"form\""));
        assert!(receipt.contains("\"profile\": \"react-web\""));
    }

    #[test]
    fn install_pattern_writes_flutter_artifact_for_flutter_app() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "flutter-app".to_string(),
            reason: "mobile build needs the form surface".to_string(),
        };
        let outcome = install_pattern(tmp.path(), &request).unwrap();
        assert!(outcome.installed);
        let artifact = std::fs::read_to_string(tmp.path().join("lib/ui/form.dart")).unwrap();
        assert!(artifact.contains("class Form"));
        assert!(artifact.contains("'form'"));
    }

    #[test]
    fn install_pattern_refuses_to_overwrite_a_customized_artifact() {
        let tmp = TempDir::new().unwrap();
        let artifact_path = tmp.path().join("src/ui/form.tsx");
        std::fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
        std::fs::write(&artifact_path, "// user already customized this\n").unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "should refuse".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-ownership-conflict");
        // Customized file left intact.
        let preserved = std::fs::read_to_string(&artifact_path).unwrap();
        assert!(preserved.contains("user already customized"));
        // Receipt must not have been written.
        assert!(!tmp
            .path()
            .join(".forge/ui-patterns/form/install.json")
            .exists());
    }

    #[test]
    fn install_pattern_is_idempotent_when_artifact_is_byte_identical() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "form".to_string(),
            profile: "react-web".to_string(),
            reason: "first install".to_string(),
        };
        let _ = install_pattern(tmp.path(), &request).unwrap();
        let second = install_pattern(tmp.path(), &request).unwrap();
        assert!(second.installed);
    }

    #[test]
    fn install_pattern_refuses_unsupported_platform() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "billing".to_string(),
            profile: "flutter-app".to_string(),
            reason: "no flutter surface".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-unsupported-platform");
        assert!(err.to_string().contains("flutter-app"));
    }

    #[test]
    fn install_pattern_refuses_unknown_id() {
        let tmp = TempDir::new().unwrap();
        let request = UiPatternInstallRequest {
            pattern_id: "made-up".to_string(),
            profile: "react-web".to_string(),
            reason: "missing".to_string(),
        };
        let err = install_pattern(tmp.path(), &request).unwrap_err();
        assert_eq!(err.code(), "ui-pattern-invalid");
    }

    #[test]
    fn ui_pattern_helpers_render_human_output() {
        let outcome = resolve_outcome(&request("react-web", &["login", "made-up"])).unwrap();
        let human = render_outcome_human(&outcome);
        assert!(human.contains("ui-pattern resolve:"));
        assert!(human.contains("login"));
        assert!(human.contains("rejections:"));
        assert!(human.contains("made-up"));
        let plan_human = render_plan_human(&outcome.plan);
        assert!(plan_human.contains("ui-pattern plan for profile 'react-web'"));
    }
}

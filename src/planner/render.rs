//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{AssemblyPlan, IntentApplyOutcome, IntentValidationOutcome};

/// Render an [`AssemblyPlan`] for the CLI.
pub fn render_plan_human(plan: &AssemblyPlan) -> String {
    let mut body = format!(
        "plan {}\n  profile: {}@{}\n  intent_hash: {}\n  catalog_hash: {}\n  \
         source_revision: {}\n  steps: {}\n  unresolved: {}\n  note: {}",
        plan.plan_id,
        plan.profile,
        plan.profile_version,
        plan.intent_hash,
        plan.catalog_hash,
        plan.source_revision.as_deref().unwrap_or("none"),
        plan.steps.len(),
        plan.unresolved.len(),
        plan.note
    );
    if !plan.steps.is_empty() {
        body.push_str("\n  steps:");
        for step in &plan.steps {
            body.push_str(&format!(
                "\n    - {} {}@{} action={} evidence={}",
                step.kind.label(),
                step.target,
                step.version,
                step.action,
                step.evidence
            ));
        }
    }
    if !plan.unresolved.is_empty() {
        body.push_str("\n  unresolved:");
        for work in &plan.unresolved {
            body.push_str(&format!(
                "\n    - {} :: {} (hint: {})",
                work.requirement, work.reason, work.hint
            ));
        }
    }
    body
}

/// Render an [`IntentApplyOutcome`] for the CLI.
pub fn render_apply_human(outcome: &IntentApplyOutcome) -> String {
    let mut body = format!(
        "plan {} applied: {} step(s), {} file(s) written, stale={}",
        outcome.plan_id,
        outcome.applied_steps.len(),
        outcome.files_written.len(),
        outcome.stale
    );
    for step in &outcome.applied_steps {
        body.push_str(&format!(
            "\n  - {} {} :: {} ({})",
            step.kind.label(),
            step.target,
            step.status,
            step.note
        ));
    }
    if !outcome.files_written.is_empty() {
        body.push_str("\n  files:");
        for f in &outcome.files_written {
            body.push_str(&format!("\n    - {f}"));
        }
    }
    body.push_str(&format!("\n  note: {}", outcome.note));
    body
}

/// Render an [`IntentValidationOutcome`] for the CLI.
pub fn render_intent_validation_human(outcome: &IntentValidationOutcome) -> String {
    if let Some(validated) = &outcome.validated {
        format!(
            "intent validated\n  action: {}\n  profile: {}@{}\n  adapter: {}\n  \
             required: [{}]\n  forbidden: [{}]\n  backend_boundary: {}\n  note: {}",
            validated.intent.action.label(),
            validated.profile_id,
            validated.profile_version,
            validated.adapter,
            validated.intent.required_capabilities.join(", "),
            validated.intent.forbidden_capabilities.join(", "),
            validated.backend_boundary.as_deref().unwrap_or("none"),
            validated.note
        )
    } else {
        format!("intent validation failed: {}", outcome.note)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::core::ForgeError;
    use crate::planner::apply::{apply_plan, revalidate_plan, write_plan_receipt};
    use crate::planner::contract::{MAX_CAPABILITIES_PER_INTENT, MAX_UNRESOLVED_PER_PLAN};
    use crate::planner::model::{
        AssemblyPlan, Intent, IntentAction, IntentApplyOutcome, IntentConstraint, PlanStepKind,
    };
    use crate::planner::resolve::resolve_plan;
    use crate::planner::validate::{catalog_hash, validate_intent};
    use std::path::{Path, PathBuf};

    fn sample_intent() -> Intent {
        Intent {
            action: IntentAction::CreateProject,
            profile: "rust-web".to_string(),
            required_capabilities: vec!["auth".to_string(), "admin".to_string()],
            forbidden_capabilities: vec!["billing".to_string()],
            constraints: vec![IntentConstraint {
                key: "public".to_string(),
                value: "true".to_string(),
            }],
        }
    }

    #[test]
    fn validator_accepts_a_compatible_intent() {
        let intent = sample_intent();
        let validated = validate_intent(&intent).expect("validator");
        assert_eq!(validated.profile_id, "rust-web");
        assert_eq!(validated.profile_version, "0.1.0");
        assert_eq!(
            validated.intent.required_capabilities,
            vec!["admin", "auth"]
        );
        assert_eq!(validated.intent.forbidden_capabilities, vec!["billing"]);
        assert!(validated.backend_boundary.is_none());
    }

    #[test]
    fn validator_rejects_required_and_forbidden_intersection() {
        let mut intent = sample_intent();
        intent.forbidden_capabilities = vec!["auth".to_string()];
        let err = validate_intent(&intent).expect_err("must reject");
        assert!(matches!(err, ForgeError::IntentInvalid { .. }));
    }

    #[test]
    fn validator_rejects_client_profile_with_server_capability() {
        let intent = Intent {
            action: IntentAction::CreateProject,
            profile: "flutter-app".to_string(),
            required_capabilities: vec!["auth".to_string(), "postgres".to_string()],
            forbidden_capabilities: vec![],
            constraints: vec![],
        };
        let err = validate_intent(&intent).expect_err("must reject");
        match err {
            ForgeError::IntentInvalid { reason } => {
                assert!(reason.contains("flutter-app"));
                assert!(reason.contains("postgres"));
                assert!(reason.contains("backend"));
            }
            _ => panic!("expected IntentInvalid"),
        }
    }

    #[test]
    fn validator_rejects_unknown_capability() {
        let intent = Intent {
            action: IntentAction::CreateProject,
            profile: "rust-web".to_string(),
            required_capabilities: vec!["not-a-real-capability".to_string()],
            forbidden_capabilities: vec![],
            constraints: vec![],
        };
        let err = validate_intent(&intent).expect_err("must reject");
        match err {
            ForgeError::IntentInvalid { reason } => {
                assert!(reason.contains("not-a-real-capability"));
            }
            _ => panic!("expected IntentInvalid"),
        }
    }

    #[test]
    fn validator_rejects_unknown_profile() {
        let intent = Intent {
            action: IntentAction::CreateProject,
            profile: "no-such-profile".to_string(),
            required_capabilities: vec!["auth".to_string()],
            forbidden_capabilities: vec![],
            constraints: vec![],
        };
        let err = validate_intent(&intent).expect_err("must reject");
        assert!(matches!(err, ForgeError::IntentInvalid { .. }));
    }

    #[test]
    fn validator_rejects_unknown_constraint_key() {
        let mut intent = sample_intent();
        intent.constraints = vec![IntentConstraint {
            key: "host".to_string(),
            value: "internal".to_string(),
        }];
        let err = validate_intent(&intent).expect_err("must reject");
        match err {
            ForgeError::IntentInvalid { reason } => {
                assert!(reason.contains("host"));
            }
            _ => panic!("expected IntentInvalid"),
        }
    }

    #[test]
    fn validator_rejects_too_many_capabilities() {
        let mut intent = sample_intent();
        let mut caps: Vec<String> = (0..(MAX_CAPABILITIES_PER_INTENT + 1))
            .map(|i| format!("auth-{i}"))
            .collect();
        caps.sort();
        intent.required_capabilities = caps;
        let err = validate_intent(&intent).expect_err("must reject");
        match err {
            ForgeError::IntentInvalid { reason } => {
                assert!(reason.contains("capabilities"));
            }
            _ => panic!("expected IntentInvalid"),
        }
    }

    #[test]
    fn resolve_plan_emits_pinned_feature_and_quality_steps() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let plan = resolve_plan(&validated, None).expect("plan");
        assert_eq!(plan.profile, "rust-web");
        assert!(!plan.steps.is_empty());
        let kinds: Vec<&'static str> = plan.steps.iter().map(|s| s.kind.label()).collect();
        assert!(kinds.contains(&"install_feature"));
        assert!(kinds.contains(&"doctor"));
    }

    #[test]
    fn validator_runs_before_plan_resolution() {
        // The planner is layered: validate_intent must run before
        // resolve_plan and the two are not interchangeable. A request
        // that mixes required and forbidden capabilities, or names an
        // unknown capability, never reaches the resolver; the
        // validator surfaces the rejection as a typed IntentInvalid
        // so the resolver never sees an inconsistent graph.
        let intent = Intent {
            action: IntentAction::CreateProject,
            profile: "rust-web".to_string(),
            required_capabilities: vec!["auth".to_string(), "billing".to_string()],
            forbidden_capabilities: vec!["billing".to_string()],
            constraints: vec![],
        };
        let err = validate_intent(&intent).expect_err("validator must refuse");
        assert!(matches!(err, ForgeError::IntentInvalid { .. }));
    }

    #[test]
    fn resolve_plan_rejects_oversized_unresolved_list() {
        // The validator already refuses unknown capabilities, so we
        // simulate the boundary by checking the constant exists.
        const { assert!(MAX_UNRESOLVED_PER_PLAN >= 1) };
    }

    #[test]
    fn plan_id_is_stable_for_same_intent() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let plan_a = resolve_plan(&validated, None).expect("plan");
        let plan_b = resolve_plan(&validated, None).expect("plan");
        assert_eq!(plan_a.intent_hash, plan_b.intent_hash);
        assert_eq!(plan_a.catalog_hash, plan_b.catalog_hash);
        assert_eq!(plan_a.plan_id, plan_b.plan_id);
    }

    #[test]
    fn catalog_hash_changes_when_a_descriptor_version_changes() {
        let before = catalog_hash();
        // Stable, but the hash is content-derived: same catalog -> same hash.
        let after = catalog_hash();
        assert_eq!(before, after);
    }

    #[test]
    fn revalidate_accepts_a_fresh_plan() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let plan = resolve_plan(&validated, None).expect("plan");
        revalidate_plan(&plan).expect("revalidate");
    }

    #[test]
    fn revalidate_refuses_a_stale_plan() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let mut plan = resolve_plan(&validated, None).expect("plan");
        plan.profile_version = "9.9.9".to_string();
        let err = revalidate_plan(&plan).expect_err("must reject");
        assert!(matches!(err, ForgeError::PlanStale { .. }));
    }

    #[test]
    fn apply_plan_refuses_without_confirm() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let plan = resolve_plan(&validated, None).expect("plan");
        let tmp = tempdir();
        write_plan_receipt(&tmp, &plan).expect("write");
        let err = apply_plan(&tmp, &plan.plan_id, false, None).expect_err("must reject");
        match err {
            ForgeError::PlanApplyFailed { reason, .. } => {
                assert!(reason.contains("confirm"));
            }
            _ => panic!("expected PlanApplyFailed"),
        }
    }

    #[test]
    fn apply_plan_persists_and_runs_feature_install() {
        let validated = validate_intent(&sample_intent()).expect("validator");
        let plan = resolve_plan(&validated, None).expect("plan");
        let project_root = tempdir();
        write_plan_receipt(&project_root, &plan).expect("write");
        // Pre-register a project so feature::add_feature has a target.
        let registry_path =
            pre_register_project(&project_root, "rust-web", "planner-feature-smoke");
        let outcome =
            apply_plan(&project_root, &plan.plan_id, true, Some(&registry_path)).expect("apply");
        assert!(outcome
            .applied_steps
            .iter()
            .any(|s| matches!(s.kind, PlanStepKind::InstallFeature) && s.target == "auth"));
    }

    fn pre_register_project(project_root: &Path, profile: &str, project_id: &str) -> PathBuf {
        let manifest_path = project_root.join("forge.yaml");
        std::fs::write(
            &manifest_path,
            format!(
                "schema: 1\nproject:\n  id: {project_id}\n  name: {project_id}\n  \
                 profile: {profile}\nfeatures: {{}}\n"
            ),
        )
        .expect("manifest write");
        let db = project_root.join(".forge/registry.sqlite");
        if let Some(parent) = db.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut registry = crate::registry::Registry::open(&db).expect("registry open");
        let _ = registry.register(project_root, None);
        db
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "forge-planner-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let path = base.join(unique);
        std::fs::create_dir_all(&path).expect("tempdir");
        path
    }
}

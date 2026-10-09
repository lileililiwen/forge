//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::ForgeError;
    use crate::procedure::catalog::{
        all_procedures, contains_bypass_marker, inspect_procedure, procedure_catalog,
        render_inspect_human, render_list_human, validate_procedure,
    };
    use crate::procedure::constants::{
        BYPASS_MARKERS, MAX_PROCEDURE_STEPS, MAX_STEP_ARGS, PROCEDURE_CONTRACT_VERSION,
        PROCEDURE_SYNTHETIC_PROJECT,
    };
    use crate::procedure::model::{CoreOperation, ProcedureSpec, ProcedureStep};

    #[test]
    fn every_catalog_entry_passes_validate_procedure() {
        for spec in all_procedures() {
            validate_procedure(&spec)
                .unwrap_or_else(|err| panic!("procedure '{}' failed validation: {}", spec.id, err));
        }
    }

    #[test]
    fn inspect_procedure_returns_catalogued_spec() {
        let spec = inspect_procedure("create-project").unwrap();
        assert_eq!(spec.id, "create-project");
        assert_eq!(spec.version, PROCEDURE_CONTRACT_VERSION);
        assert!(!spec.steps.is_empty());
        assert!(spec.steps.last().unwrap().op.is_report());
    }

    #[test]
    fn inspect_procedure_rejects_empty_id() {
        match inspect_procedure("") {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("must not be empty"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn inspect_procedure_rejects_unknown_id() {
        match inspect_procedure("nope") {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("not in the catalog"));
                assert!(reason.contains("create-project"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn inspect_procedure_rejects_non_kebab_case() {
        match inspect_procedure("Create_Project") {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("kebab-case"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_empty_steps() {
        let spec = ProcedureSpec {
            id: "empty".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "Empty".to_string(),
            description: "no steps".to_string(),
            prerequisites: vec![],
            steps: vec![],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("no steps"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_too_many_steps() {
        let total = MAX_PROCEDURE_STEPS + 1;
        let mut steps = Vec::new();
        for i in 1..=total {
            steps.push(ProcedureStep {
                ordinal: i as u32,
                op: if i == 1 {
                    CoreOperation::DoctorRun
                } else if i == total {
                    CoreOperation::ReportFindings
                } else {
                    CoreOperation::TestRun
                },
                args: vec![],
                description: "d".to_string(),
            });
        }
        let spec = ProcedureSpec {
            id: "too-many".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "Too many".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps,
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("more than"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_non_monotonic_ordinals() {
        let spec = ProcedureSpec {
            id: "ordinal".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "ordinal".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::TestRun,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("monotonic"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_workflow_without_report_findings() {
        let spec = ProcedureSpec {
            id: "no-report".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "no-report".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::TestRun,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("report_findings"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_reports_must_be_final_step() {
        let spec = ProcedureSpec {
            id: "report-not-final".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "rnf".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 3,
                    op: CoreOperation::TestRun,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("does not end with a report_findings step"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_multiple_report_findings() {
        let spec = ProcedureSpec {
            id: "two-reports".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "two".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("multiple end-of-flow reports"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_bypass_markers_in_step_args() {
        for marker in BYPASS_MARKERS {
            let spec = ProcedureSpec {
                id: "bypass".to_string(),
                version: PROCEDURE_CONTRACT_VERSION.to_string(),
                title: "bypass".to_string(),
                description: "d".to_string(),
                prerequisites: vec![],
                steps: vec![
                    ProcedureStep {
                        ordinal: 1,
                        op: CoreOperation::DoctorRun,
                        args: vec![marker.to_string()],
                        description: "d".to_string(),
                    },
                    ProcedureStep {
                        ordinal: 2,
                        op: CoreOperation::ReportFindings,
                        args: vec![],
                        description: "d".to_string(),
                    },
                ],
                verification: "v".to_string(),
            };
            match validate_procedure(&spec) {
                Err(ForgeError::ProcedureBypassRefused { reason }) => {
                    assert!(
                        reason.contains(marker),
                        "expected '{marker}' in reason, got: {reason}"
                    );
                }
                other => panic!("expected ProcedureBypassRefused for {marker}, got {other:?}"),
            }
        }
    }

    #[test]
    fn validate_procedure_rejects_bypass_marker_in_keyvalue_form() {
        let spec = ProcedureSpec {
            id: "bypass-kv".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "bypass-kv".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec!["--skip-checks=doctor".to_string()],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureBypassRefused { reason }) => {
                assert!(reason.contains("--skip-checks"));
            }
            other => panic!("expected ProcedureBypassRefused, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_accepts_mixed_case_force_flag() {
        let spec = ProcedureSpec {
            id: "force-mixed".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "f".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec!["--FORCE".to_string()],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureBypassRefused { reason }) => {
                assert!(reason.contains("--force"));
            }
            other => panic!("expected ProcedureBypassRefused, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_empty_step_description() {
        let spec = ProcedureSpec {
            id: "empty-desc".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "ed".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec![],
                    description: "   ".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("empty description"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_too_many_step_args() {
        let mut args = Vec::new();
        for i in 0..(MAX_STEP_ARGS + 1) {
            args.push(format!("--arg-{i}"));
        }
        let spec = ProcedureSpec {
            id: "too-many-args".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "tma".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args,
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("more than"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_empty_arg_value() {
        let spec = ProcedureSpec {
            id: "empty-arg".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "ea".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::DoctorRun,
                    args: vec!["".to_string()],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 2,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("empty argument"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_procedure_rejects_zero_ordinal() {
        let spec = ProcedureSpec {
            id: "zero".to_string(),
            version: PROCEDURE_CONTRACT_VERSION.to_string(),
            title: "z".to_string(),
            description: "d".to_string(),
            prerequisites: vec![],
            steps: vec![
                ProcedureStep {
                    ordinal: 0,
                    op: CoreOperation::DoctorRun,
                    args: vec![],
                    description: "d".to_string(),
                },
                ProcedureStep {
                    ordinal: 1,
                    op: CoreOperation::ReportFindings,
                    args: vec![],
                    description: "d".to_string(),
                },
            ],
            verification: "v".to_string(),
        };
        match validate_procedure(&spec) {
            Err(ForgeError::ProcedureInvalid { reason }) => {
                assert!(reason.contains("ordinal 0"));
            }
            other => panic!("expected ProcedureInvalid, got {other:?}"),
        }
    }

    #[test]
    fn core_operation_round_trips_through_label() {
        for op in [
            CoreOperation::ProfileInspect,
            CoreOperation::ProfileResolve,
            CoreOperation::ProfilePreflight,
            CoreOperation::FeatureResolve,
            CoreOperation::FeatureAdd,
            CoreOperation::FeatureRemove,
            CoreOperation::FeatureUpgrade,
            CoreOperation::ComponentResolve,
            CoreOperation::UiPatternResolve,
            CoreOperation::UiPatternInstall,
            CoreOperation::IntentValidate,
            CoreOperation::IntentResolve,
            CoreOperation::IntentApply,
            CoreOperation::DoctorRun,
            CoreOperation::TestRun,
            CoreOperation::Commit,
            CoreOperation::PolicyRun,
            CoreOperation::SpecGenerate,
            CoreOperation::SpecApply,
            CoreOperation::AgentStart,
            CoreOperation::UpgradeApply,
            CoreOperation::UpgradeFleet,
            CoreOperation::ImportRun,
            CoreOperation::DeployPlan,
            CoreOperation::DeployApply,
            CoreOperation::DeployObserve,
            CoreOperation::ReleasePrepare,
            CoreOperation::ReleaseApply,
            CoreOperation::DocsTranslate,
            CoreOperation::MirrorApply,
            CoreOperation::ReportFindings,
        ] {
            let label = op.label();
            let parsed = CoreOperation::from_label(label);
            assert_eq!(parsed, Some(op), "round-trip failed for {label}");
        }
    }

    #[test]
    fn core_operation_from_label_rejects_unknown_tokens() {
        for label in [
            "push",
            "planner.dispatch",
            "agent.pause",
            "agent.takeover",
            "planner.dispatch",
            "nope",
            "release.bypass",
            "",
        ] {
            assert!(
                CoreOperation::from_label(label).is_none(),
                "expected None for {label}"
            );
        }
    }

    #[test]
    fn report_findings_is_the_only_synthetic_step() {
        assert!(CoreOperation::ReportFindings.is_report());
        for op in [
            CoreOperation::DoctorRun,
            CoreOperation::TestRun,
            CoreOperation::SpecGenerate,
        ] {
            assert!(
                !op.is_report(),
                "{} should not be a report step",
                op.label()
            );
        }
    }

    #[test]
    fn contains_bypass_marker_returns_the_marker() {
        for marker in BYPASS_MARKERS {
            assert_eq!(contains_bypass_marker(marker), Some(*marker));
        }
        assert!(contains_bypass_marker("safe").is_none());
        assert!(contains_bypass_marker("--safe-flag").is_none());
    }

    #[test]
    fn every_catalog_procedure_carries_a_synthetic_project_constant() {
        assert!(procedure_catalog().iter().all(|e| !e.id.is_empty()));
        assert_eq!(PROCEDURE_SYNTHETIC_PROJECT, "__procedure__");
    }

    #[test]
    fn render_list_human_includes_every_procedure_id() {
        let rendered = render_list_human(&procedure_catalog());
        for entry in procedure_catalog() {
            assert!(
                rendered.contains(&entry.id),
                "rendered list missing '{}'",
                entry.id
            );
        }
    }

    #[test]
    fn render_inspect_human_carries_steps_and_verification() {
        let spec = inspect_procedure("create-project").unwrap();
        let rendered = render_inspect_human(&spec);
        assert!(rendered.contains("create-project"));
        assert!(rendered.contains("verification:"));
        assert!(rendered.contains("report_findings"));
    }

    #[test]
    fn catalog_is_platform_neutral() {
        // R1 boundary: the procedure layer carries no
        // agent provider, IDE or model identifier. The
        // simplest test is a string-scan over the catalog.
        let serialized = serde_json::to_string(&procedure_catalog()).unwrap();
        for forbidden in [
            "opencode",
            "codex",
            "claude",
            "openai",
            "anthropic",
            "vscode",
            "jetbrains",
            "cursor",
            "windsurf",
        ] {
            assert!(
                !serialized.to_ascii_lowercase().contains(forbidden),
                "catalog leaked platform identifier '{forbidden}'"
            );
        }
    }

    #[test]
    fn upgrade_procedure_includes_spec_generate_handoff() {
        // R2 success: the upgrade SOP contains a
        // `spec.generate` step so a semantic conflict can
        // route through the existing remediation surface.
        let spec = inspect_procedure("upgrade-project").unwrap();
        let has_spec_generate = spec
            .steps
            .iter()
            .any(|step| step.op == CoreOperation::SpecGenerate);
        assert!(
            has_spec_generate,
            "upgrade-project must include a spec.generate handoff"
        );
    }

    #[test]
    fn every_catalog_procedure_ends_with_report_findings() {
        for spec in all_procedures() {
            let last = spec
                .steps
                .last()
                .unwrap_or_else(|| panic!("procedure '{}' has no steps", spec.id));
            assert!(
                last.op.is_report(),
                "procedure '{}' does not end with report_findings (ends with {})",
                spec.id,
                last.op.label()
            );
        }
    }
}

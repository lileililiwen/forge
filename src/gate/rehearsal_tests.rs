//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
#[allow(clippy::module_inception)]
pub(super) mod tests {
    use crate::gate::contract::{GATE_CONTRACT_VERSION, MAX_PLAN_LINES};
    use crate::gate::engine::{journal_verdict, run_gate};
    use crate::gate::model::{GateAggregate, GateCheckState, GateConfig, GateOutcome};
    use crate::gate::rehearsal::{
        parse_rehearsal, rehearsal_envelope, render_rehearsal_human, verdict_like_keys,
        GateRehearsal,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use tempfile::TempDir;

    fn fixture(name: &str) -> serde_json::Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gate")
            .join(name);
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    /// A real (absolute) scratch path: `capture_note` scrubs the assessed
    /// project's own directory out of every captured string, so a bare
    /// `Path::new(".")` would rewrite every `.` in the contract version.
    fn parse(value: &serde_json::Value) -> Option<GateRehearsal> {
        let tmp = TempDir::new().unwrap();
        parse_rehearsal(value, tmp.path())
    }

    fn rehearsal_fixture() -> GateRehearsal {
        parse(&fixture("gate-rehearsal-plan.json")).expect("the verbatim rehearsal capture parses")
    }

    fn project_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        tmp
    }

    /// A stub runtime that answers `--version` and prints `body` verbatim.
    fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\n{body}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn cat_stub(dir: &Path, name: &str, fixture_name: &str) -> PathBuf {
        let doc = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gate")
            .join(fixture_name)
            .display()
            .to_string();
        stub(dir, name, &format!("cat '{doc}'\nexit 0"))
    }

    fn config(binary: PathBuf) -> GateConfig {
        GateConfig {
            binary: Some(OsString::from(binary)),
            timeout: Duration::from_secs(10),
        }
    }

    /// The live capture on this repository: nine planned checks, all
    /// required, one of them bound in-process, every timeout 60000, and a
    /// prediction that cannot decide.
    #[test]
    fn live_rehearsal_capture_parses_into_the_typed_document() {
        let rehearsal = rehearsal_fixture();
        assert_eq!(rehearsal.contract, "driftwatch-gate-plan/0.1.0");
        assert_eq!(rehearsal.document_kind, "gate-plan-rehearsal");
        assert_eq!(rehearsal.execution_mode, "dry-run");
        assert_eq!(rehearsal.plan_version, 1);
        assert_eq!(rehearsal.profile, "rust-product");
        assert_eq!(rehearsal.rule_pack_version, "workspace-governance@1");
        assert_eq!(rehearsal.manifest_digest, "sha256:53102e32f760136b");
        assert_eq!(rehearsal.default_timeout_ms, 60000);
        assert_eq!(rehearsal.source_size_max_lines, Some(1000));
        assert_eq!(rehearsal.blocking_policy.review_required_blocks, Some(true));
        assert_eq!(rehearsal.required_check_count(), 9);
        assert_eq!(rehearsal.planned_checks.len(), 9);
        let by_id: std::collections::BTreeMap<
            &str,
            &crate::gate::rehearsal::RehearsalPlannedCheck,
        > = rehearsal
            .planned_checks
            .iter()
            .map(|check| (check.id.as_str(), check))
            .collect();
        // The runtime executes this one in-process: the bound command
        // source is a manifest fact, not an outcome.
        assert_eq!(by_id["source-file-size"].command_source, "in-process");
        assert!(by_id["source-file-size"].required);
        assert_eq!(by_id["build"].command_source, "manifest");
        assert_eq!(by_id["tests"].selected_by, "manifest");
        assert!(rehearsal
            .planned_checks
            .iter()
            .all(|check| check.timeout_ms == 60000 && !check.exempt));
        assert!(rehearsal.unbound_required_checks.is_empty());
        assert_eq!(rehearsal.exemption_budget.cap, 2);
        assert_eq!(rehearsal.exemption_budget.declared, 0);
        assert_eq!(rehearsal.exemption_budget.headroom, 2);
        assert_eq!(rehearsal.exemption_budget.required_checks, 9);
        assert_eq!(rehearsal.exemption_budget.verdict, "within");
        assert_eq!(rehearsal.not_scheduled.len(), 5);
        // The prediction is undetermined and says so; it is not a verdict.
        assert_eq!(
            rehearsal.blocking_prediction.kind,
            "undetermined-before-execution"
        );
        assert_eq!(rehearsal.blocking_prediction.derived_from, "resolved-plan");
        assert_eq!(rehearsal.blocking_prediction.would_block, None);
        assert!(rehearsal.blocking_prediction.sufficient_but_not_necessary);
        assert!(rehearsal.blocking_prediction.reasons.is_empty());
        assert_eq!(rehearsal.would_block_label(), "unknown");
        assert!(rehearsal.notice.contains("Nothing was executed"));
    }

    #[test]
    fn human_render_names_every_check_and_the_unknown_prediction() {
        let text = render_rehearsal_human(&rehearsal_fixture());
        for id in [
            "build",
            "declared-verification",
            "governance-quality",
            "placeholder-threshold",
            "product-code-boundary",
            "repository",
            "security",
            "source-file-size",
            "tests",
        ] {
            assert!(text.contains(id), "missing planned check `{id}`:\n{text}");
        }
        assert!(text.contains("[required]"), "{text}");
        assert!(text.contains("command=in-process"), "{text}");
        assert!(text.contains("command=manifest"), "{text}");
        assert!(text.contains("timeout=60000ms"), "{text}");
        assert!(text.contains("review_required_blocks=true"), "{text}");
        assert!(text.contains("sha256:53102e32f760136b"), "{text}");
        assert!(text.contains("workspace-governance@1"), "{text}");
        assert!(text.contains("checks (9 planned)"), "{text}");
        // The prediction may prove a block, never a pass: the wording
        // names the unknown and says the document is not an outcome.
        assert!(text.contains("undetermined-before-execution"), "{text}");
        assert!(
            text.contains(
                "would_block=unknown (the bound commands decide; unknown is never a pass)"
            ),
            "{text}"
        );
        assert!(text.contains("never an observed outcome"), "{text}");
        // No raw document text survives in the render.
        assert!(!text.contains("{\""), "raw JSON leaked:\n{text}");
    }

    /// Every rule of the shape-detection table, one row each. Each must
    /// return `None` so the caller degrades to the bounded plan-line
    /// preview, which is never a verdict either.
    #[test]
    fn detection_requires_all_three_markers_and_a_complete_parse() {
        let base = fixture("gate-rehearsal-plan.json");
        let drop = |key: &str| {
            let mut value = base.clone();
            value.as_object_mut().unwrap().remove(key);
            value
        };
        let cases: Vec<(&str, serde_json::Value)> = vec![
            ("no contract at all", drop("contract")),
            (
                "a foreign contract family",
                serde_json::json!({
                    "contract": "driftwatch-gate-status/0.1.0",
                    "document_kind": "gate-plan-rehearsal",
                    "dry_run": true
                }),
            ),
            ("no document kind", drop("document_kind")),
            (
                "a foreign document kind",
                serde_json::json!({
                    "contract": "driftwatch-gate-plan/0.1.0",
                    "document_kind": "gate-status",
                    "dry_run": true
                }),
            ),
            ("no dry-run marker", drop("dry_run")),
            (
                "a false dry-run marker",
                serde_json::json!({
                    "contract": "driftwatch-gate-plan/0.1.0",
                    "document_kind": "gate-plan-rehearsal",
                    "dry_run": false
                }),
            ),
            (
                "a rehearsal that claims it executed",
                with_field(&base, "executed", serde_json::json!(true)),
            ),
            (
                "a rehearsal that claims it persisted",
                with_field(&base, "persisted", serde_json::json!(true)),
            ),
            (
                "a would_block of false, which the producer never emits",
                with_nested(
                    &base,
                    "blocking_prediction",
                    "would_block",
                    serde_json::json!(false),
                ),
            ),
            ("no planned checks", drop("planned_checks")),
            ("no blocking prediction", drop("blocking_prediction")),
            (
                "a renamed required field",
                renamed_planned_check_field(&base),
            ),
        ];
        for (label, value) in cases {
            assert!(
                parse(&value).is_none(),
                "must not be read as a rehearsal: {label}"
            );
        }
        // The untouched document is the control: it is recognised, and so
        // is an additive sibling release inside the same family (the rule
        // compares the family, not the patch version).
        assert!(parse(&base).is_some());
        assert!(parse(&with_field(
            &base,
            "contract",
            serde_json::json!("driftwatch-gate-plan/0.2.0")
        ))
        .is_some());
    }

    fn with_field(
        base: &serde_json::Value,
        key: &str,
        value: serde_json::Value,
    ) -> serde_json::Value {
        let mut document = base.clone();
        document
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), value);
        document
    }

    fn with_nested(
        base: &serde_json::Value,
        parent: &str,
        key: &str,
        value: serde_json::Value,
    ) -> serde_json::Value {
        let mut document = base.clone();
        document
            .get_mut(parent)
            .and_then(|node| node.as_object_mut())
            .unwrap()
            .insert(key.to_string(), value);
        document
    }

    fn renamed_planned_check_field(base: &serde_json::Value) -> serde_json::Value {
        let mut document = base.clone();
        let checks = document
            .get_mut("planned_checks")
            .and_then(|node| node.as_array_mut())
            .unwrap();
        let first = checks[0].as_object_mut().unwrap();
        let id = first.remove("id").unwrap();
        first.insert("check_id".to_string(), id);
        document
    }

    /// The two document kinds can never be read as each other, and the
    /// real status document still maps exactly as it did before.
    #[test]
    fn the_real_status_documents_are_never_rehearsals_and_still_map_unchanged() {
        use crate::gate::engine::{classify_aggregate, parse_gate_document};
        let cases = [
            ("gate-status-pass.json", GateAggregate::Passed, true),
            ("gate-status-blocked.json", GateAggregate::Blocked, false),
            (
                "gate-status-review-required.json",
                GateAggregate::Blocked,
                false,
            ),
            (
                "gate-status-not-applicable.json",
                GateAggregate::Passed,
                true,
            ),
        ];
        for (name, expected, exit_ok) in cases {
            let value = fixture(name);
            assert!(
                parse_rehearsal(&value, Path::new(".")).is_none(),
                "{name} must never be read as a rehearsal"
            );
            let mapped = parse_gate_document(&value, project_dir().path())
                .unwrap_or_else(|| panic!("{name} must still parse as a status document"));
            assert_eq!(
                classify_aggregate(&mapped.status, mapped.blocked, exit_ok),
                expected,
                "{name} aggregate"
            );
            assert!(!mapped.checks.is_empty(), "{name} check rows");
            assert!(mapped.manifest_digest.is_some(), "{name} digest");
            assert!(mapped.rule_pack_version.is_some(), "{name} rule pack");
        }
        // The pass capture keeps its exact row vocabulary.
        let pass = parse_gate_document(&fixture("gate-status-pass.json"), project_dir().path())
            .expect("status shape");
        assert_eq!(pass.checks.len(), 1);
        assert_eq!(pass.checks[0].id, "docs");
        assert_eq!(pass.checks[0].state, GateCheckState::Pass);
        assert_eq!(pass.checks[0].note, None);
        assert_eq!(
            pass.manifest_digest.as_deref(),
            Some("sha256:dc1fb6a3db59efd5")
        );
        assert_eq!(pass.rule_pack_version.as_deref(), Some("local"));
        // And the rehearsal is not a status document in the other
        // direction: no `blocked` bit, no `results` rows.
        assert!(
            parse_gate_document(&fixture("gate-rehearsal-plan.json"), project_dir().path())
                .is_none(),
            "a rehearsal must never normalise into evidence"
        );
    }

    /// A rehearsal reaches no persistence surface: no evidence record, no
    /// journal verdict that reads as an outcome, no evidence file on disk.
    #[test]
    fn a_rehearsal_is_never_evidence() {
        let tmp = project_dir();
        let script = cat_stub(tmp.path(), "gate-rehearsal.sh", "gate-rehearsal-plan.json");
        let outcome = run_gate(tmp.path(), "demo", &config(script), true);
        let GateOutcome::Rehearsal { rehearsal, .. } = &outcome else {
            panic!("a rehearsal document must map to the rehearsal outcome: {outcome:?}");
        };
        assert_eq!(rehearsal.planned_checks.len(), 9);
        assert_eq!(journal_verdict(&outcome), "rehearsed");
        // Nothing was written: the outcome carries no evidence record at
        // all, and no evidence file was created.
        assert!(
            !tmp.path().join(".forge").exists(),
            "a rehearsal must never persist evidence"
        );
        assert!(!matches!(outcome, GateOutcome::Evidence(_)));
    }

    /// Every degrade row lands on the existing plan-line preview: never
    /// evidence, never a verdict.
    #[test]
    fn unrecognised_shapes_degrade_to_plan_lines_never_to_a_pass() {
        let tmp = project_dir();
        // 1. The older sibling's human plan text (the Oct 5 binary).
        let human = stub(
            tmp.path(),
            "gate-human.sh",
            "printf 'gate plan (dry-run; nothing was executed)\\nchecks:\\n  - build [required] via manifest\\n'\nexit 0",
        );
        let outcome = run_gate(tmp.path(), "demo", &config(human), true);
        let GateOutcome::PlanPreview { plan, .. } = &outcome else {
            panic!("human text must stay a plan preview: {outcome:?}");
        };
        assert!(plan[0].contains("nothing was executed"));
        assert_eq!(journal_verdict(&outcome), "rehearsed");

        // 2. A rehearsal-shaped document from a contract family Forge
        //    does not know: it degrades rather than being interpreted.
        let foreign = stub(
            tmp.path(),
            "gate-foreign.sh",
            "printf '%s\\n' '{\"contract\":\"driftwatch-gate-status/0.1.0\",\"document_kind\":\"gate-plan-rehearsal\",\"dry_run\":true,\"executed\":false,\"persisted\":false,\"planned_checks\":[],\"blocking_prediction\":{\"kind\":\"undetermined-before-execution\"}}'\nexit 0",
        );
        let outcome = run_gate(tmp.path(), "demo", &config(foreign), true);
        assert!(
            matches!(outcome, GateOutcome::PlanPreview { .. }),
            "{outcome:?}"
        );
        assert!(!matches!(outcome, GateOutcome::Evidence(_)));

        // 3. Output that is not JSON at all.
        let noise = stub(
            tmp.path(),
            "gate-noise.sh",
            "echo 'driftwatch gate: nothing to gate.'\nexit 0",
        );
        let outcome = run_gate(tmp.path(), "demo", &config(noise), true);
        assert!(
            matches!(outcome, GateOutcome::PlanPreview { .. }),
            "{outcome:?}"
        );
    }

    /// A rehearsal answering a real run is a contradiction, reported as
    /// unavailable and never as evidence.
    #[test]
    fn a_rehearsal_answering_a_real_run_is_unavailable_never_evidence() {
        let tmp = project_dir();
        let script = cat_stub(tmp.path(), "gate-rehearsal.sh", "gate-rehearsal-plan.json");
        let outcome = run_gate(tmp.path(), "demo", &config(script.clone()), false);
        let GateOutcome::Unavailable { reason } = &outcome else {
            panic!("a rehearsal must never answer a real run: {outcome:?}");
        };
        assert!(reason.contains("rehearsal"), "{reason}");
        assert!(reason.contains("never evidence"), "{reason}");
        assert!(!matches!(outcome, GateOutcome::Evidence(_)));
        assert!(!tmp.path().join(".forge").exists());
        assert_eq!(journal_verdict(&outcome), "failed");
    }

    /// The envelope Forge actually emits carries no verdict-shaped key at
    /// any depth, so no consumer can read a rehearsal as a run.
    #[test]
    fn the_emitted_envelope_carries_no_verdict_shaped_key() {
        let rehearsal = rehearsal_fixture();
        let envelope = rehearsal_envelope(
            "forge",
            "driftwatchdog",
            Some("driftwatch 0.1.0"),
            &rehearsal,
        );
        assert!(verdict_like_keys(&envelope).is_empty(), "{envelope}");
        assert_eq!(envelope["dry_run"], true);
        assert_eq!(envelope["contract"], GATE_CONTRACT_VERSION);
        assert_eq!(envelope["gate"]["persisted"], false);
        assert_eq!(envelope["gate"]["journaled"], false);
        assert_eq!(envelope["gate"]["project"], "forge");
        assert_eq!(
            envelope["gate"]["rehearsal"]["planned_checks"]
                .as_array()
                .unwrap()
                .len(),
            9
        );
        // The plan-line list is retained for consumers that only ever
        // read the preview, and it carries no raw document text.
        let plan = envelope["gate"]["plan"].as_array().unwrap();
        assert!(plan.len() > 9);
        assert!(plan
            .iter()
            .all(|line| !line.as_str().unwrap_or("").contains("{\"")));
    }

    /// The human surface is bounded, and says so when it truncates.
    #[test]
    fn the_rendered_check_block_is_bounded_and_says_so() {
        let mut document = fixture("gate-rehearsal-plan.json");
        let checks = document["planned_checks"].as_array().unwrap().clone();
        let many: Vec<serde_json::Value> = (0..(MAX_PLAN_LINES + 7))
            .map(|index| {
                let mut row = checks[0].clone();
                row["id"] = serde_json::json!(format!("check-{index}"));
                row
            })
            .collect();
        document["planned_checks"] = serde_json::json!(many);
        let rehearsal = parse(&document).expect("still a complete rehearsal");
        let text = render_rehearsal_human(&rehearsal);
        assert!(text.contains(&format!("checks ({} planned)", MAX_PLAN_LINES + 7)));
        assert!(text.contains("7 more planned check(s) not shown"), "{text}");
    }
}

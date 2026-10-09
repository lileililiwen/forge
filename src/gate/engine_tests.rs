//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::gate::contract::{
        GATE_BIN_ENV, GATE_CONTRACT_VERSION, GATE_EVIDENCE_DIR, GATE_EVIDENCE_FILE, MAX_NOTE_CHARS,
    };
    use crate::gate::engine::{
        bound_note, classify_aggregate, declared_gate_runtime, evidence_freshness, evidence_path,
        is_gate_managed, journal_verdict, load_latest_evidence, parse_gate_document,
        parse_timeout_secs, resolve_runtime, run_gate, save_evidence,
    };
    use crate::gate::model::{
        DeclaredRuntime, GateAggregate, GateCheck, GateCheckState, GateConfig, GateEvidence,
        GateFreshness, GateOutcome,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use tempfile::TempDir;

    fn project_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        tmp
    }

    fn executable(path: &Path, script: &str) {
        fs::write(path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_fixture_script(tmp: &TempDir, name: &str, body: &str) -> PathBuf {
        let path = tmp.path().join(name);
        executable(
            &path,
            &format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\n{body}\n"),
        );
        path
    }

    fn fixture_value(name: &str) -> serde_json::Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gate")
            .join(name);
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn explicit_override_wins_and_never_falls_through() {
        let tmp = project_dir();
        let config = GateConfig {
            binary: Some(OsString::from("/definitely/not/a/gate-runtime-xyz")),
            timeout: Duration::from_secs(2),
        };
        let resolved = resolve_runtime(tmp.path(), &config).expect("override resolves as named");
        assert_eq!(resolved.name, "gate-runtime-xyz");
        assert!(resolved.attempts[0].contains(GATE_BIN_ENV));
        // A dead override surfaces the spawn failure honestly.
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        assert!(matches!(outcome, GateOutcome::Unavailable { .. }));
    }

    #[test]
    fn declared_unknown_runtime_is_refused_by_name() {
        let tmp = project_dir();
        fs::write(
            tmp.path().join(".project.json"),
            "{\"schema_version\":1,\"id\":\"x\",\"verification\":{\"gate_runtime\":\"jenkins-gate\"}}",
        )
        .unwrap();
        let config = GateConfig {
            binary: None,
            timeout: Duration::from_secs(2),
        };
        let err = resolve_runtime(tmp.path(), &config).unwrap_err();
        assert!(err.contains("jenkins-gate"), "{err}");
        assert!(err.contains("driftwatchdog"), "{err}");
    }

    #[test]
    fn declared_known_runtime_and_fallback_use_the_ordered_probe() {
        let tmp = project_dir();
        fs::write(
            tmp.path().join(".project.json"),
            "{\"schema_version\":1,\"id\":\"x\",\"verification\":{\"gate_runtime\":\"driftwatchdog\"}}",
        )
        .unwrap();
        // With an empty PATH nothing resolves, and the attempts say so.
        let config = GateConfig {
            binary: None,
            timeout: Duration::from_secs(2),
        };
        // (Runs with the ambient PATH; a resolution is environment
        // dependent, so assert only the refusal text shape.)
        match resolve_runtime(tmp.path(), &config) {
            Ok(resolved) => assert!(resolved
                .attempts
                .iter()
                .any(|a| a.contains("declared gate_runtime `driftwatchdog`"))),
            Err(reason) => assert!(reason.contains("no gate runtime resolvable"), "{reason}"),
        }
    }

    #[test]
    fn unreadable_declaration_still_probes_with_honest_attempts() {
        let tmp = project_dir();
        fs::write(tmp.path().join(".project.json"), "not json at all").unwrap();
        assert!(matches!(
            declared_gate_runtime(tmp.path()),
            DeclaredRuntime::Unreadable(_)
        ));
    }

    #[test]
    fn passing_document_maps_to_fresh_able_evidence() {
        let value = fixture_value("gate-status-pass.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        let aggregate = classify_aggregate(&doc.status, doc.blocked, true);
        assert_eq!(aggregate, GateAggregate::Passed);
        assert_eq!(doc.checks.len(), 1);
        assert_eq!(doc.checks[0].id, "docs");
        assert_eq!(doc.checks[0].state, GateCheckState::Pass);
        assert_eq!(
            doc.manifest_digest.as_deref(),
            Some("sha256:dc1fb6a3db59efd5")
        );
        assert_eq!(doc.rule_pack_version.as_deref(), Some("local"));
    }

    #[test]
    fn blocked_document_maps_with_real_evidence_codes() {
        let value = fixture_value("gate-status-blocked.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        let aggregate = classify_aggregate(&doc.status, doc.blocked, false);
        assert_eq!(aggregate, GateAggregate::Blocked);
        assert_eq!(doc.checks[0].state, GateCheckState::Fail);
        let note = doc.checks[0].note.as_deref().unwrap_or("");
        assert!(note.contains("diagnostic: exit 1"), "{note}");
        assert!(
            note.contains("remediation: Fix the failing command"),
            "{note}"
        );
    }

    #[test]
    fn review_required_never_counts_as_pass() {
        let value = serde_json::json!({
            "blocked": true, "status": "FAIL",
            "results": [{"gate_id": "ux-review", "status": "REVIEW_REQUIRED", "severity": "warning"}]
        });
        let doc = parse_gate_document(&value, Path::new(".")).expect("shape");
        assert_eq!(doc.checks[0].state, GateCheckState::Unresolved);
        assert_eq!(
            classify_aggregate("FAIL", true, false),
            GateAggregate::Blocked
        );
    }

    #[test]
    fn real_review_required_fixture_blocks_without_inventing_pass() {
        // Verbatim sibling capture: top-level status REVIEW_REQUIRED,
        // blocked=true, and a NOT_APPLICABLE row for the pending review.
        let value = fixture_value("gate-status-review-required.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        assert_eq!(
            classify_aggregate(&doc.status, doc.blocked, false),
            GateAggregate::Blocked
        );
        let by_id: std::collections::BTreeMap<&str, &GateCheck> =
            doc.checks.iter().map(|c| (c.id.as_str(), c)).collect();
        assert_eq!(by_id["docs"].state, GateCheckState::Pass);
        assert_eq!(by_id["ux-review"].state, GateCheckState::NotApplicable);
    }

    #[test]
    fn non_blocking_review_required_is_unknown_never_passed() {
        // review_required_blocks=false keeps exit 0; a pending review is
        // still not a passed gate, and neither is any future status.
        assert_eq!(
            classify_aggregate("REVIEW_REQUIRED", false, true),
            GateAggregate::Unknown
        );
        assert_eq!(
            classify_aggregate("SOMETHING_NEW", false, true),
            GateAggregate::Unknown
        );
    }

    #[test]
    fn real_not_applicable_fixture_passes_with_explicit_na_row() {
        // Verbatim sibling capture: status PASS, blocked=false, one PASS
        // row and one NOT_APPLICABLE row for an opted-out concern.
        let value = fixture_value("gate-status-not-applicable.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        assert_eq!(
            classify_aggregate(&doc.status, doc.blocked, true),
            GateAggregate::Passed
        );
        let by_id: std::collections::BTreeMap<&str, &GateCheck> =
            doc.checks.iter().map(|c| (c.id.as_str(), c)).collect();
        assert_eq!(by_id["build"].state, GateCheckState::Pass);
        assert_eq!(by_id["deploy"].state, GateCheckState::NotApplicable);
        assert!(by_id["deploy"]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("missing=deploy:command"));
    }

    #[test]
    fn not_applicable_and_unknown_states_are_distinct_and_never_pass() {
        let value = serde_json::json!({
            "blocked": false, "status": "PASS",
            "results": [
                {"gate_id": "deploy", "status": "NOT_APPLICABLE"},
                {"gate_id": "future", "status": "WEIRD_FUTURE_STATE"}
            ]
        });
        let doc = parse_gate_document(&value, Path::new(".")).expect("shape");
        assert_eq!(doc.checks[0].state, GateCheckState::NotApplicable);
        assert_eq!(doc.checks[1].state, GateCheckState::Unresolved);
        assert!(doc.checks[1]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("never counts as pass"));
    }

    #[test]
    fn passing_document_with_nonzero_exit_downgrades_to_unknown() {
        // A parseable document wins over the exit code for *classification
        // as evidence*, but a PASS claim riding a failure exit is a
        // contradiction and never produces a pass verdict.
        assert_eq!(
            classify_aggregate("PASS", false, false),
            GateAggregate::Unknown
        );
        assert_eq!(
            classify_aggregate("PASS", false, true),
            GateAggregate::Passed
        );
        assert_eq!(
            classify_aggregate("FAIL", false, true),
            GateAggregate::Failed
        );
    }

    #[test]
    fn real_run_executes_gate_surface_and_records_evidence() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-ok.sh",
            "cat <<'EOF'\n{\"status\":\"PASS\",\"blocked\":false,\"failures\":[],\"pending_reviews\":[],\"not_applicable\":[],\"manifest_digest\":\"sha256:abc\",\"rule_pack_version\":\"local\",\"results\":[{\"gate_id\":\"build\",\"status\":\"PASS\"}]}\nEOF\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Evidence(evidence) = outcome else {
            panic!("parseable document must map to evidence: {outcome:?}");
        };
        assert_eq!(evidence.aggregate, GateAggregate::Passed);
        assert_eq!(evidence.project_id, "demo");
        assert_eq!(evidence.runtime, "gate-ok");
        assert_eq!(
            evidence.runtime_version.as_deref(),
            Some("driftwatch 0.1.0")
        );
        assert!(!evidence.dry_run);
    }

    #[test]
    fn dry_run_reports_plan_preview_without_evidence() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-plan.sh",
            "printf 'gate plan (dry-run; nothing was executed)\\ncontract version: 1 | profile: minimal\\nchecks:\\n  - docs [required] via project-runtime\\n'\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, true);
        let GateOutcome::PlanPreview { plan, .. } = outcome else {
            panic!("plan text must map to a preview: {outcome:?}");
        };
        assert!(plan[0].contains("nothing was executed"));
        assert_eq!(plan.len(), 4);
    }

    #[test]
    fn unparseable_real_run_is_unavailable_and_names_status() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-text.sh",
            "echo 'driftwatch gate: no gate.toml or .ai-gate/gate.yaml; nothing to gate.'\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Unavailable { reason } = outcome else {
            panic!("no document is never evidence: {outcome:?}");
        };
        // The runtime answered with exit 0 text: the note is honest about
        // the missing document.
        assert!(
            reason.contains("nothing to gate") || reason.contains("no parseable"),
            "{reason}"
        );
    }

    #[test]
    fn timeout_is_bounded_and_names_the_wait() {
        let tmp = project_dir();
        let script = write_fixture_script(&tmp, "gate-hang.sh", "sleep 5\nexit 0\n");
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_millis(300),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Unavailable { reason } = outcome else {
            panic!("hang is unavailable: {outcome:?}");
        };
        assert!(reason.contains("timeout"), "{reason}");
    }

    #[test]
    fn secrets_and_bounds_apply_to_every_captured_string() {
        let noisy = format!(
            "diagnostic: exit 1 with token=ghp_{} for CI at /home/operator/secret/project",
            "A".repeat(400)
        );
        let bounded = bound_note(&noisy);
        assert!(
            bounded.contains("[REDACTED]") || !bounded.contains("ghp_"),
            "{bounded}"
        );
        assert!(
            bounded.chars().count() <= MAX_NOTE_CHARS + 32,
            "{}",
            bounded.len()
        );
    }

    #[test]
    fn evidence_round_trips_atomically_and_rehearsals_refuse_to_persist() {
        let tmp = project_dir();
        let evidence = GateEvidence {
            contract: GATE_CONTRACT_VERSION.to_string(),
            project_id: "demo".to_string(),
            runtime: "driftwatchdog".to_string(),
            runtime_version: Some("driftwatch 0.1.0".to_string()),
            revision: Some("abc123def456abc".to_string()),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: vec![GateCheck {
                id: "build".to_string(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-09-24T00:00:00Z".to_string(),
            dry_run: false,
            note: None,
        };
        let relative = save_evidence(tmp.path(), &evidence).unwrap();
        assert_eq!(relative, evidence_path("demo"));
        assert_eq!(
            load_latest_evidence(tmp.path()).unwrap().as_ref(),
            Some(&evidence)
        );
        // tmp artifact is gone after the rename.
        assert!(!tmp
            .path()
            .join(GATE_EVIDENCE_DIR)
            .join("demo")
            .join("evidence.json.tmp")
            .exists());
        let mut rehearsal = evidence.clone();
        rehearsal.dry_run = true;
        assert!(save_evidence(tmp.path(), &rehearsal).is_err());
    }

    #[test]
    fn corrupt_evidence_names_the_file_and_never_invents_absence() {
        let tmp = project_dir();
        fs::create_dir_all(tmp.path().join(GATE_EVIDENCE_DIR).join("demo")).unwrap();
        fs::write(
            tmp.path()
                .join(GATE_EVIDENCE_DIR)
                .join("demo")
                .join(GATE_EVIDENCE_FILE),
            "{ not json",
        )
        .unwrap();
        let err = load_latest_evidence(tmp.path()).unwrap_err();
        assert_eq!(err.code(), "gate-invalid");
        assert!(err.to_string().contains("evidence.json"), "{err}");
    }

    #[test]
    fn freshness_requires_an_exact_revision_binding() {
        let evidence = GateEvidence {
            contract: GATE_CONTRACT_VERSION.to_string(),
            project_id: "demo".to_string(),
            runtime: "driftwatchdog".to_string(),
            runtime_version: None,
            revision: Some("abc".to_string()),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: Vec::new(),
            observed_at: "now".to_string(),
            dry_run: false,
            note: None,
        };
        assert_eq!(
            evidence_freshness(&evidence, Some("abc")),
            GateFreshness::Fresh
        );
        assert_eq!(
            evidence_freshness(&evidence, Some("moved")),
            GateFreshness::Stale
        );
        assert_eq!(evidence_freshness(&evidence, None), GateFreshness::Stale);
        let mut unbound = evidence.clone();
        unbound.revision = None;
        assert_eq!(
            evidence_freshness(&unbound, Some("abc")),
            GateFreshness::Stale
        );
    }

    #[test]
    fn journal_verdicts_follow_the_design_mapping() {
        let evidence = |aggregate| {
            GateOutcome::Evidence(GateEvidence {
                contract: GATE_CONTRACT_VERSION.to_string(),
                project_id: "demo".to_string(),
                runtime: "driftwatchdog".to_string(),
                runtime_version: None,
                revision: None,
                manifest_digest: None,
                rule_pack_version: None,
                aggregate,
                checks: Vec::new(),
                observed_at: "now".to_string(),
                dry_run: false,
                note: None,
            })
        };
        assert_eq!(journal_verdict(&evidence(GateAggregate::Passed)), "done");
        assert_eq!(
            journal_verdict(&evidence(GateAggregate::Blocked)),
            "blocked"
        );
        assert_eq!(journal_verdict(&evidence(GateAggregate::Failed)), "failed");
        assert_eq!(journal_verdict(&evidence(GateAggregate::Unknown)), "failed");
        assert_eq!(
            journal_verdict(&GateOutcome::Unavailable {
                reason: "x".to_string()
            }),
            "failed"
        );
    }

    #[test]
    fn timeout_bounds_refuse_before_anything_runs() {
        assert_eq!(parse_timeout_secs(0).unwrap_err().code(), "gate-invalid");
        assert_eq!(
            parse_timeout_secs(86_401).unwrap_err().code(),
            "gate-invalid"
        );
        assert!(parse_timeout_secs(86_400).is_ok());
    }

    #[test]
    fn gate_managed_detection_matches_the_policy_surface() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_gate_managed(tmp.path()));
        fs::write(tmp.path().join("gate.toml"), "[gate]\nrules = []\n").unwrap();
        assert!(is_gate_managed(tmp.path()));
    }
}

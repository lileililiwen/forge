//! Provider integration evidence tests.

#[cfg(test)]
pub(super) mod tests {
    use super::super::contract::{PROVIDER_CONTRACT_VERSION, PROVIDER_IDS};
    use super::super::matrix::{
        default_binary_for, display_for, inspect, matrix, parse_provider, provider_ids,
        redact_provider_evidence,
    };
    use super::super::model::{ProviderRow, ProviderStatus, RunOptions};
    use super::super::probes::{probe_version, teardown_probe};
    use super::super::runner::{
        render_descriptor_human, render_matrix_human, render_row_human, run_controlled,
        run_controlled_temp,
    };
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn write_fixture(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn temp_case(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("forge-provider-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn provider_ids_are_stable_and_ordered() {
        assert_eq!(
            provider_ids(),
            vec![
                "driftwatch-policy",
                "gate-runtime",
                "oidc-identity",
                "analytics",
                "deploy",
                "release",
            ]
        );
    }

    #[test]
    fn parse_provider_refuses_unknown_with_typed_code() {
        let err = parse_provider("nosuch").unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
    }

    #[test]
    fn driftwatch_probe_defaults_share_the_policy_candidate_order() {
        // One ordered definition (`policy::DRIFTWATCH_BINARY_CANDIDATES`);
        // the rendered pipe form stays byte-compatible with the previous
        // table literals.
        let rendered = crate::policy::DRIFTWATCH_BINARY_CANDIDATES.join("|");
        assert_eq!(rendered, "driftwatchdog|driftwatch");
        assert_eq!(
            default_binary_for("driftwatch-policy").as_deref(),
            Some("driftwatchdog|driftwatch")
        );
        assert_eq!(
            default_binary_for("gate-runtime").as_deref(),
            Some("driftwatchdog|driftwatch")
        );
    }

    #[test]
    fn parse_provider_trims_and_accepts_known() {
        assert_eq!(parse_provider("  deploy ").unwrap(), "deploy");
    }

    #[test]
    fn matrix_defaults_to_not_run_without_live() {
        let report = matrix(false);
        assert_eq!(report.contract, PROVIDER_CONTRACT_VERSION);
        assert_eq!(report.rows.len(), 6);
        assert_eq!(report.not_run, 6);
        assert_eq!(report.supported, 0);
        for row in &report.rows {
            assert_eq!(row.status, "not-run");
            assert!(row.provenance.is_none());
            assert!(row.reason.contains("forge provider run"));
        }
    }

    #[test]
    fn run_without_flags_is_not_run_and_contacts_nothing() {
        let dir = temp_case("not-run");
        let row = run_controlled(
            "driftwatch-policy",
            &RunOptions::default(),
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "not-run");
        assert!(row.provenance.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_refuses_unknown_provider_with_typed_code() {
        let dir = temp_case("unknown");
        let err = run_controlled(
            "nosuch",
            &RunOptions::default(),
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_refuses_bad_project_id_before_any_probe() {
        let dir = temp_case("bad-project");
        let err = run_controlled(
            "deploy",
            &RunOptions {
                fixture: Some(PathBuf::from("/nonexistent")),
                ..Default::default()
            },
            "BAD_ID",
            &dir,
            "owner/repo",
        )
        .unwrap_err();
        assert_eq!(err.code(), "provider-invalid");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_without_opt_in_flag_is_not_run() {
        let dir = temp_case("live-gate");
        let row = run_controlled(
            "deploy",
            &RunOptions {
                live: true,
                ..Default::default()
            },
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        // Either the environment enables live (then a missing binary
        // yields unavailable) or the gate holds (not-run). Neither is
        // ever reported as supported without a real round trip.
        assert_ne!(row.status, "supported");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn redact_delegates_to_policy_redactor() {
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let redacted = redact_provider_evidence(&format!("token {secret} used"));
        assert!(!redacted.contains(secret));
        assert!(redacted.contains("[REDACTED]"));
        assert_eq!(
            redacted,
            crate::policy::redact_credentials(&format!("token {secret} used"))
        );
    }

    #[test]
    fn policy_fixture_success_records_fixture_provenance() {
        let dir = temp_case("policy-good");
        let bin = write_fixture(
            &dir,
            "good-driftwatch.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"driftwatch 9.9.9\"; exit 0; fi\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
        );
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        assert_eq!(provenance.sandbox, "fixture");
        assert!(provenance.source.starts_with("fixture:"));
        assert_eq!(provenance.project_id.as_deref(), Some("evidence-probe"));
        assert!(provenance.teardown);
        assert!(provenance.observed_at.contains('T'));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_failure_is_unavailable_without_success() {
        let dir = temp_case("policy-bad");
        let bin = write_fixture(&dir, "bad.sh", "#!/bin/sh\necho boom >&2\nexit 3\n");
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.provenance.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_malformed_output_is_unavailable() {
        let dir = temp_case("policy-malformed");
        let bin = write_fixture(&dir, "malformed.sh", "#!/bin/sh\necho 'not json{{'\n");
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn policy_fixture_leak_is_redacted_in_receipt() {
        let dir = temp_case("policy-leak");
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let bin = write_fixture(
            &dir,
            "leaky.sh",
            &format!("#!/bin/sh\necho '{{\"tool\":\"driftwatch\",\"note\":\"{secret}\"}}'\n"),
        );
        let row = run_controlled_temp(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        for line in &provenance.receipt {
            assert!(!line.contains(secret), "secret leaked in receipt");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn analytics_fixture_mapping_mismatch_is_ambiguous_not_success() {
        let dir = temp_case("analytics-mismatch");
        let bin = write_fixture(
            &dir,
            "mismatch.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"other/repo\",\"evidence\":[]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.evidence.iter().any(|e| e.contains("ambiguous-mapping")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn analytics_fixture_success_is_project_scoped() {
        let dir = temp_case("analytics-good");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[\"stars=42\"]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert!(row.evidence.iter().any(|e| e.contains("stars=42")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deploy_fixture_partial_and_failed_states() {
        let dir = temp_case("deploy-states");
        let good = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\ncat >/dev/null\necho '{\"contract\":\"forge-deploy-executor/0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(good),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let stale = write_fixture(
            &dir,
            "stale-contract.sh",
            "#!/bin/sh\ncat >/dev/null\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(stale),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(
            row.evidence
                .iter()
                .any(|e| e.contains("forge-deploy-executor/0.1.0")),
            "the mismatch refusal must name the expected contract: {:?}",
            row.evidence
        );
        let bad = write_fixture(&dir, "bad.sh", "#!/bin/sh\ncat >/dev/null\nexit 1\n");
        let row = run_controlled_temp(
            "deploy",
            &RunOptions {
                fixture: Some(bad),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_fixture_split_stages_stay_partial() {
        let dir = temp_case("release-partial");
        // One binary serves both stages: package delivers, container fails.
        let bin = write_fixture(
            &dir,
            "split.sh",
            "#!/bin/sh\nfor a in \"$@\"; do if [ \"$a\" = \"container\" ]; then echo failed >&2; exit 1; fi; done\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "release",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "unavailable");
        assert!(row.evidence.iter().any(|e| e.contains("partial")));
        assert!(row.evidence.iter().any(|e| e.contains("package:delivered")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_fixture_all_delivered_is_supported() {
        let dir = temp_case("release-good");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"contract\":\"0.1.0\",\"status\":\"delivered\",\"evidence\":[],\"note\":\"ok\"}'\n",
        );
        let row = run_controlled_temp(
            "release",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert!(row.evidence.iter().any(|e| e.contains("package:delivered")));
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("container:delivered")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn identity_fixture_lifecycle_terminates_probe_session() {
        let dir = temp_case("identity-good");
        let row = run_controlled_temp(
            "oidc-identity",
            &RunOptions {
                fixture: Some(PathBuf::from("in-memory")),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        let provenance = row.provenance.as_ref().unwrap();
        assert_eq!(provenance.sandbox, "fixture");
        assert!(provenance.teardown);
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("cross-project:refused")));
        assert!(row
            .evidence
            .iter()
            .any(|e| e.contains("session:terminated")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn teardown_probe_removes_marker_and_reports_clean() {
        let dir = temp_case("teardown");
        assert!(teardown_probe(&dir));
        fs::write(dir.join(".forge-provider-probe"), "probe").unwrap();
        assert!(teardown_probe(&dir));
        assert!(!dir.join(".forge-provider-probe").exists());
        assert!(teardown_probe(Path::new("/nonexistent-forge-provider-dir")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_revision_is_none_for_temp_dirs() {
        // Temp probe dirs carry no VCS revision; provenance must say
        // `unversioned` in human output rather than inventing a SHA.
        let dir = temp_case("revision");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[]}'\n",
        );
        let row = run_controlled_temp(
            "analytics",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            "owner/repo",
        )
        .unwrap();
        let provenance = row.provenance.as_ref().unwrap();
        assert!(provenance.revision.is_none());
        let human = render_row_human(&row);
        assert!(human.contains("unversioned"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_probe_output_is_redacted_before_evidence() {
        // A fixture answers every invocation (including `--version`)
        // with its payload; the captured version string must not carry
        // a credential into the receipt or the evidence line.
        let dir = temp_case("version-leak");
        let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let bin = write_fixture(
            &dir,
            "leaky-version.sh",
            &format!("#!/bin/sh\necho 'driftwatch 9.9.9 {secret}'\n"),
        );
        let version = probe_version(&bin).unwrap();
        assert!(!version.contains(secret));
        assert!(version.contains("[REDACTED]"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeated_runs_stay_attributable_to_their_own_observations() {
        // R1 repeat scenario: the same check repeated for the same
        // project revision returns independent rows, each attributable
        // to its own observation timestamp; neither overwrites the
        // other's evidence.
        let dir = temp_case("repeat");
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[]}'\n",
        );
        let options = RunOptions {
            fixture: Some(bin),
            ..Default::default()
        };
        let first =
            run_controlled_temp("analytics", &options, "evidence-probe", "owner/repo").unwrap();
        let second =
            run_controlled_temp("analytics", &options, "evidence-probe", "owner/repo").unwrap();
        assert_eq!(first.status, "supported");
        assert_eq!(second.status, "supported");
        let first_at = first.provenance.as_ref().unwrap().observed_at.clone();
        let second_at = second.provenance.as_ref().unwrap().observed_at.clone();
        assert!(!first_at.is_empty() && !second_at.is_empty());
        assert_eq!(first.evidence, second.evidence);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_revision_binds_provenance_when_the_workdir_is_versioned() {
        // A versioned workdir binds the recorded revision to the VCS
        // HEAD so a later tree movement reads as stale against the
        // recorded observation instead of silently reusing it.
        let dir = temp_case("revision-git");
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("git must run")
        };
        if !git(&["init"]).status.success() {
            return;
        }
        fs::write(dir.join("probe.txt"), "v1").unwrap();
        git(&["add", "."]);
        let commit = git(&[
            "-c",
            "user.email=evidence@example.com",
            "-c",
            "user.name=evidence",
            "commit",
            "-m",
            "probe",
        ]);
        if !commit.status.success() {
            return;
        }
        let head = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_string();
        assert!(!head.is_empty());
        let bin = write_fixture(
            &dir,
            "good.sh",
            "#!/bin/sh\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
        );
        let row = run_controlled(
            "driftwatch-policy",
            &RunOptions {
                fixture: Some(bin),
                ..Default::default()
            },
            "evidence-probe",
            &dir,
            "owner/repo",
        )
        .unwrap();
        assert_eq!(row.status, "supported");
        assert_eq!(
            row.provenance.as_ref().unwrap().revision.as_deref(),
            Some(head.as_str())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabled_rows_round_trip_through_the_schema_without_probes() {
        // Probes emit only supported/unavailable/not-run; `disabled`
        // stays owned by the manifest adapters. The schema still
        // classifies and renders a disabled row so a future targeted
        // run can report it without a shape change.
        let row = ProviderRow {
            contract: PROVIDER_CONTRACT_VERSION.to_string(),
            provider: "analytics".to_string(),
            display: display_for("analytics").to_string(),
            status: ProviderStatus::Disabled.id().to_string(),
            reason: "analytics block is disabled; provider is not contacted".to_string(),
            provenance: None,
            evidence: Vec::new(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: ProviderRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back, row);
        assert!(render_row_human(&back).contains("disabled"));
    }

    #[test]
    fn human_renderers_carry_required_fields() {
        let report = matrix(false);
        let human = render_matrix_human(&report);
        assert!(human.contains("supported=0"));
        assert!(human.contains("not-run=6"));
        for id in PROVIDER_IDS {
            assert!(human.contains(id));
        }
        let descriptor = inspect("deploy").unwrap();
        let human = render_descriptor_human(&descriptor);
        assert!(human.contains("teardown_rule"));
        assert!(human.contains("secret_rule"));
    }

    #[test]
    fn inspect_describes_every_provider_without_probing() {
        for id in PROVIDER_IDS {
            let descriptor = inspect(id).unwrap();
            assert_eq!(descriptor.contract, PROVIDER_CONTRACT_VERSION);
            assert!(!descriptor.boundary.is_empty());
        }
        assert!(inspect("nosuch").is_err());
    }
}

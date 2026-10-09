//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::policy::constants::{DRIFTWATCH_BINARY_CANDIDATES, POLICY_CONTRACT_VERSION};
    use crate::policy::driftwatch::{
        first_binary_on_path, map_checker_document, map_gate_document, observation_is_stale,
        policy_surface_args, run_driftwatch, source_revision_for,
    };
    use crate::policy::model::{
        DriftWatchConfig, PolicyFinding, PolicyObservation, PolicyOutcome, PolicyReport,
        PolicySeverity,
    };
    use crate::policy::redaction::{redact_credentials, redact_report_in_place};
    use chrono::{DateTime, Utc};
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    fn empty_report() -> PolicyReport {
        PolicyReport {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            contract: POLICY_CONTRACT_VERSION.to_string(),
            source_revision: None,
            findings: Vec::new(),
        }
    }

    fn write_script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn script_config(path: &Path) -> DriftWatchConfig {
        DriftWatchConfig {
            binary: Some(path.as_os_str().to_os_string()),
            timeout: Duration::from_secs(5),
        }
    }

    fn fixture_doc(name: &str) -> serde_json::Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/driftwatch")
            .join(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()));
        serde_json::from_str(&text).expect("fixture is valid JSON")
    }

    #[test]
    fn verbatim_sibling_fixtures_project_honestly() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let target = OsStr::new("/usr/bin/driftwatchdog");

        // The passing document maps to an honest empty report.
        match map_checker_document(
            fixture_doc("checker-report-passing.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                assert!(report.findings.is_empty(), "{:?}", report.findings);
                assert_eq!(report.tool, "driftwatchdog");
                assert_eq!(report.tool_version, "0.1.0", "document version wins");
                assert_eq!(report.contract, POLICY_CONTRACT_VERSION);
            }
            other => panic!("passing fixture must report: {other:?}"),
        }

        // The alerting document: error severity fails, declared category
        // survives, and the fixture's embedded GitHub PAT is redacted.
        match map_checker_document(
            fixture_doc("checker-report-alerting.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                // Mixed outcomes in one document: alerting maps to its
                // alert findings and the broken checker stays visible as
                // its own fail finding — no crash, no masking.
                assert_eq!(report.findings.len(), 2, "{:?}", report.findings);
                let finding = report
                    .findings
                    .iter()
                    .find(|f| f.id == "auth/AUTH-001")
                    .expect("alert finding");
                assert_eq!(finding.severity, PolicySeverity::Fail);
                assert_eq!(finding.category, "security");
                assert!(!finding.message.contains("ghp_"), "{}", finding.message);
                assert!(
                    finding.message.contains("[REDACTED]"),
                    "{}",
                    finding.message
                );
                let broken = report
                    .findings
                    .iter()
                    .find(|f| f.id == "broken")
                    .expect("isolated failure stays visible");
                assert_eq!(broken.severity, PolicySeverity::Fail);
                assert!(broken.message.contains("protocol-error"), "{broken:?}");
            }
            other => panic!("alerting fixture must report: {other:?}"),
        }

        // The blocked gate document produces the failing gate's findings.
        match map_gate_document(
            fixture_doc("gate-status-blocked.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                let finding = report
                    .findings
                    .iter()
                    .find(|f| f.id == "docs")
                    .expect("blocked gate names its failing check");
                assert_eq!(finding.severity, PolicySeverity::Fail);
                assert!(
                    report
                        .findings
                        .iter()
                        .any(|f| f.evidence.iter().any(|e| e.starts_with("remediation="))),
                    "{:?}",
                    report.findings
                );
            }
            other => panic!("blocked gate must report: {other:?}"),
        }
        // A passing gate document stays reported with nothing to show.
        match map_gate_document(
            fixture_doc("gate-status-pass.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => assert!(report.findings.is_empty()),
            other => panic!("pass gate must report: {other:?}"),
        }

        // The unknown-contract fixture is refused through the real entry
        // point, naming the version and never reporting.
        let script = write_script(
            tmp.path(),
            "dw-9.sh",
            &format!(
                "#!/bin/sh\ncat '{}'\n",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/driftwatch/checker-report-unknown-contract.json")
                    .display()
            ),
        );
        let outcome = run_driftwatch(tmp.path(), &script_config(&script));
        match outcome {
            PolicyOutcome::Unavailable { reason } => {
                assert!(reason.contains("driftwatch-checker/9.0.0"), "{reason}");
            }
            other => panic!("unknown contract must refuse: {other:?}"),
        }
    }

    #[test]
    fn first_binary_on_path_prefers_driftwatchdog_then_alias() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let both = write_script(&bin, "driftwatchdog", "#!/bin/sh\necho wd\n");
        let alias = write_script(&bin, "driftwatch", "#!/bin/sh\necho dw\n");
        let path = std::env::join_paths([&bin]).unwrap();
        assert_eq!(
            first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
            Some(both)
        );
        // Alias-only hosts resolve the npm launcher name.
        fs::remove_file(tmp.path().join("bin").join("driftwatchdog")).unwrap();
        assert_eq!(
            first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
            Some(alias)
        );
        // Empty PATH and non-executable files find nothing.
        assert_eq!(
            first_binary_on_path(OsStr::new(""), DRIFTWATCH_BINARY_CANDIDATES),
            None
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let plain = bin.join("driftwatchdog");
            fs::write(&plain, "not executable").unwrap();
            fs::set_permissions(&plain, fs::Permissions::from_mode(0o644)).unwrap();
            // The non-executable first candidate is skipped; the probe
            // continues to the alias rather than selecting a dead file.
            let found = first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES);
            assert_eq!(found, Some(bin.join("driftwatch")));
            let plain_alias = bin.join("driftwatch");
            fs::set_permissions(&plain_alias, fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(
                first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
                None,
                "a non-executable candidate must not be selected"
            );
        }
    }

    #[test]
    fn surface_follows_gate_manifests() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(
            policy_surface_args(tmp.path()),
            vec!["check", "--dry-run", "--format", "json"]
        );
        fs::write(tmp.path().join("gate.toml"), "[gate]\n").unwrap();
        assert_eq!(
            policy_surface_args(tmp.path()),
            vec!["gate", "--format", "json"]
        );
        let tmp2 = TempDir::new().unwrap();
        fs::create_dir_all(tmp2.path().join(".ai-gate")).unwrap();
        fs::write(tmp2.path().join(".ai-gate/gate.yaml"), "version: 1\n").unwrap();
        assert_eq!(
            policy_surface_args(tmp2.path()),
            vec!["gate", "--format", "json"]
        );
    }

    #[test]
    fn checker_envelope_maps_alerts_rows_and_scrubs_roots() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let doc = format!(
            "{{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"project\":\"{}\",\"generated_at\":\"2026-09-24T00:00:00Z\",\"checkers\":[{{\"name\":\"sec-scan\",\"status\":\"alerting\",\"alerts\":[{{\"severity\":\"warning\",\"message\":\"auth gap in {}\",\"source\":\"src/main.rs\",\"symbol\":\"SEC-001\",\"extra\":{{\"category\":\"security\"}}}},{{\"message\":\"unclassified\",\"source\":\"x\",\"symbol\":\"X-1\"}}]}},{{\"name\":\"gate-probe\",\"status\":\"failed\",\"alerts\":[],\"error\":\"checker exited with status 3\"}},{{\"name\":\"quiet\",\"status\":\"ok\",\"alerts\":[]}}],\"summary\":{{\"total\":3,\"ok\":1,\"alerting\":1,\"failed\":1,\"timeout\":0,\"protocol_error\":0,\"alerts\":2}}}}",
            tmp.path().display(),
            tmp.path().display()
        );
        let script = write_script(
            &proj,
            "dw-env.sh",
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.4.2'; exit 0; fi\ncat <<'DWEOF'\n{doc}\nDWEOF\n"
            ),
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome.report().expect("envelope maps to a report");
        // The observation names the resolved binary and the document's version.
        assert_eq!(report.tool, "dw-env");
        assert_eq!(report.tool_version, "0.4.2");
        assert_eq!(report.contract, POLICY_CONTRACT_VERSION);
        assert_eq!(report.findings.len(), 3, "{:?}", report.findings);
        let alert = &report.findings[0];
        assert_eq!(alert.id, "sec-scan/SEC-001");
        assert_eq!(alert.category, "security", "declared category preserved");
        assert_eq!(alert.severity, PolicySeverity::Warn);
        // Host paths from the document are scrubbed to <project>.
        assert!(
            !alert.message.contains(&tmp.path().display().to_string()),
            "leaked root: {}",
            alert.message
        );
        assert!(alert.message.contains("<project>"), "{}", alert.message);
        // A severity-less alert stays warn, never pass.
        assert_eq!(report.findings[1].id, "sec-scan/X-1");
        assert_eq!(report.findings[1].severity, PolicySeverity::Warn);
        // The failed row becomes a fail finding carrying the runtime note.
        let failed = &report.findings[2];
        assert_eq!(failed.id, "gate-probe");
        assert_eq!(failed.severity, PolicySeverity::Fail);
        assert!(failed.message.contains("exited with status 3"));
        // The clean row adds no findings; report source_revision is stamped.
        assert_eq!(
            report.findings.iter().filter(|f| f.id == "quiet").count(),
            0
        );
        assert!(report.source_revision.is_some());
    }

    #[test]
    fn checker_envelope_nonzero_exit_is_still_evidence() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-part.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 1; fi\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"checkers\":[{\"name\":\"boom\",\"status\":\"timeout\",\"alerts\":[],\"error\":\"checker exceeded timeout\"}],\"summary\":{\"total\":1,\"ok\":0,\"alerting\":0,\"failed\":0,\"timeout\":1,\"protocol_error\":0,\"alerts\":0}}\nDWEOF\nexit 2\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome
            .report()
            .expect("parseable failing document reports");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].severity, PolicySeverity::Fail);
        assert_eq!(
            report.findings[0].message,
            "checker 'boom' timeout: checker exceeded timeout"
        );
        assert_eq!(
            report.tool_version, "0.4.2",
            "document version wins over probe"
        );
    }

    #[test]
    fn blocked_gate_document_maps_findings_not_unavailable() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        // gate-managed surface selection proves itself: the stub only
        // answers the gate argv with a document.
        fs::write(proj.join("gate.toml"), "[gate]\nrules = []\n").unwrap();
        let script = write_script(
            &proj,
            "dw-gate.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatchdog 0.4.2'; exit 0; fi\ncat <<'DWEOF'\n{\"status\":\"FAIL\",\"blocked\":true,\"failures\":[\"product-quality\"],\"pending_reviews\":[\"ux-review\"],\"not_applicable\":[],\"manifest_digest\":\"abc\",\"rule_pack_version\":1,\"results\":[{\"gate_id\":\"build\",\"source\":\"ci\",\"status\":\"PASS\",\"severity\":\"info\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[]},{\"gate_id\":\"product-quality\",\"source\":\"quality\",\"status\":\"FAIL\",\"severity\":\"error\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[\"tests:run\"],\"diagnostic\":\"tests failing\",\"remediation\":\"run cargo test\"},{\"gate_id\":\"ux-review\",\"source\":\"ai\",\"status\":\"REVIEW_REQUIRED\",\"severity\":\"warning\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[\"checker:run\"]},{\"gate_id\":\"deploy\",\"source\":\"gate\",\"status\":\"NOT_APPLICABLE\",\"severity\":\"info\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[]}]}\nDWEOF\nexit 1\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome.report().expect("blocked gate is evidence");
        let by_id: std::collections::BTreeMap<&str, &PolicyFinding> =
            report.findings.iter().map(|f| (f.id.as_str(), f)).collect();
        assert_eq!(by_id["product-quality"].severity, PolicySeverity::Fail);
        assert!(by_id["product-quality"].message.contains("tests failing"));
        assert!(by_id["product-quality"]
            .evidence
            .iter()
            .any(|e| e == "missing=tests:run"));
        assert_eq!(by_id["ux-review"].severity, PolicySeverity::Warn);
        assert!(!by_id.contains_key("build"), "PASS adds nothing");
        assert!(!by_id["deploy"].applicable);
        assert_eq!(by_id["deploy"].severity, PolicySeverity::Pass);
    }

    #[test]
    fn unknown_document_contract_refuses_naming_version() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-future.sh",
            "#!/bin/sh\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/2.0.0\",\"tool\":\"driftwatch\",\"version\":\"9.9\",\"checkers\":[],\"summary\":{}}\nDWEOF\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            other => panic!("unknown contract must refuse: {other:?}"),
        };
        assert!(reason.contains("driftwatch-checker/2.0.0"), "{reason}");
        assert!(reason.contains("never a PASS"), "{reason}");
        // Same-major additions parse (forward-compat rule).
        let script = write_script(
            &proj,
            "dw-minor.sh",
            "#!/bin/sh\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/0.2.0\",\"tool\":\"driftwatch\",\"version\":\"9.9\",\"checkers\":[],\"summary\":{},\"future_field\":true}\nDWEOF\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        assert!(outcome.report().is_some(), "same-major must parse");
    }

    #[test]
    fn unrecognized_json_documents_never_report() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-junk.sh",
            "#!/bin/sh\necho '{\"surprise\":true}'\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        assert!(!outcome.is_reported());
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            _ => unreachable!(),
        };
        assert!(
            reason.contains("not a recognized policy document"),
            "{reason}"
        );
    }

    #[test]
    fn missing_override_does_not_fall_through_to_candidates() {
        // Operator override runs exactly that binary: a dead override is
        // reported unavailable, never silently replaced by PATH probing.
        let tmp = TempDir::new().unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(
                "/definitely/not/a/real/driftwatch-override-xyz",
            )),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            other => panic!("dead override must not report: {other:?}"),
        };
        assert!(reason.contains("not found"), "{reason}");
    }

    #[test]
    fn redacts_aws_github_gitlab_jwt_and_kv_secrets() {
        let cases = [
            ("aws key AKIAABCDEFGHIJKLMNOP", "AKIAABCDEFGHIJKLMNOP"),
            (
                "github token ghp_abcdefghijklmnopqrstuvwxyz0123456789",
                "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            ),
            (
                "gitlab token glpat-abcdefghijklmnopqrstuv",
                "glpat-abcdefghijklmnopqrstuv",
            ),
            (
                "jwt eyJhbGciOi.eyJzdWIiOi.signaturehere",
                "eyJhbGciOi.eyJzdWIiOi.signaturehere",
            ),
            ("password=hunter2hunter2", "hunter2hunter2"),
            (
                "api_key: bearer-token-1234567890",
                "bearer-token-1234567890",
            ),
        ];
        for (input, secret) in cases {
            let out = redact_credentials(input);
            assert!(out.contains("[REDACTED]"), "input: {input}\noutput: {out}");
            assert!(
                !out.contains(secret),
                "secret leaked: input: {input}\noutput: {out}"
            );
        }
    }

    #[test]
    fn redacts_private_key_blocks() {
        let input = "BEGIN MARKER\n-----BEGIN RSA PRIVATE KEY-----\nABCDEF\n-----END RSA PRIVATE KEY-----\nAFTER";
        let out = redact_credentials(input);
        assert!(!out.contains("ABCDEF"), "{out}");
        assert!(out.contains("BEGIN MARKER"), "{out}");
        assert!(out.contains("AFTER"), "{out}");
        assert!(out.contains("[REDACTED]"), "{out}");
    }

    #[test]
    fn redacts_slack_token() {
        let out = redact_credentials("token: xoxb-1234567890-12345-abcdefghijkl");
        assert!(out.contains("[REDACTED]"), "{out}");
        assert!(!out.contains("xoxb-"), "{out}");
    }

    #[test]
    fn leaves_benign_evidence_unchanged() {
        let inputs = [
            "no credentials in here",
            "package.json references react and typescript",
            "license: MIT",
            "auth: required for endpoints",
        ];
        for input in inputs {
            assert_eq!(redact_credentials(input), input, "input: {input}");
        }
    }

    #[test]
    fn does_not_match_short_token_shapes() {
        // No high-entropy content, no real credential — must stay verbatim.
        let inputs = [
            "AKIA",       // 4 chars, no payload
            "ghp_short",  // too short to be a real PAT
            "glpat-x",    // too short
            "password=x", // too short to be a real secret
        ];
        for input in inputs {
            assert_eq!(redact_credentials(input), input, "input: {input}");
        }
    }

    #[test]
    fn redact_report_runs_through_every_finding() {
        let mut report = empty_report();
        report.findings.push(PolicyFinding {
            id: "AUTH-001".to_string(),
            category: "security".to_string(),
            severity: PolicySeverity::Fail,
            applicable: true,
            message: "secret leaked: token=abcdef0123456789".to_string(),
            evidence: vec!["github token ghp_abcdefghijklmnopqrstuvwxyz0123456789".to_string()],
            reason: None,
        });
        redact_report_in_place(&mut report);
        assert!(report.findings[0].message.contains("[REDACTED]"));
        assert!(report.findings[0].evidence[0].contains("[REDACTED]"));
        assert!(!report.findings[0].evidence[0].contains("ghp_"));
    }

    #[test]
    fn source_revision_uses_latest_known_mtime() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("forge.yaml");
        fs::write(&path, "schema: 1\n").unwrap();
        let rev = source_revision_for(tmp.path());
        assert!(rev.is_some());
    }

    #[test]
    fn source_revision_is_none_for_empty_directory() {
        let tmp = TempDir::new().unwrap();
        assert!(source_revision_for(tmp.path()).is_none());
    }

    #[test]
    fn parse_severity_accepts_common_spellings() {
        for (raw, expected) in [
            ("pass", Some(PolicySeverity::Pass)),
            ("WARN", Some(PolicySeverity::Warn)),
            ("error", Some(PolicySeverity::Fail)),
            ("fatal", Some(PolicySeverity::Fail)),
            ("info", Some(PolicySeverity::Pass)),
            ("unknown", None),
        ] {
            assert_eq!(PolicySeverity::parse(raw), expected, "input: {raw}");
        }
    }

    #[test]
    fn missing_binary_reports_unavailable_with_reason() {
        let tmp = TempDir::new().unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from("definitely-not-a-real-binary-xyz")),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            _ => unreachable!(),
        };
        assert!(
            reason.contains("not found") || reason.contains("binary not found"),
            "reason: {reason}"
        );
    }

    #[test]
    fn missing_project_path_reports_unavailable() {
        let config = DriftWatchConfig::from_env();
        let outcome = run_driftwatch(Path::new("/definitely/not/here/zzz"), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn parses_and_normalizes_well_formed_report() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo '{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{\"id\":\"AUTH-001\",\"category\":\"security\",\"severity\":\"fail\",\"applicable\":true,\"message\":\"missing auth markers\",\"evidence\":[\"Cargo.toml has no auth dep\"]}]}'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        let report = outcome.report().expect("report should parse");
        assert_eq!(report.tool, "driftwatch");
        assert_eq!(report.tool_version, "0.1.0");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].id, "AUTH-001");
        assert_eq!(report.findings[0].severity, PolicySeverity::Fail);
    }

    #[test]
    fn invalid_output_reports_unavailable() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo 'this is not json'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn non_zero_exit_reports_unavailable() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo 'fatal: profile mismatch' >&2\n\
             exit 2\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn observation_stale_when_source_advanced() {
        let past: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let obs = PolicyObservation {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            source_revision: Some(past),
            observed_at: past,
            finding_count: 0,
            report: None,
        };
        let now = SystemTime::now();
        assert!(observation_is_stale(&obs, Some(now)));
        let old = UNIX_EPOCH + Duration::from_secs(60);
        assert!(!observation_is_stale(&obs, Some(old)));
    }

    #[test]
    fn observation_without_revision_is_never_stale() {
        let obs = PolicyObservation {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            source_revision: None,
            observed_at: Utc::now(),
            finding_count: 0,
            report: None,
        };
        assert!(!observation_is_stale(&obs, Some(SystemTime::now())));
    }

    #[test]
    fn redacts_multiple_credentials_in_one_evidence() {
        let input = "see AKIAABCDEFGHIJKLMNOP and ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let out = redact_credentials(input);
        assert_eq!(out.matches("[REDACTED]").count(), 2, "output: {out}");
    }

    #[test]
    fn redacts_token_and_password_in_one_evidence() {
        let input =
            "config/secret.yaml contains token=abcdef0123456789 and password=hunter2hunter2";
        let out = redact_credentials(input);
        assert_eq!(out.matches("[REDACTED]").count(), 2, "output: {out}");
        assert!(!out.contains("abcdef0123456789"), "leaked: {out}");
        assert!(!out.contains("hunter2hunter2"), "leaked: {out}");
    }
}

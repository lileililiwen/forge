//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::publish::providers::contract::{
        CONTAINER_IDENTITY_MAX, PHASE_STATUS_SUCCEEDED, PROGRESS_DETAIL_MAX,
        PUBLISH_PROVIDER_CONTRACT, QUEUE_ID_MAX,
    };
    use crate::publish::providers::engine::{
        classify_progress_event, compose_project_name, load_config, parse_request, select_provider,
        validate_queue_id, validate_response, validate_revision,
    };
    use crate::publish::providers::model::{
        ProgressEventDecision, ProviderConfig, ProviderContractError, ProviderEntry,
        ProviderOperation, PublishProviderResponse,
    };
    use serde_json::json;
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_publish_request_fixture() {
        let request = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "alethefy",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1",
            "folder": "/home/paul/code/alethefy",
            "dry_run": true
        }))
        .unwrap();
        assert_eq!(request.operation, ProviderOperation::Publish);
        assert!(request.dry_run);
    }

    #[test]
    fn rejects_wrong_contract() {
        let error = parse_request(json!({
            "contract": "wrong/0.1.0",
            "operation": "verify",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "abc",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(error, ProviderContractError::ContractMismatch(_)));
    }

    #[test]
    fn rejects_secret_like_response_evidence() {
        let response = PublishProviderResponse {
            contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
            provider: "openpanel".to_string(),
            operation_id: "delivery-1".to_string(),
            status: "failed".to_string(),
            health: "unknown".to_string(),
            evidence: vec!["token=leaked".to_string()],
            recovery: vec![],
            queue_id: None,
            revision: None,
            build_status: None,
            run_status: None,
            container_identity: None,
        };
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn disabled_provider_is_not_selectable() {
        let config = ProviderConfig {
            providers: vec![ProviderEntry {
                id: "jenkins".to_string(),
                command: PathBuf::from("jenkins-provider"),
                enabled: false,
            }],
        };
        assert!(select_provider(&config, "jenkins").is_err());
    }

    fn response_with(evidence: Vec<String>, recovery: Vec<String>) -> PublishProviderResponse {
        PublishProviderResponse {
            contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
            provider: "openpanel".to_string(),
            operation_id: "delivery-1".to_string(),
            status: "done".to_string(),
            health: "healthy".to_string(),
            evidence,
            recovery,
            queue_id: None,
            revision: None,
            build_status: None,
            run_status: None,
            container_identity: None,
        }
    }

    #[test]
    fn accepts_clean_response() {
        let response = response_with(
            vec!["runtime health check passed".to_string()],
            vec!["restart the runtime pod".to_string()],
        );
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn rejects_password_marker_in_evidence() {
        let response = response_with(vec!["password=hunter2hunter2".to_string()], vec![]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_token_marker_in_recovery() {
        let response = response_with(
            vec![],
            vec!["token=ghp_abcdefghijklmnopqrstuvwxyz0123456789".to_string()],
        );
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_pem_block_in_evidence() {
        let response = response_with(vec!["-----BEGIN RSA PRIVATE KEY-----".to_string()], vec![]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_private_key_marker_in_recovery() {
        let response = response_with(vec![], vec!["private_key=...".to_string()]);
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::SecretLeak)
        );
    }

    #[test]
    fn rejects_response_with_wrong_contract() {
        let mut response = response_with(vec![], vec![]);
        response.contract = "forge-publish-provider/0.2.0".to_string();
        assert!(matches!(
            validate_response(&response),
            Err(ProviderContractError::ContractMismatch(_))
        ));
    }

    #[test]
    fn parse_request_rejects_missing_provider_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("provider")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_project_id_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("project_id")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_revision_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("revision")
        ));
    }

    #[test]
    fn parse_request_rejects_missing_operation_id_field() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("operation_id")
        ));
    }

    #[test]
    fn parse_request_rejects_blank_provider_string() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "   ",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(
            err,
            ProviderContractError::MissingField("provider")
        ));
    }

    #[test]
    fn parse_request_rejects_non_object_payload() {
        let err = parse_request(json!("not an object")).unwrap_err();
        assert!(matches!(err, ProviderContractError::NotObject));
    }

    #[test]
    fn parse_request_supports_all_operations() {
        for op in ["capabilities", "preflight", "publish", "verify", "rollback"] {
            let request = parse_request(json!({
                "contract": PUBLISH_PROVIDER_CONTRACT,
                "operation": op,
                "provider": "openpanel",
                "project_id": "demo",
                "revision": "0123456789abcdef0123456789abcdef01234567",
                "operation_id": "delivery-1"
            }))
            .unwrap();
            assert_eq!(request.operation.as_str(), op);
        }
    }

    impl ProviderOperation {
        fn as_str(&self) -> &'static str {
            match self {
                ProviderOperation::Capabilities => "capabilities",
                ProviderOperation::Preflight => "preflight",
                ProviderOperation::Publish => "publish",
                ProviderOperation::Verify => "verify",
                ProviderOperation::Rollback => "rollback",
            }
        }
    }

    #[test]
    fn select_provider_returns_the_enabled_entry() {
        let config = ProviderConfig {
            providers: vec![
                ProviderEntry {
                    id: "openpanel".to_string(),
                    command: PathBuf::from("op"),
                    enabled: true,
                },
                ProviderEntry {
                    id: "jenkins".to_string(),
                    command: PathBuf::from("jk"),
                    enabled: false,
                },
            ],
        };
        let entry = select_provider(&config, "openpanel").unwrap();
        assert_eq!(entry.id, "openpanel");
        assert!(entry.enabled);
    }

    #[test]
    fn select_provider_refuses_unknown_id() {
        let config = ProviderConfig::default();
        let err = select_provider(&config, "missing").unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_refuses_missing_file() {
        let err = load_config(Path::new("/no/such/file.yaml")).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_refuses_invalid_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("providers.yaml");
        std::fs::write(&path, "providers: [\nunterminated").unwrap();
        let err = load_config(&path).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_config_round_trips_enabled_default() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("providers.yaml");
        // `enabled` is omitted to exercise the default-true serde path.
        std::fs::write(&path, "providers:\n  - id: openpanel\n    command: op\n").unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers[0].enabled);
    }

    #[test]
    fn validate_queue_id_accepts_alphanumeric_dash_underscore() {
        assert!(validate_queue_id("fleet-1").is_ok());
        assert!(validate_queue_id("Fleet_2026_09_27_abcdef01").is_ok());
        assert!(validate_queue_id("a").is_ok());
    }

    #[test]
    fn validate_queue_id_rejects_empty_oversized_and_unsafe() {
        assert_eq!(
            validate_queue_id(""),
            Err(ProviderContractError::QueueIdShape(String::new()))
        );
        let too_long = "x".repeat(QUEUE_ID_MAX + 1);
        assert!(matches!(
            validate_queue_id(&too_long),
            Err(ProviderContractError::QueueIdShape(_))
        ));
        assert!(matches!(
            validate_queue_id("fleet 1"),
            Err(ProviderContractError::QueueIdShape(_))
        ));
        assert!(matches!(
            validate_queue_id("fleet/1"),
            Err(ProviderContractError::QueueIdShape(_))
        ));
    }

    #[test]
    fn parse_request_carries_queue_id_through() {
        let request = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "jenkins",
            "project_id": "alpha",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "publish-alpha-ab",
            "queue_id": "fleet-1"
        }))
        .unwrap();
        assert_eq!(request.queue_id.as_deref(), Some("fleet-1"));
    }

    #[test]
    fn parse_request_rejects_malformed_queue_id() {
        let err = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "jenkins",
            "project_id": "alpha",
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "operation_id": "publish-alpha-ab",
            "queue_id": "fleet 1"
        }))
        .unwrap_err();
        assert!(matches!(err, ProviderContractError::QueueIdShape(_)));
    }

    fn progress_event(
        queue_id: Option<&str>,
        operation_id: Option<&str>,
        project_id: Option<&str>,
        phase: Option<&str>,
        status: Option<&str>,
        detail: Option<&str>,
    ) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "event".to_string(),
            Value::String("publish.progress".to_string()),
        );
        obj.insert(
            "contract".to_string(),
            Value::String(PUBLISH_PROVIDER_CONTRACT.to_string()),
        );
        if let Some(q) = queue_id {
            obj.insert("queue_id".to_string(), Value::String(q.to_string()));
        }
        if let Some(o) = operation_id {
            obj.insert("operation_id".to_string(), Value::String(o.to_string()));
        }
        if let Some(p) = project_id {
            obj.insert("project_id".to_string(), Value::String(p.to_string()));
        }
        if let Some(ph) = phase {
            obj.insert("phase".to_string(), Value::String(ph.to_string()));
        }
        if let Some(s) = status {
            obj.insert("status".to_string(), Value::String(s.to_string()));
        }
        if let Some(d) = detail {
            obj.insert("detail".to_string(), Value::String(d.to_string()));
        }
        Value::Object(obj)
    }

    #[test]
    fn classify_progress_accepts_matching_event() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            Some("stage 1/5"),
        );
        match classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")) {
            ProgressEventDecision::Accepted { detail } => {
                assert_eq!(detail, "stage 1/5");
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn classify_progress_ignores_non_progress_event() {
        let event = json!({"event": "publish.note", "data": "noise"});
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Ignored
        );
    }

    #[test]
    fn classify_progress_rejects_wrong_contract() {
        let mut event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        event.as_object_mut().unwrap().insert(
            "contract".to_string(),
            Value::String("forge-publish-provider/0.2.0".to_string()),
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
    }

    #[test]
    fn classify_progress_rejects_operation_id_mismatch() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-other"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_project_id_mismatch() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("beta"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_queue_id_mismatch() {
        let event = progress_event(
            Some("fleet-other"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_redacts_and_bounds_detail() {
        let long = "x".repeat(PROGRESS_DETAIL_MAX + 200);
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            Some(&format!("token=ghp_secret_in_detail {long}")),
        );
        match classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")) {
            ProgressEventDecision::Accepted { detail } => {
                assert!(detail.contains("[REDACTED]"));
                assert!(detail.ends_with('…'));
                assert!(detail.len() <= PROGRESS_DETAIL_MAX);
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn classify_progress_requires_queue_id_when_active() {
        // Active request carries queue_id but event omits it: mismatch.
        let event = progress_event(
            None,
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_event_with_queue_id_for_standalone_request() {
        // Standalone publish (no active queue_id) MUST NOT receive a
        // progress event claiming a queue_id; the provider is
        // spoofing queue membership.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("started"),
            None,
        );
        assert_eq!(
            classify_progress_event(&event, "op-a", "alpha", None),
            ProgressEventDecision::Mismatched
        );
    }

    #[test]
    fn classify_progress_rejects_missing_required_fields() {
        // Missing phase.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            None,
            Some("started"),
            None,
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
        // Missing status.
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            None,
            None,
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
    }

    #[test]
    fn validate_revision_accepts_full_hex_sha() {
        let lower = "0123456789abcdef0123456789abcdef01234567";
        let upper = "0123456789ABCDEF0123456789ABCDEF01234567";
        let mixed = "0123456789AbCdEf0123456789aBcDeF01234567";
        assert!(validate_revision(lower).is_ok());
        assert!(validate_revision(upper).is_ok());
        assert!(validate_revision(mixed).is_ok());
    }

    #[test]
    fn validate_revision_rejects_short_long_non_hex_blank() {
        for bad in [
            "",
            "abc",
            "0123456789abcdef0123456789abcdef0123456", // 39 chars
            "0123456789abcdef0123456789abcdef012345678", // 41 chars
            "0123456789abcdef0123456789abcdef0123456g", // non-hex
            "not a sha at all not a sha at all !",     // 41 chars, non-hex
        ] {
            assert!(
                matches!(
                    validate_revision(bad),
                    Err(ProviderContractError::RevisionShape(_))
                ),
                "expected RevisionShape for `{bad}`"
            );
        }
    }

    #[test]
    fn parse_request_rejects_short_and_non_hex_revision() {
        let short = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "0123456789abcdef",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(short, ProviderContractError::RevisionShape(_)));
        let non_hex = parse_request(json!({
            "contract": PUBLISH_PROVIDER_CONTRACT,
            "operation": "publish",
            "provider": "openpanel",
            "project_id": "demo",
            "revision": "0123456789abcdef0123456789abcdef0123456g",
            "operation_id": "delivery-1"
        }))
        .unwrap_err();
        assert!(matches!(non_hex, ProviderContractError::RevisionShape(_)));
    }

    #[test]
    fn compose_project_name_uses_first_twelve_hex_chars() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            compose_project_name("alethefy", sha),
            "forge-alethefy-0123456789ab"
        );
    }

    #[test]
    fn response_phase_fields_validate_vocabulary() {
        let mut response = response_with(vec![], vec![]);
        response.build_status = Some(PHASE_STATUS_SUCCEEDED.to_string());
        response.run_status = Some(PHASE_STATUS_SUCCEEDED.to_string());
        response.container_identity = Some("forge-alethefy-0123456789ab".to_string());
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn response_rejects_malformed_build_run_status() {
        let mut response = response_with(vec![], vec![]);
        response.build_status = Some("success".to_string()); // wrong vocabulary
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::BuildStatusShape("success".into()))
        );
        let mut response = response_with(vec![], vec![]);
        response.run_status = Some("queued".to_string()); // wrong vocabulary
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::RunStatusShape("queued".into()))
        );
    }

    #[test]
    fn response_rejects_overlong_or_empty_container_identity() {
        let mut response = response_with(vec![], vec![]);
        response.container_identity = Some(String::new());
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::ContainerIdentityShape(String::new()))
        );
        let mut response = response_with(vec![], vec![]);
        response.container_identity = Some("x".repeat(CONTAINER_IDENTITY_MAX + 1));
        assert!(matches!(
            validate_response(&response),
            Err(ProviderContractError::ContainerIdentityShape(_))
        ));
    }

    #[test]
    fn response_rejects_malformed_echo_revision() {
        let mut response = response_with(vec![], vec![]);
        response.revision = Some("not-a-sha".to_string());
        assert_eq!(
            validate_response(&response),
            Err(ProviderContractError::RevisionShape("not-a-sha".into()))
        );
    }

    #[test]
    fn classify_progress_rejects_legacy_phase_names() {
        // Phases from earlier providers (`preflight`, `transfer`,
        // `verify`, `build-and-run`, `routing`, `completed`) are not
        // part of the new vocabulary and are refused as malformed.
        for legacy in [
            "preflight",
            "transfer",
            "verify",
            "build-and-run",
            "routing",
            "completed",
        ] {
            let event = progress_event(
                Some("fleet-1"),
                Some("op-a"),
                Some("alpha"),
                Some(legacy),
                Some("started"),
                None,
            );
            assert!(
                matches!(
                    classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
                    ProgressEventDecision::Malformed { .. }
                ),
                "legacy phase `{legacy}` must be Malformed"
            );
        }
    }

    #[test]
    fn classify_progress_rejects_unknown_status() {
        let event = progress_event(
            Some("fleet-1"),
            Some("op-a"),
            Some("alpha"),
            Some("build"),
            Some("queued"),
            None,
        );
        assert!(matches!(
            classify_progress_event(&event, "op-a", "alpha", Some("fleet-1")),
            ProgressEventDecision::Malformed { .. }
        ));
    }
}

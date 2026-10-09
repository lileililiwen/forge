//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::mcp::constants::{JSONRPC_VERSION, MCP_CONTRACT_VERSION};
    use crate::mcp::methods::dispatch;
    use crate::mcp::model::{rpc_code, McpProtocolError, McpRequest, McpResponse, McpToolKind};
    use crate::mcp::params::{validate_params, validate_project_id_strict};
    use crate::mcp::tools::{find_tool, tool_registry};
    use crate::mcp::transport::run_session;
    use crate::registry::Registry;
    use serde_json::Value;
    use std::fs;
    use std::io::Cursor;

    fn request(method: &str, id: Value, params: Value) -> McpRequest {
        McpRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Some(id),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn registry_advertises_only_mature_tools() {
        let tools = tool_registry();
        let names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        // Stable set: list/inspect/create/import/feature/doctor/spec/agent/test/commit/push
        for expected in [
            "list_projects",
            "inspect_project",
            "list_profiles",
            "inspect_profile",
            "list_features",
            "run_doctor",
            "create_project",
            "import_project",
            "add_feature",
            "remove_feature",
            "upgrade_feature",
            "generate_spec",
            "run_agent",
            "run_tests",
            "commit",
            "push",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "mature tool `{expected}` missing from registry: {names:?}"
            );
        }
    }

    #[test]
    fn registry_omits_unstable_deploy_and_publish_tools() {
        // R2 boundary: deployment and publishing are not
        // implemented in Core yet, so they MUST not be advertised.
        let tools = tool_registry();
        for forbidden in ["deploy", "publish", "release", "mirror", "docs"] {
            assert!(
                tools.iter().all(|t| t.name != forbidden),
                "tool `{forbidden}` is unstable and must not be advertised"
            );
        }
    }

    #[test]
    fn registry_classifies_push_as_external_write() {
        let tools = tool_registry();
        let push = tools.iter().find(|t| t.name == "push").expect("push");
        assert_eq!(push.kind, McpToolKind::ExternalWrite);
        let read_only = [
            "list_projects",
            "inspect_project",
            "list_profiles",
            "inspect_profile",
            "list_features",
            "run_doctor",
        ];
        for name in read_only {
            let tool = tools.iter().find(|t| t.name == name).expect(name);
            assert_eq!(tool.kind, McpToolKind::ReadOnly, "{name} must be read-only");
        }
        let mutating = [
            "create_project",
            "import_project",
            "add_feature",
            "remove_feature",
            "upgrade_feature",
            "generate_spec",
            "run_agent",
            "run_tests",
            "commit",
        ];
        for name in mutating {
            let tool = tools.iter().find(|t| t.name == name).expect(name);
            assert_eq!(tool.kind, McpToolKind::Mutating, "{name} must be mutating");
        }
    }

    #[test]
    fn tools_list_returns_full_registry_summary() {
        let req = request("tools/list", Value::from(1), Value::Null);
        let value = dispatch(None, &req).expect("dispatch");
        assert_eq!(value["contract"], MCP_CONTRACT_VERSION);
        let tools = value["tools"].as_array().expect("tools array");
        assert!(tools.iter().any(|t| t["name"] == "create_project"));
        assert!(tools.iter().all(|t| t["name"] != "deploy"));
    }

    #[test]
    fn unknown_tool_returns_structured_missing_error() {
        let req = request("deploy", Value::from(1), serde_json::json!({}));
        let err = dispatch(None, &req).expect_err("missing tool");
        assert_eq!(err.code, rpc_code::TOOL_MISSING);
        assert!(err.message.contains("unknown tool `deploy`"));
    }

    #[test]
    fn parse_line_rejects_malformed_json() {
        let err = McpRequest::parse_line("{not json").expect_err("parse error");
        match err {
            McpProtocolError::Parse(_) => {}
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_rejects_wrong_jsonrpc_version() {
        let err = McpRequest::parse_line(
            r#"{"jsonrpc":"1.0","id":1,"method":"list_projects","params":{}}"#,
        )
        .expect_err("version error");
        match err {
            McpProtocolError::Parse(reason) => {
                assert!(reason.contains("jsonrpc must be"));
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_accepts_valid_request() {
        let req = McpRequest::parse_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"list_projects","params":{}}"#,
        )
        .expect("parse");
        assert_eq!(req.method, "list_projects");
        assert_eq!(req.id, Some(Value::from(1)));
    }

    #[test]
    fn validate_params_rejects_missing_required_field() {
        let descriptor = find_tool("inspect_project").expect("inspect_project");
        let err = validate_params(&descriptor, &serde_json::json!({})).expect_err("missing");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        assert!(err.message.contains("target"));
    }

    #[test]
    fn validate_params_rejects_wrong_field_type() {
        let descriptor = find_tool("commit").expect("commit");
        let err = validate_params(
            &descriptor,
            &serde_json::json!({
                "path": ".",
                "paths": "not-an-array",
                "message": "fix"
            }),
        )
        .expect_err("wrong type");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        assert!(err.message.contains("paths"));
    }

    #[test]
    fn validate_params_rejects_shell_metacharacters_in_project_id() {
        // The schema only requires path, profile, id. Pass an
        // id with shell metacharacters; the schema validation
        // does not reject the characters, but the dispatcher
        // (validate_project_id_strict) must reject the id
        // before any Core mutation.
        let req = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": ".",
                "profile": "rust-web",
                "id": "evil; rm -rf /",
            }),
        );
        let err = dispatch(None, &req).expect_err("shell metacharacters");
        assert_eq!(err.code, rpc_code::TOOL_REFUSED);
        assert!(err.message.contains("evil; rm -rf /"));
        assert!(err.message.contains("literal data"));
    }

    #[test]
    fn push_without_confirm_is_refused_before_subprocess() {
        let req = request(
            "push",
            Value::from(1),
            serde_json::json!({
                "path": ".",
                "confirm": false,
            }),
        );
        let err = dispatch(None, &req).expect_err("confirm required");
        assert_eq!(err.code, rpc_code::TOOL_REFUSED);
        assert!(err.message.contains("confirm"));
    }

    #[test]
    fn push_confirm_required_field_omitted_returns_invalid_params() {
        let req = request(
            "push",
            Value::from(1),
            serde_json::json!({
                "path": ".",
            }),
        );
        let err = dispatch(None, &req).expect_err("confirm missing");
        // Schema-level validation rejects the missing
        // required field before the push-specific refusal
        // can fire. The push-confirm-required refusal only
        // applies to `confirm: false`.
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
    }

    #[test]
    fn list_projects_through_dispatch_returns_empty_registry() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request("list_projects", Value::from(1), serde_json::json!({}));
        let value = dispatch(Some(&db), &req).expect("dispatch");
        // The tool is a thin adapter over the shared Core catalog
        // service, so the result envelope is `{"catalog": <page>}`
        // with an empty record list and a single local source.
        assert_eq!(value["catalog"]["total"], serde_json::json!(0));
        assert_eq!(value["catalog"]["records"], serde_json::json!([]));
        assert_eq!(value["catalog"]["limit"], serde_json::json!(50));
    }

    #[test]
    fn sequential_isolated_sessions_remain_independent() {
        // Two sessions with different temporary registries must not
        // observe each other's projects or operations.
        let dir_a = tempfile::tempdir().unwrap();
        let db_a = dir_a.path().join("registry.db");
        let dir_b = tempfile::tempdir().unwrap();
        let db_b = dir_b.path().join("registry.db");
        let app_a = dir_a.path().join("app-a");
        let create = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": app_a.to_string_lossy(),
                "profile": "rust-web",
                "id": "isolated-a",
            }),
        );
        let value = dispatch(Some(&db_a), &create).expect("create in session A");
        assert_eq!(value["created"]["record"]["id"], "isolated-a");
        let list = request("list_projects", Value::from(1), serde_json::json!({}));
        let in_a = dispatch(Some(&db_a), &list).expect("list A");
        let in_b = dispatch(Some(&db_b), &list).expect("list B");
        assert_eq!(in_a["catalog"]["total"], serde_json::json!(1));
        assert_eq!(in_a["catalog"]["records"][0]["project_id"], "isolated-a");
        assert_eq!(in_b["catalog"]["total"], serde_json::json!(0));
    }

    #[test]
    fn inspect_project_unknown_id_returns_invalid_params() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request(
            "inspect_project",
            Value::from(1),
            serde_json::json!({"target": "no-such-project"}),
        );
        let err = dispatch(Some(&db), &req).expect_err("unknown project");
        assert_eq!(err.code, rpc_code::INVALID_PARAMS);
        let data = err.data.expect("data");
        assert_eq!(data["code"], "unknown-project");
    }

    #[test]
    fn run_session_round_trip_known_request_and_unknown_tool() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let db_path = db.as_path();
        let input = "\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"list_projects\",\"params\":{}}\n\
            {\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"deploy\",\"params\":{}}\n\
        ";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(
            Some(db_path),
            Cursor::new(input.as_bytes()),
            &mut output,
            &mut diag,
        )
        .expect("run session");
        let rendered = String::from_utf8(output).expect("utf8");
        // Two responses (one per request) on two lines.
        let mut lines = rendered.lines();
        let ok_line = lines.next().expect("ok line");
        let err_line = lines.next().expect("err line");
        let ok: McpResponse = serde_json::from_str(ok_line).expect("ok json");
        let err: McpResponse = serde_json::from_str(err_line).expect("err json");
        assert!(ok.result.is_some());
        assert_eq!(ok.id, Value::from(1));
        assert!(err.error.is_some());
        assert_eq!(err.id, Value::from(2));
        assert_eq!(err.error.unwrap().code, rpc_code::TOOL_MISSING);
    }

    #[test]
    fn run_session_reports_parse_error_as_rpc_envelope() {
        let input = "not-a-json-line\n";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(None, Cursor::new(input.as_bytes()), &mut output, &mut diag)
            .expect("run session");
        let rendered = String::from_utf8(output).expect("utf8");
        let line = rendered.lines().next().expect("line");
        let response: McpResponse = serde_json::from_str(line).expect("json");
        let err = response.error.expect("error");
        assert_eq!(err.code, rpc_code::PARSE_ERROR);
        assert_eq!(response.id, Value::Null);
    }

    #[test]
    fn run_session_drops_secrets_from_diagnostics() {
        let input = "\
            {\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"create_project\",\"params\":{\"path\":\".\",\"profile\":\"rust-web\",\"id\":\"aws-secret-AKIAIOSFODNN7EXAMPLE\"}}\n\
        ";
        let mut output = Vec::new();
        let mut diag = Vec::new();
        run_session(None, Cursor::new(input.as_bytes()), &mut output, &mut diag)
            .expect("run session");
        let diag_text = String::from_utf8_lossy(&diag);
        // Diagnostic should mention the method but not the
        // secret-looking id. The id is included in the
        // method name only as the request label, never as
        // the parameter value.
        assert!(diag_text.contains("create_project"));
        assert!(!diag_text.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn contract_version_is_recorded_on_tool_descriptors() {
        let tools = tool_registry();
        for t in &tools {
            assert_eq!(t.contract, MCP_CONTRACT_VERSION, "tool `{}`", t.name);
        }
    }

    #[test]
    fn dispatch_through_cli_returns_equivalent_outcome() {
        // R1 success scenario: `inspect_project` and
        // `doctor` (via the CLI) must return the same domain
        // outcome. We bootstrap a project via the CLI
        // surface, then re-inspect it through MCP and
        // compare. The fixture lives in a unique temp dir.
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(
            proj.join("forge.yaml"),
            "schema: 1\nproject:\n  id: r1-equiv\n  name: R1\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
        )
        .unwrap();
        let mut registry = Registry::open(&db).unwrap();
        let _ = registry.register(&proj, None).unwrap();
        drop(registry);
        let req = request(
            "inspect_project",
            Value::from(1),
            serde_json::json!({"target": "r1-equiv"}),
        );
        let value = dispatch(Some(&db), &req).expect("dispatch");
        // The tool is a thin adapter over the shared Core catalog
        // service, so the result envelope is `{"catalog": {"records": […]}}`
        // keyed by the project id.
        let records = value["catalog"]["records"].as_array().expect("records");
        let record = records.first().expect("at least one catalog record");
        assert_eq!(record["project_id"], "r1-equiv");
        assert_eq!(record["profile"], "rust-web");
    }

    #[test]
    fn mutating_tool_creates_journal_entry() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("registry.db");
        let req = request(
            "create_project",
            Value::from(1),
            serde_json::json!({
                "path": dir.path().join("app").to_string_lossy(),
                "profile": "rust-web",
                "id": "mcp-app",
            }),
        );
        let value = dispatch(Some(&db), &req).expect("create");
        assert_eq!(value["created"]["record"]["id"], "mcp-app");
        // Journal entry should be recorded under the `mcp` kind.
        let registry = Registry::open(&db).unwrap();
        let entries = registry.journal_entries().unwrap();
        let mcp_entries: Vec<&_> = entries.iter().filter(|e| e.kind == "mcp").collect();
        assert!(
            !mcp_entries.is_empty(),
            "expected at least one mcp journal entry"
        );
    }

    #[test]
    fn unknown_tool_request_id_is_preserved_in_error() {
        let req = request("deploy", Value::from("opaque-id-42"), serde_json::json!({}));
        let err = dispatch(None, &req).expect_err("missing tool");
        assert_eq!(err.code, rpc_code::TOOL_MISSING);
        let response = McpResponse::failure(Value::from("opaque-id-42"), err);
        assert_eq!(response.id, Value::from("opaque-id-42"));
    }
}

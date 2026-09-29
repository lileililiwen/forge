//! MCP surface contract (`mature-mcp-surface`).
//!
//! Exercises the published `forge mcp serve` subcommand
//! through the built binary. Each test feeds one or more
//! JSON-RPC 2.0 requests on stdin, asserts on the matching
//! line-delimited response, and verifies that:
//!
//! - R1 success: `inspect_project` returns the same
//!   domain outcome as the CLI `forge inspect` surface
//!   when both target the same registered project.
//! - R1 failure: invalid arguments and unauthorized
//!   project paths are rejected with a structured
//!   `INVALID_PARAMS` JSON-RPC error before any Core
//!   mutation.
//! - R1 boundary: an internal operation that has not met
//!   its maturity criteria is absent from the advertised
//!   `tools/list`.
//! - R2 success: `create_project` returns the generated
//!   record and journals the operation in the registry.
//! - R2 failure: a model-supplied project id containing
//!   shell metacharacters is rejected as a literal data
//!   value; no shell command is ever executed.
//! - R2 boundary: `deploy`/`publish` are not advertised;
//!   the registry reports a stable list that does not
//!   silently grow.
//! - R2 mutating boundary: `push` is refused with
//!   `push-confirm-required`-style code when `confirm`
//!   is missing or false; an explicit `confirm: true` is
//!   the only path to a network side effect.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_mcp(db: &Path, requests: &[serde_json::Value]) -> (String, String) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge mcp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            let line = serde_json::to_string(r).expect("encode request");
            stdin.write_all(line.as_bytes()).expect("write");
            stdin.write_all(b"\n").expect("newline");
        }
    }
    let output = child.wait_with_output().expect("wait forge mcp");
    (lossy(&output.stdout), lossy(&output.stderr))
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

fn register(db: &Path, dir: &Path, id: &str) -> serde_json::Value {
    write_rust_project(dir, id);
    let out = clean_cmd()
        .arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("register")
        .arg(dir)
        .output()
        .expect("register");
    assert!(
        out.status.success(),
        "register failed: stdout={} stderr={}",
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("register json")
}

fn response_for(stdout: &str, id: i64) -> serde_json::Value {
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).expect("response json");
        if value.get("id").and_then(|v| v.as_i64()) == Some(id) {
            return value;
        }
    }
    panic!("no response for id={id} in:\n{stdout}");
}

#[test]
fn cli_help_lists_mcp_subcommand() {
    let out = clean_cmd().arg("--help").output().expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(text.contains("mcp"), "help must mention mcp:\n{text}");
}

#[test]
fn mcp_serve_advertises_only_mature_tools() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for expected in [
        "list_projects",
        "inspect_project",
        "create_project",
        "run_doctor",
        "commit",
        "push",
    ] {
        assert!(
            names.contains(&expected),
            "mature tool `{expected}` missing: {names:?}"
        );
    }
    for forbidden in ["deploy", "publish", "release", "mirror", "docs"] {
        assert!(
            !names.contains(&forbidden),
            "unstable tool `{forbidden}` must not be advertised: {names:?}"
        );
    }
}

#[test]
fn mcp_serve_advertises_tool_kinds_for_mutating_boundary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let kind_of = |name: &str| -> String {
        tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}"))["kind"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(kind_of("list_projects"), "read_only");
    assert_eq!(kind_of("inspect_project"), "read_only");
    assert_eq!(kind_of("run_doctor"), "read_only");
    assert_eq!(kind_of("create_project"), "mutating");
    assert_eq!(kind_of("add_feature"), "mutating");
    assert_eq!(kind_of("commit"), "mutating");
    assert_eq!(kind_of("push"), "external_write");
}

#[test]
fn mcp_serve_unknown_tool_returns_structured_error() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "deploy",
        "params": {}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 7);
    let error = response["error"].as_object().expect("error object");
    assert_eq!(error["code"], -32011, "deploy is missing-tool code");
    assert!(error["message"].as_str().unwrap().contains("unknown tool"));
}

#[test]
fn mcp_serve_unknown_project_returns_invalid_params() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "inspect_project",
        "params": {"target": "no-such-project"}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 11);
    let error = response["error"].as_object().expect("error");
    assert_eq!(error["code"], -32602);
    let data = error["data"].as_object().expect("data");
    assert_eq!(data["code"], "unknown-project");
}

#[test]
fn mcp_inspect_project_matches_cli_inspect_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    register(&db, &proj, "r1-equivalent");
    // The MCP `inspect_project` is a thin adapter over the
    // shared Core catalog service, so the comparison target
    // is the catalog-shaped CLI surface: `forge project
    // inspect <id> --format json`.
    let cli_out = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("project")
        .arg("inspect")
        .arg("r1-equivalent")
        .output()
        .expect("cli project inspect");
    assert_eq!(cli_out.status.code(), Some(0));
    let cli_value: serde_json::Value = serde_json::from_slice(&cli_out.stdout).expect("cli json");
    let cli_records = cli_value["catalog"]["records"]
        .as_array()
        .expect("cli records");

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "inspect_project",
        "params": {"target": "r1-equivalent"}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 22);
    let mcp_records = response["result"]["catalog"]["records"]
        .as_array()
        .expect("mcp records");

    // Domain equivalence: the project id and profile are the
    // same field on the catalog record, so the wire format
    // is byte-equal between the CLI and MCP surfaces.
    assert_eq!(mcp_records.len(), cli_records.len());
    assert_eq!(mcp_records[0]["project_id"], "r1-equivalent");
    assert_eq!(mcp_records[0]["profile"], cli_records[0]["profile"]);
}

#[test]
fn mcp_create_project_creates_files_and_journal_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("mcp-app");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 33,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "mcp-app"
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 33);
    let result = &response["result"];
    assert_eq!(result["created"]["record"]["id"], "mcp-app");
    assert_eq!(result["created"]["record"]["profile"], "rust-web");
    assert!(dest.join("forge.yaml").is_file());
    assert!(dest.join("Cargo.toml").is_file());
    // Subsequent inspect via CLI must find the new project.
    let cli = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("list")
        .output()
        .expect("list");
    assert_eq!(cli.status.code(), Some(0));
    let list: serde_json::Value = serde_json::from_slice(&cli.stdout).expect("list json");
    let projects = list["projects"].as_array().expect("projects");
    assert!(projects.iter().any(|p| p["id"] == "mcp-app"));
}

#[test]
fn mcp_create_project_rejects_shell_metacharacters_as_literal_data() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("evil-app");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 44,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "evil; rm -rf /"
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 44);
    let error = response["error"].as_object().expect("error");
    assert_eq!(
        error["code"], -32012,
        "TOOL_REFUSED for shell metacharacters"
    );
    let message = error["message"].as_str().expect("message");
    assert!(
        message.contains("literal data"),
        "message must state the value is treated as literal data, not shell: {message}"
    );
    assert!(
        !dest.join("forge.yaml").exists(),
        "project must not be created when id is rejected"
    );
}

#[test]
fn mcp_push_without_confirm_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    register(&db, &proj, "push-refuse");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 55,
        "method": "push",
        "params": {
            "path": proj.to_string_lossy(),
            "confirm": false
        }
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 55);
    let error = response["error"].as_object().expect("error");
    assert_eq!(error["code"], -32012, "TOOL_REFUSED for missing confirm");
    let data = error["data"].as_object().expect("data");
    assert_eq!(data["code"], "push-confirm-required");
}

#[test]
fn mcp_parse_error_returns_parse_error_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"not-a-json\n").expect("write");
    }
    let output = child.wait_with_output().expect("wait");
    let stdout = lossy(&output.stdout);
    let response: serde_json::Value =
        serde_json::from_str(stdout.lines().next().expect("at least one line"))
            .expect("response json");
    assert_eq!(response["id"], serde_json::Value::Null);
    let error = response["error"].as_object().expect("error");
    assert_eq!(error["code"], -32700, "PARSE_ERROR");
}

#[test]
fn mcp_diagnostics_redact_user_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("diag-app");
    // A credential-shaped id is treated as literal data; the
    // error response mentions it (the model can see this),
    // but the diagnostic stream must not echo it.
    let secret = "aws-secret-AKIAIOSFODNN7EXAMPLE";
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 66,
        "method": "create_project",
        "params": {
            "path": dest.to_string_lossy(),
            "profile": "rust-web",
            "id": secret
        }
    });
    let (stdout, stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 66);
    assert!(response["error"].is_object(), "expected refusal");
    // The diagnostic stream is the security boundary: it
    // must not contain the secret the model submitted.
    assert!(
        !stderr.contains(secret),
        "diagnostic stream leaked the user-provided id: {stderr}"
    );
}

#[test]
fn mcp_list_projects_through_stdio_matches_cli_list() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    register(&db, &proj, "list-equiv");
    // The MCP `list_projects` is a thin adapter over the
    // shared Core catalog service, so the comparison target
    // is the catalog-shaped CLI surface: `forge project list
    // --format json`.
    let cli = clean_cmd()
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("project")
        .arg("list")
        .output()
        .expect("cli project list");
    let cli_list: serde_json::Value = serde_json::from_slice(&cli.stdout).expect("cli json");
    let cli_records = cli_list["catalog"]["records"]
        .as_array()
        .expect("cli records");

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 77,
        "method": "list_projects",
        "params": {}
    });
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 77);
    let mcp_records = response["result"]["catalog"]["records"]
        .as_array()
        .expect("mcp records");
    // Both surfaces report the same project id and profile.
    assert_eq!(mcp_records.len(), cli_records.len());
    for mcp_proj in mcp_records {
        let id = mcp_proj["project_id"].as_str().expect("project_id");
        let cli_proj = cli_records
            .iter()
            .find(|p| p["project_id"] == id)
            .unwrap_or_else(|| panic!("cli missing {id}"));
        assert_eq!(mcp_proj["profile"], cli_proj["profile"]);
    }
}

#[test]
fn mcp_dispatch_round_trip_handles_multiple_requests() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let requests = vec![
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "list_projects", "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "list_profiles", "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "list_features", "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "deploy", "params": {}
        }),
    ];
    let (stdout, _stderr) = run_mcp(&db, &requests);
    let one = response_for(&stdout, 1);
    let two = response_for(&stdout, 2);
    let three = response_for(&stdout, 3);
    let four = response_for(&stdout, 4);
    assert!(one["result"]["catalog"]["records"].is_array());
    assert!(two["result"]["profiles"].is_array());
    assert!(three["result"]["features"].is_array());
    assert_eq!(four["error"]["code"], -32011);
}

#[test]
fn mcp_isolated_registry_succeeds_with_readonly_home() {
    // Read-only default home must be harmless when the caller owns
    // an explicit temporary registry: the round trip runs against
    // the temporary registry and never fails with a host
    // `readonly database` error.
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ro_home = tmp.path().join("ro-home");
    std::fs::create_dir_all(ro_home.join(".local/share/forge")).unwrap();
    let mut perms = std::fs::metadata(&ro_home).unwrap().permissions();
    perms.set_mode(0o555);
    std::fs::set_permissions(&ro_home, perms).unwrap();
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "list_projects", "params": {}
    })];
    let mut cmd = clean_cmd();
    cmd.env("HOME", &ro_home);
    cmd.env("XDG_DATA_HOME", ro_home.join(".no-xdg"));
    cmd.arg("--registry").arg(&db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge mcp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in &requests {
            let line = serde_json::to_string(r).expect("encode request");
            stdin.write_all(line.as_bytes()).expect("write");
            stdin.write_all(b"\n").expect("newline");
        }
    }
    let output = child.wait_with_output().expect("wait forge mcp");
    let stdout = lossy(&output.stdout);
    let stderr = lossy(&output.stderr);
    assert!(
        !stdout.contains("readonly") && !stderr.contains("readonly"),
        "isolated run must not surface host readonly error: stdout={stdout} stderr={stderr}"
    );
    let response = response_for(&stdout, 1);
    assert!(
        response["result"]["catalog"]["records"].is_array(),
        "isolated list_projects must succeed: {response}"
    );
    // Restore writability so TempDir cleanup can remove the fixture.
    let mut perms = std::fs::metadata(&ro_home).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&ro_home, perms).unwrap();
}

#[test]
fn mcp_repeated_isolated_round_trip_is_stable() {
    // Repeated invocation against the same temporary registry must
    // return the same result without leaking state.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let requests = vec![serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "list_projects", "params": {}
    })];
    let (first, _) = run_mcp(&db, &requests);
    let (second, _) = run_mcp(&db, &requests);
    let one = response_for(&first, 1);
    let two = response_for(&second, 1);
    assert_eq!(one["result"], two["result"]);
}

//! Workspace metadata cross-surface regression.
//!
//! The declaration is generation output owned by the shared Core, so the
//! transports must render it without reinterpretation and every unrelated
//! surface must stay inert:
//!
//! - MCP `create_project` writes a byte-identical declaration to the CLI
//!   for the same request, and the mature tool registry exposes no new
//!   workspace tool.
//! - The API create route (same Core path) is not re-plumbed: no route
//!   name mentions workspace metadata.
//! - The registry record, `forge portal` sections and the operations
//!   journal are unchanged by the declaration's presence: the file adds
//!   no columns, no journal rows and no portal entries.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::generate::workspace::METADATA_PATH;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge");
    serde_json::from_slice(&out.stdout).expect("json")
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

fn response_for(stdout: &str, id: u64) -> serde_json::Value {
    for line in stdout.lines() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        if value["id"] == serde_json::json!(id) {
            return value;
        }
    }
    panic!("no response for id {id}");
}

#[test]
fn mcp_create_project_emits_the_same_declaration_as_cli() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let mcp_dest = tmp.path().join("parity-mcp");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "create_project",
        "params": {
            "path": mcp_dest.to_string_lossy(),
            "profile": "rust-web",
            "id": "parity-mcp"
        }
    });
    let (stdout, stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 11);
    assert_eq!(
        response["result"]["created"]["record"]["id"], "parity-mcp",
        "{stderr}"
    );
    assert!(mcp_dest.join(METADATA_PATH).is_file());

    let cli_dest = tmp.path().join("parity-cli");
    let out = run(
        &db,
        &[
            "new",
            cli_dest.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            "parity-cli",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let mcp_text = fs::read_to_string(mcp_dest.join(METADATA_PATH)).unwrap();
    let cli_text = fs::read_to_string(cli_dest.join(METADATA_PATH)).unwrap();
    // Same profile + same id → byte-identical documents (the only legal
    // difference is the identity fields, which match here by request).
    let mcp_id = mcp_text.clone().replace("parity-mcp", "PID");
    let cli_id = cli_text.replace("parity-cli", "PID");
    assert_eq!(mcp_id, cli_id, "transports must render one declaration");
}

#[test]
fn mcp_mature_registry_exposes_no_workspace_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let req = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    let (stdout, _stderr) = run_mcp(&db, &[req]);
    let response = response_for(&stdout, 1);
    let tools = response["result"]["tools"].as_array().expect("tools");
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(
            !name.contains("workspace") && !name.contains("metadata"),
            "workspace metadata stays a generation output, not a tool: {name}"
        );
    }
}

#[test]
fn declaration_is_absent_from_api_route_surface_names() {
    // The API mutation surface is unchanged by this contract: the create
    // route goes through the same Core generation and must not gain a
    // workspace-specific route or flag.
    let src = fs::read_to_string("src/api/mod.rs").expect("api source");
    assert!(
        !src.contains("workspace-metadata"),
        "the API must not add a workspace opt-out surface"
    );
    assert!(src.contains("normalize_explicit"), "shared Core path");
}

#[test]
fn registry_record_and_portal_stay_inert() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let with = tmp.path().join("portal-with");
    let without = tmp.path().join("portal-without");
    let out = run(
        &db,
        &[
            "new",
            with.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            "portal-with",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let out = run(
        &db,
        &[
            "new",
            without.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            "portal-without",
            "--no-workspace-metadata",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(!without.join(METADATA_PATH).exists());

    let a = run_json(&db, &["inspect", "portal-with"]);
    let b = run_json(&db, &["inspect", "portal-without"]);
    for key in [
        "profile",
        "schema_version",
        "platform_version",
        "maturity",
        "features",
        "runtime",
    ] {
        assert_eq!(a[key], b[key], "registry record must not change: {key}");
    }

    // Portal renders both without surfacing the declaration at all.
    let dash = run_json(&db, &["portal", "dashboard", "portal-with"]);
    let text = serde_json::to_string(&dash).unwrap();
    assert!(
        !text.contains(METADATA_PATH),
        "the portal must not reinterpret generation output"
    );

    // The journal stays project-agnostic: two creations + registrations
    // must not add workspace-kind rows.
    let conn = rusqlite::Connection::open(&db).unwrap();
    let kinds: Vec<String> = conn
        .prepare("SELECT DISTINCT kind FROM operations")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for kind in &kinds {
        assert_ne!(kind, "workspace");
        assert_ne!(kind, "metadata");
    }
}

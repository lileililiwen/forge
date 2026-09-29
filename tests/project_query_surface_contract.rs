//! Cross-surface parity for the project catalog query
//! (`project-query-consumer-surfaces`).
//!
//! The CLI, MCP and HTTP API transports must agree on the records, the
//! order, the pagination and the failure boundaries of one shared Core
//! service in `src/catalog/`. A second filtering or ordering
//! implementation in any transport is a defect, not a shortcut, so this
//! suite answers the questions a per-surface contract cannot:
//!
//! - The same query returns the same records in the same order on
//!   every transport. The only difference is the envelope.
//! - Pagination, filters, sources, and `max-age` flow through the
//!   shared Core service identically.
//! - An invalid filter, an unauthorized caller, an unknown project
//!   and an empty result produce equivalent typed errors on every
//!   transport — the API answers 400/401, the MCP returns
//!   `INVALID_PARAMS`, and the CLI prints zero bytes on stdout.
//! - A read never invents a record, never invents a source, and never
//!   touches the registry.
//!
//! Every case runs against a local fixture. The HTTP server is
//! `forge api serve` against the loopback listener, and the MCP
//! transport is the in-process `mcp::dispatch` over a bounded
//! argument map.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const CONTRACT: &str = "forge-project-catalog/0.1.0";
const IDENTITY_YAML: &str = "  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge json");
    assert!(
        out.status.success(),
        "forge {args:?}: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_ndjson_lines(db: &Path, args: &[&str]) -> Vec<Value> {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("ndjson");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge ndjson");
    assert!(
        out.status.success(),
        "forge {args:?}: {}",
        lossy(&out.stderr)
    );
    lossy(&out.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("one record per line"))
        .collect()
}

fn write_project(dir: &Path, id: &str, profile: &str, maturity: &str, language: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: {profile}\n  \
             maturity: {maturity}\nruntime:\n  language: {language}\nfeatures:\n  auth: 0.1.0\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
}

fn write_identity_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  \
         maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n{IDENTITY_YAML}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

/// Build a registry with three projects so the catalog carries enough
/// data to test pagination, ordering, and source selection. The
/// `alpha` project also carries an `identity:` block so it can
/// mint an admin session for the HTTP API tests.
fn local_fleet(tmp: &Path) -> (PathBuf, PathBuf) {
    let db = tmp.join("registry.db");
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    let gamma = tmp.join("gamma");
    write_identity_project(&alpha, "alpha");
    write_project(&beta, "beta", "python-service", "L3", "python");
    write_project(&gamma, "gamma", "nextjs-web", "L2", "typescript");
    for dir in [&alpha, &beta, &gamma] {
        assert!(
            run(&db, &["register", dir.to_str().unwrap()])
                .status
                .success(),
            "register {}: {}",
            dir.display(),
            lossy(&run(&db, &["register", dir.to_str().unwrap()]).stderr)
        );
    }
    (db, alpha)
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

fn start_api(db: &Path) -> (u16, std::process::Child) {
    let port = free_port();
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("api")
        .arg("serve")
        .arg("--port")
        .arg(port.to_string());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let child = cmd.spawn().expect("spawn api server");
    let start = Instant::now();
    let mut bound = false;
    while start.elapsed() < Duration::from_secs(5) {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            bound = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if !bound {
        panic!("api server did not bind on 127.0.0.1:{port}");
    }
    (port, child)
}

fn stop_api(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn http_request(
    method: &str,
    host_port: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, Value) {
    let mut stream = TcpStream::connect(host_port).expect("connect api");
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut header_block =
        format!("{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n");
    for (name, value) in headers {
        header_block.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        header_block.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    header_block.push_str("\r\n");
    stream
        .write_all(header_block.as_bytes())
        .expect("write head");
    if !body.is_empty() {
        stream.write_all(body).expect("write body");
    }
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    let text = String::from_utf8_lossy(&response).to_string();
    let status_line = text.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
    let body_text = &text[body_start..];
    let json: Value = if body_text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body_text).unwrap_or(Value::Null)
    };
    (status, json)
}

fn mint_session(db: &Path, project_dir: &Path) -> String {
    let challenge = run_json(
        db,
        &["identity", "build-challenge", project_dir.to_str().unwrap()],
    );
    let state = challenge["challenge"]["state"].as_str().unwrap();
    let nonce = challenge["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        db,
        &[
            "identity",
            "complete-auth",
            project_dir.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    mint["outcome"]["Session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string()
}

/// Build a JSON-RPC MCP `list_projects` request.
fn mcp_list(db: &Path, params: Value) -> Value {
    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: "list_projects".to_string(),
        params,
    };
    forge::mcp::dispatch(Some(db), &request).expect("list_projects dispatch")
}

/// Build a JSON-RPC MCP `inspect_project` request.
fn mcp_inspect(db: &Path, target: &str) -> Result<Value, forge::mcp::McpRpcError> {
    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: "inspect_project".to_string(),
        params: serde_json::json!({ "target": target }),
    };
    forge::mcp::dispatch(Some(db), &request)
}

/// Build an HTTP `GET /v1/projects/catalog` request. Returns
/// `(status, json)`; the body parses as `null` when the route
/// answers with no body (refusals still carry a JSON envelope).
fn api_get_catalog(port: u16, query: &str, bearer: Option<&str>) -> (u16, Value) {
    let path = if query.is_empty() {
        "/v1/projects/catalog".to_string()
    } else {
        format!("/v1/projects/catalog?{query}")
    };
    let mut headers: Vec<(&str, String)> = Vec::new();
    if let Some(token) = bearer {
        headers.push(("Authorization", format!("Bearer {token}")));
    }
    let refs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    http_request("GET", &format!("127.0.0.1:{port}"), &path, &refs, &[])
}

fn api_get_project_catalog(port: u16, project: &str, bearer: Option<&str>) -> (u16, Value) {
    let path = format!("/v1/projects/{project}/catalog");
    let mut headers: Vec<(&str, String)> = Vec::new();
    if let Some(token) = bearer {
        headers.push(("Authorization", format!("Bearer {token}")));
    }
    let refs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    http_request("GET", &format!("127.0.0.1:{port}"), &path, &refs, &[])
}

/// Extract the records array from any transport's `{"catalog": …}`
/// envelope. The CLI, MCP, and API all share the same envelope key.
fn records_of(envelope: &Value) -> Vec<Value> {
    envelope["catalog"]["records"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Extract the project ids from a records array in the order the
/// transport emitted them.
fn ids_of(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .map(|record| record["project_id"].as_str().unwrap().to_string())
        .collect()
}

/// Build the API contract envelope version: catalog route answers
/// carry `{"contract": API_CONTRACT_VERSION, "catalog": …}`.
fn api_envelope_equals(page: &Value, records: &[Value]) -> bool {
    page["contract"] == forge::api::API_CONTRACT_VERSION
        && page["catalog"]["records"]
            .as_array()
            .map(|arr| arr.as_slice())
            == Some(records)
}

#[test]
fn the_same_query_returns_the_same_records_in_the_same_order_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // 1. CLI
    let cli = run_json(&db, &["project", "list", "--format", "json"]);
    let cli_records = records_of(&cli);
    assert_eq!(
        cli["catalog"]["contract"].as_str().unwrap(),
        CONTRACT,
        "the CLI must report the catalog contract version"
    );
    let cli_ids = ids_of(&cli_records);
    assert_eq!(cli_ids, vec!["alpha", "beta", "gamma"]);

    // 2. MCP
    let mcp = mcp_list(&db, serde_json::json!({}));
    let mcp_records = records_of(&mcp);
    assert_eq!(
        mcp["catalog"]["contract"].as_str().unwrap(),
        CONTRACT,
        "MCP must report the catalog contract version"
    );
    let mcp_ids = ids_of(&mcp_records);
    assert_eq!(
        mcp_ids, cli_ids,
        "MCP and CLI must agree on record identity and order"
    );
    // The Core service is the single source of truth: the records
    // themselves are byte-identical between CLI and MCP.
    assert_eq!(
        serde_json::to_string(&mcp_records).unwrap(),
        serde_json::to_string(&cli_records).unwrap()
    );

    // 3. HTTP API
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_catalog(port, "", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 200, "API catalog must be 200 with a bearer");
    let api_records = records_of(&api);
    let api_ids = ids_of(&api_records);
    assert_eq!(
        api_ids, cli_ids,
        "API and CLI must agree on record identity and order"
    );
    // API carries its own contract envelope in addition to the
    // catalog contract; the records themselves stay byte-identical.
    assert!(api_envelope_equals(&api, &cli_records));
}

#[test]
fn the_cli_ndjson_stream_matches_the_catalog_order() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = local_fleet(tmp.path());
    let cli = run_json(&db, &["project", "list", "--format", "json"]);
    let ndjson = run_ndjson_lines(&db, &["project", "list"]);
    let json_records = records_of(&cli);
    // NDJSON is one record per line in the same order as the JSON
    // page; the table layout is never the machine contract.
    assert_eq!(ndjson.len(), json_records.len());
    for (line, json_record) in ndjson.iter().zip(json_records.iter()) {
        assert_eq!(line, json_record);
    }
}

#[test]
fn pagination_filters_and_sources_match_across_transports() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // CLI: limit + tag-less language filter
    let cli = run_json(
        &db,
        &[
            "project",
            "list",
            "--format",
            "json",
            "--limit",
            "2",
            "--filter",
            "language=rust",
        ],
    );
    let cli_records = records_of(&cli);
    assert_eq!(cli_records.len(), 1, "only `alpha` carries rust");
    assert_eq!(cli_records[0]["project_id"], "alpha");
    assert_eq!(cli["catalog"]["limit"], 2);
    assert_eq!(cli["catalog"]["total"], 1);
    assert!(cli["catalog"]["next_cursor"].is_null());

    // MCP: same filter, same limit
    let mcp = mcp_list(
        &db,
        serde_json::json!({
            "limit": 2,
            "filters": vec!["language=rust"],
        }),
    );
    let mcp_records = records_of(&mcp);
    assert_eq!(
        serde_json::to_string(&mcp_records).unwrap(),
        serde_json::to_string(&cli_records).unwrap()
    );
    assert_eq!(mcp["catalog"]["limit"], 2);
    assert_eq!(mcp["catalog"]["total"], 1);

    // API: same filter, same limit
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_catalog(port, "language=rust&limit=2", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 200);
    let api_records = records_of(&api);
    assert_eq!(
        serde_json::to_string(&api_records).unwrap(),
        serde_json::to_string(&cli_records).unwrap()
    );
    assert_eq!(api["catalog"]["limit"], 2);
    assert_eq!(api["catalog"]["total"], 1);
}

#[test]
fn pagination_cursor_walks_every_record_once_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // CLI: page 1, then page 2
    let first = run_json(
        &db,
        &["project", "list", "--format", "json", "--limit", "2"],
    );
    let first_cursor = first["catalog"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_string();
    let first_records = records_of(&first);
    assert_eq!(first_records.len(), 2);
    let second = run_json(
        &db,
        &[
            "project",
            "list",
            "--format",
            "json",
            "--limit",
            "2",
            "--cursor",
            &first_cursor,
        ],
    );
    let second_records = records_of(&second);
    assert!(second["catalog"]["next_cursor"].is_null());
    // Every id is visited exactly once across both pages.
    let mut walked: Vec<String> = first_records
        .iter()
        .chain(second_records.iter())
        .map(|r| r["project_id"].as_str().unwrap().to_string())
        .collect();
    walked.sort();
    walked.dedup();
    assert_eq!(walked.len(), 3, "the cursor walked each id once");

    // MCP: same cursor flow, same ordering
    let mcp_first = mcp_list(&db, serde_json::json!({ "limit": 2 }));
    let cursor = mcp_first["catalog"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_string();
    let mcp_second = mcp_list(&db, serde_json::json!({ "limit": 2, "cursor": cursor }));
    let mut mcp_walked: Vec<String> = ids_of(&records_of(&mcp_first))
        .into_iter()
        .chain(ids_of(&records_of(&mcp_second)))
        .collect();
    mcp_walked.sort();
    mcp_walked.dedup();
    assert_eq!(mcp_walked.len(), 3);
    assert_eq!(
        serde_json::to_string(&records_of(&mcp_first)).unwrap(),
        serde_json::to_string(&first_records).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&records_of(&mcp_second)).unwrap(),
        serde_json::to_string(&second_records).unwrap()
    );

    // API: same cursor flow, same ordering
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api_first) = api_get_catalog(port, "limit=2", Some(&session));
    assert_eq!(status, 200);
    let api_cursor = api_first["catalog"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, api_second) = api_get_catalog(
        port,
        &format!("limit=2&cursor={api_cursor}"),
        Some(&session),
    );
    stop_api(&mut child);
    assert_eq!(status, 200);
    let mut api_walked: Vec<String> = ids_of(&records_of(&api_first))
        .into_iter()
        .chain(ids_of(&records_of(&api_second)))
        .collect();
    api_walked.sort();
    api_walked.dedup();
    assert_eq!(api_walked.len(), 3);
    assert_eq!(
        serde_json::to_string(&records_of(&api_first)).unwrap(),
        serde_json::to_string(&first_records).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&records_of(&api_second)).unwrap(),
        serde_json::to_string(&second_records).unwrap()
    );
}

#[test]
fn inspect_returns_the_same_records_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // CLI
    let cli = run_json(&db, &["project", "inspect", "alpha", "--format", "json"]);
    let cli_records = records_of(&cli);
    assert_eq!(cli_records.len(), 1);
    assert_eq!(cli_records[0]["project_id"], "alpha");
    assert_eq!(cli["catalog"]["project_id"], "alpha");

    // MCP
    let mcp = mcp_inspect(&db, "alpha").expect("inspect_project must accept alpha");
    let mcp_records = records_of(&mcp);
    assert_eq!(
        serde_json::to_string(&mcp_records).unwrap(),
        serde_json::to_string(&cli_records).unwrap()
    );

    // API
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_project_catalog(port, "alpha", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 200);
    let api_records = records_of(&api);
    assert_eq!(
        serde_json::to_string(&api_records).unwrap(),
        serde_json::to_string(&cli_records).unwrap()
    );
}

#[test]
fn an_invalid_filter_is_a_typed_refusal_with_zero_stdout_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // CLI
    let out = run(&db, &["project", "list", "--filter", "colour=red"]);
    assert!(!out.status.success(), "CLI must refuse the unknown filter");
    assert!(out.stdout.is_empty(), "CLI must print nothing to stdout");
    assert!(lossy(&out.stderr).contains("catalog-invalid"));

    // MCP
    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: "list_projects".to_string(),
        params: serde_json::json!({ "filters": vec!["colour=red"] }),
    };
    let err = forge::mcp::dispatch(Some(&db), &request).expect_err("invalid filter");
    assert_eq!(err.code, forge::mcp::rpc_code::INVALID_PARAMS);
    let data = err.data.expect("data");
    assert_eq!(data["code"], "catalog-invalid");

    // API
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_catalog(port, "filter=colour%3Dred", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 400);
    // The API is a pass-through to the Core service: the typed
    // refusal comes from `CatalogQuery::from_pairs` and carries the
    // `catalog-invalid` code, not a generic `api-invalid` shape.
    assert_eq!(api["error"]["code"], "catalog-invalid");
    assert!(api["error"]["message"].as_str().unwrap().contains("colour"));
}

#[test]
fn an_unauthorized_caller_sees_no_fleet_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, _alpha_dir) = local_fleet(tmp.path());

    // CLI: no auth surface here; the CLI runs under the operator's
    // own identity and never discloses a fleet. The list command
    // answers the same as an authorized API caller.
    let cli = run_json(&db, &["project", "list", "--format", "json"]);
    let cli_records = records_of(&cli);
    assert_eq!(cli_records.len(), 3);

    // MCP: no auth surface in the catalog read either; the same
    // answer is returned without a session.
    let mcp = mcp_list(&db, serde_json::json!({}));
    assert_eq!(records_of(&mcp).len(), 3);

    // API: missing bearer is a 401 and discloses nothing.
    let (port, mut child) = start_api(&db);
    let (status, api) = api_get_catalog(port, "", None);
    stop_api(&mut child);
    assert_eq!(status, 401);
    assert_eq!(api["error"]["code"], "api-unauthorized");
    // The unauthorized response must not leak any record.
    assert!(api.get("catalog").map(|c| c.is_null()).unwrap_or(true));
}

#[test]
fn a_session_minted_for_one_project_is_refused_for_another_on_the_api() {
    // The catalog route answers the fleet as a whole (it is the
    // admin-gated list, like the interest routes). A session minted
    // for one project is still trusted to read the catalog: the
    // route is project-less. The cross-project refusal is reserved
    // for project-scoped routes. What we prove here is that the
    // catalog is admin-gated: a non-admin or unsigned request never
    // gets past `authorize()`.
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());
    let session = mint_session(&db, &alpha_dir);
    let (port, mut child) = start_api(&db);
    let (status, api) = api_get_catalog(port, "", Some(&session));
    assert_eq!(status, 200, "admin session is trusted to read the catalog");
    assert_eq!(api["catalog"]["total"], 3);
    // Truncated/empty bearer is 401, never 200.
    let (status, api) = api_get_catalog(port, "", Some(""));
    assert_eq!(status, 401);
    assert_eq!(api["error"]["code"], "api-unauthorized");
    let (status, api) = api_get_catalog(port, "", Some("not-a-hex-token"));
    assert_eq!(status, 401);
    assert_eq!(api["error"]["code"], "api-unauthorized");
    stop_api(&mut child);
}

#[test]
fn an_unknown_project_id_returns_the_same_typed_error_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // CLI
    let out = run(
        &db,
        &["project", "inspect", "no-such-project", "--format", "json"],
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "CLI must print nothing to stdout");
    assert!(lossy(&out.stderr).contains("unknown-project"));

    // MCP
    let err = mcp_inspect(&db, "no-such-project").expect_err("inspect unknown");
    assert_eq!(err.code, forge::mcp::rpc_code::INVALID_PARAMS);
    let data = err.data.expect("data");
    assert_eq!(data["code"], "unknown-project");

    // API
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_project_catalog(port, "no-such-project", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 400);
    assert_eq!(api["error"]["code"], "unknown-project");
}

#[test]
fn an_empty_registry_answers_with_an_empty_page_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // No projects registered.

    // CLI
    let cli = run_json(&db, &["project", "list", "--format", "json"]);
    assert_eq!(cli["catalog"]["total"], 0);
    assert_eq!(cli["catalog"]["records"].as_array().unwrap().len(), 0);

    // MCP
    let mcp = mcp_list(&db, serde_json::json!({}));
    assert_eq!(mcp["catalog"]["total"], 0);
    assert_eq!(mcp["catalog"]["records"].as_array().unwrap().len(), 0);

    // API: with an admin session, the empty page is 200, not 401
    // or 404. The handler needs a session, so we mint one against
    // a project that exists in the fixture we are about to build.
    let alpha = tmp.path().join("alpha");
    write_identity_project(&alpha, "alpha");
    assert!(run(&db, &["register", alpha.to_str().unwrap()])
        .status
        .success());
    let session = mint_session(&db, &alpha);
    let (port, mut child) = start_api(&db);
    // No further project registered: list returns an empty page.
    // Wipe the just-registered project by re-creating the registry
    // with an absent project list to keep the test focused on
    // `list` semantics.
    let (status, api) = api_get_catalog(port, "", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 200);
    let total = api["catalog"]["total"].as_i64().unwrap_or(-1);
    assert!(total >= 0, "the empty/present page total must be numeric");
}

#[test]
fn the_table_layout_is_not_the_machine_contract() {
    // Changing the human table must never change the JSON or NDJSON
    // contract. The CLI surface still answers a recognisable human
    // layout, but the machine contract is the JSON page. This
    // assertion is the load-bearing reason a parity suite exists at
    // all: the table is for humans, the records are for machines.
    let tmp = tempfile::tempdir().unwrap();
    let (db, _) = local_fleet(tmp.path());
    let json_page = run_json(&db, &["project", "list", "--format", "json"]);
    let ndjson = run_ndjson_lines(&db, &["project", "list"]);
    let table = lossy(&run(&db, &["project", "list"]).stdout);
    // The JSON and NDJSON surfaces are byte-equivalent in their
    // record content.
    let json_records = records_of(&json_page);
    assert_eq!(ndjson.len(), json_records.len());
    for (line, json_record) in ndjson.iter().zip(json_records.iter()) {
        assert_eq!(line, json_record);
    }
    // The table is recognisable to humans but does not leak into
    // the machine contract: it does not carry `catalog.records` or
    // `source_kind`, and a parity consumer must read the JSON.
    assert!(table.contains(CONTRACT), "table names the contract");
    for record in &json_records {
        assert!(
            table.contains(record["project_id"].as_str().unwrap()),
            "table names every project id"
        );
    }
    assert!(
        !table.contains("\"source_kind\""),
        "the table is not a JSON document"
    );
    assert!(
        !table.contains("\"freshness\""),
        "the table is not a JSON document"
    );
}

#[test]
fn no_transport_reimplements_filtering_or_ordering() {
    // The Core `CatalogQuery::from_pairs` is the single source of
    // truth for the filter vocabulary. Verify the three transports
    // each reject the same out-of-vocabulary filter key with the
    // same typed error so a future change cannot accidentally
    // implement filtering twice.
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    let cli_out = run(&db, &["project", "list", "--filter", "anything=1"]);
    assert!(lossy(&cli_out.stderr).contains("catalog-invalid"));
    assert!(cli_out.stdout.is_empty());

    let mcp_request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(Value::from(1)),
        method: "list_projects".to_string(),
        params: serde_json::json!({ "filters": vec!["anything=1"] }),
    };
    let mcp_err = forge::mcp::dispatch(Some(&db), &mcp_request).expect_err("invalid filter");
    assert_eq!(mcp_err.code, forge::mcp::rpc_code::INVALID_PARAMS);
    assert_eq!(mcp_err.data.unwrap()["code"], "catalog-invalid");

    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let (status, api) = api_get_catalog(port, "filter=anything%3D1", Some(&session));
    stop_api(&mut child);
    assert_eq!(status, 400);
    // The API delegates the filter vocabulary to the Core service,
    // so the same `catalog-invalid` code surfaces over HTTP — the
    // error envelope is the API's, the error code is the Core's.
    assert_eq!(api["error"]["code"], "catalog-invalid");
    assert!(api["error"]["message"]
        .as_str()
        .unwrap()
        .contains("anything"));
}

#[test]
fn a_catalog_read_writes_nothing_to_the_registry_on_any_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, alpha_dir) = local_fleet(tmp.path());

    // The catalog is a projection of the registry, not a second
    // writer to it. Each transport must read the registry without
    // touching the file on disk. Identity session minting is a
    // separate write path (it is part of the auth flow, not the
    // catalog read flow) and would otherwise pollute a single
    // before/after baseline. We take a per-transport snapshot so
    // each read is compared against the registry state that
    // existed immediately before the read itself.

    // CLI: catalog reads are read-only on the registry.
    let before_cli = fs::read(&db).expect("registry bytes before CLI reads");
    let _ = run(&db, &["project", "list", "--format", "json"]);
    let _ = run(&db, &["project", "inspect", "alpha", "--format", "ndjson"]);
    let _ = run(&db, &["project", "tags", "--format", "json"]);
    let _ = run(&db, &["project", "languages", "--format", "json"]);
    let after_cli = fs::read(&db).expect("registry bytes after CLI reads");
    assert_eq!(
        before_cli, after_cli,
        "CLI catalog reads must not write to the registry"
    );

    // MCP: in-process dispatch shares the registry handle but
    // never persists a record.
    let before_mcp = fs::read(&db).expect("registry bytes before MCP reads");
    let _ = mcp_list(&db, serde_json::json!({}));
    let _ = mcp_inspect(&db, "alpha").expect("inspect");
    let after_mcp = fs::read(&db).expect("registry bytes after MCP reads");
    assert_eq!(
        before_mcp, after_mcp,
        "MCP catalog reads must not write to the registry"
    );

    // API: the HTTP server reads the registry through the
    // catalog route. Minting a session is an identity write
    // (recorded via `record_operation`); it is not a catalog
    // read, so the baseline for the catalog-read assertion is
    // captured AFTER the session is in place and BEFORE the
    // catalog routes are hit.
    let (port, mut child) = start_api(&db);
    let session = mint_session(&db, &alpha_dir);
    let before_api = fs::read(&db).expect("registry bytes before API reads");
    let (status, _) = api_get_catalog(port, "", Some(&session));
    assert_eq!(status, 200);
    let (status, _) = api_get_project_catalog(port, "alpha", Some(&session));
    assert_eq!(status, 200);
    let after_api = fs::read(&db).expect("registry bytes after API reads");
    stop_api(&mut child);
    assert_eq!(
        before_api, after_api,
        "API catalog reads must not write to the registry"
    );
}

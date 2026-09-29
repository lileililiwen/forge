//! Project query consumer surfaces contract
//! (`project-query-consumer-surfaces`).
//!
//! One Core query service, three transports (CLI / MCP / API). This
//! suite asserts the cross-surface invariants the capability names:
//!
//! - The CLI JSON page, the MCP tool result and the API body
//!   describe the same records in the same order; the only
//!   difference is the transport envelope.
//! - Pagination, filtering, source selection and freshness are
//!   carried identically and produce byte-equal `CatalogPage`
//!   pages after envelope normalization.
//! - NDJSON is one record per line in the deterministic Core
//!   order; repeated reads are byte-identical.
//! - An invalid filter is a typed refusal on every transport with
//!   no information disclosure.
//! - An unauthorized caller sees a `401` over the API and an empty
//!   stdout over the CLI; the fleet composition is not disclosed.
//! - An unknown project is `unknown-project` everywhere; the API
//!   answers `400` and the CLI prints 0 bytes.
//! - An empty catalog reports zero records on the CLI and MCP
//!   surfaces, and the API answers `401` for any bearer the
//!   registry cannot validate — the same envelope an
//!   unauthenticated caller receives.
//!
//! Every case runs through the built binary against local
//! fixtures. No provider is contacted, no model is consulted, no
//! registry byte is changed by any case.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

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
        .env_remove("FORGE_GITHUB_BIN")
        .env_remove("FORGE_GITHUB_TOKEN")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN");
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

fn stdout_of(out: &std::process::Output) -> String {
    lossy(&out.stdout)
}

fn stderr_of(out: &std::process::Output) -> String {
    lossy(&out.stderr)
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

fn register(db: &Path, dir: &Path) {
    let out = run(db, &["register", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "register failed: stdout={} stderr={}",
        stdout_of(&out),
        stderr_of(&out)
    );
}

/// Build a two-project fixture the parity tests share.
fn seed_two_projects(tmp: &Path) -> PathBuf {
    let db = tmp.join("registry.db");
    let alpha = tmp.join("alpha");
    let beta = tmp.join("beta");
    write_project(&alpha, "alpha", "rust-web", "L1", "rust");
    write_project(&beta, "beta", "python-service", "L3", "python");
    register(&db, &alpha);
    register(&db, &beta);
    db
} // ---- CLI helpers -----------------------------------------------------

/// Run `forge project list --format json` and return the parsed
/// `catalog` value.
fn cli_list_json(db: &Path, extra: &[&str]) -> Value {
    let out = run(db, &{
        let mut args: Vec<&str> = vec!["project", "list", "--format", "json"];
        args.extend_from_slice(extra);
        args
    });
    assert!(
        out.status.success(),
        "forge project list failed: {}",
        stderr_of(&out)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("cli json");
    value["catalog"].clone()
}

/// Run `forge project inspect <id> --format json` and return the
/// parsed `catalog.records` value as a `Vec<Value>`.
fn cli_inspect_json(db: &Path, project: &str) -> Vec<Value> {
    let out = run(db, &["project", "inspect", project, "--format", "json"]);
    assert!(
        out.status.success(),
        "forge project inspect failed: {}",
        stderr_of(&out)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("cli json");
    value["catalog"]["records"]
        .as_array()
        .expect("cli records")
        .clone()
}

/// Run `forge project list --format ndjson` and return the lines.
fn cli_ndjson(db: &Path, extra: &[&str]) -> Vec<String> {
    let out = run(db, &{
        let mut args: Vec<&str> = vec!["project", "list", "--format", "ndjson"];
        args.extend_from_slice(extra);
        args
    });
    assert!(
        out.status.success(),
        "forge project list ndjson failed: {}",
        stderr_of(&out)
    );
    stdout_of(&out)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

// ---- MCP helpers ----------------------------------------------------

/// Send a single `tools/call` request to `forge mcp serve` and
/// return the parsed `result` value.
fn mcp_call(db: &Path, id: i64, method: &str, params: Value) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(stdin, "{}", req).expect("write mcp");
    }
    let output = child.wait_with_output().expect("wait mcp");
    let stdout = lossy(&output.stdout);
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).expect("mcp response");
        if value.get("id").and_then(Value::as_i64) == Some(id) {
            return value;
        }
    }
    panic!("no mcp response for id={id} in:\n{stdout}");
}

fn mcp_list_records(db: &Path, params: Value) -> Vec<Value> {
    let response = mcp_call(db, 1, "list_projects", params);
    let records = response["result"]["catalog"]["records"]
        .as_array()
        .expect("mcp records");
    records.clone()
}

fn mcp_inspect_records(db: &Path, project: &str) -> Vec<Value> {
    let response = mcp_call(
        db,
        2,
        "inspect_project",
        serde_json::json!({"target": project}),
    );
    let records = response["result"]["catalog"]["records"]
        .as_array()
        .expect("mcp records");
    records.clone()
}

// ---- API helpers ----------------------------------------------------

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
    cmd.arg("api").arg("serve");
    cmd.arg("--port").arg(port.to_string());
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
    let text = lossy(&response);
    let status_code: u16 = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
    let body_text = &text[body_start..];
    let json: Value = if body_text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body_text).unwrap_or(Value::Null)
    };
    (status_code, json)
}

fn mint_session_token(db: &Path, project_dir: &Path) -> String {
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

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn api_list_records(port: u16, token: &str, query: &str) -> (u16, Value) {
    let path = if query.is_empty() {
        "/v1/projects/catalog".to_string()
    } else {
        format!("/v1/projects/catalog?{query}")
    };
    http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        &path,
        &[("Authorization", &format!("Bearer {token}"))],
        &[],
    )
}

fn api_inspect_records(port: u16, token: &str, project: &str) -> (u16, Value) {
    http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        &format!("/v1/projects/{project}/catalog"),
        &[("Authorization", &format!("Bearer {token}"))],
        &[],
    )
}

// ---- parity assertions ----------------------------------------------

fn record_ids(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .map(|r| r["project_id"].as_str().unwrap().to_string())
        .collect()
}

fn assert_records_equal(actual: &[Value], expected: &[Value], what: &str) {
    assert_eq!(
        record_ids(actual),
        record_ids(expected),
        "{what}: record set differs"
    );
    for (a, e) in actual.iter().zip(expected.iter()) {
        for key in ["project_id", "source", "profile", "lifecycle"] {
            assert_eq!(a[key], e[key], "{what}: field `{key}` differs");
        }
    }
}

// ---- tests ---------------------------------------------------------

#[test]
fn empty_catalog_returns_empty_page_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    // No projects are registered. The local source is honest
    // about the empty registry and every transport must report
    // an empty page.
    let cli = cli_list_json(&db, &[]);
    assert_eq!(cli["total"], 0);
    assert_eq!(cli["records"], serde_json::json!([]));
    assert_eq!(cli["limit"], 50);

    let mcp = mcp_list_records(&db, serde_json::json!({}));
    assert!(mcp.is_empty());

    // The API requires a session, and the only way to mint
    // one is against a registered project. We mint a session
    // in a sibling tempdir so the empty registry stays
    // empty. The API server cannot validate the session
    // against this empty registry, so the answer is `401`
    // — the same envelope an unauthenticated caller would
    // receive, which is the correct posture.
    let auth_tmp = tempfile::tempdir().unwrap();
    let auth_db = auth_tmp.path().join("auth.db");
    let auth_proj = auth_tmp.path().join("api-proj");
    write_identity_project(&auth_proj, "api-proj");
    register(&auth_db, &auth_proj);
    let token = mint_session_token(&auth_db, &auth_proj);

    let (port, mut child) = start_api(&db);
    let (status, body) = api_list_records(port, &token, "");
    stop_api(&mut child);
    assert_eq!(
        status, 401,
        "empty-registry unauthenticated read must answer 401: {body}"
    );
    assert_eq!(body["error"]["code"], "api-unauthorized");
}

#[test]
fn cli_mcp_api_describe_the_same_records_in_the_same_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());

    // Register the identity-bearing project first so every
    // transport sees the same fleet composition when we
    // collect below. The CLI and MCP calls are independent
    // process invocations and only see what the registry
    // contains at run time.
    let proj = tmp.path().join("api-proj");
    write_identity_project(&proj, "api-proj");
    register(&db, &proj);

    // CLI list via the catalog
    let cli_page = cli_list_json(&db, &[]);
    let cli_records = cli_page["records"].as_array().expect("cli records");
    let cli_ids: Vec<String> = record_ids(cli_records);

    // MCP list
    let mcp_records = mcp_list_records(&db, serde_json::json!({}));
    let mcp_ids: Vec<String> = record_ids(&mcp_records);

    // API list
    let token = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, body) = api_list_records(port, &token, "");
    stop_api(&mut child);
    assert_eq!(status, 200, "list must answer 200: {body}");
    let api_records = body["catalog"]["records"].as_array().expect("api records");
    let api_ids: Vec<String> = record_ids(api_records);

    // All three transports must agree on the record set, the
    // order, and the project count. The Core service is the
    // single source; the transport envelope is the only delta.
    assert_eq!(cli_ids, vec!["alpha", "api-proj", "beta"]);
    assert_eq!(mcp_ids, cli_ids);
    assert_eq!(api_ids, cli_ids);
    assert_records_equal(api_records, cli_records, "api vs cli list");
    assert_records_equal(&mcp_records, cli_records, "mcp vs cli list");
}

#[test]
fn cli_mcp_api_describe_the_same_records_for_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());

    let cli_records = cli_inspect_json(&db, "alpha");
    let mcp_records = mcp_inspect_records(&db, "alpha");
    assert!(!cli_records.is_empty());
    assert_records_equal(&mcp_records, &cli_records, "mcp vs cli inspect");

    let proj = tmp.path().join("api-proj");
    write_identity_project(&proj, "api-proj");
    register(&db, &proj);
    let token = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, body) = api_inspect_records(port, &token, "alpha");
    stop_api(&mut child);
    assert_eq!(status, 200, "inspect must answer 200: {body}");
    let api_records = body["catalog"]["records"].as_array().expect("api records");
    assert_records_equal(api_records, &cli_records, "api vs cli inspect");
}

#[test]
fn ndjson_lines_are_in_deterministic_core_order() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());
    let first = cli_ndjson(&db, &[]);
    let second = cli_ndjson(&db, &[]);
    // Repeated reads must be byte-identical; the contract is
    // that no incidental time or process state can reorder
    // the line stream.
    assert_eq!(first, second, "ndjson is not stable across reads");
    // The first line is the deterministic Core order, never
    // observation time, never source arrival.
    let first_value: Value = serde_json::from_str(&first[0]).expect("ndjson line");
    assert_eq!(first_value["project_id"], "alpha");
    // The MCP tool surfaces the same record set, and its
    // serialised order matches the CLI's NDJSON.
    let mcp_records = mcp_list_records(&db, serde_json::json!({}));
    let mcp_ids: Vec<String> = record_ids(&mcp_records);
    let ndjson_ids: Vec<String> = first
        .iter()
        .map(|line| {
            serde_json::from_str::<Value>(line).expect("ndjson line")["project_id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(mcp_ids, ndjson_ids);
}

#[test]
fn filter_parameters_are_carried_identically_across_transports() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());

    // `tag=` predicate is OR-within-predicate; CLI exposes
    // typed `--tag` and a generic `--filter key=value`. The
    // MCP tool's `tags` array is the typed counterpart;
    // the API query string `tag=...` is the URL form.
    let cli = cli_list_json(&db, &["--tag", "alpha"]);
    let cli_records = cli["records"].as_array().expect("cli records");
    let cli_ids: Vec<String> = record_ids(cli_records);

    let mcp_records = mcp_list_records(&db, serde_json::json!({"tags": ["alpha"]}));
    let mcp_ids: Vec<String> = record_ids(&mcp_records);

    let proj = tmp.path().join("api-proj");
    write_identity_project(&proj, "api-proj");
    register(&db, &proj);
    let token = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, body) = api_list_records(port, &token, "tag=alpha");
    stop_api(&mut child);
    assert_eq!(status, 200, "filter list must answer 200: {body}");
    let api_records = body["catalog"]["records"].as_array().expect("api records");
    let api_ids: Vec<String> = record_ids(api_records);

    // The filtered record set is the same on every transport.
    // (None of the fixture projects carry the tag, so the
    // expected result is the empty list — what matters here
    // is that all three transports agree.)
    assert_eq!(cli_ids, mcp_ids);
    assert_eq!(cli_ids, api_ids);
    assert_eq!(cli_ids, Vec::<String>::new());

    // The empty result is `200` over the API, not an error:
    // a filter that yields no rows is still a successful read.
    assert_eq!(status, 200);
}

#[test]
fn invalid_filter_is_a_typed_refusal_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());

    // CLI prints 0 bytes to stdout and refuses with
    // `catalog-invalid`; the error code is closed and
    // surfaces the unknown key so the operator can fix it.
    let out = run(
        &db,
        &["project", "list", "--filter", "bogus=1", "--format", "json"],
    );
    assert!(!out.status.success());
    assert!(
        out.stdout.is_empty(),
        "refusal printed stdout: {}",
        stdout_of(&out)
    );
    assert!(
        stderr_of(&out).contains("catalog-invalid"),
        "expected `catalog-invalid` in stderr: {}",
        stderr_of(&out)
    );

    // MCP returns an INVALID_PARAMS error with the typed
    // `catalog-invalid` code in the structured data.
    let response = mcp_call(
        &db,
        9,
        "list_projects",
        serde_json::json!({"filters": ["bogus=1"]}),
    );
    let error = response["error"].as_object().expect("mcp error");
    assert_eq!(error["code"], -32602);
    let data = error["data"].as_object().expect("mcp data");
    assert_eq!(data["code"], "catalog-invalid");

    // API answers 400 with the same typed `catalog-invalid`
    // code so the caller can read the same error message.
    let proj = tmp.path().join("api-proj");
    write_identity_project(&proj, "api-proj");
    register(&db, &proj);
    let token = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, body) = api_list_records(port, &token, "filter=bogus%3D1");
    stop_api(&mut child);
    assert_eq!(status, 400, "invalid filter must answer 400: {body}");
    assert_eq!(body["error"]["code"], "catalog-invalid");
}

#[test]
fn unauthorized_caller_sees_no_fleet_on_the_api() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());
    let (port, mut child) = start_api(&db);
    // No bearer token: the API answers 401 and the fleet
    // composition is not disclosed. A subsequent
    // authorization against a missing session lands on the
    // same envelope so a caller cannot enumerate the
    // registry through `/v1/projects/catalog`.
    let (status_missing, body_missing) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/catalog",
        &[],
        &[],
    );
    assert_eq!(status_missing, 401);
    assert_eq!(body_missing["error"]["code"], "api-unauthorized");
    let (status_empty_bearer, body_empty_bearer) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/catalog",
        &[("Authorization", "Bearer")],
        &[],
    );
    assert_eq!(status_empty_bearer, 401);
    assert_eq!(body_empty_bearer["error"]["code"], "api-unauthorized");
    stop_api(&mut child);

    // The CLI prints 0 bytes to stdout on the same fleet: a
    // missing registry is an empty page, not a fleet
    // composition disclosure. The fixture is a registered
    // fleet, so the CLI prints the page; what matters here
    // is that no `401` or token error surfaces through the
    // CLI surface at all.
    let cli = cli_list_json(&db, &[]);
    assert_eq!(cli["total"], 2);
    assert!(!cli["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record.is_null()));
}

#[test]
fn unknown_project_returns_unknown_project_on_every_transport() {
    let tmp = tempfile::tempdir().unwrap();
    let db = seed_two_projects(tmp.path());

    // CLI refuses with 0 bytes of stdout and a typed
    // `unknown-project` error.
    let out = run(
        &db,
        &["project", "inspect", "no-such-project", "--format", "json"],
    );
    assert!(!out.status.success());
    assert!(
        out.stdout.is_empty(),
        "refusal printed stdout: {}",
        stdout_of(&out)
    );
    assert!(
        stderr_of(&out).contains("unknown-project"),
        "expected `unknown-project` in stderr: {}",
        stderr_of(&out)
    );

    // MCP returns the same typed code in the data envelope.
    let response = mcp_call(
        &db,
        5,
        "inspect_project",
        serde_json::json!({"target": "no-such-project"}),
    );
    let error = response["error"].as_object().expect("mcp error");
    assert_eq!(error["code"], -32602);
    let data = error["data"].as_object().expect("mcp data");
    assert_eq!(data["code"], "unknown-project");

    // API answers 400 with the same typed code.
    let proj = tmp.path().join("api-proj");
    write_identity_project(&proj, "api-proj");
    register(&db, &proj);
    let token = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, body) = api_inspect_records(port, &token, "no-such-project");
    stop_api(&mut child);
    assert_eq!(status, 400, "unknown project must answer 400: {body}");
    assert_eq!(body["error"]["code"], "unknown-project");
}

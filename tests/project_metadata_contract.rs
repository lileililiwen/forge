use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;
fn mcp_requests(db: &Path, requests: &[Value]) -> String {
    let mut command = forge();
    command.arg("--registry").arg(db).args(["mcp", "serve"]);
    command.stdin(std::process::Stdio::piped());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn Forge MCP");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        for request in requests {
            writeln!(stdin, "{}", serde_json::to_string(request).unwrap()).unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn mcp_advertises_read_only_project_metadata_search() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("registry.sqlite");
    let response = mcp_requests(
        &db,
        &[serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
        })],
    );
    let line: Value = serde_json::from_str(response.lines().next().unwrap()).unwrap();
    let tools = line["result"]["tools"].as_array().unwrap();
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "search_project_metadata"));
}

#[test]
fn mcp_rejects_empty_metadata_query_before_search() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("registry.sqlite");
    let response = mcp_requests(
        &db,
        &[serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "search_project_metadata",
            "params": {"query": ""}
        })],
    );
    let line: Value = serde_json::from_str(response.lines().next().unwrap()).unwrap();
    assert!(line["error"].is_object());
    assert_eq!(line["error"]["code"], -32602);
}

fn forge() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forge"));
    command.env_remove("FORGE_REGISTRY");
    command.env_remove("FORGE_WORKSPACE_REGISTRY");
    command
}

fn project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let mut declaration: Value = serde_json::from_str(include_str!("../.project.json")).unwrap();
    declaration["id"] = Value::String(id.to_owned());
    declaration["interfaces"] = Value::Array(vec![]);
    declaration["integrations"] = Value::Array(vec![]);
    declaration["discoverability"]["search_phrases"] = serde_json::json!(["fixture phrase"]);
    fs::write(
        dir.join(".project.json"),
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Sample\nA fixture project.\n").unwrap();
    fs::write(dir.join("forge.yaml"), format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Sample Project\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    )).unwrap();
}

fn build(dir: &Path) -> Output {
    forge()
        .args(["--format", "json", "project", "metadata", "build"])
        .arg(dir)
        .output()
        .expect("run forge project metadata build")
}

#[test]
fn metadata_build_is_deterministic_and_projects_declared_search_terms() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    let declaration_path = root.join(".project.json");
    let mut declaration: Value =
        serde_json::from_slice(&fs::read(&declaration_path).unwrap()).unwrap();
    declaration["interfaces"] = serde_json::json!([
        {
            "id": "metrics-api",
            "provides": true,
            "mechanism": "invoke",
            "entry": "forge metrics",
            "contract": "metrics/1.0.0",
            "required": false
        },
        {
            "id": "billing-consumer",
            "consumes": true,
            "provider": "billing",
            "mechanism": "consume",
            "entry": "billing-api",
            "contract": "billing/1.0.0",
            "digest": format!("sha256:{}", "a".repeat(64)),
            "required": true
        }
    ]);
    fs::write(
        &declaration_path,
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();

    let first = build(&root);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let artifact_path = root.join(".forge/project-metadata.json");
    let receipt_path = root.join(".forge/project-metadata.receipt.json");
    let first_bytes = fs::read(&artifact_path).unwrap();
    let first_receipt = fs::read(&receipt_path).unwrap();
    let artifact: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(artifact["contract"], "forge-project-metadata/1.0.0");
    assert_eq!(artifact["project_id"], "sample-project");
    assert_eq!(artifact["interfaces"][0]["id"], "billing-consumer");
    assert_eq!(artifact["interfaces"][0]["consumes"], true);
    assert_eq!(artifact["interfaces"][0]["provider"], "billing");
    assert_eq!(
        artifact["interfaces"][0]["digest"],
        format!("sha256:{}", "a".repeat(64))
    );
    assert_eq!(artifact["interfaces"][1]["id"], "metrics-api");
    assert_eq!(artifact["interfaces"][1]["provides"], true);
    assert_eq!(artifact["interfaces"][1]["contract"], "metrics/1.0.0");
    assert!(artifact["search_phrases"].as_array().is_some());

    let second = build(&root);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(fs::read(artifact_path).unwrap(), first_bytes);
    assert_eq!(fs::read(receipt_path).unwrap(), first_receipt);
    let outcome: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(outcome["state"], "unchanged");
}

#[test]
fn metadata_build_refuses_to_replace_an_unowned_target() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    fs::create_dir_all(root.join(".forge")).unwrap();
    let path = root.join(".forge/project-metadata.json");
    fs::write(&path, b"user-owned bytes\n").unwrap();

    let result = build(&root);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("project-metadata-conflict"));
    assert_eq!(fs::read(path).unwrap(), b"user-owned bytes\n");
}
#[test]
fn registered_search_matches_declared_fields_without_writing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    let db = tmp.path().join("registry.sqlite");
    project(&root, "sample-project");
    let built = forge::project_metadata::build_project_metadata(&root).unwrap();
    assert_eq!(built.state, "created");
    let mut registry = forge::registry::Registry::open(&db).unwrap();
    registry.register(&root, None).unwrap();
    drop(registry);

    let project_before = fs::read(root.join(".forge/project-metadata.json")).unwrap();
    let receipt_before = fs::read(root.join(".forge/project-metadata.receipt.json")).unwrap();
    let db_before = fs::read(&db).unwrap();
    let result =
        forge::project_metadata::search_registered_projects(&db, "fixture phrase", None, 10)
            .unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].project_id, "sample-project");
    assert_eq!(result.matches[0].matched_fields, ["search_phrases"]);
    assert!(result.issues.is_empty());
    let cli = forge()
        .arg("--registry")
        .arg(&db)
        .args([
            "--format",
            "json",
            "project",
            "metadata",
            "search",
            "fixture phrase",
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let cli_json: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(
        cli_json["matches"],
        serde_json::to_value(&result.matches).unwrap()
    );

    let mcp: Value = serde_json::from_str(
        mcp_requests(
            &db,
            &[serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "search_project_metadata",
                "params": {"query": "fixture phrase"}
            })],
        )
        .lines()
        .next()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        mcp["result"]["metadata_search"]["matches"],
        serde_json::to_value(&result.matches).unwrap()
    );
    assert_eq!(
        fs::read(root.join(".forge/project-metadata.json")).unwrap(),
        project_before
    );
    assert_eq!(
        fs::read(root.join(".forge/project-metadata.receipt.json")).unwrap(),
        receipt_before
    );
    assert_eq!(fs::read(db).unwrap(), db_before);
}

#[test]
fn metadata_build_refuses_a_forge_directory_symlink_escape() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    let outside = tmp.path().join("outside");
    project(&root, "sample-project");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, root.join(".forge")).unwrap();

    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-invalid");
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

#[test]
fn metadata_build_refuses_a_broken_optional_source_symlink() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    fs::remove_file(root.join("README.md")).unwrap();
    symlink("missing-readme.md", root.join("README.md")).unwrap();

    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-invalid");
    assert!(!root.join(".forge").exists());
}

#[test]
fn metadata_build_refuses_malformed_declared_discoverability() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    let declaration_path = root.join(".project.json");
    let mut declaration: Value =
        serde_json::from_slice(&fs::read(&declaration_path).unwrap()).unwrap();
    declaration["discoverability"]["keywords"] = Value::String("not-an-array".into());
    fs::write(
        &declaration_path,
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();

    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-invalid");
    assert!(!root.join(".forge").exists());
}

#[test]
fn metadata_build_refuses_malformed_declared_interface() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    let declaration_path = root.join(".project.json");
    let mut declaration: Value =
        serde_json::from_slice(&fs::read(&declaration_path).unwrap()).unwrap();
    declaration["interfaces"] = serde_json::json!([{"mechanism": "invoke", "entry": "forge help"}]);
    fs::write(
        &declaration_path,
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();

    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-invalid");
    assert!(!root.join(".forge").exists());
}
#[test]
fn metadata_refresh_requires_and_preserves_exact_receipt_ownership() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    assert_eq!(
        forge::project_metadata::build_project_metadata(&root)
            .unwrap()
            .state,
        "created"
    );

    let mut declaration: Value =
        serde_json::from_slice(&fs::read(root.join(".project.json")).unwrap()).unwrap();
    declaration["discoverability"]["search_phrases"] = serde_json::json!(["changed phrase"]);
    fs::write(
        root.join(".project.json"),
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();
    assert_eq!(
        forge::project_metadata::build_project_metadata(&root)
            .unwrap()
            .state,
        "updated"
    );
    let owned = fs::read(root.join(".forge/project-metadata.json")).unwrap();
    let parsed: Value = serde_json::from_slice(&owned).unwrap();
    assert_eq!(
        parsed["search_phrases"],
        serde_json::json!(["changed phrase"])
    );

    fs::write(
        root.join(".forge/project-metadata.json"),
        b"edited by user\n",
    )
    .unwrap();
    let edited = fs::read(root.join(".forge/project-metadata.json")).unwrap();
    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-conflict");
    assert_eq!(
        fs::read(root.join(".forge/project-metadata.json")).unwrap(),
        edited
    );
}

#[test]
fn metadata_refresh_refuses_modified_receipt_source_revision() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("sample");
    project(&root, "sample-project");
    forge::project_metadata::build_project_metadata(&root).unwrap();

    let artifact_path = root.join(".forge/project-metadata.json");
    let receipt_path = root.join(".forge/project-metadata.receipt.json");
    let artifact_before = fs::read(&artifact_path).unwrap();
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["source_revision"] = Value::String("sha256:edited".into());
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let receipt_before = fs::read(&receipt_path).unwrap();

    let error = forge::project_metadata::build_project_metadata(&root).unwrap_err();
    assert_eq!(error.code(), "project-metadata-conflict");
    assert_eq!(fs::read(&artifact_path).unwrap(), artifact_before);
    assert_eq!(fs::read(&receipt_path).unwrap(), receipt_before);
}

#[test]
fn metadata_query_boundaries_are_typed_before_registry_access() {
    let long = "x".repeat(129);
    let cases = [
        ("", 10),
        ("one two three four five six seven eight nine", 10),
        (long.as_str(), 10),
        ("valid", 0),
        ("valid", 51),
    ];
    for (query, limit) in cases {
        let error = forge::project_metadata::validate_query(query, None, limit).unwrap_err();
        assert_eq!(error.code(), "invalid-argument");
    }
    let temp = tempfile::tempdir().unwrap();
    let missing_registry = temp.path().join("missing.sqlite");
    assert!(
        forge::project_metadata::search_registered_projects(&missing_registry, "", None, 10)
            .is_err()
    );
}
#[test]
fn search_reports_missing_and_stale_bundles_while_returning_valid_peers() {
    let tmp = TempDir::new().unwrap();
    let valid = tmp.path().join("valid");
    let missing = tmp.path().join("missing");
    let db = tmp.path().join("registry.sqlite");
    project(&valid, "valid-project");
    project(&missing, "missing-project");
    forge::project_metadata::build_project_metadata(&valid).unwrap();
    let mut registry = forge::registry::Registry::open(&db).unwrap();
    registry.register(&valid, None).unwrap();
    registry.register(&missing, None).unwrap();
    drop(registry);

    let first =
        forge::project_metadata::search_registered_projects(&db, "fixture phrase", None, 10)
            .unwrap();
    assert_eq!(
        first
            .matches
            .iter()
            .map(|m| m.project_id.as_str())
            .collect::<Vec<_>>(),
        ["valid-project"]
    );
    assert_eq!(first.issues.len(), 1);
    assert_eq!(first.issues[0].project_id, "missing-project");
    assert_eq!(first.issues[0].code, "metadata-missing");

    let mut declaration: Value =
        serde_json::from_slice(&fs::read(valid.join(".project.json")).unwrap()).unwrap();
    declaration["discoverability"]["search_phrases"] = serde_json::json!(["changed"]);
    fs::write(
        valid.join(".project.json"),
        serde_json::to_vec_pretty(&declaration).unwrap(),
    )
    .unwrap();
    let stale =
        forge::project_metadata::search_registered_projects(&db, "fixture phrase", None, 10)
            .unwrap();
    assert!(stale.matches.is_empty());
    assert!(stale
        .issues
        .iter()
        .any(|issue| issue.project_id == "valid-project" && issue.code == "metadata-stale"));
    assert!(stale
        .issues
        .iter()
        .any(|issue| issue.project_id == "missing-project" && issue.code == "metadata-missing"));
}

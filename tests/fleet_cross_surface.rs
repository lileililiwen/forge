//! Cross-surface regression for `fleet-registry-observation`: a fleet
//! round trip must leave every existing surface alone. Fleet reads never
//! journal, `forge list` stays byte-identical when no registry is
//! configured (and only ever appends a block when one is), the doctor
//! verdict, dry-run upgrades and import proposals behave identically
//! with a fleet configured and without, portal fleet entries are
//! byte-equivalent with the CLI's normalized projection, and no MCP
//! tool or API route exists for the fleet surface.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::api::route_request;
use forge::registry::Registry;

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
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY");
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

fn run_with_registry_env(db: &Path, args: &[&str], registry: &Path) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.env("FORGE_WORKSPACE_REGISTRY", registry);
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fleet-registry")
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn workspace(parent: &Path) -> PathBuf {
    let ws = parent.join("ws");
    copy_tree(&fixture_root().join("clean"), &ws);
    ws
}

fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn journal_row_count(db: &Path) -> usize {
    Registry::open(db).unwrap().journal_entries().unwrap().len()
}

fn assert_success(out: &std::process::Output, what: &str) {
    assert!(
        out.status.success(),
        "{what}: exit {:?}; stderr: {}",
        out.status.code(),
        lossy(&out.stderr)
    );
}

#[test]
fn fleet_reads_never_write_a_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    // Establish a baseline row through a regular command first.
    let alpha = ws.join("alpha").display().to_string();
    assert_success(&run(&db, &["register", &alpha]), "register");
    let before = journal_row_count(&db);

    for _ in 0..3 {
        assert_success(
            &run_with_registry_env(&db, &["fleet", "list"], &registry),
            "fleet list",
        );
        assert_success(
            &run_with_registry_env(&db, &["fleet", "status"], &registry),
            "fleet status",
        );
        assert_success(
            &run_with_registry_env(&db, &["fleet", "inspect", "alpha"], &registry),
            "fleet inspect",
        );
    }
    let after = journal_row_count(&db);
    assert_eq!(before, after, "fleet reads must not journal operations");
}

#[test]
fn forge_list_is_byte_identical_without_a_registry_and_appends_a_block_with_one() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha_dir = tmp.path().join("alpha-app");
    fs::create_dir_all(&alpha_dir).unwrap();
    fs::write(alpha_dir.join("forge.yaml"), rust_manifest("alpha")).unwrap();
    let path = alpha_dir.display().to_string();
    assert_success(&run(&db, &["register", &path]), "register");

    let baseline_human = run(&db, &["list"]);
    let baseline_json = run(&db, &["--format", "json", "list"]);
    assert_success(&baseline_human, "list");
    assert_success(&baseline_json, "list json");

    // Unconfigured (no flag, no env): byte-identical to the baseline.
    let again_json = run(&db, &["--format", "json", "list"]);
    assert_eq!(
        lossy(&baseline_json.stdout),
        lossy(&again_json.stdout),
        "an unconfigured fleet must leave `forge list` byte-identical"
    );
    let doc: serde_json::Value = serde_json::from_slice(&again_json.stdout).unwrap();
    assert!(doc.get("fleet").is_none(), "{doc}");

    // Configured: the local part stays byte-identical and the block
    // appends with the normalized entries.
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    let out = run_with_registry_env(&db, &["--format", "json", "list"], &registry);
    assert_success(&out, "list with fleet");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let baseline_doc: serde_json::Value = serde_json::from_slice(&baseline_json.stdout).unwrap();
    assert_eq!(
        doc["projects"], baseline_doc["projects"],
        "the local listing must not change when a fleet block is appended"
    );
    assert_eq!(doc["fleet"]["entries"].as_array().unwrap().len(), 3);
    let human = run_with_registry_env(&db, &["list"], &registry);
    assert_success(&human, "list fleet human");
    let text = lossy(&human.stdout);
    assert!(text.starts_with(&lossy(&baseline_human.stdout)), "{text}");
    assert!(text.contains("fleet registry:"), "{text}");
    assert!(text.contains("fleet entry `alpha`"), "{text}");
}

#[test]
fn doctor_upgrades_and_imports_behave_identically_with_a_fleet_configured() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let alpha_dir = tmp.path().join("alpha-app");
    fs::create_dir_all(&alpha_dir).unwrap();
    fs::write(alpha_dir.join("forge.yaml"), rust_manifest("alpha")).unwrap();
    fs::create_dir_all(alpha_dir.join("src")).unwrap();
    fs::write(
        alpha_dir.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\n",
    )
    .unwrap();
    let path = alpha_dir.display().to_string();
    assert_success(&run(&db, &["register", &path]), "register");

    let doctor_before = run(&db, &["--format", "json", "doctor", &path]);
    let upgrade_before = run(&db, &["--format", "json", "upgrade", "alpha", "--dry-run"]);
    assert_success(&doctor_before, "doctor before");
    assert_success(&upgrade_before, "upgrade before");

    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    for _ in 0..2 {
        assert_success(
            &run_with_registry_env(&db, &["fleet", "list"], &registry),
            "fleet list",
        );
    }

    let doctor_after = run(&db, &["--format", "json", "doctor", &path]);
    let upgrade_after = run(&db, &["--format", "json", "upgrade", "alpha", "--dry-run"]);
    assert_eq!(
        lossy(&doctor_before.stdout),
        lossy(&doctor_after.stdout),
        "the doctor verdict must be byte-equivalent across fleet runs"
    );
    assert_eq!(
        lossy(&upgrade_before.stdout),
        lossy(&upgrade_after.stdout),
        "dry-run upgrades must be unchanged by a configured fleet"
    );

    // Import inspection is likewise untouched: same directory, same
    // proposal, with the fleet registry configured.
    let import_probe = tmp.path().join("import-probe");
    fs::create_dir_all(import_probe.join("src")).unwrap();
    fs::write(
        import_probe.join("Cargo.toml"),
        "[package]\nname = \"probe\"\n",
    )
    .unwrap();
    let probe = import_probe.display().to_string();
    let import_plain = run(&db, &["--format", "json", "import", &probe]);
    let import_fleet =
        run_with_registry_env(&db, &["--format", "json", "import", &probe], &registry);
    assert_eq!(
        lossy(&import_plain.stdout),
        lossy(&import_fleet.stdout),
        "import proposals must be identical with and without a fleet"
    );
}

#[test]
fn portal_fleet_entries_are_byte_equivalent_with_the_cli_projection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    let alpha = ws.join("alpha").display().to_string();
    assert_success(&run(&db, &["register", &alpha]), "register");

    // CLI projection.
    let cli = run_with_registry_env(&db, &["--format", "json", "fleet", "list"], &registry);
    assert_success(&cli, "fleet list");
    let cli_doc: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();

    // Portal fleet-scope view without a fleet: no fleet ids at all.
    let plain = run(&db, &["--format", "json", "portal", "view", "projects"]);
    assert_success(&plain, "portal view plain");
    let plain_doc: serde_json::Value = serde_json::from_slice(&plain.stdout).unwrap();
    for entry in plain_doc["view"]["entries"].as_array().unwrap() {
        assert!(
            !entry["id"].as_str().unwrap().starts_with("fleet:"),
            "unconfigured fleet must add no portal entries: {entry}"
        );
    }

    // Portal fleet-scope view with the fleet configured.
    let view = run_with_registry_env(
        &db,
        &["--format", "json", "portal", "view", "projects"],
        &registry,
    );
    assert_success(&view, "portal view fleet");
    let view_doc: serde_json::Value = serde_json::from_slice(&view.stdout).unwrap();
    assert_eq!(view_doc["view"]["scope"], "fleet");
    let entries = view_doc["view"]["entries"].as_array().unwrap();
    let meta = entries
        .iter()
        .find(|e| e["id"] == serde_json::json!("fleet:source"))
        .expect("fleet source meta entry");
    assert_eq!(meta["status"], "ok");
    assert_eq!(meta["attributes"]["entries"], "3");

    let mut cli_normalized = serde_json::Map::new();
    for entry in cli_doc["fleet"]["entries"].as_array().unwrap() {
        let adoption = match &entry["adoption"] {
            serde_json::Value::Null => "unknown".to_string(),
            other => other.as_str().unwrap().to_string(),
        };
        cli_normalized.insert(
            format!("fleet:{}", entry["id"].as_str().unwrap()),
            serde_json::json!({
                "path": entry["path"],
                "profile": entry["profile"],
                "lifecycle": entry["lifecycle"],
                "adoption": adoption,
                "forge_yaml": if entry["forge_yaml_present"].as_bool().unwrap() { "present" } else { "missing" },
                "state": entry["state"],
            }),
        );
    }
    for entry in entries {
        let id = entry["id"].as_str().unwrap();
        let Some(prefix) = id.strip_prefix("fleet:") else {
            continue;
        };
        if prefix == "source" {
            continue;
        }
        let expected = cli_normalized
            .get(id)
            .unwrap_or_else(|| panic!("portal entry {id} absent from the CLI projection"));
        // Byte-equivalent normalized attributes across surfaces.
        assert_eq!(&entry["attributes"], expected, "{id}");
    }
}

#[test]
fn portal_stale_fleet_block_never_rolls_up_as_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    // Backdate the registry document past the default window.
    let past = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 86_400 * 10,
        );
    let file = fs::OpenOptions::new().write(true).open(&registry).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(past))
        .unwrap();
    drop(file);

    let view = run_with_registry_env(
        &db,
        &["--format", "json", "portal", "view", "projects"],
        &registry,
    );
    assert_success(&view, "portal stale view");
    let doc: serde_json::Value = serde_json::from_slice(&view.stdout).unwrap();
    assert_eq!(doc["view"]["status"], "warn", "stale must not render ok");
    for entry in doc["view"]["entries"].as_array().unwrap() {
        if entry["id"].as_str().unwrap().starts_with("fleet:") {
            assert_eq!(entry["status"], "warn", "{entry}");
        }
    }
    let meta = doc["view"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == serde_json::json!("fleet:source"))
        .unwrap();
    assert!(
        meta["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line.as_str().unwrap().contains("freshness=stale")),
        "{meta}"
    );
}

#[test]
fn malformed_fleet_registry_never_masks_the_portal_gap() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    fs::write(&registry, r#"{"schema_version": 99, "projects": []}"#).unwrap();

    // The portal keeps rendering; the fleet block surfaces the typed
    // failure as `unavailable` instead of an ok or a hard portal error.
    let view = run_with_registry_env(
        &db,
        &["--format", "json", "portal", "view", "projects"],
        &registry,
    );
    assert_success(&view, "portal view with malformed fleet");
    let doc: serde_json::Value = serde_json::from_slice(&view.stdout).unwrap();
    let meta = doc["view"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == serde_json::json!("fleet:source"))
        .expect("fleet meta entry");
    assert_eq!(meta["status"], "unavailable");
    assert!(meta["evidence"][0]
        .as_str()
        .unwrap()
        .contains("fleet-registry-invalid"));
    assert_ne!(doc["view"]["status"], "ok");
}

#[test]
fn no_mcp_tool_or_api_route_exists_for_fleet_surfaces() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path());
    let registry = ws.join("projects.json");
    assert_success(
        &run_with_registry_env(&db, &["fleet", "list"], &registry),
        "fleet list",
    );

    let mcp_list = |id: i64| {
        let mut cmd = Command::new(forge_bin());
        cmd.env_remove("FORGE_REGISTRY")
            .env_remove("FORGE_WORKSPACE_REGISTRY")
            .arg("--registry")
            .arg(&db)
            .arg("mcp")
            .arg("serve");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn forge mcp serve");
        {
            let stdin = child.stdin.as_mut().expect("stdin");
            let request = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/list"
            });
            writeln!(stdin, "{request}").expect("write request");
        }
        let output = child.wait_with_output().expect("wait");
        lossy(&output.stdout)
    };

    let names_of = |stdout: &str, id: i64| -> Vec<String> {
        for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value["id"] == serde_json::json!(id) {
                return value["result"]["tools"]
                    .as_array()
                    .expect("tools array")
                    .iter()
                    .map(|tool| tool["name"].as_str().expect("tool name").to_string())
                    .collect();
            }
        }
        panic!("no tools/list response for id {id} in: {stdout}");
    };

    let before = names_of(&mcp_list(1), 1);
    let after = names_of(&mcp_list(2), 2);
    assert_eq!(
        before, after,
        "the tools/list snapshot must not grow through the fleet surface"
    );
    for name in &before {
        assert!(!name.contains("fleet"), "{name}");
    }

    // No API route exists for the fleet surface.
    for (method, path) in [
        ("GET", "/v1/fleet"),
        ("POST", "/v1/fleet"),
        ("GET", "/v1/fleet/list"),
        ("GET", "/v1/projects/alpha/fleet"),
        ("POST", "/v1/projects/alpha/fleet"),
    ] {
        assert!(
            route_request(method, path).is_none(),
            "{method} {path} must not route anywhere"
        );
    }
}

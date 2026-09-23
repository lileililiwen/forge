//! CLI contract for `forge fleet list|status|inspect`
//! (`fleet-registry-observation`).
//!
//! Exercises the read-only fleet projection end to end through the built
//! binary against fixture registries copied from the real Workspace
//! Governance `projects.json` shape:
//!
//! - Projection: declared entries surface with id, verbatim profile,
//!   lifecycle, adoption, `forge.yaml` presence and the managed/unmanaged
//!   join, plus the registry path and observation timestamp.
//! - Rejection taxonomy: unknown schema version, duplicate ids and
//!   oversized/invalid `--max-age` refuse with typed
//!   `error[fleet-registry-invalid]` and empty stdout; escaping and
//!   malformed entries are named and excluded while the rest renders.
//! - Honest freshness: a backdated registry reports `stale`, never a
//!   healthy source; an unconfigured surface reports `unconfigured` and
//!   exits 0 without contacting anything.
//! - Read-only: every file in the fixture tree survives any number of
//!   calls unchanged; credential-shaped declared fields are redacted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run_with_workspace_env(db: &Path, args: &[&str], registry: &Path) -> std::process::Output {
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

/// Recursively copy a fixture tree so every test mutates only its own
/// disposable copy.
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

/// Copy the `clean` fixture tree, optionally overwriting its registry
/// document with a named variant's `projects.json`.
fn workspace(parent: &Path, variant: Option<&str>) -> PathBuf {
    let ws = parent.join("ws");
    copy_tree(&fixture_root().join("clean"), &ws);
    if let Some(name) = variant {
        let source = fixture_root().join(name).join("projects.json");
        fs::copy(source, ws.join("projects.json")).unwrap();
    }
    ws
}

fn registry_in(ws: &Path) -> PathBuf {
    ws.join("projects.json")
}

fn backdate(path: &Path, days: u64) {
    let past = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 86_400 * days,
        );
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(past))
        .unwrap();
}

fn json_run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut full: Vec<&str> = vec!["--format", "json"];
    full.extend_from_slice(args);
    run(db, &full)
}

fn json_out(db: &Path, args: &[&str]) -> serde_json::Value {
    let out = json_run(db, args);
    assert!(
        out.status.success(),
        "exit {:?}; stderr: {}",
        out.status.code(),
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("valid JSON on stdout")
}

#[test]
fn top_level_help_advertises_the_fleet_surface() {
    let out = run(Path::new("/unused"), &["--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("fleet"), "{text}");
}

#[test]
fn fleet_help_names_the_workspace_registry_and_max_age() {
    let out = run(Path::new("/unused"), &["fleet", "list", "--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("--workspace-registry"), "{text}");
    assert!(text.contains("--max-age"), "{text}");
    let status = run(Path::new("/unused"), &["fleet", "status", "--help"]);
    assert!(status.status.success());
    let inspect = run(Path::new("/unused"), &["fleet", "inspect", "--help"]);
    assert!(inspect.status.success());
    assert!(lossy(&inspect.stdout).contains("ENTRY"), "{inspect:?}");
}

#[test]
fn unconfigured_registry_reports_unconfigured_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let human = run(&db, &["fleet", "list"]);
    assert!(human.status.success(), "{}", lossy(&human.stderr));
    let text = lossy(&human.stdout);
    assert!(text.contains("unconfigured"), "{text}");
    assert!(
        text.contains("local workflows continue unaffected"),
        "{text}"
    );

    let doc = json_out(&db, &["fleet", "list"]);
    assert_eq!(doc["contract"], "0.1.0");
    assert_eq!(doc["fleet"]["freshness"], "unconfigured");
    assert!(doc["fleet"]["source"].is_null());
    assert_eq!(doc["fleet"]["entries"].as_array().unwrap().len(), 0);

    let status = run(&db, &["fleet", "status"]);
    assert!(status.status.success());
    assert!(lossy(&status.stdout).contains("health: unconfigured"));
}

#[test]
fn clean_registry_lists_declared_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = registry_in(&ws);
    let registry_arg = registry.display().to_string();

    let human = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert!(human.status.success(), "{}", lossy(&human.stderr));
    let text = lossy(&human.stdout);
    for needle in [
        "contract 0.1.0",
        &format!("source={registry_arg}"),
        "freshness=fresh",
        "observed_at=",
        "fleet entry `alpha`: path=alpha profile=rust-product lifecycle=active adoption=adopted forge_yaml=present state=unmanaged",
        "fleet entry `beta`",
        "fleet entry `gamma`",
        "adoption=unknown",
        "forge_yaml=missing",
        "read-only",
    ] {
        assert!(text.contains(needle), "{needle} missing in:\n{text}");
    }

    let doc = json_out(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    let entries = doc["fleet"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0]["profile"], "rust-product");
    assert_eq!(entries[0]["adoption"], "adopted");
    assert_eq!(entries[1]["adoption"], serde_json::Value::Null);
    assert_eq!(
        entries[2]["adoption"],
        serde_json::Value::Null,
        "missing adoption"
    );
    assert_eq!(entries[0]["state"], "unmanaged");
    assert_eq!(entries[0]["forge_yaml_present"], true);
    assert_eq!(entries[2]["forge_yaml_present"], false);
    assert_eq!(doc["fleet"]["freshness"], "fresh");
    assert!(doc["fleet"]["source"]
        .as_str()
        .unwrap()
        .ends_with("projects.json"));
    assert!(!doc["fleet"]["observed_at"].as_str().unwrap().is_empty());
    assert_eq!(doc["fleet"]["malformed"].as_array().unwrap().len(), 0);
}

#[test]
fn env_var_selects_the_registry_without_a_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = registry_in(&ws);
    let out = run_with_workspace_env(&db, &["fleet", "list"], &registry);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("fleet entry `alpha`"), "{text}");
    assert!(
        text.contains(&format!("source={}", registry.display())),
        "{text}"
    );
}

#[test]
fn managed_join_labels_locally_registered_entries_without_registration() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = registry_in(&ws);
    // Register the alpha project locally: the same id in the fleet
    // document becomes `managed`. beta and gamma stay `unmanaged`.
    let alpha = ws.join("alpha").display().to_string();
    let registered = run(&db, &["register", &alpha]);
    assert!(registered.status.success(), "{}", lossy(&registered.stderr));
    let registry_arg = registry.display().to_string();
    let doc = json_out(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    let entries = doc["fleet"]["entries"].as_array().unwrap();
    let states: Vec<(&str, &str)> = entries
        .iter()
        .map(|e| (e["id"].as_str().unwrap(), e["state"].as_str().unwrap()))
        .collect();
    assert_eq!(
        states,
        vec![
            ("alpha", "managed"),
            ("beta", "unmanaged"),
            ("gamma", "unmanaged")
        ]
    );
    assert!(entries[0]["locally_registered"].as_bool().unwrap());
}

#[test]
fn stale_registry_reports_stale_never_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = registry_in(&ws);
    backdate(&registry, 10);
    let registry_arg = registry.display().to_string();
    let human = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert!(human.status.success(), "{}", lossy(&human.stderr));
    let text = lossy(&human.stdout);
    assert!(text.contains("freshness=stale"), "{text}");
    assert!(!text.contains("freshness=fresh"), "{text}");

    let doc = json_out(
        &db,
        &["fleet", "status", "--workspace-registry", &registry_arg],
    );
    assert_eq!(doc["health"]["freshness"], "stale");
    assert!(doc["health"]["age_seconds"].as_i64().unwrap() > 86_400);
}

#[test]
fn max_age_window_controls_staleness() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry_arg = registry_in(&ws).display().to_string();
    let doc = json_out(
        &db,
        &[
            "fleet",
            "status",
            "--workspace-registry",
            &registry_arg,
            "--max-age",
            "1",
        ],
    );
    assert_eq!(doc["health"]["max_age_seconds"], 1);
    assert_eq!(doc["health"]["freshness"], "fresh", "just-written file");
    backdate(&ws.join("projects.json"), 1);
    let doc = json_out(
        &db,
        &[
            "fleet",
            "status",
            "--workspace-registry",
            &registry_arg,
            "--max-age",
            "1",
        ],
    );
    assert_eq!(doc["health"]["freshness"], "stale");
}

#[test]
fn escaping_entry_is_named_and_the_rest_renders() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), Some("escaping"));
    let registry_arg = registry_in(&ws).display().to_string();
    let human = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert!(human.status.success(), "{}", lossy(&human.stderr));
    let text = lossy(&human.stdout);
    assert!(text.contains("fleet entry `alpha`"), "{text}");
    assert!(text.contains("name=runner"), "{text}");
    assert!(
        text.contains("outside the registry workspace root"),
        "{text}"
    );
    assert!(!text.contains("fleet entry `runner`"), "{text}");

    let doc = json_out(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert_eq!(doc["fleet"]["entries"].as_array().unwrap().len(), 1);
    let malformed = doc["fleet"]["malformed"].as_array().unwrap();
    assert_eq!(malformed.len(), 1);
    assert_eq!(malformed[0]["name"], "runner");
}

#[test]
fn duplicate_ids_refuse_the_report_naming_the_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), Some("duplicate"));
    let registry_arg = registry_in(&ws).display().to_string();
    let out = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "stdout must stay empty");
    let err = lossy(&out.stderr);
    assert!(err.contains("error[fleet-registry-invalid]"), "{err}");
    assert!(err.contains("`dup`"), "{err}");
}

#[test]
fn unknown_schema_version_refuses_the_report() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), Some("unknown-version"));
    let registry_arg = registry_in(&ws).display().to_string();
    let out = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = lossy(&out.stderr);
    assert!(err.contains("error[fleet-registry-invalid]"), "{err}");
    assert!(err.contains("schema_version"), "{err}");
}

#[test]
fn malformed_registry_document_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = ws.join("projects.json");
    fs::write(&registry, "{ this is not json").unwrap();
    let registry_arg = registry.display().to_string();
    let out = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = lossy(&out.stderr);
    assert!(err.contains("error[fleet-registry-invalid]"), "{err}");
    assert!(err.contains("malformed JSON"), "{err}");
}

#[test]
fn max_age_bound_is_validated_before_any_read() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry_arg = registry_in(&ws).display().to_string();
    for bad in ["0", "31536001", "999999999999"] {
        let out = run(
            &db,
            &[
                "fleet",
                "list",
                "--workspace-registry",
                &registry_arg,
                "--max-age",
                bad,
            ],
        );
        assert_eq!(out.status.code(), Some(1), "{bad}");
        assert!(out.stdout.is_empty());
        assert!(
            lossy(&out.stderr).contains("error[fleet-registry-invalid]"),
            "{bad}: {}",
            lossy(&out.stderr)
        );
    }
}

#[test]
fn inspect_surfaces_one_entry_and_refuses_absent_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), Some("escaping"));
    let registry_arg = registry_in(&ws).display().to_string();
    let out = run(
        &db,
        &[
            "fleet",
            "inspect",
            "alpha",
            "--workspace-registry",
            &registry_arg,
        ],
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("id=alpha"), "{text}");
    assert!(text.contains("state=unmanaged"), "{text}");
    assert!(
        text.contains("unmanaged entries can only be inspected"),
        "{text}"
    );

    let out = run(
        &db,
        &[
            "fleet",
            "inspect",
            "runner",
            "--workspace-registry",
            &registry_arg,
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = lossy(&out.stderr);
    assert!(err.contains("error[fleet-registry-invalid]"), "{err}");
    assert!(err.contains("malformed"), "{err}");

    let out = run(
        &db,
        &[
            "fleet",
            "inspect",
            "ghost",
            "--workspace-registry",
            &registry_arg,
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let err = lossy(&out.stderr);
    assert!(err.contains("no fleet entry `ghost`"), "{err}");
}

#[test]
fn credential_shaped_declared_fields_are_redacted() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = ws.join("projects.json");
    let body = r#"{
  "schema_version": 1,
  "workspace_root": null,
  "projects": [
    {"id": "leaky", "path": "alpha", "profile": "rust-product ghp_abcdefghijklmnopqrstuvwxyz0123456789", "lifecycle": "active", "adoption": "token=xoxb-1234567890-abcdefghijklmonpqrstu"}
  ]
}"#;
    fs::write(&registry, body).unwrap();
    let registry_arg = registry.display().to_string();
    let out = run(
        &db,
        &["fleet", "list", "--workspace-registry", &registry_arg],
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(!text.contains("ghp_abcdefghijklmnopqrstuvwxyz"), "{text}");
    assert!(!text.contains("xoxb-1234567890"), "{text}");
    assert!(text.contains("[REDACTED]"), "{text}");
}

#[test]
fn fleet_calls_never_mutate_the_fixture_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let ws = workspace(tmp.path(), None);
    let registry = registry_in(&ws);
    let registry_arg = registry.display().to_string();

    fn tree_state(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        fn walk(dir: &Path, acc: &mut Vec<(String, Vec<u8>)>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, acc);
                } else {
                    acc.push((path.display().to_string(), fs::read(&path).unwrap()));
                }
            }
        }
        walk(dir, &mut out);
        out.sort();
        out
    }

    let before = tree_state(&ws);
    for _ in 0..3 {
        let list = run(
            &db,
            &["fleet", "list", "--workspace-registry", &registry_arg],
        );
        assert!(list.status.success(), "{}", lossy(&list.stderr));
        let status = run(
            &db,
            &["fleet", "status", "--workspace-registry", &registry_arg],
        );
        assert!(status.status.success(), "{}", lossy(&status.stderr));
        let inspect = run(
            &db,
            &[
                "fleet",
                "inspect",
                "alpha",
                "--workspace-registry",
                &registry_arg,
            ],
        );
        assert!(inspect.status.success(), "{}", lossy(&inspect.stderr));
    }
    let after = tree_state(&ws);
    assert_eq!(before.len(), after.len(), "no file may appear or disappear");
    assert_eq!(before, after, "every file must be byte-identical");
}

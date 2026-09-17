//! Cross-surface regression: manifest + registry + CLI combined.
//!
//! Exercises R1×R2×R3 interactions end to end through the built binary:
//! register a valid fixture, restart (new process), inspect persisted
//! identity, verify repeatability and failure isolation.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(db: &Path, args: &[String]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn s(value: &str) -> String {
    value.to_string()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn copy_fixture_to_tmp(name: &str, dest: &Path) -> PathBuf {
    fs::create_dir_all(dest).unwrap();
    for file in ["forge.yaml", "platform.yaml"] {
        let src = fixture(name).join(file);
        if src.exists() {
            fs::copy(&src, dest.join(file)).unwrap();
        }
    }
    dest.to_path_buf()
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

#[test]
fn register_restart_inspect_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = copy_fixture_to_tmp("valid-full", &tmp.path().join("work"));
    let path = proj.display().to_string();

    let out = run(&db, &[s("register"), path.clone()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // Second register of the same project is an idempotent refresh.
    let again = run(&db, &[s("register"), path.clone()]);
    assert_eq!(again.status.code(), Some(0), "{}", lossy(&again.stderr));

    // "Restart": fresh process, same database file.
    let inspect = run(&db, &[s("inspect"), s("mortality-reflection")]);
    assert_eq!(inspect.status.code(), Some(0), "{}", lossy(&inspect.stderr));
    let text = lossy(&inspect.stdout);
    assert!(text.contains("schema: 1"), "{text}");
    assert!(text.contains("auth=2.1"), "{text}");
    assert!(text.contains("rust-web"), "{text}");

    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    let inspect_json = cmd
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("inspect")
        .arg("mortality-reflection")
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&inspect_json.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["features"]["auth"], "2.1");
    assert_eq!(value["available"], true);

    let list = run(&db, &[s("list")]);
    assert_eq!(list.status.code(), Some(0));
    assert!(lossy(&list.stdout).contains("mortality-reflection"));
}

#[test]
fn manifest_failures_never_mutate_or_register() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // Unsupported schema: nonzero, file untouched, nothing registered.
    let bad = copy_fixture_to_tmp("unsupported", &tmp.path().join("bad"));
    let before = fs::read(bad.join("forge.yaml")).unwrap();
    let out = run(&db, &[s("register"), bad.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert!(lossy(&out.stderr).contains("error[unsupported-schema]"));
    assert_eq!(fs::read(bad.join("forge.yaml")).unwrap(), before);
    assert_eq!(
        run(&db, &[s("inspect"), s("future-project")]).status.code(),
        Some(1)
    );

    // Ambiguous dual manifests: nonzero, both files untouched.
    let dual = copy_fixture_to_tmp("dual", &tmp.path().join("dual"));
    let forge_before = fs::read(dual.join("forge.yaml")).unwrap();
    let platform_before = fs::read(dual.join("platform.yaml")).unwrap();
    let out = run(&db, &[s("register"), dual.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    assert!(lossy(&out.stderr).contains("error[ambiguous-manifest]"));
    assert_eq!(fs::read(dual.join("forge.yaml")).unwrap(), forge_before);
    assert_eq!(
        fs::read(dual.join("platform.yaml")).unwrap(),
        platform_before
    );

    // Legacy-only directory requires the explicit flag, then succeeds.
    let legacy = copy_fixture_to_tmp("legacy-only", &tmp.path().join("legacy"));
    let out = run(&db, &[s("register"), legacy.display().to_string()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("error[legacy-manifest-requires-explicit]"));
    let out = run(
        &db,
        &[
            s("register"),
            legacy.display().to_string(),
            s("--manifest"),
            s("platform.yaml"),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(
        run(&db, &[s("inspect"), s("legacy-only")]).status.code(),
        Some(0)
    );
}

#[test]
fn upgrade_picks_up_feature_lifecycle_dependencies_and_keeps_source_manifest_registry_aligned() {
    // R1×feature-lifecycle: a project that already carries `admin` (which
    // depends on `auth`) must remain consistent after a fleet upgrade:
    // both features reach 0.1.0 in dependency order, source/manifest/
    // registry agree, and the section preservation rule still holds.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: cross-up\n  name: Cross\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n  version: stable\ndeployment:\n  type: docker\n  target: home-server-01\ndocs:\n  source_language: en\nfeatures:\n  auth: \"0.0.9\"\n  admin: \"0.0.9\"\n",
    )
    .unwrap();
    let out = run(&db, &[s("register"), proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    for id in ["auth", "admin"] {
        let descriptor = forge::feature::inspect_feature(id).unwrap();
        let receipt = proj.join(format!(".forge/features/{id}.receipt"));
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        fs::write(
            &receipt,
            forge::feature::expected_receipt(&descriptor, "0.0.9"),
        )
        .unwrap();
    }

    let out = run(&db, &[s("upgrade"), proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(manifest.contains("auth: 0.1.0"), "{manifest}");
    assert!(manifest.contains("admin: 0.1.0"), "{manifest}");
    assert!(manifest.contains("target: home-server-01"), "{manifest}");
    assert!(manifest.contains("source_language: en"), "{manifest}");

    let inspect_json = run(
        &db,
        &[s("--format"), s("json"), s("inspect"), s("cross-up")],
    );
    let value: serde_json::Value = serde_json::from_slice(&inspect_json.stdout).unwrap();
    assert_eq!(value["features"]["auth"], "0.1.0");
    assert_eq!(value["features"]["admin"], "0.1.0");
}

#[test]
fn upgrade_semantic_conflict_handoff_resolves_through_spec_apply() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("conflict-up");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: conflict-up\n  name: Conflict\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: \"0.0.9\"\n",
    )
    .unwrap();
    let descriptor = forge::feature::inspect_feature("auth").unwrap();
    let receipt = proj.join(".forge/features/auth.receipt");
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    fs::write(
        &receipt,
        forge::feature::expected_receipt(&descriptor, "0.0.9"),
    )
    .unwrap();
    // Drift the receipt so the precondition sweep blocks with a
    // semantic-conflict handoff.
    fs::write(&receipt, "operator runbook notes\n").unwrap();
    let receipt_before = fs::read_to_string(&receipt).unwrap();

    let out = run(&db, &[s("upgrade"), proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr).to_string();
    assert!(
        stderr.contains("error[feature-ownership-conflict]"),
        "{stderr}"
    );
    assert!(stderr.contains("forge spec generate"), "{stderr}");

    // Apply the spec route for the same semantic finding: the
    // router should classify the conflict as a semantic route and
    // produce a bounded proposal under `.forge/specs/`.
    let value = run(
        &db,
        &[
            s("--format"),
            s("json"),
            s("spec"),
            s("apply"),
            s("semantic-auth"),
            proj.to_str().unwrap().to_string(),
        ],
    );
    let parsed: serde_json::Value = serde_json::from_slice(&value.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stderr={}", lossy(&value.stderr)));
    assert_eq!(parsed["apply"]["status"], "spec-generated");
    assert_eq!(parsed["apply"]["decision"]["route"], "semantic");
    let spec_dir = proj.join(".forge/specs");
    assert!(spec_dir.is_dir(), "spec directory was not written");
    let entries: Vec<PathBuf> = fs::read_dir(&spec_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(entries.len(), 1);
    let manifest = fs::read_to_string(entries[0].join("manifest.json")).unwrap();
    let manifest_value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest_value["provenance"]["project_id"], "conflict-up");
    assert!(manifest_value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f.as_str() == Some("semantic-auth")));

    // Filesystem state: the receipt is preserved (R1 failure
    // scenario contract) and the manifest is unchanged.
    assert_eq!(fs::read_to_string(&receipt).unwrap(), receipt_before);
    let manifest_text = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(manifest_text.contains("auth: \"0.0.9\""), "{manifest_text}");
}

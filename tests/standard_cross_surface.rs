//! Standard-pack cross-surface regression: the opt-in `.standard/` snapshot
//! and `forge standard` surface must not disturb the existing `new`,
//! `profile`, `inspect` and `list` contracts, and a pinned deprecated pack
//! must stay inspectable/checkable for an existing project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PACK: &str = "baseline-service@1.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let out = run(db, &[&["--format", "json"], args].concat());
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn new_project(db: &Path, dest: &Path, profile: &str, extra: &[&str]) {
    let mut args = vec!["new", dest.to_str().unwrap(), "--profile", profile];
    args.extend_from_slice(extra);
    let out = run(db, &args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn existing_new_profile_inspect_and_list_contracts_stay_intact() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("compat-svc");
    new_project(&db, &dest, "rust-web", &[]);

    // Default generation carries no standard subtree.
    assert!(!dest.join(".standard").exists());

    // The registry still records the generated project.
    let record = run_json(&db, &["inspect", "compat-svc"]);
    assert_eq!(record["profile"], "rust-web");
    assert_eq!(
        record["path"],
        dest.canonicalize().unwrap().display().to_string()
    );

    // `list` never mentions standard-pack data.
    let list = run_json(&db, &["list"]);
    assert!(!serde_json::to_string(&list)
        .unwrap()
        .contains("baseline-service"));

    // Profile descriptors carry no standard-pack key.
    let profile = run_json(&db, &["profile", "inspect", "rust-web"]);
    assert!(profile.get("asset_digest").is_none());
    assert!(profile.get("standard_pack").is_none());

    // JSON `new` with a selection advertises the owned snapshot paths.
    let files = {
        let out = run_json(
            &db,
            &[
                "new",
                tmp.path().join("json-svc").to_str().unwrap(),
                "--profile",
                "python-service",
                "--standard-pack",
                PACK,
            ],
        );
        out["files"].clone()
    };
    let paths: Vec<&str> = files
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        paths.iter().any(|p| p.starts_with(".standard/")),
        "{paths:?}"
    );
    assert!(paths.contains(&"forge.yaml"), "{paths:?}");
}

#[test]
fn deprecated_pack_remains_inspectable_and_checkable_when_pinned() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("pinned-svc");
    new_project(&db, &dest, "rust-web", &["--standard-pack", PACK]);
    // The deprecated descriptor is still inspectable.
    let inspect = run_json(&db, &["standard", "inspect", "baseline-service@0.9.0"]);
    assert_eq!(inspect["support_state"], "deprecated");
    // 0.9.0 declares no CI asset, so the current CI file is reported as
    // orphaned (preserved, never silently deleted) and survives the pin.
    let diff = run_json(
        &db,
        &[
            "standard",
            "diff",
            dest.to_str().unwrap(),
            "--against",
            "baseline-service@0.9.0",
        ],
    );
    assert!(diff["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["path"] == ".standard/ci/verify.yml" && e["change"] == "orphaned"));
    let out = run(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@0.9.0",
            "--confirm",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let check = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(check["state"], "rendered");
    assert_eq!(check["version"], "0.9.0");
    assert!(
        dest.join(".standard/ci/verify.yml").is_file(),
        "orphan preserved"
    );
    assert!(dest.join(".standard/compose/docker-compose.yaml").is_file());
}

#[test]
fn selection_is_portable_across_profiles_and_leaves_no_trace_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("py-svc");
    new_project(&db, &dest, "python-service", &["--standard-pack", PACK]);
    assert!(dest.join(".standard/receipt.json").is_file());
    let check = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(check["state"], "rendered");
    let verify = fs::read_to_string(dest.join(".standard/scripts/verify.sh")).unwrap();
    assert!(
        verify.contains("python3 -m unittest discover -s tests -v"),
        "{verify}"
    );

    // A subsequent check writes nothing: the directory entry count is stable.
    let before: Vec<PathBuf> = fs::read_dir(&dest)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let _ = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    let after: Vec<PathBuf> = fs::read_dir(&dest)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(before, after);
}

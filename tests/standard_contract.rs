//! Standard-pack registry and snapshot contract: `forge standard
//! list|inspect|check|diff|upgrade` and the opt-in `.standard/` snapshot
//! rendered by `forge new --standard-pack`.
//!
//! Covers the `standard-pack-registry-and-snapshots` scenarios end to end
//! through the built binary: versioned descriptors with support/evidence
//! state, typed refusal of unknown or non-selectable packs, atomic snapshot
//! generation with a digest receipt only when a pack is explicitly
//! selected, read-only diff, conflict-safe upgrade that preserves unrelated
//! files, and standalone operation (no Forge or sibling checkout needed
//! later).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PACK: &str = "baseline-service@1.1.0";
const STANDARD_DIR: &str = ".standard";
const RECEIPT: &str = ".standard/receipt.json";
const VERIFY: &str = ".standard/scripts/verify.sh";
const PROFILE_YAML: &str = ".standard/profile.yaml";

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
    let out = run(db, &[&["--format", "json"], args].concat());
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn expect_refusal(db: &Path, args: &[&str], code: &str) {
    let out = run(db, args);
    assert!(
        !out.status.success(),
        "forge {args:?} unexpectedly succeeded"
    );
    assert!(
        out.stdout.is_empty(),
        "refusal printed stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(code),
        "expected `{code}` in stderr: {stderr}"
    );
}

fn new_project(db: &Path, dest: &Path, profile: &str, extra: &[&str]) {
    let mut args = vec!["new", dest.to_str().unwrap(), "--profile", profile];
    args.extend_from_slice(extra);
    let out = run(db, &args);
    assert!(
        out.status.success(),
        "new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(dir: &Path, rel: &str, bytes: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

#[test]
fn list_exposes_versioned_packs_with_state_and_profiles() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["standard", "list"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let human = String::from_utf8_lossy(&out.stdout);
    assert!(human.contains("baseline-service"), "{human}");
    assert!(human.contains("1.1.0"), "{human}");
    assert!(human.contains("supported"), "{human}");
    assert!(human.contains("deprecated"), "{human}");
    assert!(human.contains("proposed"), "{human}");
    assert!(human.contains("rust-web"), "{human}");

    let value = run_json(&db, &["standard", "list"]);
    let packs = value["packs"].as_array().expect("packs array");
    let supported: Vec<&serde_json::Value> = packs
        .iter()
        .filter(|p| p["support_state"] == "supported")
        .collect();
    assert!(!supported.is_empty());
    for pack in supported {
        assert_eq!(pack["evidence"], "verified");
        assert!(!pack["compatible_profiles"].as_array().unwrap().is_empty());
        assert_eq!(pack["asset_digest"].as_str().unwrap().len(), 64);
    }
}

#[test]
fn inspect_names_identity_evidence_and_digest() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let value = run_json(&db, &["standard", "inspect", PACK]);
    assert_eq!(value["id"], "baseline-service");
    assert_eq!(value["version"], "1.1.0");
    assert_eq!(value["support_state"], "supported");
    assert_eq!(value["evidence"], "verified");
    assert_eq!(value["asset_digest"].as_str().unwrap().len(), 64);
    assert_eq!(
        value["external_source"],
        "workspace-governance/templates/runtime"
    );

    // A proposed version is inspectable but not selectable.
    let proposed = run_json(&db, &["standard", "inspect", "baseline-service@2.0.0"]);
    assert_eq!(proposed["support_state"], "proposed");
    assert_eq!(proposed["evidence"], "unverified");
}

#[test]
fn unknown_or_malformed_pack_selector_is_typed() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    for spec in ["nope@1.0.0", "baseline-service@9.9.9", "baseline-service"] {
        expect_refusal(&db, &["standard", "inspect", spec], "standard-invalid");
    }
}

#[test]
fn new_selects_nothing_implicitly_and_renders_the_named_pack() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    // No pack named, no snapshot: the output is the prior release.
    let plain = tmp.path().join("plain-svc");
    new_project(&db, &plain, "rust-web", &[]);
    assert!(!plain.join(STANDARD_DIR).exists());
    let value = run_json(&db, &["standard", "check", plain.to_str().unwrap()]);
    assert_eq!(value["state"], "absent");

    // An unsupported request is refused before anything is written.
    for spec in [
        "baseline-service@0.9.0",
        "baseline-service@2.0.0",
        "baseline-service@9.9.9",
        "baseline-service",
    ] {
        let dest = tmp
            .path()
            .join(format!("refused-{}", spec.replace(['@', '.'], "-")));
        expect_refusal(
            &db,
            &[
                "new",
                dest.to_str().unwrap(),
                "--profile",
                "rust-web",
                "--standard-pack",
                spec,
            ],
            "standard-invalid",
        );
        assert!(!dest.exists());
    }

    // An incompatible profile is refused by name before anything is written.
    let client = tmp.path().join("refused-client");
    expect_refusal(
        &db,
        &[
            "new",
            client.to_str().unwrap(),
            "--profile",
            "react-web",
            "--standard-pack",
            PACK,
        ],
        "standard-invalid",
    );
    assert!(!client.exists());
}

#[test]
fn selected_snapshot_is_receipted_and_standalone() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("snap-svc");
    new_project(&db, &dest, "rust-web", &["--standard-pack", PACK]);

    for rel in [
        RECEIPT,
        PROFILE_YAML,
        VERIFY,
        ".standard/ci/verify.yml",
        ".standard/quality/quality.yaml",
        ".standard/compose/docker-compose.yaml",
    ] {
        assert!(dest.join(rel).is_file(), "missing {rel}");
    }
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dest.join(RECEIPT)).unwrap()).unwrap();
    assert_eq!(receipt["pack"], "baseline-service");
    assert_eq!(receipt["version"], "1.1.0");
    assert_eq!(receipt["profile"], "rust-web");
    assert_eq!(receipt["project"], "snap-svc");
    assert!(receipt["generator"].as_str().unwrap().starts_with("forge@"));
    let owned = receipt["files"].as_array().unwrap();
    assert!(!owned.is_empty());
    for file in owned {
        let rel = file["path"].as_str().unwrap();
        assert!(rel.starts_with(STANDARD_DIR), "{rel}");
        let digest = crate_sha256(&fs::read(dest.join(rel)).unwrap());
        assert_eq!(file["digest"].as_str().unwrap(), digest, "{rel}");
    }

    // `check` agrees the snapshot is rendered.
    let value = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(value["state"], "rendered");
    assert_eq!(value["version"], "1.1.0");

    // Standalone: no Forge invocation and no absolute host path/sibling ref.
    for rel in [RECEIPT, PROFILE_YAML, VERIFY, ".standard/ci/verify.yml"] {
        let text = fs::read_to_string(dest.join(rel)).unwrap();
        assert!(!text.contains("/workspace-governance"), "{rel}: {text}");
        assert!(!text.contains("/home/"), "{rel}: {text}");
        assert!(!text.contains("/Users/"), "{rel}: {text}");
        for line in text.lines() {
            assert!(
                !line.trim_start().starts_with("forge "),
                "{rel} must not invoke a Forge runtime: {line}"
            );
        }
    }
    // The verification entry point runs the profile's own test command.
    let verify = fs::read_to_string(dest.join(VERIFY)).unwrap();
    assert!(verify.contains("cargo test"), "{verify}");
}

#[test]
fn interactive_mode_honours_an_explicit_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cli-app");
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("new")
        .arg(dest.to_str().unwrap())
        .arg("--standard-pack")
        .arg(PACK);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"rust-web\ncli-app\nCLI App\n\n")
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait forge");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(RECEIPT).is_file());
    let value = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(value["state"], "rendered");
}

#[test]
fn diff_is_read_only_and_upgrade_requires_confirmation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("diff-svc");
    new_project(&db, &dest, "rust-web", &["--standard-pack", PACK]);
    let before = fs::read_to_string(dest.join(VERIFY)).unwrap();

    let value = run_json(
        &db,
        &[
            "standard",
            "diff",
            dest.to_str().unwrap(),
            "--against",
            "baseline-service@1.0.0",
        ],
    );
    assert_eq!(value["from"], "baseline-service@1.1.0");
    assert_eq!(value["against"], "baseline-service@1.0.0");
    let entries = value["entries"].as_array().unwrap();
    assert!(entries
        .iter()
        .any(|e| e["path"] == VERIFY && e["change"] == "updated"));
    // Diff wrote nothing.
    assert_eq!(fs::read_to_string(dest.join(VERIFY)).unwrap(), before);

    expect_refusal(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.0.0",
        ],
        "standard-invalid",
    );
    // Still 1.1.0 after the refused upgrade.
    let value = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(value["version"], "1.1.0");

    let out = run(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.0.0",
            "--confirm",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(value["state"], "rendered");
    assert_eq!(value["version"], "1.0.0");
}

#[test]
fn modified_owned_file_conflicts_and_force_supplies_the_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("conflict-svc");
    new_project(&db, &dest, "rust-web", &["--standard-pack", PACK]);
    // Move to 1.0.0 so upgrading back to 1.1.0 rewrites verify.sh.
    let out = run(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.0.0",
            "--confirm",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let verify = fs::read_to_string(dest.join(VERIFY)).unwrap();
    let edited = format!("{verify}\n# local edit\n");
    write(&dest, VERIFY, &edited);
    write(&dest, "user-notes.txt", "mine");

    let value = run_json(
        &db,
        &[
            "standard",
            "diff",
            dest.to_str().unwrap(),
            "--against",
            "baseline-service@1.1.0",
        ],
    );
    assert!(value["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c == VERIFY));

    expect_refusal(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.1.0",
            "--confirm",
        ],
        "standard-invalid",
    );
    assert_eq!(fs::read_to_string(dest.join(VERIFY)).unwrap(), edited);

    let out = run(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.1.0",
            "--confirm",
            "--force",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_ne!(fs::read_to_string(dest.join(VERIFY)).unwrap(), edited);
    // Unrelated files survive every upgrade.
    assert_eq!(
        fs::read_to_string(dest.join("user-notes.txt")).unwrap(),
        "mine"
    );
    // The receipt refreshed to the new version.
    let value = run_json(&db, &["standard", "check", dest.to_str().unwrap()]);
    assert_eq!(value["version"], "1.1.0");
}

#[test]
fn upgrade_without_snapshot_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("empty");
    fs::create_dir_all(&dest).unwrap();
    expect_refusal(
        &db,
        &[
            "standard",
            "diff",
            dest.to_str().unwrap(),
            "--against",
            "baseline-service@1.1.0",
        ],
        "standard-invalid",
    );
    expect_refusal(
        &db,
        &[
            "standard",
            "upgrade",
            dest.to_str().unwrap(),
            "--to",
            "baseline-service@1.1.0",
            "--confirm",
        ],
        "standard-invalid",
    );
}

/// Local SHA-256 (the test binary cannot reach the crate-private helper).
fn crate_sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The pinned shared rehearsal script, read from the repository's committed
/// asset tree: the pack payload and the committed bytes must be the same file.
fn pinned_shared_script() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("standards/baseline-service/1.2.0/verify-local.sh");
    fs::read_to_string(path).expect("pinned shared script")
}

const SHARED_PACK: &str = "baseline-service@1.2.0";

#[test]
fn install_into_an_existing_project_adopts_identical_files_and_writes_only_owned_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("sibling");
    fs::create_dir_all(project.join("src")).unwrap();
    // The fleet's real starting state: the canonical rehearsal script is already
    // there (74 repositories carry exactly these bytes), plus unrelated files.
    write(&project, "verify-local.sh", &pinned_shared_script());
    write(&project, "README.md", "# sibling\n");
    write(&project, "src/main.rs", "fn main() {}\n");

    let out = run(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let human = String::from_utf8_lossy(&out.stdout);
    assert!(human.contains("adopted"), "{human}");

    // The shared scripts are in place, byte-identical to the pinned assets.
    assert_eq!(
        fs::read_to_string(project.join("verify-local.sh")).unwrap(),
        pinned_shared_script()
    );
    assert_eq!(
        fs::read_to_string(project.join("scripts/check-openspec-change-names.mjs")).unwrap(),
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("standards/baseline-service/1.2.0/scripts/check-openspec-change-names.mjs")
        )
        .unwrap()
    );
    assert!(project.join("scripts/check-accessibility.py").is_file());
    assert!(project.join("scripts/check-privacy.py").is_file());
    assert!(project.join(RECEIPT).is_file());
    // Unrelated files are untouched: only owned paths plus the receipt are new.
    assert_eq!(
        fs::read_to_string(project.join("README.md")).unwrap(),
        "# sibling\n"
    );
    assert_eq!(
        fs::read_to_string(project.join("src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
}

#[test]
fn install_refusals_are_typed_and_write_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("sibling");
    fs::create_dir_all(&project).unwrap();
    write(&project, "README.md", "# sibling\n");

    // Confirmation is required.
    expect_refusal(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
        ],
        "standard-invalid",
    );
    // A profile is required without a manifest.
    expect_refusal(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--confirm",
        ],
        "standard-invalid",
    );
    // A divergent owned path refuses the whole install and names the path.
    write(&project, "scripts/check-privacy.py", "# local checker\n");
    let out = run(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("scripts/check-privacy.py"), "{stderr}");
    assert_eq!(
        fs::read_to_string(project.join("scripts/check-privacy.py")).unwrap(),
        "# local checker\n",
        "the repository's own file is preserved"
    );
    assert!(!project.join(RECEIPT).exists());
    assert!(!project.join("verify-local.sh").exists());

    // With the conflict resolved the install succeeds, and installing over an
    // existing snapshot is refused.
    fs::remove_file(project.join("scripts/check-privacy.py")).unwrap();
    let out = run(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--confirm",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    expect_refusal(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--confirm",
        ],
        "standard-invalid",
    );
}

#[test]
fn installed_snapshot_checks_and_diffs_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project = tmp.path().join("sibling");
    fs::create_dir_all(&project).unwrap();
    let out = run(
        &db,
        &[
            "standard",
            "install",
            SHARED_PACK,
            project.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--confirm",
            "--format",
            "json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let installed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(installed["pack"], "baseline-service");
    assert_eq!(installed["version"], "1.2.0");
    assert_eq!(installed["profile"], "rust-web");
    assert!(installed["adopted"].is_null() || installed["adopted"].as_array().unwrap().is_empty());

    let checked = run_json(&db, &["standard", "check", project.to_str().unwrap()]);
    assert_eq!(checked["state"], "rendered");
    let files = checked["files"].as_array().unwrap();
    assert_eq!(files.len(), 9, "{checked}");
    assert!(files.iter().all(|f| f["state"] == "present"));

    let diff = run_json(
        &db,
        &[
            "standard",
            "diff",
            project.to_str().unwrap(),
            "--against",
            SHARED_PACK,
        ],
    );
    // An empty conflict list is omitted rather than serialized as `[]`.
    let conflicts = diff.get("conflicts").and_then(|v| v.as_array());
    assert!(conflicts.map(|c| c.is_empty()).unwrap_or(true), "{diff}");
    assert!(
        diff["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["change"] == "unchanged"),
        "{diff}"
    );
}

#[test]
fn generation_with_the_shared_asset_pack_puts_the_scripts_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("generated");
    new_project(
        &db,
        &dest,
        "rust-web",
        &["--id", "generated", "--standard-pack", SHARED_PACK],
    );
    assert_eq!(
        fs::read_to_string(dest.join("verify-local.sh")).unwrap(),
        pinned_shared_script(),
        "generation must stage the same bytes the pack pins"
    );
    assert!(dest
        .join("scripts/check-openspec-change-names.mjs")
        .is_file());
    assert!(dest.join(VERIFY).is_file());
    assert!(dest.join(RECEIPT).is_file());
    // The previous version keeps working unchanged.
    let pinned = tmp.path().join("pinned");
    new_project(
        &db,
        &pinned,
        "rust-web",
        &["--id", "pinned", "--standard-pack", PACK],
    );
    assert!(!pinned.join("verify-local.sh").exists());
    assert!(pinned.join(VERIFY).is_file());
}

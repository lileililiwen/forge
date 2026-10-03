//! Workspace metadata emission contract (`workspace-metadata-emission`).
//!
//! Covers the `deterministic-project-generation` delta end to end through
//! the built binary: per-profile declaration emission with the honest
//! planned/non-deployable shape, opt-out parity with the pre-change
//! release (pinned tree digests captured from the prior binary),
//! ownership-conflict refusal on user-edited declarations at upgrade,
//! refresh of unedited stale content, and import observing foreign
//! metadata without ever writing it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::generate::workspace::{METADATA_PATH, RECEIPT_PATH};

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

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Canonical tree digest: sorted `(relative-path, bytes)` pairs hashed by
/// the same construction the pre-change release baseline was captured with.
fn tree_digest(root: &Path) -> String {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    collect(root, root, &mut entries);
    entries.sort();
    let mut packed: Vec<u8> = Vec::new();
    for (rel, bytes) in &entries {
        packed.extend_from_slice(rel.as_bytes());
        packed.push(0);
        packed.extend_from_slice(bytes);
        packed.push(0);
    }
    forge::generate::workspace::sha256_hex(&packed)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, fs::read(&path).unwrap()));
        }
    }
}

fn new_project(tmp: &Path, db: &Path, profile: &str, id: &str, extra: &[&str]) -> PathBuf {
    let dest = tmp.join(id);
    let mut args: Vec<&str> = vec![
        "new",
        dest.to_str().unwrap(),
        "--profile",
        profile,
        "--id",
        id,
    ];
    args.extend_from_slice(extra);
    let out = run(db, &args);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    dest
}

/// `(governance profile, native test command)` per mapped Forge profile,
/// mirroring the descriptor table, plus the id used for the baseline
/// digest capture with the pre-change binary.
const MATRIX: [(&str, &str, &str, &str); 6] = [
    ("aspnet-web", "dotnet-product", "dotnet test", "net-app"),
    ("flutter-app", "flutter-product", "flutter test", "mbl-app"),
    ("nextjs-web", "typescript-product", "npm test", "node-app"),
    (
        "python-service",
        "python-product",
        "python3 -m unittest discover -s tests -v",
        "py-app",
    ),
    ("react-web", "typescript-product", "npm test", "rs-app"),
    ("rust-web", "rust-product", "cargo test", "rs-web-app"),
];

/// Tree digests for `--no-workspace-metadata` output, one per profile. The
/// guard's purpose is that opting out of the metadata declaration changes
/// *nothing else*: these pin the exact bytes the opt-out path produces, so
/// any unintended future drift fails here.
///
/// Re-captured, every profile, by `scaffold-prewires-shared-layer`, which
/// changed what a scaffold contains by design: the `kit` block in
/// `forge.yaml`, the README shared-layer section, the `aspnet-web` TFM and
/// pre-wired package references, and the vendored `.platform/` token subtree.
/// `net-app` re-captured a second time by the same change's amendment, when
/// the owner rejected the environment-variable feed: the `NuGet.config` now
/// names a repo-relative source, the `RestoreAdditionalProjectSources` block
/// and the Dockerfile build argument are gone, and the committed
/// `packages/platform-feed/` tree is new.
/// `net-app` re-captured a third time, after the owner's .NET 10 ruling
/// (2026-10-02) removed SDK 8 from the machine: the `aspnet-web` profile's
/// `net8.0` fallback became `net10.0`, and the generated README now names the
/// pinned kit version. Only `net-app` moved — the other five profiles are
/// language-agnostic and re-captured unchanged, which is itself the evidence
/// that the drift is the intended TFM change and not a general rendering
/// regression.
/// Earlier re-captures: `rs-app` by `react-web-live-preview` for the runnable
/// Vite+React client, the first set by `jenkins-deploy-adapter-consumption`.
const PRE_RELEASE_DIGESTS: [(&str, &str); 6] = [
    (
        "net-app",
        "1337ef1c4e17114610938e908c98f3470d423bf2bbc4e78904894608f4c10ab8",
    ),
    (
        "mbl-app",
        "aaa0eb726917fe371022a61fb0ebcded40d9b185043bda6e6ad66fe5a4f710a8",
    ),
    (
        "node-app",
        "9b58ced791c2ae8da9316740e9794d13b2f441305240d5e6c2da31d247724759",
    ),
    (
        "py-app",
        "bed945af018867d0bc04740649f7868a5e5e142f0ee53f9e5e05658c953e5380",
    ),
    (
        "rs-app",
        "a5cf216c6306c43ff7384b64cdf3052b55c205728150d72e93a2390c8446fb42",
    ),
    (
        "rs-web-app",
        "548fd195ecf7005372d9c4b00305498d9170a504eb8d5fb99b83ee916ea864f5",
    ),
];

#[test]
fn mapped_profiles_emit_honest_declaration_matrix() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    for (profile, governance, command, id) in MATRIX {
        let dest = new_project(tmp.path(), &db, profile, id, &[]);
        let text = fs::read_to_string(dest.join(METADATA_PATH)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["schema_version"], 1, "{profile}");
        assert_eq!(value["id"], id, "{profile}");
        assert_eq!(value["kind"], "product", "{profile}");
        assert_eq!(value["profile"], governance, "{profile}");
        assert_eq!(value["lifecycle"], "active", "{profile}");
        assert_eq!(value["verification"]["command"], command, "{profile}");
        assert_eq!(
            value["verification"]["evidence_status"], "planned",
            "{profile}"
        );
        // Honesty: no shared Gate Runtime is declared, so no gate is claimed.
        assert!(
            value["verification"].get("gate_runtime").is_none(),
            "{profile}: {text}"
        );
        assert_eq!(value["deployment"]["deployable"], false, "{profile}");
        assert!(value["deployment"]["jenkins_job"].is_null(), "{profile}");
        assert!(value["deployment"]["compose_file"].is_null(), "{profile}");
        // No host paths or machine-specific roots anywhere in the file.
        assert!(!text.contains(tmp.path().to_str().unwrap()), "{profile}");
        assert!(
            !text.contains('/') && !text.contains('\\'),
            "{profile}: {text}"
        );
        let receipt = fs::read_to_string(dest.join(RECEIPT_PATH)).unwrap();
        assert!(
            receipt.contains(&forge::generate::workspace::sha256_hex(text.as_bytes())),
            "{profile}: receipt must record the declaration hash"
        );
    }
}

#[test]
fn opt_out_is_byte_identical_to_pre_release() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    for (profile, _governance, _command, id) in MATRIX {
        let dest = new_project(tmp.path(), &db, profile, id, &["--no-workspace-metadata"]);
        assert!(
            !dest.join(METADATA_PATH).exists(),
            "{profile}: declaration must be absent"
        );
        assert!(
            !dest.join(RECEIPT_PATH).exists(),
            "{profile}: receipt must be absent"
        );
        assert!(
            !dest.join(".forge").exists(),
            "{profile}: no .forge tree must exist"
        );
        assert_eq!(
            tree_digest(&dest),
            PRE_RELEASE_DIGESTS
                .iter()
                .find(|(name, _)| *name == id)
                .map(|(_, digest)| *digest)
                .unwrap(),
            "{profile}: opt-out tree must equal the prior release byte-for-byte"
        );
    }
}

#[test]
fn default_output_reports_metadata_without_notes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run_json(
        &db,
        &[
            "new",
            tmp.path().join("json-app").to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            "json-app",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: Vec<&str> = value["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(files.contains(&METADATA_PATH), "{files:?}");
    assert!(files.contains(&RECEIPT_PATH), "{files:?}");
    // A mapped profile emits without notes; the JSON stays at the prior
    // key set so consumers see no shape change.
    assert!(value.get("notes").is_none(), "{value}");
    let human = run(
        &db,
        &[
            "new",
            tmp.path().join("human-app").to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            "human-app",
        ],
    );
    let stdout = lossy(&human.stdout);
    assert!(stdout.contains(METADATA_PATH), "{stdout}");
    assert!(!stdout.contains("omitted"), "{stdout}");
}

#[test]
fn unmapped_profile_note_shape_is_unit_covered() {
    // No selectable supported profile lacks a mapping today, so the
    // omission note cannot fire through `forge new`; the pure-function
    // behavior is covered in `workspace::tests`. This test pins the
    // sentinel invariant instead: every selectable profile maps.
    for p in forge::profile::list_profiles() {
        assert!(
            p.workspace.is_some(),
            "{} must declare a governance mapping while selectable",
            p.id
        );
    }
    for p in forge::profile::planned_profiles() {
        assert!(p.workspace.is_none(), "{}", p.id);
    }
}

/// Age the generated `auth` feature to `0.0.9` in manifest and receipt so
/// a subsequent upgrade has a mutating step (mirrors upgrade_contract).
fn age_generated_feature(dest: &Path, db: &Path) {
    let manifest_path = dest.join("forge.yaml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    assert!(
        manifest.contains("auth: \"0.1.0\""),
        "generated manifest must pin auth at the tested version: {manifest}"
    );
    fs::write(
        &manifest_path,
        manifest.replace("auth: \"0.1.0\"", "auth: \"0.0.9\""),
    )
    .unwrap();
    // Re-register so the registry record agrees with the aged manifest.
    let out = run(db, &["register", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let descriptor = forge::feature::inspect_feature("auth").unwrap();
    let receipt = dest.join(".forge/features/auth.receipt");
    fs::create_dir_all(receipt.parent().unwrap()).unwrap();
    fs::write(
        &receipt,
        forge::feature::expected_receipt(&descriptor, "0.0.9"),
    )
    .unwrap();
}

#[test]
fn upgrade_refuses_edited_metadata_and_preserves_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = new_project(
        tmp.path(),
        &db,
        "rust-web",
        "meta-conflict",
        &["--feature", "auth"],
    );
    age_generated_feature(&dest, &db);
    let metadata = dest.join(METADATA_PATH);
    let original = fs::read(&metadata).unwrap();
    let edited = String::from_utf8(original.clone())
        .unwrap()
        .replace("cargo test", "whatever i want");
    fs::write(&metadata, edited.as_bytes()).unwrap();
    let manifest_before = fs::read(dest.join("forge.yaml")).unwrap();

    let out = run(&db, &["upgrade", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stderr));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[feature-ownership-conflict]"),
        "{stderr}"
    );
    assert!(stderr.contains(METADATA_PATH), "{stderr}");
    assert!(stderr.contains("no files were changed"), "{stderr}");
    // The user's edited file survives untouched and nothing else moved.
    assert_eq!(fs::read(&metadata).unwrap(), edited.as_bytes());
    assert_eq!(fs::read(dest.join("forge.yaml")).unwrap(), manifest_before);
    // The upgrade did not rewrite the feature either (pre-mutation block).
    assert!(lossy(&out.stdout).is_empty(), "stdout must stay empty");
}

#[test]
fn upgrade_succeeds_with_unedited_metadata_and_rewrites_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = new_project(
        tmp.path(),
        &db,
        "rust-web",
        "meta-clean",
        &["--feature", "auth"],
    );
    age_generated_feature(&dest, &db);
    let metadata = dest.join(METADATA_PATH);
    let original = fs::read(&metadata).unwrap();

    let out = run_json(&db, &["upgrade", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["upgrade"]["changed"], true);
    let files: Vec<&str> = value["upgrade"]["files_changed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(files.contains(&"forge.yaml"), "{files:?}");
    assert!(
        !files.contains(&METADATA_PATH),
        "{files:?}: unedited current content must not be rewritten"
    );
    assert_eq!(fs::read(&metadata).unwrap(), original);
}

#[test]
fn upgrade_refreshes_unedited_stale_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = new_project(
        tmp.path(),
        &db,
        "rust-web",
        "meta-refresh",
        &["--feature", "auth"],
    );
    age_generated_feature(&dest, &db);
    // The manifest profile moved on; the declaration is stale but unedited.
    let manifest_path = dest.join("forge.yaml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        manifest.replace("profile: rust-web", "profile: python-service"),
    )
    .unwrap();

    let out = run_json(&db, &["upgrade", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: Vec<&str> = value["upgrade"]["files_changed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(files.contains(&METADATA_PATH), "{files:?}");
    assert!(files.contains(&RECEIPT_PATH), "{files:?}");
    let text = fs::read_to_string(dest.join(METADATA_PATH)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["profile"], "python-product", "{text}");
    assert_eq!(
        parsed["verification"]["command"], "python3 -m unittest discover -s tests -v",
        "{text}"
    );
    let receipt = fs::read_to_string(dest.join(RECEIPT_PATH)).unwrap();
    assert!(
        receipt.contains(&forge::generate::workspace::sha256_hex(text.as_bytes())),
        "receipt must follow the refreshed content"
    );

    // Idempotent: a repeat upgrade has nothing to refresh.
    let out = run_json(&db, &["upgrade", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["upgrade"]["changed"], false, "{value}");
    assert_eq!(fs::read_to_string(dest.join(METADATA_PATH)).unwrap(), text);
}

#[test]
fn declaration_is_inert_for_native_builds_present_and_absent() {
    // Task 3.1 inertness: generated projects build and test with their
    // native toolchain with the declaration present (default) and with
    // it absent (--no-workspace-metadata). The file is metadata, never
    // a build input.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let none: &[&str] = &[];
    let opt_out: &[&str] = &["--no-workspace-metadata"];
    let cases: [(&str, &[&str]); 2] = [("inert-on", none), ("inert-off", opt_out)];
    for (id, extra) in cases {
        let dest = tmp.path().join(id);
        let mut args: Vec<&str> = vec![
            "new",
            dest.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            id,
        ];
        args.extend_from_slice(extra);
        let created = run(&db, &args);
        assert_eq!(created.status.code(), Some(0), "{}", lossy(&created.stderr));
        let dest = tmp.path().join(id);
        let build = Command::new("cargo")
            .arg("build")
            .current_dir(&dest)
            .env_remove("FORGE_REGISTRY")
            .output()
            .expect("cargo build");
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
        let test = Command::new("cargo")
            .arg("test")
            .current_dir(&dest)
            .env_remove("FORGE_REGISTRY")
            .output()
            .expect("cargo test");
        assert!(
            test.status.success(),
            "{}",
            String::from_utf8_lossy(&test.stderr)
        );
    }
    assert!(tmp.path().join("inert-on/.project.json").is_file());
    assert!(!tmp.path().join("inert-off/.project.json").exists());
}

#[test]
fn foreign_declaration_survives_upgrade_and_import_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("foreign-app");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("Cargo.toml"),
        "[package]\nname = \"foreign-app\"\n",
    )
    .unwrap();
    let foreign = serde_json::json!({
        "schema_version": 1,
        "id": "foreign-app",
        "kind": "product",
        "profile": "rust-product",
        "lifecycle": "planning",
        "verification": {
            "command": "make verify",
            "gate_runtime": "driftwatchdog",
            "evidence_status": "planned"
        },
        "deployment": {"deployable": false, "jenkins_job": null, "compose_file": null}
    });
    let foreign_bytes = format!("{}\n", serde_json::to_string_pretty(&foreign).unwrap());
    fs::write(proj.join(METADATA_PATH), foreign_bytes.as_bytes()).unwrap();

    let out = run_json(&db, &["import", proj.to_str().unwrap(), "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let proposal_out = run_json(&db, &["import", proj.to_str().unwrap()]);
    let value: serde_json::Value = serde_json::from_slice(&proposal_out.stdout).unwrap();
    assert_eq!(
        value["proposal"]["workspace_metadata"]["status"],
        "detected"
    );
    // Import never writes: foreign bytes are untouched and no Forge
    // receipt is invented around a file Forge does not own.
    assert_eq!(
        fs::read_to_string(proj.join(METADATA_PATH)).unwrap(),
        foreign_bytes
    );
    assert!(!proj.join(RECEIPT_PATH).exists());

    // An upgrade against the foreign declaration never rewrites or blocks
    // on it: it is not Forge-managed content.
    fs::write(
        proj.join(".forge/features/auth.receipt"),
        forge::feature::expected_receipt(
            &forge::feature::inspect_feature("auth").unwrap(),
            "0.0.9",
        ),
    )
    .ok();
    let manifest_path = proj.join("forge.yaml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        &manifest_path,
        format!("{manifest}features:\n  auth: \"0.0.9\"\n"),
    )
    .unwrap();
    let out = run_json(&db, &["upgrade", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert_eq!(
        fs::read_to_string(proj.join(METADATA_PATH)).unwrap(),
        foreign_bytes
    );
    assert!(!proj.join(RECEIPT_PATH).exists());
}

#[test]
fn import_observes_absent_metadata_without_writing_it() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("plain-app");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("Cargo.toml"), "[package]\nname = \"plain-app\"\n").unwrap();

    let out = run_json(&db, &["import", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["proposal"]["workspace_metadata"]["status"], "missing");

    let human = run(&db, &["import", proj.to_str().unwrap()]);
    assert!(
        lossy(&human.stdout).contains("Workspace metadata: missing"),
        "{}",
        lossy(&human.stdout)
    );

    let out = run(&db, &["import", proj.to_str().unwrap(), "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    // Non-goal honored: adoption never writes metadata retroactively.
    assert!(!proj.join(METADATA_PATH).exists());
    assert!(!proj.join(RECEIPT_PATH).exists());
}

#[test]
fn doctor_reports_presence_as_informational_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = new_project(tmp.path(), &db, "rust-web", "meta-doctor", &[]);

    let out = run_json(&db, &["doctor", dest.to_str().unwrap()]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let finding = value["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "workspace-metadata")
        .expect("presence must be reported");
    assert_eq!(finding["status"], "pass");
    assert_eq!(finding["applicable"], false);
    assert_eq!(finding["evidence"][0], METADATA_PATH);

    // Opt-out projects carry no such finding at all: absence is not a
    // problem Forge claims, so health and verdicts stay unchanged.
    let off = new_project(
        tmp.path(),
        &db,
        "rust-web",
        "meta-doctor-off",
        &["--no-workspace-metadata"],
    );
    let out = run_json(&db, &["doctor", off.to_str().unwrap()]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let found = value["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["id"] == "workspace-metadata");
    assert!(!found);
}

#[test]
fn checker_emits_no_workspace_alerts() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = new_project(tmp.path(), &db, "rust-web", "meta-check", &[]);
    let out = run_json(&db, &["check", dest.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(
        !text.contains("workspace"),
        "informational presence must never surface as an alert: {text}"
    );
}

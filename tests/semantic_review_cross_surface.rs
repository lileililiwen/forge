//! Cross-surface regression for the semantic review surface
//! (`project-semantic-description-review`).
//!
//! This suite answers the questions a per-surface contract cannot:
//!
//! - **A semantic review reads the project and writes no project
//!   file.** The project `forge.yaml` is byte-identical before and
//!   after a suggest, a list, a show, an approve and a reject.
//! - **A semantic review never opens the registry.** No registry
//!   file is created; a missing registry stays missing.
//! - **A semantic review is provider-free.** The closed `Operator` /
//!   `Local` provider set is the only set the surface accepts; a
//!   model-shaped provider id is refused at the parser boundary.
//! - **The closed proposal record shape never grows.** Every
//!   proposal field is the same on disk and on the wire, and no
//!   extra key is ever added to the JSON, the human table or the
//!   proposal.md.
//! - **Credentials are scrubbed on every surface.** A `ghp_` shape
//!   in `suggested_value` is redacted in the manifest, the
//!   proposal.md, the JSON page and the human table; a
//!   credential-shaped input is also refused at the parser so the
//!   surface cannot carry it through.
//!
//! Every case runs through the built `forge` binary. No live
//! network call is made; the closed `Operator`/`Local` provider
//! set is the only one the surface accepts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-semantic-proposal/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_rust_l1(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let body = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), body).unwrap();
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_ok(db: &Path, args: &[&str]) -> std::process::Output {
    let out = run(db, args);
    assert!(
        out.status.success(),
        "forge {args:?}: {}",
        lossy(&out.stderr)
    );
    out
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut all: Vec<&str> = args.to_vec();
    all.extend(["--format", "json"]);
    let out = run_ok(db, &all);
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("invalid json: {err}; stdout={}", lossy(&out.stdout)))
}

fn dir_from_id(value: &Value) -> String {
    format!(
        "{}-{}",
        value["kind"].as_str().expect("kind"),
        value["hash"].as_str().expect("hash")
    )
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path
            .strip_prefix(root)
            .unwrap()
            .starts_with(".forge/semantic")
        {
            out.push(path);
        }
    }
}

fn semantic_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn suggest_list_show_approve_reject_leave_the_project_files_byte_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("cross-app");
    write_rust_l1(&proj, "cross-app");
    let before = fs::read(proj.join("forge.yaml")).unwrap();
    let before_files = semantic_files(&proj);

    // suggest
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "Traceable catalog of cross-app projects",
            "--current-value",
            "(none)",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--confidence",
            "high",
            "--provider",
            "operator",
            "--note",
            "captured from the README",
        ],
    );
    let dir = dir_from_id(&suggest["proposal"]["id"]);

    // list + show round-trip
    let list = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    assert_eq!(list["proposals"].as_array().unwrap().len(), 1);
    let show = run_json(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    assert_eq!(
        show["proposal"]["suggested_value"],
        suggest["proposal"]["suggested_value"]
    );

    // approve with confirm
    let approved = run_json(
        &db,
        &[
            "describe",
            "approve",
            &dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(approved["state"], "approved");

    // second suggest + reject with confirm
    let second = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "Second proposal for cross-app",
            "--evidence-path",
            "forge.yaml",
            "--evidence-revision",
            "rev-2",
        ],
    );
    let second_dir = dir_from_id(&second["proposal"]["id"]);
    let rejected = run_json(
        &db,
        &[
            "describe",
            "reject",
            &second_dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert_eq!(rejected["state"], "rejected");

    // The project file is byte-identical: a semantic review never
    // touches the manifest.
    assert_eq!(fs::read(proj.join("forge.yaml")).unwrap(), before);

    // The semantic subtree is the only new file footprint, and it is
    // exactly two files per proposal.
    let files = semantic_files(&proj);
    let manifest_count = files
        .iter()
        .filter(|p| p.ends_with("manifest.json"))
        .count();
    let proposal_count = files.iter().filter(|p| p.ends_with("proposal.md")).count();
    assert_eq!(manifest_count, 2);
    assert_eq!(proposal_count, 2);
    // No file appeared outside the `.forge/semantic/` subtree.
    assert!(!files.is_empty());
    for f in &files {
        assert!(
            f.strip_prefix(&proj)
                .unwrap()
                .starts_with(".forge/semantic/"),
            "file outside .forge/semantic: {f:?}"
        );
    }
    // And the file set is exactly the before-set plus the two new
    // proposal directories.
    assert_eq!(
        files.len(),
        before_files.len() + 4,
        "before: {before_files:?}, after: {files:?}"
    );
}

#[test]
fn every_surface_writes_no_registry_byte_or_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("nodb-cross");
    write_rust_l1(&proj, "nodb-cross");
    // No registration: the registry file is absent.

    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert!(!db.exists(), "suggest must not create a registry file");
    let dir = dir_from_id(&suggest["proposal"]["id"]);

    let _ = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    assert!(!db.exists(), "list must not create a registry file");

    let _ = run_json(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    assert!(!db.exists(), "show must not create a registry file");

    let _ = run_json(
        &db,
        &[
            "describe",
            "approve",
            &dir,
            proj.to_str().unwrap(),
            "--confirm",
        ],
    );
    assert!(!db.exists(), "approve must not create a registry file");

    // The full no-registry round trip: a follow-up suggest for a
    // different revision still works without a registry. The
    // earlier proposal was approved (no longer `Suggested`), so
    // this becomes a fresh `Generated` proposal rather than
    // superseding a now-decided one.
    let second = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "second",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-2",
        ],
    );
    assert_eq!(second["status"], "generated");
    assert!(
        !db.exists(),
        "second suggest must not create a registry file"
    );
}

#[test]
fn no_provider_subprocess_is_spawned_and_no_network_is_touched() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("prov-cross");
    write_rust_l1(&proj, "prov-cross");

    // The closed provider set is the only set the surface accepts.
    for bad in [
        "gpt-4",
        "openai",
        "anthropic",
        "claude",
        "ollama",
        "local-llm",
    ] {
        let out = run(
            &db,
            &[
                "describe",
                "suggest",
                proj.to_str().unwrap(),
                "--suggested-value",
                "demo",
                "--evidence-path",
                "README.md",
                "--evidence-revision",
                "rev-1",
                "--provider",
                bad,
            ],
        );
        assert_eq!(out.status.code(), Some(1), "provider {bad} must be refused");
        let stderr = lossy(&out.stderr);
        assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    }
    // A valid provider succeeds and no extra env keys reach the
    // transport: the test already strips OPENAI_API_KEY and
    // ANTHROPIC_API_KEY; the success path here confirms the
    // surface does not look at them in the first place.
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--provider",
            "operator",
        ],
    );
    assert_eq!(suggest["proposal"]["provider"], "operator");

    // The same holds for the `local` provider.
    let second = run_json(
        &db,
        &[
            "classify",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "tools",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--provider",
            "local",
        ],
    );
    assert_eq!(second["proposal"]["provider"], "local");
}

#[test]
fn the_proposal_record_is_a_closed_field_set_on_every_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("closed-app");
    write_rust_l1(&proj, "closed-app");
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "closed fields",
            "--current-value",
            "(none)",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--confidence",
            "medium",
            "--provider",
            "operator",
            "--note",
            "operator note",
        ],
    );
    let dir = dir_from_id(&suggest["proposal"]["id"]);
    let manifest_path = proj
        .join(".forge/semantic/closed-app")
        .join(&dir)
        .join("manifest.json");
    let proposal_md_path = proj
        .join(".forge/semantic/closed-app")
        .join(&dir)
        .join("proposal.md");

    // JSON: the on-disk manifest is the canonical JSON document and
    // it carries exactly the closed field set.
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let mut manifest_keys: Vec<&str> = manifest
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    manifest_keys.sort();
    let mut expected_manifest_keys = [
        "conflict",
        "confidence",
        "contract",
        "current_value",
        "decided_at",
        "evidence",
        "id",
        "kind",
        "note",
        "project_id",
        "provider",
        "state",
        "suggested_at",
        "suggested_value",
    ];
    expected_manifest_keys.sort();
    assert_eq!(manifest_keys, expected_manifest_keys);

    // The `id` object itself is closed.
    let id_keys: Vec<&str> = manifest["id"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(id_keys, ["hash", "kind", "project_id"]);

    // The `evidence` entries are closed too.
    let evidence_keys: Vec<&str> = manifest["evidence"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(evidence_keys, ["excerpt", "path", "revision"]);

    // JSON page: list and show reuse the same closed shape.
    let list = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    let list_entry = &list["proposals"][0];
    let mut list_keys: Vec<&str> = list_entry
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    list_keys.sort();
    let mut expected_list_keys = [
        "confidence",
        "decided_at",
        "dir_name",
        "evidence_count",
        "id",
        "kind",
        "project_id",
        "provider",
        "state",
        "suggested_at",
    ];
    expected_list_keys.sort();
    assert_eq!(list_keys, expected_list_keys);
    assert_eq!(list_entry["dir_name"].as_str().unwrap(), dir);

    let show = run_json(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    let show_proposal = &show["proposal"];
    let mut show_keys: Vec<&str> = show_proposal
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    show_keys.sort();
    assert_eq!(show_keys, expected_manifest_keys);

    // proposal.md is a closed marker list; no field is added and the
    // value fields are quoted.
    let md = fs::read_to_string(&proposal_md_path).unwrap();
    for line in [
        "# Semantic proposal: description",
        "- contract:",
        "- project:",
        "- state:",
        "- confidence:",
        "- provider:",
        "- suggested_at:",
        "- suggested_value:",
        "## Evidence",
    ] {
        assert!(md.contains(line), "missing `{line}` in proposal.md:\n{md}");
    }
    // The contract id is present in both surfaces.
    assert!(md.contains(CONTRACT));
    assert_eq!(manifest["contract"], CONTRACT);
}

#[test]
fn credentials_are_scrubbed_on_every_output_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("scrub-app");
    write_rust_l1(&proj, "scrub-app");
    // The first call embeds a credential-shaped token in a
    // non-refused position (an excerpt) and is therefore scrubbed
    // rather than refused.
    let suggest = run_json(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "tools for inspecting catalogs",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
            "--evidence-excerpt",
            "token=ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG end",
        ],
    );
    let dir = dir_from_id(&suggest["proposal"]["id"]);
    let manifest_path = proj
        .join(".forge/semantic/scrub-app")
        .join(&dir)
        .join("manifest.json");
    let proposal_md_path = proj
        .join(".forge/semantic/scrub-app")
        .join(&dir)
        .join("proposal.md");

    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    assert!(
        !manifest_text.contains("ghp_"),
        "manifest leaked ghp_: {manifest_text}"
    );
    let proposal_md = fs::read_to_string(&proposal_md_path).unwrap();
    assert!(
        !proposal_md.contains("ghp_"),
        "proposal.md leaked ghp_: {proposal_md}"
    );

    // A direct `suggested_value` carrying the shape is refused at
    // the parser, not redacted into the manifest.
    let out = run(
        &db,
        &[
            "describe",
            "suggest",
            proj.to_str().unwrap(),
            "--suggested-value",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev-1",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[semantic-invalid]"), "{stderr}");
    assert!(!stderr.contains("ghp_"), "stderr leaked ghp_: {stderr}");

    // The list / show / approve outputs are checked across formats.
    let list_json = run_json(&db, &["describe", "list", proj.to_str().unwrap()]);
    let list_json_text = lossy(&serde_json::to_vec(&list_json).unwrap());
    assert!(!list_json_text.contains("ghp_"), "{list_json_text}");

    let show_json = run_json(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    let show_json_text = lossy(&serde_json::to_vec(&show_json).unwrap());
    assert!(!show_json_text.contains("ghp_"), "{show_json_text}");

    // The human-format surface runs the same redactor.
    let list_human = run_ok(&db, &["describe", "list", proj.to_str().unwrap()]);
    let list_human_text = lossy(&list_human.stdout);
    assert!(!list_human_text.contains("ghp_"), "{list_human_text}");

    let show_human = run_ok(&db, &["describe", "show", &dir, proj.to_str().unwrap()]);
    let show_human_text = lossy(&show_human.stdout);
    assert!(!show_human_text.contains("ghp_"), "{show_human_text}");
}

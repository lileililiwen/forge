//! Contract test for the scaffold's changelog bootstrap
//! (`project-changelog-bootstrap`).
//!
//! `forge release prepare` refuses a project with no changelog file. The
//! scaffold used to write none, so a fresh project could not prepare a
//! release until its owner hand-wrote the file Forge declined to write.
//!
//! All scenarios drive the CLI through the built binary:
//!
//! - The scaffold writes `CHANGELOG.md` at the project's root, in
//!   keep-a-changelog form, with one `## [Unreleased]` section that names
//!   the scaffold and the profile.
//! - The file names no released version and no date: nothing has been
//!   released, and the assembly is deterministic.
//! - `forge release prepare` on a scaffolded project captures that
//!   changelog instead of refusing for a missing one.
//! - A project whose changelog is gone is still refused, naming the
//!   missing file. The refusal is unchanged.
//! - The same creation request renders byte-identical changelogs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_CONTAINER_BIN")
        .env_remove("FORGE_NOTES_BIN");
    cmd
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
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

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
}

/// Scaffold a project and give it a git revision, which the release lane
/// captures as the plan's source revision.
fn scaffold(db: &Path, dest: &Path, id: &str) {
    let out = run(
        db,
        &[
            "new",
            dest.to_str().unwrap(),
            "--profile",
            "rust-web",
            "--id",
            id,
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "forge new failed: {}",
        lossy(&out.stderr)
    );
    run_git(dest, &["init", "-q"]);
    run_git(dest, &["config", "user.email", "forge@example.com"]);
    run_git(dest, &["config", "user.name", "Forge Test"]);
    run_git(dest, &["config", "init.defaultBranch", "main"]);
    run_git(dest, &["add", "-A"]);
    run_git(dest, &["commit", "-q", "-m", "initial"]);
}

/// The `## [` headings of a changelog.
fn version_headings(changelog: &str) -> Vec<&str> {
    changelog
        .lines()
        .filter(|line| line.starts_with("## ["))
        .collect()
}

/// Whether the text names a date in the shape `YYYY-MM-DD`.
fn names_a_date(text: &str) -> bool {
    let groups: Vec<&str> = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|group| !group.is_empty())
        .collect();
    groups
        .windows(3)
        .any(|w| w[0].len() == 4 && w[1].len() == 2 && w[2].len() == 2)
}

#[test]
fn the_scaffold_writes_a_changelog_that_claims_no_release() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("changelog-app");
    scaffold(&db, &dest, "changelog-app");

    let path = dest.join("CHANGELOG.md");
    assert!(
        path.is_file(),
        "the scaffold must write a changelog: {}",
        path.display()
    );
    let changelog = fs::read_to_string(&path).expect("changelog is readable");
    assert!(changelog.starts_with("# Changelog\n"), "{changelog}");
    assert!(
        changelog.contains("All notable changes to changelog-app are documented here."),
        "{changelog}"
    );
    assert!(
        changelog.contains("[Keep a Changelog](https://keepachangelog.com/en/1.0.0/)"),
        "{changelog}"
    );
    assert!(
        changelog.contains("[Semantic Versioning](https://semver.org/spec/v2.0.0.html)"),
        "{changelog}"
    );
    // One `## [Unreleased]` section, and no version heading that starts
    // with a digit: nothing has been released.
    assert_eq!(
        version_headings(&changelog),
        vec!["## [Unreleased]"],
        "{changelog}"
    );
    assert!(
        !version_headings(&changelog).iter().any(|line| line
            .trim_start_matches("## [")
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())),
        "{changelog}"
    );
    // No date: the assembly is deterministic and carries no clock.
    assert!(!names_a_date(&changelog), "{changelog}");
    // The scaffold names itself and the profile.
    assert!(changelog.contains("### Added"), "{changelog}");
    assert!(
        changelog.contains("- Initial Forge scaffold for the `rust-web` profile."),
        "{changelog}"
    );
}

#[test]
fn a_scaffolded_project_prepares_a_release_with_the_generated_changelog() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("release-app");
    scaffold(&db, &dest, "release-app");

    let value = run_json(
        &db,
        &[
            "release",
            "prepare",
            dest.to_str().unwrap(),
            "--version",
            "0.1.0",
        ],
    );
    let plan = &value["plan"];
    // The changelog was captured, not refused for.
    assert_eq!(plan["changelog"]["path"], "CHANGELOG.md");
    let excerpt = plan["changelog"]["excerpt"].as_str().unwrap();
    assert!(excerpt.contains("## [Unreleased]"), "{excerpt}");
    assert!(
        excerpt.contains("- Initial Forge scaffold for the `rust-web` profile."),
        "{excerpt}"
    );
    // The captured hash is the hash of the file the scaffold wrote.
    let bytes = fs::read(dest.join("CHANGELOG.md")).unwrap();
    assert_eq!(
        plan["changelog"]["content_hash"].as_str().unwrap(),
        forge::release::prepare::hash_bytes(&bytes),
    );
}

#[test]
fn a_project_without_a_changelog_is_still_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("stripped-app");
    scaffold(&db, &dest, "stripped-app");
    fs::remove_file(dest.join("CHANGELOG.md")).unwrap();

    let out = run(
        &db,
        &[
            "release",
            "prepare",
            dest.to_str().unwrap(),
            "--version",
            "0.1.0",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("release-invalid"),
        "stderr must report release-invalid: {stderr}"
    );
    assert!(
        stderr.contains("CHANGELOG.md"),
        "the refusal must name the missing changelog: {stderr}"
    );
}

#[test]
fn the_same_creation_request_renders_byte_identical_changelogs() {
    let tmp = tempfile::tempdir().unwrap();
    let one = tmp.path().join("one");
    let two = tmp.path().join("two");
    // Two registries, so the second run is not refused for an id collision.
    scaffold(&tmp.path().join("registry-one.db"), &one, "same-id");
    scaffold(&tmp.path().join("registry-two.db"), &two, "same-id");

    let first = fs::read(one.join("CHANGELOG.md")).unwrap();
    let second = fs::read(two.join("CHANGELOG.md")).unwrap();
    assert_eq!(
        first, second,
        "the same creation request must render byte-identical changelogs"
    );
}

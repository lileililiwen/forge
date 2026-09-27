//! Contract for the artifact and CI baseline (`artifact-and-ci-baseline`).
//!
//! - Licence/package consistency: `LICENSE` exists and matches
//!   `license = "MIT"`; `CHANGELOG.md` exists at the release plane's
//!   default path; `forge --version` equals the Cargo version and the
//!   changelog's newest entry (disagreement fails quoting all three).
//! - Truthful entry point: `Cargo.toml` carries `[workspace]` so the
//!   recorded `cargo test --workspace` describes the real package graph;
//!   shared floors live in `[workspace.dependencies]` with no restated
//!   `[dev-dependencies]` duplicates.
//! - Toolchain floor: `rust-version` is declared (derivation in ADR 0002).
//! - CI honesty: every job sets `permissions: contents: read` and an
//!   explicit `timeout-minutes`; no step label names the gate runtime
//!   without invoking it; no workflow pushes, tags, publishes or deploys.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).expect("baseline file must exist")
}

fn cargo_version() -> String {
    let text = read("Cargo.toml");
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("version"))
        .expect("Cargo.toml carries a version");
    line.split('"')
        .nth(1)
        .expect("version is quoted")
        .to_string()
}

fn changelog_newest() -> String {
    let text = read("CHANGELOG.md");
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("## ["))
        .expect("CHANGELOG.md carries an entry");
    line.split(['[', ']'])
        .nth(1)
        .expect("entry names a version")
        .to_string()
}

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

#[test]
fn licence_file_matches_declaration() {
    let manifest = read("Cargo.toml");
    assert!(
        manifest.lines().any(|l| l.trim() == "license = \"MIT\""),
        "Cargo.toml declares license = MIT"
    );
    let licence = read("LICENSE");
    assert!(
        licence.contains("MIT License") && licence.contains("Permission is hereby granted"),
        "LICENSE carries the MIT text the manifest declares"
    );
}

#[test]
fn changelog_resolves_at_default_path() {
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("CHANGELOG.md")
            .is_file(),
        "CHANGELOG.md exists at src/release DEFAULT_CHANGELOG"
    );
    assert!(!changelog_newest().is_empty());
}

#[test]
fn reported_manifest_changelog_versions_agree() {
    let out = Command::new(forge_bin())
        .arg("--version")
        .output()
        .expect("forge --version must run");
    assert!(out.status.success());
    let reported = String::from_utf8_lossy(&out.stdout);
    let reported = reported.trim().rsplit(' ').next().unwrap_or("").to_string();
    let manifest = cargo_version();
    let changelog = changelog_newest();
    assert_eq!(
        reported, manifest,
        "CLI version ({reported}) disagrees with Cargo.toml ({manifest})"
    );
    assert_eq!(
        manifest, changelog,
        "Cargo.toml ({manifest}) disagrees with CHANGELOG newest entry ({changelog})"
    );
}

#[test]
fn workspace_and_inherited_floors() {
    let manifest = read("Cargo.toml");
    assert!(
        manifest.lines().any(|l| l.trim() == "[workspace]"),
        "Cargo.toml carries [workspace] so `cargo test --workspace` is real"
    );
    assert!(
        manifest.contains("[workspace.dependencies]"),
        "shared floors live in [workspace.dependencies]"
    );
    assert!(
        manifest.contains("rust-version"),
        "rust-version floor is declared (ADR 0002)"
    );
    for dep in ["tempfile", "serde_json", "rusqlite"] {
        let restated = manifest
            .split("[dev-dependencies]")
            .nth(1)
            .map(|dev| dev.lines().any(|l| l.trim_start().starts_with(dep)))
            .unwrap_or(false);
        assert!(!restated, "[dev-dependencies] must not restate {dep}");
    }
}

#[test]
fn packaging_scripts_present_and_executable() {
    for script in [
        "scripts/package.sh",
        "scripts/checksum.sh",
        "scripts/install.sh",
        "scripts/smoke.sh",
        "scripts/bump.sh",
        "scripts/release-check.sh",
    ] {
        let path = repo_root().join(script);
        assert!(path.is_file(), "{script} exists");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("stat script")
                .permissions()
                .mode();
            assert!(mode & 0o111 != 0, "{script} is executable");
        }
    }
    assert!(repo_root().join("deny.toml").is_file(), "deny.toml exists");
    let gitignore = read(".gitignore");
    assert!(
        gitignore.lines().any(|l| l.trim() == "/dist/"),
        "packaging output dist/ stays gitignored"
    );
}

#[test]
fn ci_jobs_honest_and_bounded() {
    let ci = read(".github/workflows/ci.yml");
    assert!(
        !ci.contains("auto-tag"),
        "no tag automation (explicit non-goal)"
    );
    for forbidden in [
        "crates.io",
        "npm publish",
        "docker push",
        "gh release",
        "release.yml",
    ] {
        assert!(
            !ci.contains(forbidden),
            "CI must not publish or promote ({forbidden})"
        );
    }
    let jobs_section = ci
        .split_once("\njobs:\n")
        .expect("CI defines a jobs section")
        .1;
    let mut jobs: Vec<String> = Vec::new();
    for line in jobs_section.lines() {
        let trimmed = line.trim_end();
        if trimmed.len() > 2
            && line.starts_with("  ")
            && !line.starts_with("   ")
            && trimmed.ends_with(':')
        {
            jobs.push(String::new());
        }
        if let Some(last) = jobs.last_mut() {
            last.push_str(line);
            last.push('\n');
        }
    }
    assert!(jobs.len() >= 10, "CI carries the full job graph");
    for job in &jobs {
        assert!(
            job.contains("permissions:") && job.contains("contents: read"),
            "job sets explicit read permissions: {job:.40}"
        );
        assert!(
            job.contains("timeout-minutes:"),
            "job sets an explicit timeout: {job:.40}"
        );
    }
    // Only the real gate job may label itself with the gate runtime.
    for job in &jobs {
        let name = job.lines().next().unwrap_or("");
        if name.trim() == "gate:" {
            assert!(job.contains("forge gate ."), "gate job runs the real gate");
        } else {
            for line in job
                .lines()
                .filter(|l| l.trim_start().starts_with("- name:"))
            {
                let label = line.to_lowercase();
                if label.contains("gate") {
                    assert!(
                        label.contains("readiness"),
                        "non-gate job labels only the readiness gate it runs: {line}"
                    );
                }
            }
            assert!(
                !job.contains("forge gate "),
                "only the gate job invokes the gate runtime: {name}"
            );
        }
    }
}

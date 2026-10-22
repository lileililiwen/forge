//! Profile build-command runnable contract (`profile-build-command-runnable`).
//!
//! Two rules, one check. A profile's declared build command must run the
//! toolchain the profile declares, and the declared build command must
//! actually run in a project the profile generates. The command is
//! exercised through Forge's own build lane, so the command, the runner,
//! the working directory and the report are observed together rather than
//! through a hand-rolled invocation.
//!
//! A profile whose toolchain the host does not have is skipped in the
//! house's own way: the check says nothing about it and never reports it
//! as passing. The profile table below covers the whole supported
//! catalogue, so a new supported profile must be added to it.

use forge::profile::{inspect_profile, list_profiles};
use std::path::{Path, PathBuf};
use std::process::Command;

/// `(profile, project directory leaf)` for every supported profile. The
/// leaf is kebab-cased into the project's default id, so each profile
/// gets its own id and its own directory.
const PROJECTS: [(&str, &str); 6] = [
    ("aspnet-web", "dotnet-app"),
    ("flutter-app", "flutter-app"),
    ("nextjs-web", "next-app"),
    ("python-service", "python-app"),
    ("react-web", "react-app"),
    ("rust-web", "rust-app"),
];

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

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

/// True when the host has the toolchain the profile declares.
fn toolchain_present(toolchain: &str) -> bool {
    Command::new(toolchain).arg("--version").output().is_ok()
}

/// The programs a profile's own toolchain provides.
///
/// The catalogue pins `nextjs-web` to `node` and `react-web` to `npm`, and
/// both build with `npm run build`: `npm` is the Node.js toolchain's own
/// command runner, and the repository already treats the two names as one
/// toolchain (`src/kit/registry.rs` maps both to the npm ecosystem,
/// `src/target/mod.rs` gives both the same target form). Every other
/// toolchain provides exactly one program: its own name.
fn toolchain_programs(toolchain: &str) -> Vec<&str> {
    match toolchain {
        "node" | "npm" => vec!["node", "npm"],
        other => vec![other],
    }
}

#[test]
fn the_build_command_runs_the_profiles_own_toolchain() {
    for descriptor in list_profiles() {
        let program = descriptor
            .build_command
            .split_whitespace()
            .next()
            .unwrap_or_default();
        let own = toolchain_programs(&descriptor.toolchain);
        assert!(
            own.contains(&program),
            "profile `{}` must run its own toolchain `{}` as the program of its build command `{}`, not `{}`",
            descriptor.id,
            descriptor.toolchain,
            descriptor.build_command,
            program
        );
    }
}

#[test]
fn the_profile_table_covers_every_supported_profile() {
    let supported = list_profiles();
    assert_eq!(
        supported.len(),
        PROJECTS.len(),
        "the profile table must cover every supported profile"
    );
    for descriptor in &supported {
        assert!(
            PROJECTS.iter().any(|(id, _)| *id == descriptor.id),
            "{} must be in the profile table",
            descriptor.id
        );
    }
}

#[test]
fn every_declared_build_command_runs_in_a_generated_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let mut available = 0usize;
    let mut exercised = 0usize;

    for (profile, leaf) in PROJECTS {
        let descriptor = inspect_profile(profile)
            .unwrap_or_else(|err| panic!("{profile} must be inspectable: {err}"));
        if !toolchain_present(&descriptor.toolchain) {
            // The host lacks this profile's toolchain: skip it and claim
            // nothing about it.
            continue;
        }
        available += 1;

        let dest = tmp.path().join(leaf);
        let out = run(
            &db,
            &["new", &dest.display().to_string(), "--profile", profile],
        );
        assert_eq!(
            out.status.code(),
            Some(0),
            "profile `{profile}` must generate a project: {}",
            lossy(&out.stderr)
        );

        let out = run(&db, &["build", &dest.display().to_string()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "profile `{profile}` must build with `{}`: {}",
            descriptor.build_command,
            lossy(&out.stderr)
        );
        let stdout = lossy(&out.stdout);
        assert!(
            stdout.contains("status: passed"),
            "profile `{profile}` must report a passed build: {stdout}"
        );
        exercised += 1;
    }

    assert_eq!(
        exercised, available,
        "every profile whose toolchain the host has must be exercised"
    );
}

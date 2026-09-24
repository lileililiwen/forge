//! CLI contract for Workspace Governance adapter consumption
//! (`workspace-governance-adapter-consumption`).
//!
//! Drives the built binary through the packaged-preset selection surface
//! against executable audit-shaped stubs (copies of the fixtures captured
//! from the real sibling adapter):
//!
//! - Preset selection: `use workspace-governance` without `--adapter`
//!   resolves `<root>/workspace-governance/scripts/forge_governance_adapter.py`
//!   from the
//!   `--workspace-root` argument or `FORGE_WORKSPACE_ROOT`, stores the
//!   absolute resolved path, and the next `status` consumes it through
//!   the v0.1.0 boundary.
//! - Refusal taxonomy: an unresolved root, a missing candidate, a
//!   directory candidate and a non-executable candidate each refuse with
//!   `error[governance-invalid]` naming the exact candidate path, leave
//!   the previously selected provider in force, and write nothing.
//! - Precedence: an explicit `--adapter` always wins over a resolvable
//!   preset; the argument root wins over the environment root.
//! - Isolation: a sibling checkout removed after selection yields
//!   `unavailable` (never a PASS), local commands continue unchanged,
//!   and `forge.yaml` survives every call byte-identical.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Every call runs with a hermetic environment: no workspace root, no
/// registry, no proxy or provider credential leaks.
fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    cmd
}

fn run(args: &[&str]) -> Output {
    run_with_workspace_env(args, None)
}

fn run_with_workspace_env(args: &[&str], workspace_root: Option<&Path>) -> Output {
    let mut cmd = clean_cmd();
    match workspace_root {
        Some(root) => {
            cmd.env("FORGE_WORKSPACE_ROOT", root);
        }
        None => {
            cmd.env_remove("FORGE_WORKSPACE_ROOT");
        }
    }
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/governance-audit")
        .join(name)
}

fn project_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("forge.yaml"),
        "schema: 1\nproject:\n  id: preset-demo\n  name: Preset Demo\n  profile: rust-web\n",
    )
    .unwrap();
    dir
}

#[cfg(unix)]
fn stage_candidate(root: &Path, fixture_name: &str, executable: bool) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let candidate = root.join("workspace-governance/scripts/forge_governance_adapter.py");
    fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    fs::copy(fixture(fixture_name), &candidate).unwrap();
    fs::set_permissions(
        &candidate,
        fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
    )
    .unwrap();
    candidate
}

#[cfg(not(unix))]
fn stage_candidate(root: &Path, fixture_name: &str, _executable: bool) -> PathBuf {
    let candidate = root.join("workspace-governance/scripts/forge_governance_adapter.py");
    fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    fs::copy(fixture(fixture_name), &candidate).unwrap();
    candidate
}

fn governance_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("json on stdout")
}

fn provider_config_path(project: &Path) -> PathBuf {
    project.join(".forge/providers.yaml")
}

#[test]
fn use_help_advertises_the_workspace_root_input() {
    let out = clean_cmd()
        .args(["governance", "use", "--help"])
        .output()
        .expect("run help");
    assert!(out.status.success());
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("--workspace-root"), "{stdout}");
    assert!(
        stdout.contains("never searched implicitly")
            || stdout.contains("Never searched")
            || stdout.contains("$FORGE_WORKSPACE_ROOT"),
        "help must name the explicit-only resolution rule: {stdout}"
    );
}

#[test]
fn workspace_root_without_a_preset_and_without_adapter_refuses() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    let failed = run(&[
        "governance",
        "use",
        "external",
        project.path().to_str().unwrap(),
        "--workspace-root",
        workspace.path().to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    assert!(
        failed.stdout.is_empty(),
        "stdout: {}",
        lossy(&failed.stdout)
    );
    let stderr = lossy(&failed.stderr);
    assert!(stderr.contains("error[governance-invalid]"), "{stderr}");
    assert!(stderr.contains("has no packaged adapter"), "{stderr}");
    assert!(!provider_config_path(project.path()).exists());
}

#[test]
fn explicit_adapter_with_workspace_root_stores_and_supplies_the_root() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    let root = workspace.path().canonicalize().unwrap();
    let stub = project.path().join("env.sh");
    fs::write(
        &stub,
        r#"#!/bin/sh
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"pass","evidence":["workspace=%s"],"detail":"env"}' "$pid" "${WORKSPACE_ROOT:-UNSET}"
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let selected = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
        "--adapter",
        stub.to_str().unwrap(),
        "--workspace-root",
        root.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));
    assert_eq!(
        governance_json(&selected)["adapter"],
        stub.to_str().unwrap()
    );
    assert_eq!(
        governance_json(&selected)["workspace_root"],
        root.to_str().unwrap()
    );

    // The stored root is re-supplied on every check with the environment
    // absent: the adapter reports exactly what Forge persisted.
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let observation = &governance_json(&status)["observation"];
    assert_eq!(observation["status"], "pass");
    assert_eq!(
        observation["evidence"][0],
        format!("workspace={}", root.display())
    );
}

#[test]
fn preset_selection_stores_resolved_path_and_status_consumes_it() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    let candidate = stage_candidate(workspace.path(), "adopted-clean.sh", true);
    let manifest_before = fs::read(project.path().join("forge.yaml")).unwrap();

    let selected = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
        "--workspace-root",
        workspace.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));
    let body = governance_json(&selected);
    assert_eq!(body["provider"], "workspace-governance");
    assert_eq!(
        body["adapter"],
        candidate.canonicalize().unwrap().display().to_string()
    );
    // The resolved absolute path is stored, so the next check needs no
    // environment at all, and it runs through the existing v0.1.0
    // boundary: the fixture's `pass` observation surfaces end to end.
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(status.status.success(), "{}", lossy(&status.stderr));
    let observation = &governance_json(&status)["observation"];
    assert_eq!(observation["provider"], "workspace-governance");
    assert_eq!(observation["status"], "pass");
    assert_eq!(observation["project_id"], "preset-demo");
    assert_eq!(
        observation["source_revision"],
        "888d8058e7a2615e0c6525a533463e5029d56487"
    );
    let stored = fs::read_to_string(provider_config_path(project.path())).unwrap();
    assert!(stored.contains(&candidate.canonicalize().unwrap().display().to_string()));
    assert!(stored.contains("workspace-governance"));
    // Selection never touched the manifest.
    assert_eq!(
        fs::read(project.path().join("forge.yaml")).unwrap(),
        manifest_before
    );
}

#[test]
fn environment_root_resolves_and_survives_env_removal() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    stage_candidate(workspace.path(), "adoption-gap.sh", true);

    let selected = run_with_workspace_env(
        &[
            "governance",
            "use",
            "workspace-governance",
            project.path().to_str().unwrap(),
            "--format",
            "json",
        ],
        Some(workspace.path()),
    );
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));

    // The stored path is absolute: a later check with no environment set
    // still reaches the same adapter and maps `blocked` verbatim.
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let observation = &governance_json(&status)["observation"];
    assert_eq!(observation["status"], "blocked");
    assert_eq!(observation["provider"], "workspace-governance");
    assert!(!governance_json(&status)["observation"]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line == "pass"));
}

#[test]
fn argument_root_wins_over_the_environment() {
    let project = project_dir();
    let env_root = TempDir::new().unwrap();
    stage_candidate(env_root.path(), "adoption-gap.sh", true);
    let flag_root = TempDir::new().unwrap();
    stage_candidate(flag_root.path(), "adopted-clean.sh", true);

    let selected = run_with_workspace_env(
        &[
            "governance",
            "use",
            "workspace-governance",
            project.path().to_str().unwrap(),
            "--workspace-root",
            flag_root.path().to_str().unwrap(),
            "--format",
            "json",
        ],
        Some(env_root.path()),
    );
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));
    assert_eq!(
        governance_json(&selected)["adapter"],
        flag_root
            .path()
            .join("workspace-governance/scripts/forge_governance_adapter.py")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(governance_json(&status)["observation"]["status"], "pass");
}

#[test]
fn explicit_adapter_wins_over_a_resolvable_preset() {
    let project = project_dir();
    let preset_root = TempDir::new().unwrap();
    // The preset candidate reports `blocked`; the explicit adapter reports
    // `pass` — the stored selection proves which one the operator chose.
    stage_candidate(preset_root.path(), "adoption-gap.sh", true);
    let explicit = project.path().join("explicit-adapter.sh");
    fs::copy(fixture("adopted-clean.sh"), &explicit).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&explicit, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let selected = run_with_workspace_env(
        &[
            "governance",
            "use",
            "workspace-governance",
            project.path().to_str().unwrap(),
            "--adapter",
            explicit.to_str().unwrap(),
            "--format",
            "json",
        ],
        Some(preset_root.path()),
    );
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));
    assert_eq!(
        governance_json(&selected)["adapter"],
        explicit.to_str().unwrap()
    );
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(governance_json(&status)["observation"]["status"], "pass");
}

#[test]
fn unresolved_root_refuses_naming_both_inputs_and_stores_nothing() {
    let project = project_dir();
    let failed = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    assert!(
        failed.stdout.is_empty(),
        "stdout: {}",
        lossy(&failed.stdout)
    );
    let stderr = lossy(&failed.stderr);
    assert!(stderr.contains("error[governance-invalid]"), "{stderr}");
    assert!(stderr.contains("--workspace-root"), "{stderr}");
    assert!(stderr.contains("FORGE_WORKSPACE_ROOT"), "{stderr}");
    assert!(!provider_config_path(project.path()).exists());
}

#[test]
fn non_executable_candidate_refuses_naming_exact_path_and_keeps_previous_provider() {
    let project = project_dir();
    // Previous selection: the built-in local provider, healthy.
    let prior = run(&[
        "governance",
        "use",
        "local",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(prior.status.success(), "{}", lossy(&prior.stderr));
    let stored_before = fs::read(provider_config_path(project.path())).unwrap();

    let workspace = TempDir::new().unwrap();
    let candidate = stage_candidate(workspace.path(), "adopted-clean.sh", false);
    let failed = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
        "--workspace-root",
        workspace.path().to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    let stderr = lossy(&failed.stderr);
    assert!(stderr.contains("error[governance-invalid]"), "{stderr}");
    assert!(
        stderr.contains(&candidate.display().to_string()),
        "refusal must name the exact candidate: {stderr}"
    );

    // The previously selected provider remains in force and still works.
    let stored_after = fs::read(provider_config_path(project.path())).unwrap();
    assert_eq!(stored_before, stored_after);
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(status.status.success(), "{}", lossy(&status.stderr));
    let observation = &governance_json(&status)["observation"];
    assert_eq!(observation["provider"], "local");
    assert_eq!(observation["status"], "pass");
}

#[test]
fn directory_candidate_refuses_as_not_a_regular_file() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    fs::create_dir_all(
        workspace
            .path()
            .join("workspace-governance/scripts/forge_governance_adapter.py"),
    )
    .unwrap();
    let failed = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
        "--workspace-root",
        workspace.path().to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    let stderr = lossy(&failed.stderr);
    assert!(stderr.contains("not a regular file"), "{stderr}");
    assert!(
        stderr.contains("scripts/forge_governance_adapter.py"),
        "{stderr}"
    );
}

#[test]
fn selection_never_searches_parent_directories() {
    // A fully valid candidate two levels above the project must NOT be
    // discovered implicitly: without the flag or env the command refuses
    // exactly as if nothing existed on disk.
    let outer = TempDir::new().unwrap();
    stage_candidate(outer.path(), "adopted-clean.sh", true);
    let nested = outer.path().join("work/project");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("forge.yaml"),
        "schema: 1\nproject:\n  id: preset-demo\n  name: Preset Demo\n  profile: rust-web\n",
    )
    .unwrap();

    let failed = run(&[
        "governance",
        "use",
        "workspace-governance",
        nested.to_str().unwrap(),
    ]);
    assert!(!failed.status.success());
    let stderr = lossy(&failed.stderr);
    assert!(stderr.contains("--workspace-root"), "{stderr}");
    assert!(
        !stderr.contains("scripts/forge_governance_adapter.py"),
        "no implicit path may be reported: {stderr}"
    );
    assert!(!provider_config_path(&nested).exists());
}

#[test]
fn removed_checkout_is_unavailable_and_local_surfaces_continue() {
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    let candidate = stage_candidate(workspace.path(), "adopted-clean.sh", true);
    let selected = run(&[
        "governance",
        "use",
        "workspace-governance",
        project.path().to_str().unwrap(),
        "--workspace-root",
        workspace.path().to_str().unwrap(),
    ]);
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));

    // The sibling checkout is removed after selection.
    fs::remove_file(&candidate).unwrap();
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        status.status.success(),
        "status must not crash: {}",
        lossy(&status.stderr)
    );
    let observation = &governance_json(&status)["observation"];
    assert_eq!(observation["status"], "unavailable");
    assert_eq!(observation["provider"], "workspace-governance");
    let detail = observation["detail"].as_str().unwrap();
    assert!(!detail.is_empty(), "unavailability must name a cause");
    assert!(detail.chars().count() <= 2_000, "detail must stay bounded");

    // Local commands continue unchanged: list, doctor-style inspect on a
    // different provider, and switching back to local all still work.
    let list = run(&[
        "governance",
        "list",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(list.status.success());
    let back = run(&[
        "governance",
        "use",
        "local",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(back.status.success(), "{}", lossy(&back.stderr));
    let status = run(&[
        "governance",
        "status",
        project.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(governance_json(&status)["observation"]["provider"], "local");
    assert_eq!(governance_json(&status)["observation"]["status"], "pass");
}

#[test]
fn selection_and_status_never_touch_the_registry_database() {
    // The registry file (if any exists) must be byte-identical across a
    // full preset use + status cycle, because governance state lives only
    // under .forge/.
    let project = project_dir();
    let workspace = TempDir::new().unwrap();
    stage_candidate(workspace.path(), "error-findings.sh", true);
    let registry = project.path().join("isolation.db");
    let init = Command::new(forge_bin())
        .env("FORGE_REGISTRY", &registry)
        .args(["list", "--format", "json"])
        .output()
        .expect("run forge list");
    assert!(init.status.success(), "{}", lossy(&init.stderr));
    let db_before = fs::read(&registry).unwrap();

    let mut cmd = clean_cmd();
    cmd.env("FORGE_REGISTRY", &registry);
    let selected = cmd
        .args([
            "governance",
            "use",
            "workspace-governance",
            project.path().to_str().unwrap(),
            "--workspace-root",
            workspace.path().to_str().unwrap(),
        ])
        .output()
        .expect("run governance use");
    assert!(selected.status.success(), "{}", lossy(&selected.stderr));
    let mut cmd = clean_cmd();
    cmd.env("FORGE_REGISTRY", &registry);
    let status = cmd
        .args([
            "governance",
            "status",
            project.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run governance status");
    assert!(status.status.success(), "{}", lossy(&status.stderr));
    assert_eq!(governance_json(&status)["observation"]["status"], "fail");

    assert_eq!(
        fs::read(&registry).unwrap(),
        db_before,
        "governance must not mutate the registry"
    );
}

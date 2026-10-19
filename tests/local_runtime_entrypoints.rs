#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn executable(dir: &Path, name: &str, script: &str) {
    let path = dir.join(name);
    fs::write(&path, script).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn invoke(root: &Path, path: &Path, log: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .current_dir(root)
        .env("PATH", path)
        .env("FORGE_TEST_LOG", log)
        .args(args)
        .output()
        .unwrap()
}

fn recorded(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn compose_defaults_to_up_and_forwards_profile_and_service_selection() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("project with spaces");
    let bin = temp.path().join("bin");
    fs::create_dir_all(root.join("deploy")).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(root.join("deploy/compose.yaml"), "services: {}\n").unwrap();
    let log = temp.path().join("argv.log");
    executable(
        &bin,
        "docker",
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$FORGE_TEST_LOG\"\nprintf 'compose-output\\n'\nexit 7\n",
    );

    let output = invoke(
        temp.path(),
        &bin,
        &log,
        &[
            "run",
            "compose",
            "--project-root",
            root.to_str().unwrap(),
            "--file",
            "deploy/compose.yaml",
            "--profile",
            "api",
            "--profile",
            "web",
            "--",
            "up",
            "--build",
            "api",
            "web",
        ],
    );

    assert_eq!(output.status.code(), Some(7));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "compose-output\n");
    assert_eq!(
        recorded(&log),
        vec![
            root.to_string_lossy(),
            "compose".into(),
            "--project-directory".into(),
            root.to_string_lossy(),
            "-f".into(),
            "deploy/compose.yaml".into(),
            "--profile".into(),
            "api".into(),
            "--profile".into(),
            "web".into(),
            "up".into(),
            "--build".into(),
            "api".into(),
            "web".into(),
        ]
    );
}

#[test]
fn compose_without_action_defaults_to_up() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(root.join("compose.yaml"), "services: {}\n").unwrap();
    let log = temp.path().join("argv.log");
    executable(
        &bin,
        "docker",
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FORGE_TEST_LOG\"\nexit 0\n",
    );

    let output = invoke(
        temp.path(),
        &bin,
        &log,
        &[
            "run",
            "compose",
            "--project-root",
            root.to_str().unwrap(),
            "--",
            "--build",
        ],
    );

    assert!(output.status.success());
    assert!(recorded(&log).ends_with(&["up".into(), "--build".into()]));
}

#[test]
fn local_uses_installed_gitrunbykaru_and_propagates_child_status() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("local app");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&bin).unwrap();
    let log = temp.path().join("argv.log");
    executable(
        &bin,
        "gitrunbykaru",
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$FORGE_TEST_LOG\"\nprintf 'local-output\\n'\nexit 9\n",
    );

    let output = invoke(
        temp.path(),
        &bin,
        &log,
        &[
            "run",
            "local",
            root.to_str().unwrap(),
            "--port",
            "4180",
            "--no-open",
            "--keep",
            "--docker",
            "--json",
        ],
    );

    assert_eq!(output.status.code(), Some(9));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "local-output\n");
    assert_eq!(
        recorded(&log),
        vec![
            root.to_string_lossy(),
            root.to_string_lossy(),
            "--port".into(),
            "4180".into(),
            "--no-open".into(),
            "--keep".into(),
            "--docker".into(),
            "--json".into(),
        ]
    );
}

#[test]
fn invalid_compose_file_and_missing_local_runtime_do_not_spawn() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    let bin = temp.path().join("empty-bin");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&bin).unwrap();
    let log = temp.path().join("should-not-exist");

    let compose = invoke(
        temp.path(),
        &bin,
        &log,
        &[
            "run",
            "compose",
            "--project-root",
            project.to_str().unwrap(),
            "--file",
            "missing.yaml",
        ],
    );
    assert_ne!(compose.status.code(), Some(0));
    assert!(!log.exists());

    let local = invoke(&project, &bin, &log, &["run", "local", "."]);
    assert_ne!(local.status.code(), Some(0));
    assert!(!log.exists());
    assert!(String::from_utf8_lossy(&local.stderr).contains("gitrunbykaru"));
}

#[test]
fn missing_docker_and_invalid_local_path_refuse_without_package_install() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    let bin = temp.path().join("empty-bin");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(project.join("compose.yaml"), "services: {}\n").unwrap();
    let log = temp.path().join("should-not-exist");

    let compose = invoke(
        temp.path(),
        &bin,
        &log,
        &[
            "run",
            "compose",
            "--project-root",
            project.to_str().unwrap(),
        ],
    );
    assert_eq!(compose.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&compose.stderr).contains("Docker Compose is unavailable"));
    assert!(!String::from_utf8_lossy(&compose.stderr).contains("npx"));
    assert!(!log.exists());

    let local = invoke(
        temp.path(),
        &bin,
        &log,
        &["run", "local", "missing-directory"],
    );
    assert_eq!(local.status.code(), Some(2));
    assert!(!log.exists());
}

#[test]
fn signal_terminated_child_returns_stable_failure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(root.join("compose.yaml"), "services: {}\n").unwrap();
    executable(&bin, "docker", "#!/bin/sh\nkill -TERM $$\n");

    let output = invoke(
        temp.path(),
        &bin,
        &temp.path().join("unused-log"),
        &["run", "compose", "--project-root", root.to_str().unwrap()],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("terminated without an exit code"));
}

#[test]
fn spawn_failure_returns_127_with_program_diagnostic() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("project");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&bin).unwrap();
    fs::write(root.join("compose.yaml"), "services: {}\n").unwrap();
    let docker = bin.join("docker");
    fs::write(&docker, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&docker, fs::Permissions::from_mode(0o644)).unwrap();

    let output = invoke(
        temp.path(),
        &bin,
        &temp.path().join("unused-log"),
        &["run", "compose", "--project-root", root.to_str().unwrap()],
    );

    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to start `docker compose`"));
}

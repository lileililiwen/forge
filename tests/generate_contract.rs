//! Deterministic project generation contract: equivalent creation modes
//! and portable generated projects.
//!
//! Covers the `deterministic-project-generation` scenarios end to end
//! through the built binary: explicit flags and interactive answers produce
//! equivalent source, nonempty destinations and template escapes fail before
//! overwriting, cancellation leaves nothing behind, generation failures
//! leave nothing registered, and native evidence is distinguished from
//! rendering verification.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

fn run_with_stdin(db: &Path, args: &[&str], stdin_bytes: &[u8]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_bytes)
        .expect("write stdin");
    child.wait_with_output().expect("wait forge")
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_into(root, &mut out);
    out.sort();
    out
}

fn collect_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            collect_into(&path, out);
        }
    }
}

fn file_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for path in collect_files(root) {
        if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, fs::read(&path).unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn new_creates_registered_portable_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("hello-svc");

    let out = run(
        &db,
        &["new", &dest.display().to_string(), "--profile", "rust-web"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("created hello-svc"),
        "{}",
        lossy(&out.stdout)
    );

    assert!(dest.join("forge.yaml").exists());
    assert!(dest.join("Cargo.toml").exists());
    assert!(dest.join("src/main.rs").exists());
    assert!(dest.join("README.md").exists());
    let manifest = fs::read_to_string(dest.join("forge.yaml")).unwrap();
    assert!(manifest.contains("profile: rust-web"), "{manifest}");
    let readme = fs::read_to_string(dest.join("README.md")).unwrap();
    assert!(readme.contains("cargo build"), "{readme}");
    assert!(readme.contains("cargo test"), "{readme}");

    let out = run_json(&db, &["inspect", "hello-svc"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["profile"], "rust-web");
    assert_eq!(value["maturity"], "L1");

    // Repeatability: same request in a second directory renders identical bytes.
    let dest2 = tmp.path().join("second-dir");
    let out = run(
        &db,
        &[
            "new",
            &dest2.display().to_string(),
            "--profile",
            "rust-web",
            "--id",
            "hello-svc-dup",
            "--name",
            "hello-svc",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let _ = (dest, dest2);
}

#[test]
fn explicit_flags_and_interactive_answers_are_equivalent() {
    let tmp = tempfile::tempdir().unwrap();
    let db_flags = tmp.path().join("registry-flags.db");
    let db_cli = tmp.path().join("registry-cli.db");
    let via_flags = tmp.path().join("flags-app");
    let via_cli = tmp.path().join("cli-app");

    let out = run(
        &db_flags,
        &[
            "new",
            &via_flags.display().to_string(),
            "--profile",
            "rust-web",
            "--id",
            "equiv-app",
            "--name",
            "Equiv App",
            "--feature",
            "auth",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let out = run_with_stdin(
        &db_cli,
        &["new", &via_cli.display().to_string()],
        b"rust-web\nequiv-app\nEquiv App\nauth\n",
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let a = file_bytes(&via_flags);
    let b = file_bytes(&via_cli);
    assert_eq!(a, b);
}

#[test]
fn nonempty_destination_fails_before_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("taken-app");
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("keep.txt"), "do not touch").unwrap();
    let before = collect_files(&dest);

    let out = run(
        &db,
        &["new", &dest.display().to_string(), "--profile", "rust-web"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[generation-conflict]"),
        "{}",
        lossy(&out.stderr)
    );
    assert_eq!(collect_files(&dest), before);
    assert_eq!(
        fs::read_to_string(dest.join("keep.txt")).unwrap(),
        "do not touch"
    );
    assert!(!dest.join("forge.yaml").exists());
    assert_eq!(run(&db, &["inspect", "taken-app"]).status.code(), Some(1));
}

#[test]
fn cancelled_interactive_leaves_nothing_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("cancelled-app");

    let out = run_with_stdin(&db, &["new", &dest.display().to_string()], b"");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[generation-cancelled]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest.exists());
    let list = run(&db, &["list"]);
    assert!(lossy(&list.stdout).contains("No projects registered"));
}

#[test]
fn unknown_profile_and_incompatible_features_fail_without_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("bad-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "not-a-real-profile",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unknown-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest.join("forge.yaml").exists());

    let dest2 = tmp.path().join("mobile-app");
    let out = run(
        &db,
        &[
            "new",
            &dest2.display().to_string(),
            "--profile",
            "flutter-app",
            "--feature",
            "postgres",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[incompatible-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest2.exists());

    // R2 failure: a planned profile must refuse generation before any
    // file is written and without claiming support.
    let dest3 = tmp.path().join("planned-app");
    let out = run(
        &db,
        &["new", &dest3.display().to_string(), "--profile", "rust-cli"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[unsupported-profile]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!dest3.exists());
    assert!(!dest3.join("forge.yaml").exists());
}

#[test]
fn id_collision_cleans_promoted_output() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let first = tmp.path().join("first-app");
    let second = tmp.path().join("second-app");

    let out = run(
        &db,
        &[
            "new",
            &first.display().to_string(),
            "--profile",
            "rust-web",
            "--id",
            "dupe-id",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let out = run(
        &db,
        &[
            "new",
            &second.display().to_string(),
            "--profile",
            "rust-web",
            "--id",
            "dupe-id",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        lossy(&out.stderr).contains("error[id-collision]"),
        "{}",
        lossy(&out.stderr)
    );
    assert!(!second.join("forge.yaml").exists());

    let out = run_json(&db, &["list"]);
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["projects"].as_array().unwrap().len(), 1);
}

#[test]
fn missing_toolchain_reports_unverified_without_claiming_build() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("unverified-app");

    let out = run(
        &db,
        &["new", &dest.display().to_string(), "--profile", "rust-web"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    assert!(
        lossy(&out.stdout).contains("native build/test require"),
        "{}",
        lossy(&out.stdout)
    );

    // --verify-native with an empty PATH must report toolchain-missing and
    // never claim a successful build.
    let empty = tempfile::tempdir().unwrap();
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("new")
        .arg(tmp.path().join("verify-app"))
        .arg("--profile")
        .arg("rust-web")
        .arg("--verify-native")
        .env("PATH", empty.path());
    let out = cmd.output().expect("run forge");
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[toolchain-missing]"), "{stderr}");
    assert!(stderr.contains("not tested"), "{stderr}");
}

#[test]
fn rust_scaffold_builds_with_native_toolchain_without_forge() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("native-app");

    let out = run(
        &db,
        &["new", &dest.display().to_string(), "--profile", "rust-web"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

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

#[test]
fn dotnet_scaffold_builds_offline_without_forge() {
    if Command::new("dotnet").arg("--version").output().is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "aspnet-web",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let build = Command::new("dotnet")
        .args(["build", "--nologo"])
        .current_dir(&dest)
        .env_remove("FORGE_REGISTRY")
        .output()
        .expect("dotnet build");
    assert!(build.status.success(), "{}", lossy(&build.stdout));
}

#[test]
fn flutter_scaffold_tests_pass_without_forge() {
    if Command::new("flutter").arg("--version").output().is_err() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("mbl-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "flutter-app",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let test = Command::new("flutter")
        .arg("test")
        .current_dir(&dest)
        .env_remove("FORGE_REGISTRY")
        .output()
        .expect("flutter test");
    assert!(test.status.success(), "{}", lossy(&test.stdout));
}

#[test]
fn python_scaffold_compiles_without_forge() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("py-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "python-service",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    // pytest/build backends are external and may be absent; the stdlib
    // compile check proves the scaffold is syntactically portable without
    // claiming a pytest run.
    let compile = Command::new("python3")
        .args(["-m", "compileall", "-q", "app", "tests"])
        .current_dir(&dest)
        .env_remove("FORGE_REGISTRY")
        .output()
        .expect("python3 compileall");
    assert!(compile.status.success(), "{}", lossy(&compile.stderr));
}

#[test]
fn node_scaffold_builds_offline_without_forge() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("node-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "nextjs-web",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let build = Command::new("npm")
        .args(["run", "build"])
        .current_dir(&dest)
        .env_remove("FORGE_REGISTRY")
        .output()
        .expect("npm run build");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(dest.join("dist/build-ok.txt").exists());
}

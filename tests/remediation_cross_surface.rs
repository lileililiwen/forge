use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn project(tmp: &Path) -> PathBuf {
    let target = tmp.join("demo");
    fs::create_dir_all(target.join(".standard")).unwrap();
    fs::write(
        target.join("forge.yaml"),
        "schema: 1\nproject:\n  id: demo\n  name: Demo\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(target.join("owned.txt"), "keep me\n").unwrap();
    target
}

fn run(target: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env(
            "FORGE_REGISTRY",
            target.parent().unwrap().join("registry.db"),
        )
        .args(args)
        .arg("--target")
        .arg(target);
    cmd.output().unwrap()
}

#[test]
fn preview_does_not_create_registry_or_change_unrelated_files() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let before = fs::read(target.join("owned.txt")).unwrap();
    let registry = tmp.path().join("registry.db");
    let out = run(
        &target,
        &[
            "remediate",
            "diff",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(target.join("owned.txt")).unwrap(), before);
    assert!(!registry.exists());
    assert!(!target.join(".standard/ci/verify.yml").exists());
}

#[test]
fn unowned_existing_asset_is_a_conflict_and_preserves_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let asset = target.join(".standard/ci/verify.yml");
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    fs::write(&asset, "user-owned\n").unwrap();
    let out = run(
        &target,
        &[
            "remediate",
            "apply",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
            "--confirm",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-conflict"));
    assert_eq!(fs::read(&asset).unwrap(), b"user-owned\n");
}

#[cfg(unix)]
#[test]
fn symlinked_standard_directory_cannot_escape_target() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let target = project(tmp.path());
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, target.join(".standard/ci")).unwrap();
    let out = run(
        &target,
        &[
            "remediate",
            "plan",
            "--finding",
            "gaps.ci.demo.ci",
            "--pack",
            "baseline-service@1.1.0",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("remediation-conflict"));
    assert!(!outside.join("verify.yml").exists());
}

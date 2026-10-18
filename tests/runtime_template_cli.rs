use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};

fn forge() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn bundle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/governance/runtime-templates")
}

fn copy_tree(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
fn forge_command_uses_the_bundled_renderer_and_preserves_output() {
    let output = Command::new(forge())
        .args(["runtime-template", "list"])
        .output()
        .expect("forge runtime-template starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let registry: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(registry.as_object().unwrap().len(), 10);
    assert_eq!(registry["rust-web-postgres"]["version"], 1);
}

#[test]
fn invalid_explicit_bundle_fails_without_falling_back_to_source() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(forge())
        .args(["runtime-template", "list"])
        .env("FORGE_RUNTIME_TEMPLATES_HOME", temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("FORGE_RUNTIME_TEMPLATES_HOME"), "{error}");
}

#[test]
fn explicit_bundle_override_is_supported() {
    let output = Command::new(forge())
        .args(["runtime-template", "list"])
        .env("FORGE_RUNTIME_TEMPLATES_HOME", bundle())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn installed_prefix_layout_resolves_without_source_or_override() {
    let prefix = tempfile::tempdir().unwrap();
    let bin_dir = prefix.path().join("bin");
    let resource_dir = prefix.path().join("share/forge/runtime-templates");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::create_dir_all(&resource_dir).unwrap();
    let installed_forge = bin_dir.join("forge");
    std::fs::copy(forge(), &installed_forge).unwrap();
    let source = bundle();
    copy_tree(&source, &resource_dir);
    let output = Command::new(installed_forge)
        .args(["runtime-template", "list"])
        .env_remove("FORGE_RUNTIME_TEMPLATES_HOME")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn renderer_refusal_exit_is_forwarded() {
    let output = Command::new(forge())
        .args(["runtime-template", "not-a-renderer-operation"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid choice"));
}

#[test]
fn missing_python_is_reported_without_claiming_success() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(forge())
        .args(["runtime-template", "list"])
        .env("PATH", temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot run python3"));
}

#[test]
fn owned_bundle_manifest_pins_every_renderer_and_template_asset() {
    let root = bundle();
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).expect("Forge-owned bundle manifest exists"),
    )
    .unwrap();
    assert_eq!(manifest["schema_version"], 1);
    let entries = manifest["files"].as_array().unwrap();
    let mut expected = std::collections::BTreeSet::new();
    for entry in entries {
        let relative = entry["path"].as_str().unwrap();
        assert!(
            expected.insert(relative.to_string()),
            "duplicate path {relative}"
        );
        let path = root.join(relative);
        let digest = format!("sha256:{:x}", Sha256::digest(std::fs::read(&path).unwrap()));
        assert_eq!(entry["sha256"], digest, "digest mismatch for {relative}");
    }
    fn collect(
        root: &std::path::Path,
        directory: &std::path::Path,
        files: &mut std::collections::BTreeSet<String>,
    ) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                collect(root, &path, files);
            } else if path.file_name().is_some_and(|name| name != "manifest.json") {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut actual = std::collections::BTreeSet::new();
    collect(&root, &root, &mut actual);
    assert_eq!(actual, expected, "bundle manifest and owned files differ");
}

#[test]
fn inspect_render_and_check_work_through_forge() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join(".project.json"),
        r#"{"id":"template-smoke","deployment":{"deployable":false}}"#,
    )
    .unwrap();
    std::fs::write(project.path().join("index.html"), "<h1>demo</h1>\n").unwrap();

    let inspect = Command::new(forge())
        .args(["runtime-template", "inspect", "--root"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(inspect.status.success());
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("static-site"));

    let render = Command::new(forge())
        .args(["runtime-template", "render", "--root"])
        .arg(project.path())
        .args([
            "--type",
            "static-site",
            "--var",
            "project_name=template-smoke",
            "--var",
            "port=8080",
            "--var",
            "health_path=/",
            "--var",
            "output_dir=dist",
        ])
        .output()
        .unwrap();
    assert!(
        render.status.success(),
        "{}",
        String::from_utf8_lossy(&render.stderr)
    );

    let check = Command::new(forge())
        .args(["runtime-template", "check", "--root"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(project.path().join("compose.yaml").is_file());
    let declaration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(project.path().join(".project.json")).unwrap())
            .unwrap();
    assert_eq!(declaration["deployment"]["template_type"], "static-site");
    assert_eq!(declaration["deployment"]["deployable"], false);
}

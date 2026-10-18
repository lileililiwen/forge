use std::path::PathBuf;
use std::process::Command;

fn forge() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

#[test]
fn concern_command_forwards_a_real_checker_pass_result() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("app.py"), "print('hello')\n").unwrap();
    let output = Command::new(forge())
        .args([
            "concern",
            "check",
            "privacy",
            project.path().to_str().unwrap(),
        ])
        .output()
        .expect("forge concern check starts");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["concern"], "privacy");
    assert_eq!(result["status"], "PASS");
}

#[test]
fn concern_command_preserves_the_checker_finding_exit_and_json() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("app.py"),
        "email = \"alice@example.com\"\n",
    )
    .unwrap();

    let output = Command::new(forge())
        .args([
            "concern",
            "check",
            "privacy",
            project.path().to_str().unwrap(),
        ])
        .output()
        .expect("forge concern check starts");

    assert_eq!(output.status.code(), Some(1));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "FAIL");
    assert_eq!(result["summary"]["errors"], 1);
    assert_eq!(result["findings"][0]["file"], "app.py");
}

#[test]
fn concern_command_routes_product_quality_ids_through_forge() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join(".project.json"),
        r#"{"quality":{"product_roots":["src"],"languages":["rust"],"placeholder_threshold":0}}"#,
    )
    .unwrap();

    let output = Command::new(forge())
        .args([
            "concern",
            "check",
            "product-code-boundary",
            project.path().to_str().unwrap(),
        ])
        .output()
        .expect("forge concern check starts");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], "PASS");
}

#[test]
fn concern_command_rejects_unknown_ids_and_invalid_roots_before_checking() {
    let unknown = Command::new(forge())
        .args(["concern", "check", "governance", "."])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("invalid value"));

    let invalid = Command::new(forge())
        .args(["concern", "check", "privacy", "/path/that/does/not/exist"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("project root"));
}

#[test]
fn concern_command_accepts_every_generic_concern_id() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("app.py"), "print('hello')\n").unwrap();
    for id in [
        "accessibility",
        "fixtures",
        "privacy",
        "provenance",
        "schemas",
    ] {
        let output = Command::new(forge())
            .args(["concern", "check", id, project.path().to_str().unwrap()])
            .output()
            .expect("forge concern check starts");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("{id} emitted invalid JSON: {error}"));
        assert_eq!(result["concern"], id);
    }
}

#[test]
fn installed_binary_uses_its_forge_governance_resource() {
    let prefix = tempfile::tempdir().unwrap();
    let bin_dir = prefix.path().join("bin");
    let resource_dir = prefix.path().join("share/forge/governance");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::create_dir_all(&resource_dir).unwrap();
    let installed_forge = bin_dir.join("forge");
    std::fs::copy(forge(), &installed_forge).unwrap();
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("modules/workspace-governance/scripts/check_concern.py"),
        resource_dir.join("check_concern.py"),
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("app.py"), "print('hello')\n").unwrap();

    let output = Command::new(installed_forge)
        .args([
            "concern",
            "check",
            "privacy",
            project.path().to_str().unwrap(),
        ])
        .env_remove("FORGE_RUNTIME_TEMPLATES_HOME")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "PASS");
}

#[test]
fn installed_binary_uses_forge_quality_resources() {
    let prefix = tempfile::tempdir().unwrap();
    let bin_dir = prefix.path().join("bin");
    let governance_dir = prefix.path().join("share/forge/governance");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::create_dir_all(&governance_dir).unwrap();
    let installed_forge = bin_dir.join("forge");
    std::fs::copy(forge(), &installed_forge).unwrap();
    let module_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules/workspace-governance");
    std::fs::copy(
        module_root.join("scripts/product_code_quality.py"),
        governance_dir.join("product_code_quality.py"),
    )
    .unwrap();
    std::fs::copy(
        module_root.join("scripts/vocabulary.py"),
        governance_dir.join("vocabulary.py"),
    )
    .unwrap();
    std::fs::copy(
        module_root.join("vocabulary.json"),
        prefix.path().join("share/forge/vocabulary.json"),
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(
        project.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join(".project.json"),
        r#"{"quality":{"product_roots":["src"],"languages":["rust"],"placeholder_threshold":0}}"#,
    )
    .unwrap();

    let output = Command::new(installed_forge)
        .args([
            "concern",
            "check",
            "product-code-boundary",
            project.path().to_str().unwrap(),
        ])
        .env_remove("FORGE_RUNTIME_TEMPLATES_HOME")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["status"], "PASS");
}

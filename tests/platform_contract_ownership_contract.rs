//! CLI contract for Forge-owned platform schema support and typed refusal.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn forge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forge"))
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let dest = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[test]
fn contract_list_reports_the_forge_owned_support_boundary() {
    let output = forge()
        .args(["contract", "list", "--format", "json"])
        .output()
        .expect("forge contract list runs");
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(value["manifest"]["source"], "forge");
    let families = value["platform_families"].as_array().expect("families");
    assert_eq!(families.len(), 10);
    for family in [
        "platform.identity-subject",
        "platform.permission-decision",
        "platform.tenant-context",
        "platform.job-outcome",
    ] {
        let row = families.iter().find(|row| row["family"] == family).unwrap();
        assert_eq!(row["status"], "unsupported");
        assert!(row["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()));
    }
}

#[test]
fn unsupported_family_refuses_inspect_emit_and_validate() {
    let inspect = forge()
        .args(["contract", "inspect", "platform.job-outcome"])
        .output()
        .expect("inspect runs");
    assert!(!inspect.status.success());
    let inspect_error = String::from_utf8_lossy(&inspect.stderr);
    assert!(
        inspect_error.contains("contract-unsupported-family"),
        "{inspect_error}"
    );

    let emit = forge()
        .args([
            "contract",
            "emit",
            "platform.job-outcome",
            "/does-not-exist",
        ])
        .output()
        .expect("emit runs");
    assert!(!emit.status.success());
    let emit_error = String::from_utf8_lossy(&emit.stderr);
    assert!(
        emit_error.contains("contract-unsupported-family"),
        "{emit_error}"
    );

    let document = r#"{"contract":"platform.job-outcome/0.1.0","generated_at":"2026-10-19T00:00:00Z","payload":{}}"#;
    let mut validate = forge();
    validate.args(["contract", "validate", "-"]);
    validate.stdout(std::process::Stdio::piped());
    validate.stderr(std::process::Stdio::piped());
    validate.stdin(std::process::Stdio::piped());
    let mut child = validate.spawn().expect("validate spawns");
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(document.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let error = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(error.contains("contract-unsupported-family"), "{error}");
}

#[test]
fn contract_asset_override_fails_on_tampered_missing_and_unlisted_files() {
    let temp = tempfile::tempdir().unwrap();
    let contracts = temp.path().join("contracts");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts"),
        &contracts,
    );

    let run_list = || {
        forge()
            .args(["contract", "list", "--format", "json"])
            .env("FORGE_CONTRACTS_DIR", &contracts)
            .output()
            .unwrap()
    };
    let healthy = run_list();
    assert!(healthy.status.success());

    let schema = contracts.join("schemas/gate-result.schema.json");
    let original = std::fs::read(&schema).unwrap();
    std::fs::write(&schema, [original.as_slice(), b" "].concat()).unwrap();
    let tampered = run_list();
    assert!(!tampered.status.success());
    let tampered_error = format!(
        "{}{}",
        String::from_utf8_lossy(&tampered.stdout),
        String::from_utf8_lossy(&tampered.stderr)
    );
    assert!(
        tampered_error.contains("schemas/gate-result.schema.json"),
        "{tampered_error}"
    );
    std::fs::write(&schema, original).unwrap();

    std::fs::remove_file(contracts.join("schemas/readiness.schema.json")).unwrap();
    let missing = run_list();
    assert!(!missing.status.success());
    let missing_error = format!(
        "{}{}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(
        missing_error.contains("schemas/readiness.schema.json"),
        "{missing_error}"
    );

    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts/schemas/readiness.schema.json"),
        contracts.join("schemas/readiness.schema.json"),
    )
    .unwrap();
    std::fs::write(contracts.join("unexpected.json"), "{}\n").unwrap();
    let unlisted = run_list();
    assert!(!unlisted.status.success());
    let unlisted_error = format!(
        "{}{}",
        String::from_utf8_lossy(&unlisted.stdout),
        String::from_utf8_lossy(&unlisted.stderr)
    );
    assert!(
        unlisted_error.contains("unexpected.json"),
        "{unlisted_error}"
    );
}

#[test]
fn malformed_duplicate_and_unsafe_manifest_entries_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let contracts = temp.path().join("contracts");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts"),
        &contracts,
    );
    let manifest_path = contracts.join("manifest.json");
    let original = std::fs::read(&manifest_path).unwrap();
    let run_list = || {
        forge()
            .args(["contract", "list", "--format", "json"])
            .env("FORGE_CONTRACTS_DIR", &contracts)
            .output()
            .unwrap()
    };

    std::fs::write(&manifest_path, b"{").unwrap();
    let malformed = run_list();
    assert!(!malformed.status.success());
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("contracts/manifest.json invalid"));
    std::fs::write(&manifest_path, &original).unwrap();

    let mut manifest: Value = serde_json::from_slice(&original).unwrap();
    let first = manifest["files"][0].clone();
    manifest["files"].as_array_mut().unwrap().push(first);
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let duplicate = run_list();
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("duplicate contract asset path"));
    std::fs::write(&manifest_path, &original).unwrap();

    let mut manifest: Value = serde_json::from_slice(&original).unwrap();
    manifest["files"][0]["path"] = Value::String("../outside.json".to_string());
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let unsafe_path = run_list();
    assert!(!unsafe_path.status.success());
    assert!(String::from_utf8_lossy(&unsafe_path.stderr).contains("unsafe contract asset path"));
}

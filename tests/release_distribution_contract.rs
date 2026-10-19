//! Contracts for the tagged release workflow and its immutable identity.

use std::path::Path;
use std::process::Command;

const WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");

#[test]
fn release_job_waits_for_every_ci_job_and_only_writes_on_version_tags() {
    let release = WORKFLOW
        .split_once("\n  release:\n")
        .map(|(_, release)| release)
        .expect("release job exists");
    assert!(release
        .contains("if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')"));
    assert!(release.contains("permissions:\n      contents: write"));
    assert!(release.contains("actions/download-artifact@v4"));
    assert!(release.contains("gh release create --repo lileililiwen/forge \"$RELEASE_VERSION\""));
    assert!(release.contains("--verify-tag"));
    assert!(!release.contains("--clobber"));
    assert!(!release.contains("gh release delete"));
    for required_job in [
        "fmt",
        "clippy",
        "test",
        "native-scaffold",
        "msrv",
        "deny",
        "governance",
        "readiness",
        "surfaces",
        "gate",
        "artifact",
    ] {
        assert!(
            release.contains(&format!("      - {required_job}\n")),
            "missing {required_job} dependency:\n{release}"
        );
    }
    assert!(WORKFLOW.contains("contents: read"));
}

#[test]
fn artifact_job_checks_version_before_package_and_uploads_exact_archive_pair() {
    let artifact = WORKFLOW
        .split_once("\n  artifact:\n")
        .and_then(|(_, tail)| {
            tail.split_once("\n  release:\n")
                .map(|(artifact, _)| artifact)
        })
        .expect("artifact job exists before release job");
    assert!(artifact.contains("scripts/validate-release-tag.sh \"$GITHUB_REF_NAME\""));
    assert!(artifact.contains("scripts/package.sh"));
    assert!(artifact.contains("scripts/checksum.sh --verify"));
    assert!(artifact.contains("scripts/smoke.sh"));
    assert!(artifact.contains("name: forge-package"));
    assert!(artifact.contains("dist/forge-*.tar.gz.sha256"));
}

#[test]
fn release_tag_guard_matches_cargo_version_and_rejects_mismatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let version = env!("CARGO_PKG_VERSION");
    let script = root.join("scripts/validate-release-tag.sh");
    let matching = Command::new("sh")
        .arg(&script)
        .arg(format!("v{version}"))
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        matching.status.success(),
        "{}",
        String::from_utf8_lossy(&matching.stderr)
    );

    let mismatch = Command::new("sh")
        .arg(&script)
        .arg("v0.0.0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(!mismatch.status.success());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("does not match"));
}

#[test]
fn hosted_and_local_contract_checks_are_offline_and_required() {
    assert!(!WORKFLOW.contains("contract-parity"));
    assert!(!WORKFLOW.contains("platform-contracts.git"));
    assert!(WORKFLOW.contains("pip install -r requirements-dev.txt"));
    assert!(WORKFLOW.contains("node scripts/check-contracts.mjs"));
    assert!(WORKFLOW.contains("python3 tests/contract_schema_tests.py"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = std::fs::read_to_string(root.join("scripts/release-check.sh")).unwrap();
    assert!(release.contains("node scripts/check-contracts.mjs"));
    assert!(release.contains("python3 tests/contract_schema_tests.py"));
    assert!(!release.contains("PLATFORM_CONTRACTS_DIR"));
    assert!(!release.contains("../platform-contracts"));
}

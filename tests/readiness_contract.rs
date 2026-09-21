//! CLI contract for `forge readiness` (`profile-and-release-readiness`).
//!
//! Exercises native profile evidence and the release gate end to end
//! through the built binary:
//!
//! - R1 success: `matrix --profile rust-web` renders a `passed` row with
//!   toolchain version, commands, source hash and `forge_absent_from_path`
//!   (skipped where `cargo` is unavailable: a missing toolchain is
//!   `unverified`, never a test pass).
//! - R1 failure: an unknown id is refused with `error[unknown-profile]`
//!   and a planned id with `error[unsupported-profile]`, both before any
//!   fixture is generated.
//! - R1 boundary: duplicate `--profile` flags collapse to one row.
//! - R2 success: `artifact` reports the binary path, SHA-256, crate
//!   version and a non-empty `forge --version` smoke line.
//! - R2 gate: `check` exits 0 with `gate ready=true` when the selected row
//!   passes, and exits 1 with `error[readiness-not-ready]` on stderr while
//!   keeping the full gate report on stdout when the selected row does not
//!   pass (asserted structurally so the test holds on any host).

use std::path::PathBuf;
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN");
    cmd
}

fn run(args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("readiness CLI must run")
}

fn binary_on_path(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if dir.join(name).is_file() {
            return true;
        }
    }
    false
}

#[test]
fn readiness_help_lists_matrix_artifact_and_check() {
    let out = run(&["readiness", "--help"]);
    assert!(out.status.success());
    let text = lossy(&out.stdout);
    assert!(text.contains("matrix"), "{text}");
    assert!(text.contains("artifact"), "{text}");
    assert!(text.contains("check"), "{text}");
    let top = run(&["--help"]);
    assert!(top.status.success());
    assert!(lossy(&top.stdout).contains("readiness"));
}

#[test]
fn matrix_refuses_unknown_profile_before_any_fixture() {
    let out = run(&["readiness", "matrix", "--profile", "nosuch"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[unknown-profile]"), "{stderr}");
}

#[test]
fn matrix_refuses_planned_profile_before_any_fixture() {
    let out = run(&["readiness", "matrix", "--profile", "rust-cli"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[unsupported-profile]"), "{stderr}");
}

#[test]
fn matrix_reports_passed_rust_web_row_with_full_evidence() {
    if !binary_on_path("cargo") {
        return;
    }
    let out = run(&[
        "readiness",
        "matrix",
        "--profile",
        "rust-web",
        "--format",
        "json",
    ]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("matrix JSON must parse");
    assert_eq!(value["contract"], "0.1.0");
    let rows = value["matrix"]["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["profile"], "rust-web");
    assert_eq!(row["result"], "passed");
    assert_eq!(row["build_command"], "cargo build");
    assert_eq!(row["test_command"], "cargo test");
    assert_eq!(row["forge_absent_from_path"], true);
    let hash = row["source_hash"].as_str().unwrap_or("");
    assert_eq!(hash.len(), 64, "source identity must be SHA-256");
    assert!(!row["generated_at"].as_str().unwrap_or("").is_empty());
    assert!(row["toolchain_version"]
        .as_str()
        .unwrap_or("")
        .contains("cargo"));
    assert_eq!(row["build"]["success"], true);
    assert_eq!(row["test"]["success"], true);
    assert_eq!(value["matrix"]["ready"], true);
}

#[test]
fn matrix_dedupes_repeated_profile_flags() {
    if !binary_on_path("cargo") {
        return;
    }
    let out = run(&[
        "readiness",
        "matrix",
        "--profile",
        "rust-web",
        "--profile",
        "rust-web",
        "--format",
        "json",
    ]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("matrix JSON must parse");
    assert_eq!(value["matrix"]["rows"].as_array().unwrap().len(), 1);
}

#[test]
fn artifact_reports_binary_path_checksum_version_and_smoke() {
    let out = run(&["readiness", "artifact", "--format", "json"]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("artifact JSON must parse");
    assert_eq!(value["contract"], "0.1.0");
    let artifact = &value["artifact"];
    assert!(!artifact["binary_path"].as_str().unwrap_or("").is_empty());
    let sha = artifact["sha256"].as_str().unwrap_or("");
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    let version = artifact["version"].as_str().unwrap_or("");
    assert!(!version.is_empty());
    let smoke = artifact["version_smoke"].as_str().unwrap_or("");
    assert!(smoke.contains(version), "{smoke}");
    let direct = run(&["--version"]);
    assert!(direct.status.success());
    assert!(lossy(&direct.stdout).trim() == smoke.trim());
}

#[test]
fn check_passes_for_a_passing_row() {
    if !binary_on_path("cargo") {
        return;
    }
    let out = run(&[
        "readiness",
        "check",
        "--profile",
        "rust-web",
        "--format",
        "json",
    ]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("gate JSON must parse");
    assert_eq!(value["gate"]["ready"], true);
    assert_eq!(value["gate"]["blocking"].as_array().unwrap().len(), 0);
    assert!(!value["gate"]["artifact"]["sha256"]
        .as_str()
        .unwrap_or("")
        .is_empty());
}

#[test]
fn check_blocks_with_report_on_stdout_and_typed_error_on_stderr() {
    // Structural assertion: the exit code always matches the row outcome,
    // so this holds whether or not the host provides the python toolchain.
    let out = run(&[
        "readiness",
        "check",
        "--profile",
        "python-service",
        "--format",
        "json",
    ]);
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("blocked gate keeps its report on stdout");
    let rows = value["gate"]["matrix"]["rows"]
        .as_array()
        .expect("rows array");
    assert_eq!(rows.len(), 1);
    let result = rows[0]["result"].as_str().unwrap_or("");
    assert!(
        result == "passed" || result == "failed" || result == "unverified",
        "{result}"
    );
    if result == "passed" {
        assert!(out.status.success());
        assert_eq!(value["gate"]["ready"], true);
    } else {
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(value["gate"]["ready"], false);
        let blocking = value["gate"]["blocking"].as_array().unwrap();
        assert!(!blocking.is_empty());
        assert!(blocking[0]
            .as_str()
            .unwrap_or("")
            .contains("python-service"));
        let stderr = lossy(&out.stderr);
        assert!(stderr.contains("readiness-not-ready"), "{stderr}");
        assert!(stderr.contains("python-service"), "{stderr}");
    }
}

#[test]
fn check_refuses_unknown_profile_selection() {
    let out = run(&["readiness", "check", "--profile", "nosuch"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("error[unknown-profile]"));
}

#[test]
fn human_matrix_names_profile_result_and_toolchain() {
    if !binary_on_path("cargo") {
        return;
    }
    let out = run(&["readiness", "matrix", "--profile", "rust-web"]);
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    let text = lossy(&out.stdout);
    assert!(text.contains("rust-web"), "{text}");
    assert!(text.contains("passed"), "{text}");
    assert!(text.contains("cargo"), "{text}");
    assert!(text.contains("contract 0.1.0"), "{text}");
}

//! Gate runtime evidence CLI contract (`gate-runtime-evidence`).
//!
//! Drives the real `forge` binary with `FORGE_GATE_BIN` pointed at stub
//! scripts emitting the verbatim sibling documents captured in
//! `tests/fixtures/gate/` (see `NOTES.md` there). Covers the normative
//! scenarios: a passing run persists revision-bound evidence, journals
//! `done` and exits zero; a blocked document (non-zero exit) is evidence
//! not adapter failure and mirrors the block in the exit code; an
//! unresolvable runtime lists its attempts while prior evidence stays
//! byte-identical; a rehearsal never persists or journals; and every
//! read surface agrees on the same aggregate, revision, runtime and
//! timestamp.
//!
//! The target was a single 1,157-line file; it is now a directory of focused
//! submodules, each owning one scenario family. The shared helpers below stay
//! reachable to every submodule through `super::`.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::registry::Registry;

mod blocked;
mod documented_help;
mod passing;
mod review_required;
mod unknown_runtime;

pub(crate) fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

pub(crate) fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate")
        .join(name)
}

pub(crate) fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

pub(crate) fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

pub(crate) fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

pub(crate) fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

pub(crate) fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {}", lossy(&out.stderr));
}

/// A git scratch project carrying the manifest and one commit, so the
/// evidence revision binding is a real HEAD.
pub(crate) fn project(tmp: &Path, id: &str) -> PathBuf {
    let proj = tmp.join(id);
    fs::create_dir_all(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest(id));
    write_file(&proj, "src/lib.rs", "// demo\n");
    git(&proj, &["init", "-q"]);
    git(&proj, &["config", "user.email", "forge@example.com"]);
    git(&proj, &["config", "user.name", "Forge Test"]);
    git(&proj, &["config", "commit.gpgsign", "false"]);
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "initial"]);
    proj
}

pub(crate) struct GateRun {
    pub(crate) status: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn gate(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

pub(crate) fn gate_human(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry").arg(db).arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

pub(crate) fn gate_json(run: &GateRun) -> serde_json::Value {
    assert!(
        !run.stdout.is_empty(),
        "empty stdout; stderr: {}",
        run.stderr
    );
    serde_json::from_str(&run.stdout).expect("gate json")
}

/// Stub answering `--version` and the real gate surface with a fixture.
pub(crate) fn document_stub(dir: &Path, name: &str, document: &str, exit: &str) -> PathBuf {
    let doc = fixture(document).display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit {exit}\n"
        ),
    )
}

pub(crate) fn passing_stub(dir: &Path) -> PathBuf {
    document_stub(dir, "gate-pass.sh", "gate-status-pass.json", "0")
}

pub(crate) fn blocked_stub(dir: &Path) -> PathBuf {
    document_stub(dir, "gate-blocked.sh", "gate-status-blocked.json", "1")
}

pub(crate) fn journal_for(db: &Path, project_id: &str) -> Vec<(String, String, String)> {
    Registry::open(db)
        .unwrap()
        .operations_for_project(project_id, 50)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.kind == "gate")
        .map(|entry| (entry.kind, entry.state, entry.detail.unwrap_or_default()))
        .collect()
}

pub(crate) fn evidence_file(proj: &Path, id: &str) -> PathBuf {
    proj.join(".forge/gate").join(id).join("evidence.json")
}

pub(crate) fn head(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ─── gate evidence-export consumption tests ───────────────────────────────────

/// Evidence export fixture path.
pub(crate) fn evidence_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate-evidence")
        .join(name)
}

/// Stub that emits the evidence-export JSON via `gate evidence-export --format json`.
/// Copies the fixture to a temp file, replaces the hardcoded project_id with the
/// test's project id, and cats it.
pub(crate) fn evidence_export_stub_with_project(
    dir: &Path,
    name: &str,
    fixture: &str,
    project_id: &str,
    exit: &str,
) -> PathBuf {
    let src = evidence_fixture(fixture);
    let text = fs::read_to_string(&src).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut adjusted = json;
    adjusted["project_id"] = serde_json::json!(project_id);
    let json_path = dir.join(format!("{name}.json"));
    fs::write(&json_path, serde_json::to_string_pretty(&adjusted).unwrap()).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit {exit}; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

/// Stub emitting an export with a specific project id.
pub(crate) fn evidence_export_stub(
    dir: &Path,
    name: &str,
    fixture: &str,
    project_id: &str,
    exit: &str,
) -> PathBuf {
    evidence_export_stub_with_project(dir, name, fixture, project_id, exit)
}

/// Stub that produces no export document (runtime unavailable).
pub(crate) fn evidence_unavailable_stub(dir: &Path, name: &str) -> PathBuf {
    write_script(
        dir,
        name,
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
echo 'no gate run found'
exit 1
",
    )
}

/// Stub that emits malformed JSON.
pub(crate) fn evidence_malformed_stub(dir: &Path, name: &str) -> PathBuf {
    write_script(
        dir,
        name,
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
echo '{ not json'
exit 0
",
    )
}

/// Stub emitting an export with an unknown field name.
pub(crate) fn evidence_unknown_field_stub(dir: &Path, name: &str, project_id: &str) -> PathBuf {
    // Write the JSON to a temp file and cat it.
    let json_path = dir.join(format!("{name}.json"));
    let json = format!(
        r#"{{"schema_version": 1, "project_id": "{project_id}", "revision": "deadbeef12345678", "toolchain": "driftwatchdog@0.1.0", "fields": [{{"field": "unknown-field-x", "state": "unverified"}}, {{"field": "revision", "state": "verified", "evidence_ref": "revision-ref"}}], "gate_run_id": 99}}"#,
        project_id = project_id
    );
    fs::write(&json_path, json).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

/// Stub emitting a verified publication without digests (contradiction).
pub(crate) fn evidence_publication_contradiction_stub(
    dir: &Path,
    name: &str,
    project_id: &str,
) -> PathBuf {
    let json_path = dir.join(format!("{name}.json"));
    let json = format!(
        r#"{{"schema_version": 1, "project_id": "{project_id}", "revision": "deadbeef12345678", "toolchain": "driftwatchdog@0.1.0", "fields": [{{"field": "publication", "state": "verified", "evidence_ref": "pub-ref"}}, {{"field": "digests", "state": "unverified"}}], "gate_run_id": 99}}"#,
        project_id = project_id
    );
    fs::write(&json_path, json).unwrap();
    let json_str = json_path.display().to_string();
    write_script(
        dir,
        name,
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit 0; fi
echo 'unrecognized'
exit 1
"
        ),
    )
}

pub(crate) fn gate_evidence_cmd(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> GateRun {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("gate");
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("run forge gate evidence");
    GateRun {
        status: out.status.code().unwrap_or(-1),
        stdout: lossy(&out.stdout).to_string(),
        stderr: lossy(&out.stderr).to_string(),
    }
}

pub(crate) fn evidence_json(run: &GateRun) -> serde_json::Value {
    serde_json::from_str(&run.stdout).expect("evidence json")
}

pub(crate) fn evidence_status_file(proj: &Path, id: &str) -> PathBuf {
    proj.join(".forge/gate")
        .join(id)
        .join("release-evidence.json")
}

//! Gate evidence cross-surface contract (`gate-runtime-evidence`).
//!
//! Proves the honest projection boundaries: every read surface agrees on
//! the same aggregate/revision/runtime/timestamp bytes; no surface
//! renders an absent or stale gate as passing; the gate run journals
//! only its own rows; and no gate tool, API route or portal section
//! exists.

use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::api::route_request;
use forge::registry::Registry;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate")
        .join(name)
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}");
}

fn head(dir: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn project(tmp: &Path, id: &str) -> PathBuf {
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

fn passing_stub(dir: &Path) -> PathBuf {
    let doc = fixture("gate-status-pass.json").display().to_string();
    let path = dir.join("gate-pass.sh");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit 0\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn run(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_REGISTRY")
        .env_remove("FORGE_WORKSPACE_ROOT")
        .env_remove("FORGE_WORKSPACE_REGISTRY");
    if let Some(binary) = gate_bin {
        cmd.env("FORGE_GATE_BIN", binary);
    }
    cmd.arg("--registry").arg(db);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge")
}

fn json_of(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("json: {err}; stderr: {}", lossy(&out.stderr)))
}

fn findings_of(value: &serde_json::Value, id: &str) -> Vec<serde_json::Value> {
    value["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["id"] == id)
        .cloned()
        .collect()
}

/// Persist a verbatim fixture document as a Forge evidence record bound
/// to the project's current HEAD, so the read surfaces can be examined
/// without re-invoking the runtime.
fn persist_gate(proj: &Path, id: &str, document: &Path) {
    let text = fs::read_to_string(document).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let aggregate = if value["blocked"].as_bool().unwrap_or(false) {
        "blocked"
    } else {
        "passed"
    };
    let checks: Vec<serde_json::Value> = value["results"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row["gate_id"],
                "state": match row["status"].as_str().unwrap_or("") {
                    "PASS" => "pass",
                    "FAIL" => "fail",
                    "NOT_APPLICABLE" => "not_applicable",
                    _ => "unresolved",
                },
            })
        })
        .collect();
    let record = serde_json::json!({
        "contract": "0.1.0",
        "project_id": id,
        "runtime": "driftwatchdog",
        "runtime_version": "driftwatch 0.1.0",
        "revision": head(proj),
        "manifest_digest": value["manifest_digest"],
        "rule_pack_version": value["rule_pack_version"],
        "aggregate": aggregate,
        "checks": checks,
        "observed_at": "2026-09-24T12:00:00Z",
        "dry_run": false,
    });
    write_file(
        proj,
        &format!(".forge/gate/{id}/evidence.json"),
        &serde_json::to_string_pretty(&record).unwrap(),
    );
}

#[test]
fn doctor_gate_finding_never_invents_health_from_absence() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let plain = project(tmp.path(), "gate-xplain");
    let declared = project(tmp.path(), "gate-xdeclared");
    write_file(
        &declared,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-xdeclared\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"driftwatchdog\"}}",
    );

    // Plain project (no gate markers at all): the finding is present
    // with the honest not-applicable shape, never a health gate.
    let out = run(
        &db,
        &["--format", "json", "doctor", &plain.display().to_string()],
        None,
    );
    let value = json_of(&out);
    let finding = &findings_of(&value, "gate-evidence")[0];
    assert_eq!(finding["status"], "pass", "{finding}");
    assert_eq!(finding["applicable"], false, "{finding}");

    // Declared gate runtime, never run: unverified, applicable and
    // non-healthy — absence is never a pass.
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "doctor",
            &declared.display().to_string(),
        ],
        None,
    );
    let value = json_of(&out);
    let finding = &findings_of(&value, "gate-evidence")[0];
    assert_eq!(finding["status"], "unavailable", "{finding}");
    assert_eq!(finding["applicable"], true, "{finding}");
    assert!(
        finding["detail"]
            .as_str()
            .unwrap_or("")
            .contains("never run"),
        "{finding}"
    );
}

#[test]
fn doctor_reads_blocked_evidence_as_fail_and_stale_passing_as_warn() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    let blocked = project(tmp.path(), "gate-xblocked");
    persist_gate(
        &blocked,
        "gate-xblocked",
        &fixture("gate-status-blocked.json"),
    );
    let out = run(
        &db,
        &["--format", "json", "doctor", &blocked.display().to_string()],
        None,
    );
    let value = json_of(&out);
    let finding = &findings_of(&value, "gate-evidence")[0];
    assert_eq!(finding["status"], "fail", "{finding}");
    assert_eq!(value["doctor"]["healthy"], false);
    let summary = finding["evidence"][0].as_str().unwrap_or("");
    assert!(summary.contains("aggregate=blocked"), "{summary}");

    let passed = project(tmp.path(), "gate-xstale");
    persist_gate(&passed, "gate-xstale", &fixture("gate-status-pass.json"));
    let out = run(
        &db,
        &["--format", "json", "doctor", &passed.display().to_string()],
        None,
    );
    let finding = &findings_of(&json_of(&out), "gate-evidence")[0];
    assert_eq!(finding["status"], "pass", "{finding}");
    assert_eq!(finding["detail"], "gate evidence is fresh and passing");

    // The revision moves: the same record is a warning, never current.
    write_file(&passed, "notes.md", "moved\n");
    git(&passed, &["add", "-A"]);
    git(&passed, &["commit", "-q", "-m", "second"]);
    let out = run(
        &db,
        &["--format", "json", "doctor", &passed.display().to_string()],
        None,
    );
    let finding = &findings_of(&json_of(&out), "gate-evidence")[0];
    assert_eq!(finding["status"], "warn", "{finding}");
    assert!(
        finding["detail"].as_str().unwrap_or("").contains("stale"),
        "{finding}"
    );
}

#[test]
fn read_surfaces_agree_on_the_same_evidence_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-xparity");
    let stub = passing_stub(tmp.path());

    let out = run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let run_value = json_of(&out);
    let observed = run_value["gate"]["observed_at"].as_str().unwrap_or("");
    let revision = run_value["gate"]["revision"].as_str().unwrap_or("");

    let status = run(
        &db,
        &[
            "--format",
            "json",
            "gate",
            "status",
            &proj.display().to_string(),
        ],
        None,
    );
    assert!(status.status.success(), "{}", lossy(&status.stderr));
    let status_value = json_of(&status);
    assert_eq!(status_value["gate"]["observed_at"], observed);
    assert_eq!(status_value["gate"]["revision"], revision);
    assert_eq!(status_value["gate"]["aggregate"], "passed");
    assert_eq!(status_value["gate"]["runtime"], "gate-pass");

    let doctor = run(
        &db,
        &["--format", "json", "doctor", &proj.display().to_string()],
        None,
    );
    let finding = &findings_of(&json_of(&doctor), "gate-evidence")[0];
    let summary = finding["evidence"][0].as_str().unwrap_or("");
    assert!(summary.contains(observed), "{summary}");
    assert!(summary.contains(&revision[..12]), "{summary}");
    assert!(summary.contains("aggregate=passed"), "{summary}");
    assert!(summary.contains("runtime=gate-pass"), "{summary}");
}

#[test]
fn checker_plane_projects_gate_states_and_stays_clean_otherwise() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let plain = project(tmp.path(), "gate-xcheck-plain");
    run(&db, &["register", &plain.display().to_string()], None);

    // A project without any gate involvement: no gate alert appears.
    let out = run(&db, &["check", "gate-xcheck-plain"], None);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        !document["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["symbol"] == "doctor/gate-evidence"),
        "{document}"
    );

    // A blocked fresh record projects as an error alert.
    let blocked = project(tmp.path(), "gate-xcheck-blocked");
    run(&db, &["register", &blocked.display().to_string()], None);
    persist_gate(
        &blocked,
        "gate-xcheck-blocked",
        &fixture("gate-status-blocked.json"),
    );
    let out = run(&db, &["check", "gate-xcheck-blocked"], None);
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let alert = document["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["symbol"] == "doctor/gate-evidence")
        .expect("blocked gate surfaces as an alert");
    assert_eq!(alert["severity"], "error", "{alert}");

    // Declared but never run projects as a warning naming the gap.
    let declared = project(tmp.path(), "gate-xcheck-declared");
    write_file(
        &declared,
        ".project.json",
        "{\"schema_version\":1,\"id\":\"gate-xcheck-declared\",\"kind\":\"product\",\"verification\":{\"command\":\"true\",\"gate_runtime\":\"driftwatchdog\"}}",
    );
    run(&db, &["register", &declared.display().to_string()], None);
    let out = run(&db, &["check", "gate-xcheck-declared"], None);
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let alert = document["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["symbol"] == "doctor/gate-evidence")
        .expect("declared-never-run gate surfaces as a warning");
    assert_eq!(alert["severity"], "warning", "{alert}");
}

#[test]
fn gate_runs_journal_only_their_own_rows_and_reads_journal_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-xjournal");
    let stub = passing_stub(tmp.path());

    // An unrelated journal row must survive untouched.
    {
        let registry = Registry::open(&db).unwrap();
        registry
            .record_operation("provider", "gate-xjournal", "done", "pre-existing")
            .unwrap();
    }
    let out = run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));

    let registry = Registry::open(&db).unwrap();
    let rows = registry
        .operations_for_project("gate-xjournal", 50)
        .unwrap();
    let provider_rows: Vec<_> = rows.iter().filter(|r| r.kind == "provider").collect();
    let gate_rows: Vec<_> = rows.iter().filter(|r| r.kind == "gate").collect();
    assert_eq!(provider_rows.len(), 1, "gate run altered foreign rows");
    assert_eq!(provider_rows[0].detail.as_deref(), Some("pre-existing"));
    assert_eq!(gate_rows.len(), 1);
    assert_eq!(gate_rows[0].state, "done");

    // Reads (status, doctor) add no rows at all.
    let before = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    run(&db, &["gate", "status", &proj.display().to_string()], None);
    run(&db, &["doctor", &proj.display().to_string()], None);
    let after = Registry::open(&db)
        .unwrap()
        .journal_entries()
        .unwrap()
        .len();
    assert_eq!(before, after, "gate read surfaces journaled");
}

#[test]
fn no_gate_tool_or_api_route_exists_after_gate_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-xsurfaces");
    let stub = passing_stub(tmp.path());
    run(&db, &["register", &proj.display().to_string()], None);
    run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub),
    );

    // MCP: the mature registry stays silent about gate tools even with
    // persisted evidence present.
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("mcp")
        .arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge mcp serve");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list"
        });
        writeln!(stdin, "{request}").expect("write request");
    }
    let output = child.wait_with_output().expect("wait");
    let text = lossy(&output.stdout).to_string();
    assert!(text.contains("create_project"), "tools/list empty: {text}");
    for forbidden in ["gate", "run_gate", "gate_status", "gate-run"] {
        assert!(
            !text.contains(&format!("\"{forbidden}\"")),
            "MCP tools/list advertises `{forbidden}`: {text}"
        );
    }

    // API: no gate route is ever served.
    for (method, path) in [
        ("GET", "/gate"),
        ("GET", "/v1/gate"),
        ("POST", "/v1/projects/gate-xsurfaces/gate"),
        ("POST", "/gate/run"),
    ] {
        assert!(
            route_request(method, path).is_none(),
            "gate route {method} {path} unexpectedly exists"
        );
    }
}

#[test]
fn portal_stays_read_only_and_gains_no_gate_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-xportal");
    let stub = passing_stub(tmp.path());
    run(&db, &["register", &proj.display().to_string()], None);
    let out = run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));

    // The canonical read-only surfaces still render after a gate run.
    let settings = run(
        &db,
        &["--format", "json", "portal", "view", "settings"],
        None,
    );
    assert!(settings.status.success(), "{}", lossy(&settings.stderr));

    // `gate` is not a portal section: the surface refuses it rather
    // than inventing a new projection, and the capability adds no
    // write operations anywhere.
    let gate_view = run(&db, &["--format", "json", "portal", "view", "gate"], None);
    assert_eq!(gate_view.status.code(), Some(1));
    let stderr = lossy(&gate_view.stderr).to_string();
    assert!(stderr.contains("portal-invalid"), "{stderr}");
}

#[test]
fn release_gate_check_cites_fresh_passing_evidence_only() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-xrelease");
    // A release configuration whose only check kind is `gate`.
    write_file(
        &proj,
        "forge.yaml",
        "schema: 1\nproject:\n  id: gate-xrelease\n  name: gate-xrelease\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: gate\n  changelog: CHANGELOG.md\n",
    );
    write_file(&proj, "CHANGELOG.md", "## 1.0.0\n- initial release\n");

    // Never-run: the gate check captures unavailable, not ready.
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "release",
            "prepare",
            "--version",
            "1.0.0",
            &proj.display().to_string(),
        ],
        None,
    );
    let value = json_of(&out);
    let checks = value["plan"]["checks"].as_array().unwrap();
    let gate = checks
        .iter()
        .find(|c| c["kind"] == "gate")
        .expect("gate check");
    assert_eq!(gate["status"], "unavailable", "{gate}");
    assert_eq!(value["plan"]["ready"], false, "{value}");

    // Fresh passing record: the check passes at the captured revision.
    let stub = passing_stub(tmp.path());
    let out = run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub),
    );
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "release",
            "prepare",
            "--version",
            "1.0.0",
            &proj.display().to_string(),
        ],
        None,
    );
    let value = json_of(&out);
    let checks = value["plan"]["checks"].as_array().unwrap();
    let gate = checks
        .iter()
        .find(|c| c["kind"] == "gate")
        .expect("gate check");
    assert_eq!(gate["status"], "pass", "{gate}");
    assert_eq!(gate["applicable"], true);
    assert_eq!(value["plan"]["ready"], true, "{value}");

    // The revision moves: stale evidence cannot back a release claim.
    write_file(&proj, "notes.md", "moved\n");
    git(&proj, &["add", "-A"]);
    git(&proj, &["commit", "-q", "-m", "second"]);
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "release",
            "prepare",
            "--version",
            "1.0.0",
            &proj.display().to_string(),
        ],
        None,
    );
    let value = json_of(&out);
    let checks = value["plan"]["checks"].as_array().unwrap();
    let gate = checks
        .iter()
        .find(|c| c["kind"] == "gate")
        .expect("gate check");
    assert_eq!(gate["status"], "stale", "{gate}");
    assert_eq!(value["plan"]["ready"], false, "{value}");
}

// ─── gate evidence-export cross-surface tests ─────────────────────────────────

/// Stub that emits evidence-export JSON for a specific project id.
fn evidence_export_stub(dir: &Path, name: &str, project_id: &str, exit_code: &str) -> PathBuf {
    let doc = format!(
        r#"{{"schema_version": 1, "project_id": "{project_id}", "revision": "deadbeef12345678", "toolchain": "driftwatchdog@0.1.0", "fields": [{{"field": "revision", "state": "unverified"}}, {{"field": "version", "state": "unverified"}}, {{"field": "toolchain", "state": "unverified"}}, {{"field": "artifacts", "state": "unverified"}}, {{"field": "digests", "state": "unverified"}}, {{"field": "sbom", "state": "unverified"}}, {{"field": "provenance", "state": "unverified"}}, {{"field": "checks", "state": "unverified"}}, {{"field": "publication", "state": "unverified"}}], "gate_run_id": 99}}
"#
    );
    let json_path = dir.join(format!("{name}.json"));
    fs::write(&json_path, doc).unwrap();
    let json_str = json_path.display().to_string();
    let path = dir.join(name);
    fs::write(
        &path,
        format!(
            "#!/bin/sh
if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi
if [ \"$1\" = \"gate\" ] && [ \"$2\" = \"evidence-export\" ]; then cat '{json_str}'; exit {exit_code}; fi
echo 'unrecognized'
exit 1
"
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn run_json(db: &Path, args: &[&str], gate_bin: Option<&Path>) -> serde_json::Value {
    let out = run(db, args, gate_bin);
    serde_json::from_str(&lossy(&out.stdout)).expect("json")
}

#[test]
fn evidence_command_reads_nothing_without_prior_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "evid-cross-absent");

    let stub = evidence_export_stub(tmp.path(), "gate-evid.sh", "evid-cross-absent", "0");
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "gate",
            "evidence",
            "status",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert_eq!(out.status.code(), Some(1));
    let value = serde_json::from_str::<serde_json::Value>(&lossy(&out.stdout)).unwrap();
    assert!(value["release_evidence"].is_null());
    assert_eq!(value["freshness"], "absent");
}

#[test]
fn evidence_command_produces_unchanged_gate_verdict_surface() {
    // Adding `forge gate evidence` must not change the gate verdict output.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "gate-unchanged");

    let stub = evidence_export_stub(tmp.path(), "gate-evid.sh", "gate-unchanged", "0");

    // Run evidence export against the project by path.
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "gate",
            "evidence",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value = serde_json::from_str::<serde_json::Value>(&lossy(&out.stdout)).unwrap();
    assert_eq!(value["release_evidence"]["project_id"], "gate-unchanged");

    // Gate verdict surface is unchanged: read absent gate verdict.
    let out2 = run(
        &db,
        &[
            "--format",
            "json",
            "gate",
            "status",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert_eq!(out2.status.code(), Some(1), "{}", lossy(&out2.stderr)); // absent gate verdict

    // Now run gate verdict (not evidence) with a passing stub.
    let stub2 = passing_stub(&proj);
    let out3 = run(
        &db,
        &["--format", "json", "gate", &proj.display().to_string()],
        Some(&stub2),
    );
    assert_eq!(out3.status.code(), Some(0), "{}", lossy(&out3.stderr));
    let value3 = serde_json::from_str::<serde_json::Value>(&lossy(&out3.stdout)).unwrap();
    // Gate verdict surface unchanged.
    assert_eq!(value3["contract"], "0.1.0");
    assert_eq!(value3["gate"]["aggregate"], "passed");
}

#[test]
fn consumed_evidence_cannot_become_deployable() {
    // Consuming release evidence cannot set deployable: true.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "deploy-evid");

    // Run evidence export.
    let stub = evidence_export_stub(tmp.path(), "gate-evid.sh", "deploy-evid", "0");
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "gate",
            "evidence",
            &proj.display().to_string(),
        ],
        Some(&stub),
    );
    assert_eq!(out.status.code(), Some(0));
    let value = serde_json::from_str::<serde_json::Value>(&lossy(&out.stdout)).unwrap();

    // The release-evidence record should not contain a deployable field.
    let evid = &value["release_evidence"];
    assert!(!evid.as_object().unwrap().contains_key("deployable"));
}

#[test]
fn doctor_release_evidence_finding_does_not_gate_health() {
    // The release-evidence finding is non-gating.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project(tmp.path(), "dr-evid");

    let out = run(
        &db,
        &["--format", "json", "doctor", &proj.display().to_string()],
        None,
    );
    let value = serde_json::from_str::<serde_json::Value>(&lossy(&out.stdout)).unwrap();

    // Doctor should run without error.
    assert_eq!(out.status.code(), Some(0));

    // If release-evidence finding exists, it should not be blocking.
    let findings = value["doctor"]["findings"].as_array().unwrap();
    for f in findings {
        if f["id"] == "release-evidence" {
            assert!(!f["blocking"].as_bool().unwrap_or(false));
        }
    }
}

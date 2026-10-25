//! Contract for `studio-journal-verdict-truth`.
//!
//! One rule names what a Studio journal row records: the action the
//! Studio Core performed. A refinement request that was validated and
//! recorded is a recorded request, not a performed refinement, so its
//! row reads `recorded` and never `done`. A saved spec stays a
//! completed save and a preview row keeps naming the observed preview
//! state — every writer reads the same rule, so no row carries a state
//! string a writer invented.
//!
//! - a recorded refinement journals `recorded`, never `done`;
//! - a saved spec still journals `done`;
//! - a preview row still names the observed preview state;
//! - a refused refinement writes no row at all;
//! - the `forge studio refine` transport records the same row.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use forge::registry::Registry;
use forge::studio::{
    parse_spec_text, record_refinement, save_spec, start_preview, stop_preview, FakeRunner,
    PreviewState, StudioAction, STUDIO_PORT_RANGE_START_ENV,
};

#[path = "support/studio_ports.rs"]
mod studio_ports;
use studio_ports::shared_port_base;

/// Serialize the tests: the Studio port range and the startup-timeout
/// env are process-global, so parallel tests would race the allocator.
static SERIAL: Mutex<()> = Mutex::new(());

/// Which candidate this binary prefers. Distinct per test target, so two
/// Studio targets running at the same time do not pick the same window —
/// see `support/studio_ports.rs`.
const SLOT: u16 = 4;

const PROJECT_ID: &str = "studio-journal-verdict";

const APP_SPEC: &str = "schema_version: \"1\"\n\
project_id: studio-journal-verdict\n\
name: Studio Journal Verdict\n\
profile: react-web\n\
pages:\n\
\x20\x20- route: /\n\
\x20\x20\x20\x20title: Home\n\
\x20\x20\x20\x20sections:\n\
\x20\x20\x20\x20\x20\x20- id: hero-block\n\
\x20\x20\x20\x20\x20\x20\x20\x20kind: hero\n";

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
        .env_remove("FORGE_STUDIO_PORT_RANGE_START");
    cmd
}

fn set_test_env() {
    std::env::set_var(
        STUDIO_PORT_RANGE_START_ENV,
        shared_port_base(SLOT).to_string(),
    );
    std::env::set_var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "8");
}

/// A registered project with a saved AppSpec.
fn setup(tmp: &Path) -> (Registry, PathBuf) {
    let project_root = tmp.join("project");
    fs::create_dir_all(&project_root).unwrap();
    fs::write(
        project_root.join("forge.yaml"),
        "schema: 1\nproject:\n  id: studio-journal-verdict\n  name: Studio Journal Verdict\n  profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
    )
    .unwrap();
    let mut registry = Registry::open(&tmp.join("registry.db")).unwrap();
    registry.register(&project_root, None).unwrap();
    let spec = parse_spec_text(APP_SPEC).unwrap();
    save_spec(&registry, PROJECT_ID, &project_root, spec, "r0").unwrap();
    (registry, project_root)
}

/// The journal rows of one kind, oldest first.
fn journal_rows(registry: &Registry, kind: &str) -> Vec<(String, String)> {
    registry
        .operations_for_project(PROJECT_ID, 50)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.kind == kind)
        .map(|entry| (entry.state, entry.detail.unwrap_or_default()))
        .collect()
}

#[test]
fn a_recorded_refinement_journals_a_recorded_request() {
    // The Core validates the request, records it and bumps the
    // revision. No refinement runs, so the row names the recorded
    // request and never a completed action.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());

    record_refinement(
        &registry,
        &project_root,
        "r1",
        "make the hero bigger",
        &["src/app.tsx".to_string()],
    )
    .expect("the refinement request is accepted");

    let rows = journal_rows(&registry, "studio.refine");
    assert_eq!(rows.len(), 1, "one refinement row: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(
        state, "recorded",
        "only the request was recorded; detail={detail}"
    );
    assert_ne!(
        state, "done",
        "no refinement ran, so the row must not claim one; detail={detail}"
    );
    assert!(
        detail.contains("app_revision=r2"),
        "the row carries the new revision: {detail}"
    );
    assert!(
        detail.contains("make the hero bigger"),
        "the row carries the request: {detail}"
    );
    assert!(
        detail.contains("src/app.tsx"),
        "the row carries the selected files: {detail}"
    );
}

#[test]
fn a_saved_spec_still_journals_a_completed_save() {
    // The rule is not a blanket demotion: a saved spec is a completed
    // save, and its row still says so.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, _project_root) = setup(tmp.path());

    let rows = journal_rows(&registry, "studio.spec.save");
    assert_eq!(rows.len(), 1, "one spec-save row: {rows:?}");
    let (state, detail) = &rows[0];
    assert_eq!(state, "done", "the spec was saved; detail={detail}");
}

#[test]
fn a_preview_row_still_names_the_observed_state() {
    // The preview writers read the same rule, and the observed preview
    // state is still what they record.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());

    let runner = Box::new(FakeRunner::default());
    let (_session, live) = start_preview(&registry, &project_root, runner).expect("preview ready");
    let rows = journal_rows(&registry, "studio.preview.start");
    assert_eq!(rows.len(), 1, "one preview-start row: {rows:?}");
    assert_eq!(rows[0].0, "ready", "detail={}", rows[0].1);

    stop_preview(&registry, &project_root, Some(live)).expect("preview stopped");
    let rows = journal_rows(&registry, "studio.preview.stop");
    assert_eq!(rows.len(), 1, "one preview-stop row: {rows:?}");
    assert_eq!(rows[0].0, "stopped", "detail={}", rows[0].1);
}

#[test]
fn a_refused_refinement_writes_no_row() {
    // A refusal stays a refusal: no row, and the recorded-request
    // state never appears for a request the Core refused.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let (registry, project_root) = setup(tmp.path());

    let err = record_refinement(&registry, &project_root, "r9", "make it blue", &[])
        .expect_err("a stale revision is refused");
    assert!(
        matches!(err, forge::core::ForgeError::StudioRevisionConflict { .. }),
        "{err:?}"
    );
    assert!(
        journal_rows(&registry, "studio.refine").is_empty(),
        "a refused refinement writes no row"
    );
}

#[test]
fn the_cli_refine_transport_records_the_same_row() {
    // The operator-facing command writes the same row the Core does.
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    set_test_env();
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let project_root = tmp.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    fs::write(
        project_root.join("forge.yaml"),
        "schema: 1\nproject:\n  id: studio-journal-verdict\n  name: Studio Journal Verdict\n  profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
    )
    .unwrap();
    fs::write(project_root.join("forge.app.yaml"), APP_SPEC).unwrap();

    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(&project_root)
        .arg("--format")
        .arg("json");
    let out = cmd.output().expect("run register");
    assert!(out.status.success(), "register: {}", lossy(&out.stderr));

    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("studio")
        .arg("spec")
        .arg(PROJECT_ID)
        .arg("--confirm")
        .arg("yes")
        .arg("--expected-revision")
        .arg("r0")
        .arg("--format")
        .arg("json");
    let out = cmd.output().expect("run studio spec");
    assert!(out.status.success(), "studio spec: {}", lossy(&out.stderr));

    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("studio")
        .arg("refine")
        .arg(PROJECT_ID)
        .arg("--expected-revision")
        .arg("r1")
        .arg("--request")
        .arg("make the hero bigger")
        .arg("--format")
        .arg("json");
    let out = cmd.output().expect("run studio refine");
    assert!(
        out.status.success(),
        "studio refine: {}",
        lossy(&out.stderr)
    );

    let registry = Registry::open(&db).unwrap();
    let rows = journal_rows(&registry, "studio.refine");
    assert_eq!(rows.len(), 1, "one refinement row: {rows:?}");
    assert_eq!(
        rows[0].0, "recorded",
        "the transport records the request, not a performed refinement; detail={}",
        rows[0].1
    );
}

#[test]
fn one_rule_names_every_studio_action() {
    // One rule, read by every writer: the Core's action decides the
    // row's state, so no writer carries a state string of its own.
    assert_eq!(StudioAction::SpecSaved.journal_state(), "done");
    assert_eq!(StudioAction::RefinementRecorded.journal_state(), "recorded");
    assert_eq!(
        StudioAction::PreviewObserved(PreviewState::Ready).journal_state(),
        "ready"
    );
    assert_eq!(
        StudioAction::PreviewObserved(PreviewState::Failed).journal_state(),
        "failed"
    );
    assert_eq!(
        StudioAction::PreviewObserved(PreviewState::Stopped).journal_state(),
        "stopped"
    );
}

//! CLI contract for the docs translate verdict
//! (`docs-translate-review-truth`).
//!
//! A translator that drops a link destination produces a derivative the
//! report itself marks `needs-review`. The aggregate verdict must name
//! that review status, so no surface calls such a run complete:
//!
//! - A locale that needs review: the verdict is `needs-review`, the
//!   report is not healthy, the summary names the locale instead of
//!   saying "complete", the command still exits 0 and the journal
//!   records the operation as `done`.
//! - Every locale reviewed clean: the verdict is `complete`, the report
//!   is healthy and the command exits 0.
//! - A failed locale: the verdict is `partial` and the command exits 1.
//! - A failure outranks a review: one failed locale and one locale that
//!   needs review still make the run `partial`.
//! - The regression guard: the per-locale outcome still carries its
//!   review status and its reasons, and the derivative and its recorded
//!   state are written exactly as before.
//!
//! Each scenario drives the real `forge` binary through
//! `FORGE_DOCS_TRANSLATOR_BIN` so a fixture shell script stands in for
//! a real translation provider; the fake is a pure `sed` filter that
//! echoes the request JSON back with translated words, which is valid
//! because the adapter ignores unknown fields and matches segments by
//! id.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn manifest(id: &str, docs: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n{docs}"
    )
}

fn docs_block(locales: &str) -> String {
    format!("docs:\n  source_language: en\n  translations:\n{locales}")
}

/// The source carries one link destination, so a translator that
/// drops it is caught by the preservation check.
const SOURCE: &str = "# Alpha Guide\n\nForge alpha introduction paragraph.\n\n```sh\nforge list --format json\n```\n\nbeta section with [the manual](https://example.com/manual).\n\ngamma closing paragraph.\n";

/// Clean fake: a pure filter translating the fixture words. The link
/// destination survives, so the review status stays `ok`.
fn clean_translator() -> String {
    "#!/bin/sh\nsed 's/alpha/ALPHA/g; s/beta/BETA/g; s/gamma/GAMMA/g'\n".to_string()
}

/// Dropping fake: the translator rewrites the link into plain text,
/// so the derivative loses the destination the source carried and the
/// report marks the locale `needs-review`.
fn dropping_translator() -> String {
    "#!/bin/sh\nsed 's|\\[the manual\\](https://example.com/manual)|the manual|'\n".to_string()
}

/// Failing fake: the provider refuses the run.
fn failing_translator() -> String {
    "#!/bin/sh\necho 'translator refused the run' >&2\nexit 1\n".to_string()
}

/// A fake that drops the link for every locale except `fr`, which it
/// refuses outright. One locale ends up needing a review, the other
/// fails: the failure must outrank the review.
fn mixed_translator() -> String {
    "#!/bin/sh\ncase \"$*\" in\n  *\"--locale fr\"*) echo 'translator refused fr' >&2; exit 1 ;;\nesac\nsed 's|\\[the manual\\](https://example.com/manual)|the manual|'\n".to_string()
}

fn setup_project(tmp: &Path, id: &str, locales: &str) -> (PathBuf, PathBuf) {
    let db = tmp.join("registry.db");
    let proj = tmp.join(id);
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &manifest(id, &docs_block(locales)));
    write_file(&proj, "README.md", SOURCE);
    (db, proj)
}

fn run_translate(
    db: &Path,
    proj: &Path,
    extra_env: &[(&str, &Path)],
    args: &[&str],
) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY");
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    cmd.arg("--registry").arg(db).arg("docs").arg("translate");
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--project").arg(proj);
    cmd.output().expect("run forge")
}

fn run_translate_json(
    db: &Path,
    proj: &Path,
    extra_env: &[(&str, &Path)],
    args: &[&str],
) -> serde_json::Value {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY");
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("docs")
        .arg("translate");
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--project").arg(proj);
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

/// The journal row the command wrote for the `docs` operation.
fn journal_row(db: &Path) -> (String, String) {
    let conn = rusqlite::Connection::open(db).expect("open db");
    conn.query_row(
        "SELECT state, detail FROM operations WHERE kind = 'docs' ORDER BY op_id DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("a docs journal row")
}

/// The `summary:` line of the human report.
fn summary_line(stdout: &str) -> String {
    stdout
        .lines()
        .find(|line| line.starts_with("summary: "))
        .unwrap_or_else(|| panic!("no summary line in:\n{stdout}"))
        .to_string()
}

#[test]
fn a_locale_that_needs_review_is_never_complete() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = write_script(tmp.path(), "dropping-translator.sh", &dropping_translator());
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-review-truth",
        "    zh-CN:\n      enabled: true\n",
    );

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    // The translation succeeded: a human must review it, so the
    // command does not fail.
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("verdict: needs-review"),
        "the human report must name the verdict:\n{stdout}"
    );
    let summary = summary_line(&stdout);
    assert!(
        !summary.contains("complete"),
        "the summary must not call a run that waits for a human complete: {summary}"
    );
    assert!(
        summary.contains("needs review"),
        "the summary must name the verdict: {summary}"
    );
    assert!(
        summary.contains("zh-CN"),
        "the summary must name the locale that needs review: {summary}"
    );

    // The JSON document publishes the verdict, and the report is not
    // healthy.
    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let report = &value["translate"];
    assert_eq!(report["verdict"], "needs-review");
    assert_eq!(report["healthy"], false);
    let note = report["note"].as_str().unwrap();
    assert!(note.contains("needs review"), "{note}");
    assert!(!note.contains("complete"), "{note}");
    assert!(note.contains("zh-CN"), "{note}");

    // The journal records the operation as done: the translation
    // succeeded, and the detail names the verdict.
    let (state, detail) = journal_row(&db);
    assert_eq!(state, "done", "detail={detail}");
    assert!(detail.contains("verdict=needs-review"), "{detail}");
}

#[test]
fn every_locale_reviewed_clean_is_complete() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = write_script(tmp.path(), "clean-translator.sh", &clean_translator());
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-clean-truth",
        "    zh-CN:\n      enabled: true\n",
    );

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("verdict: complete"),
        "the human report must name the verdict:\n{stdout}"
    );
    assert!(
        summary_line(&stdout).contains("complete"),
        "the summary must say the run is complete:\n{stdout}"
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let report = &value["translate"];
    assert_eq!(report["verdict"], "complete");
    assert_eq!(report["healthy"], true);
    assert_eq!(report["outcomes"][0]["review"], "ok");

    let (state, detail) = journal_row(&db);
    assert_eq!(state, "done", "detail={detail}");
    assert!(detail.contains("verdict=complete"), "{detail}");
}

#[test]
fn a_failed_locale_is_partial() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = write_script(tmp.path(), "failing-translator.sh", &failing_translator());
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-failed-truth",
        "    zh-CN:\n      enabled: true\n",
    );

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("verdict: partial"),
        "the human report must name the verdict:\n{stdout}"
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let report = &value["translate"];
    assert_eq!(report["verdict"], "partial");
    assert_eq!(report["healthy"], false);

    let (state, detail) = journal_row(&db);
    assert_eq!(state, "partial", "detail={detail}");
    assert!(detail.contains("verdict=partial"), "{detail}");
}

#[test]
fn a_failure_outranks_a_review() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = write_script(tmp.path(), "mixed-translator.sh", &mixed_translator());
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-mixed-truth",
        "    zh-CN:\n      enabled: true\n    fr:\n      enabled: true\n",
    );

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["--all"],
    );
    // The definite failure fails the command, even though the other
    // locale only waits for a human.
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("verdict: partial"),
        "the human report must name the verdict:\n{stdout}"
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["--all"],
    );
    let report = &value["translate"];
    assert_eq!(report["verdict"], "partial");
    assert_eq!(report["healthy"], false);
    let outcomes = report["outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), 2, "{outcomes:?}");
    let reviews: Vec<&str> = outcomes
        .iter()
        .map(|o| o["review"].as_str().unwrap())
        .collect();
    assert!(reviews.contains(&"needs-review"), "{reviews:?}");
    assert!(reviews.contains(&"ok"), "{reviews:?}");
}

#[test]
fn the_derivative_and_its_recorded_state_are_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = write_script(tmp.path(), "dropping-translator.sh", &dropping_translator());
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-derivative-truth",
        "    zh-CN:\n      enabled: true\n",
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let outcome = &value["translate"]["outcomes"][0];
    // The per-locale outcome still carries the review status and
    // its reason.
    assert_eq!(outcome["status"], "translated");
    assert_eq!(outcome["review"], "needs-review");
    let reasons = outcome["review_reasons"].as_array().unwrap();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(
        reasons[0]
            .as_str()
            .unwrap()
            .contains("https://example.com/manual"),
        "{reasons:?}"
    );
    assert_eq!(outcome["derivative"], "docs/README.zh-CN.md");

    // The derivative is written exactly as the provider returned it:
    // the link destination is gone, the code block is verbatim.
    let derivative = fs::read_to_string(proj.join("docs/README.zh-CN.md")).unwrap();
    assert!(
        derivative.contains("the manual"),
        "the derivative must carry the provider output:\n{derivative}"
    );
    assert!(
        !derivative.contains("https://example.com/manual"),
        "the derivative must not carry the dropped link:\n{derivative}"
    );
    assert!(
        derivative.contains("forge list --format json"),
        "the derivative must keep the code block verbatim:\n{derivative}"
    );

    // The recorded state names the same review status, the same
    // reason and the source hash of the unchanged source.
    let state_raw = fs::read_to_string(proj.join(".forge/docs/zh-CN/state.json")).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
    assert_eq!(state["review"], "needs-review");
    let state_reasons = state["review_reasons"].as_array().unwrap();
    assert_eq!(state_reasons.len(), 1, "{state_reasons:?}");
    assert!(state_reasons[0]
        .as_str()
        .unwrap()
        .contains("https://example.com/manual"));
    let source_hash = state["source_hash"].as_str().unwrap();
    assert_eq!(source_hash.len(), 64, "{source_hash}");
    assert_eq!(outcome["source_hash"], source_hash);
    assert_eq!(state["contract"], "0.1.0");
}

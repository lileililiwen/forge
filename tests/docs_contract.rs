//! CLI contract for `forge docs translate` (`documentation-translation`).
//!
//! Exercises the docs surface end to end through the built binary:
//!
//! - R1 success: an enabled locale translates against an unchanged
//!   source; the derivative records its source hash and review
//!   status, code blocks stay verbatim and link destinations are
//!   preserved.
//! - R1 failure: a derivative path that resolves to the source
//!   file or outside the project fails with `error[docs-invalid]`
//!   before writing anything.
//! - R1 boundary: a disabled locale is refused without invoking
//!   its provider or creating output; `--all` skips disabled
//!   locales while translating the enabled ones.
//! - R2 success: changing one source paragraph retranslates only
//!   that segment (the provider sees exactly one segment) and
//!   retains the unchanged technical blocks.
//! - R2 failure: a failing provider leaves the prior derivative
//!   and state intact and the attempt reports `failed` with a
//!   redacted `error[translation-failed]`.
//! - R2 boundary: an unchanged source hash reports `current`
//!   without repeating a provider request.
//!
//! Each scenario drives the CLI through `FORGE_DOCS_TRANSLATOR_BIN`
//! so a fixture shell script stands in for a real translation
//! provider; the success fake is a pure `sed` filter that echoes
//! the request JSON back with translated words, which is valid
//! because the adapter ignores unknown fields and matches
//! segments by id. A call-count file proves how many provider
//! requests each run made.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
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

const SOURCE_V1: &str = "# Alpha Guide\n\nForge alpha introduction paragraph.\n\n```sh\nforge list --format json\n```\n\nbeta section with [the manual](https://example.com/manual).\n\ngamma closing paragraph.\n";

/// Success fake: a pure filter translating the fixture words.
/// Unknown request fields pass through untouched, which the
/// adapter accepts; segment ids are preserved by construction.
fn good_translator(calls: &Path, last_stdin: &Path) -> String {
    format!(
        "#!/bin/sh\n\
         echo call >> '{calls}'\n\
         tee '{stdin}' | sed 's/alpha/ALPHA/g; s/beta/BETA/g; s/gamma/GAMMA/g'\n",
        calls = calls.display(),
        stdin = last_stdin.display()
    )
}

fn call_count(calls: &Path) -> usize {
    fs::read_to_string(calls)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

fn last_request_segments(last_stdin: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(last_stdin).expect("provider must have seen stdin");
    serde_json::from_str(&raw).expect("provider stdin must be JSON")
}

fn setup_project(tmp: &Path, id: &str, locales: &str, source: &str) -> (PathBuf, PathBuf) {
    let db = tmp.join("registry.db");
    let proj = tmp.join(id);
    fs::create_dir(&proj).unwrap();
    write_file(&proj, "forge.yaml", &manifest(id, &docs_block(locales)));
    write_file(&proj, "README.md", source);
    (db, proj)
}

#[test]
fn help_lists_docs_translate() {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    let out = cmd.arg("--help").output().expect("run forge");
    assert_eq!(out.status.code(), Some(0));
    assert!(
        lossy(&out.stdout).contains("docs"),
        "{}",
        lossy(&out.stdout)
    );

    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    let out = cmd
        .arg("docs")
        .arg("--help")
        .output()
        .expect("run forge docs");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(text.contains("translate"), "{text}");
    assert!(text.contains("--all"), "{text}");
}

#[test]
fn translate_success_records_hash_and_review() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = tmp.path().join("calls.log");
    let last_stdin = tmp.path().join("last-stdin.json");
    let fake = write_script(
        tmp.path(),
        "fake-translator.sh",
        &good_translator(&calls, &last_stdin),
    );
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-app",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let report = &value["translate"];
    assert_eq!(report["contract"], "0.1.0");
    assert_eq!(report["healthy"], true);
    let outcome = &report["outcomes"][0];
    assert_eq!(outcome["locale"], "zh-CN");
    assert_eq!(outcome["status"], "translated");
    assert_eq!(outcome["review"], "ok");
    assert_eq!(outcome["derivative"], "docs/README.zh-CN.md");

    // The derivative carries the translation, the verbatim code
    // block and the preserved link destination.
    let derivative = fs::read_to_string(proj.join("docs/README.zh-CN.md")).unwrap();
    assert!(
        derivative.contains("ALPHA introduction paragraph"),
        "{derivative}"
    );
    assert!(derivative.contains("BETA section"), "{derivative}");
    assert!(
        derivative.contains("GAMMA closing paragraph"),
        "{derivative}"
    );
    assert!(
        derivative.contains("forge list --format json"),
        "{derivative}"
    );
    assert!(
        derivative.contains("https://example.com/manual"),
        "{derivative}"
    );

    // The state records the source hash and review status, and
    // the provider saw every text segment exactly once.
    let state_raw = fs::read_to_string(proj.join(".forge/docs/zh-CN/state.json")).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
    assert!(state["source_hash"].as_str().unwrap().len() >= 16);
    assert_eq!(state["review"], "ok");
    assert_eq!(state["segments"].as_object().unwrap().len(), 4);
    assert_eq!(call_count(&calls), 1);
    assert_eq!(
        last_request_segments(&last_stdin)["segments"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn unchanged_source_reports_current_without_provider_request() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = tmp.path().join("calls.log");
    let last_stdin = tmp.path().join("last-stdin.json");
    let fake = write_script(
        tmp.path(),
        "fake-translator.sh",
        &good_translator(&calls, &last_stdin),
    );
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-current",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );

    let first = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    assert_eq!(first.status.code(), Some(0), "{}", lossy(&first.stderr));
    assert_eq!(call_count(&calls), 1);
    let before = fs::read(proj.join("docs/README.zh-CN.md")).unwrap();

    // Point at a binary that would fail loudly if invoked: the
    // second run must not contact any provider.
    let bogus = tmp.path().join("definitely-not-a-translator");
    let second = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["zh-CN"],
    );
    assert_eq!(second.status.code(), Some(0), "{}", lossy(&second.stderr));
    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["zh-CN"],
    );
    assert_eq!(value["translate"]["outcomes"][0]["status"], "current");
    assert_eq!(call_count(&calls), 1, "no second provider request");
    assert_eq!(fs::read(proj.join("docs/README.zh-CN.md")).unwrap(), before);
}

#[test]
fn incremental_run_retranslates_only_changed_segments() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = tmp.path().join("calls.log");
    let last_stdin = tmp.path().join("last-stdin.json");
    let fake = write_script(
        tmp.path(),
        "fake-translator.sh",
        &good_translator(&calls, &last_stdin),
    );
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-incr",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );

    let first = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    assert_eq!(first.status.code(), Some(0), "{}", lossy(&first.stderr));

    // Change exactly one paragraph; the code block and the
    // other paragraphs are untouched.
    let v2 = SOURCE_V1.replace(
        "gamma closing paragraph.",
        "gamma closing paragraph with an update.",
    );
    write_file(&proj, "README.md", &v2);
    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["zh-CN"],
    );
    let outcome = &value["translate"]["outcomes"][0];
    assert_eq!(outcome["status"], "translated");
    assert_eq!(value["translate"]["healthy"], true);
    assert_eq!(call_count(&calls), 2);
    let request = last_request_segments(&last_stdin);
    let segments = request["segments"].as_array().unwrap();
    assert_eq!(segments.len(), 1, "{request}");
    assert!(segments[0]["text"].as_str().unwrap().contains("gamma"));
    assert_eq!(outcome["segments_translated"], 1);
    assert_eq!(outcome["segments_reused"], 3);
    let derivative = fs::read_to_string(proj.join("docs/README.zh-CN.md")).unwrap();
    assert!(
        derivative.contains("GAMMA closing paragraph with an update"),
        "{derivative}"
    );
    assert!(
        derivative.contains("forge list --format json"),
        "{derivative}"
    );
    assert!(
        derivative.contains("https://example.com/manual"),
        "{derivative}"
    );
}

#[test]
fn derivative_equal_to_source_fails_before_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-self",
        "    zh-CN:\n      enabled: true\n      path: README.md\n",
        SOURCE_V1,
    );
    let bogus = tmp.path().join("definitely-not-a-translator");
    let before = fs::read(proj.join("README.md")).unwrap();

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", lossy(&out.stdout));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[docs-invalid]"), "{stderr}");
    assert!(stderr.contains("canonical source"), "{stderr}");
    assert_eq!(fs::read(proj.join("README.md")).unwrap(), before);
    assert!(!proj.join(".forge/docs/zh-CN/state.json").exists());
}

#[test]
fn derivative_outside_project_fails_before_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-escape",
        "    zh-CN:\n      enabled: true\n      path: ../evil.md\n",
        SOURCE_V1,
    );
    let bogus = tmp.path().join("definitely-not-a-translator");

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[docs-invalid]"), "{stderr}");
    assert!(stderr.contains("outside the project"), "{stderr}");
    assert!(!tmp.path().join("evil.md").exists());
}

#[test]
fn disabled_locale_is_refused_and_all_skips_it() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = tmp.path().join("calls.log");
    let last_stdin = tmp.path().join("last-stdin.json");
    let fake = write_script(
        tmp.path(),
        "fake-translator.sh",
        &good_translator(&calls, &last_stdin),
    );
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-disabled",
        "    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
        SOURCE_V1,
    );

    // Explicit request for the disabled locale: refused, no
    // provider contact, no output.
    let out = run_translate(&db, &proj, &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)], &["fr"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[docs-invalid]"), "{stderr}");
    assert!(stderr.contains("is disabled"), "{stderr}");
    assert_eq!(call_count(&calls), 0);
    assert!(!proj.join("docs/README.fr.md").exists());

    // `--all` translates the enabled locale and never invokes
    // the disabled locale's provider or creates its output.
    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["--all"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &fake)],
        &["--all"],
    );
    assert_eq!(value["translate"]["locales"], serde_json::json!(["zh-CN"]));
    assert_eq!(call_count(&calls), 1);
    assert!(proj.join("docs/README.zh-CN.md").exists());
    assert!(!proj.join("docs/README.fr.md").exists());
    assert!(!proj.join(".forge/docs/fr").exists());
}

#[test]
fn unknown_locale_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-unknown",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );
    let bogus = tmp.path().join("definitely-not-a-translator");

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["fr"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[docs-invalid]"), "{stderr}");
    assert!(stderr.contains("unknown locale"), "{stderr}");
}

#[test]
fn provider_failure_keeps_prior_derivative_and_redacts_secrets() {
    let tmp = tempfile::tempdir().unwrap();
    let calls = tmp.path().join("calls.log");
    let last_stdin = tmp.path().join("last-stdin.json");
    let good = write_script(
        tmp.path(),
        "good-translator.sh",
        &good_translator(&calls, &last_stdin),
    );
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-fail",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );
    let first = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &good)],
        &["zh-CN"],
    );
    assert_eq!(first.status.code(), Some(0), "{}", lossy(&first.stderr));
    let prior_derivative = fs::read(proj.join("docs/README.zh-CN.md")).unwrap();
    let prior_state = fs::read(proj.join(".forge/docs/zh-CN/state.json")).unwrap();

    // Failing provider that leaks a credential-shaped secret on
    // stderr; the source changed so the provider path is taken.
    let failing = write_script(
        tmp.path(),
        "failing-translator.sh",
        "#!/bin/sh\necho 'translator blew up; token ghp_abcdefghijklmnopqrstuvwxyz0123456789' >&2\nexit 1\n",
    );
    write_file(&proj, "README.md", &SOURCE_V1.replace("gamma", "delta"));

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &failing)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[translation-failed]"), "{stderr}");
    // The leaked secret is redacted everywhere it surfaces.
    assert!(
        !stderr.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "{stderr}"
    );
    // Prior derivative and state are byte-identical.
    assert_eq!(
        fs::read(proj.join("docs/README.zh-CN.md")).unwrap(),
        prior_derivative
    );
    assert_eq!(
        fs::read(proj.join(".forge/docs/zh-CN/state.json")).unwrap(),
        prior_state
    );

    // Re-run in JSON format to inspect the structured per-locale
    // outcome and confirm the redacted evidence surfaces on stdout.
    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &failing)],
        &["zh-CN"],
    );
    assert_eq!(value["translate"]["healthy"], false);
    assert_eq!(value["translate"]["outcomes"][0]["status"], "failed");
    let rendered = serde_json::to_string(&value).unwrap();
    assert!(rendered.contains("[REDACTED]"), "{rendered}");
    assert!(
        !rendered.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        "{rendered}"
    );
}

#[test]
fn missing_translator_binary_fails_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-nobin",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );
    let bogus = tmp.path().join("definitely-not-a-translator");

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bogus)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(lossy(&out.stderr).contains("error[translation-failed]"));
    assert!(!proj.join("docs/README.zh-CN.md").exists());
    assert!(!proj.join(".forge/docs/zh-CN/state.json").exists());
}

#[test]
fn unparseable_translator_output_fails_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-badout",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );
    let bad = write_script(
        tmp.path(),
        "bad-translator.sh",
        "#!/bin/sh\necho 'not json'\n",
    );

    let out = run_translate(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &bad)],
        &["zh-CN"],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[translation-failed]"), "{stderr}");
    assert!(!proj.join("docs/README.zh-CN.md").exists());
}

#[test]
fn altered_links_and_terms_mark_needs_review() {
    let tmp = tempfile::tempdir().unwrap();
    let (db, proj) = setup_project(
        tmp.path(),
        "docs-review",
        "    zh-CN:\n      enabled: true\n",
        SOURCE_V1,
    );
    // Manifest gains an explicit non-translatable term; the
    // fake rewrites both the link destination and the term.
    write_file(
        &proj,
        "forge.yaml",
        &manifest(
            "docs-review",
            "docs:\n  source_language: en\n  non_translatable:\n    - Forge\n  translations:\n    zh-CN:\n      enabled: true\n",
        ),
    );
    let mangling = write_script(
        tmp.path(),
        "mangling-translator.sh",
        "#!/bin/sh\nsed 's|https://example.com/manual|https://example.com/changed|g; s/Forge/FORGE/g'\n",
    );

    let value = run_translate_json(
        &db,
        &proj,
        &[("FORGE_DOCS_TRANSLATOR_BIN", &mangling)],
        &["zh-CN"],
    );
    // Translation completes (exit 0) but quality is not
    // claimed: the review state names both violations.
    assert_eq!(value["translate"]["healthy"], true);
    let outcome = &value["translate"]["outcomes"][0];
    assert_eq!(outcome["status"], "translated");
    assert_eq!(outcome["review"], "needs-review");
    let reasons = outcome["review_reasons"].as_array().unwrap();
    assert_eq!(reasons.len(), 2, "{reasons:?}");
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("https://example.com/manual")));
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("Forge")));
    let state_raw = fs::read_to_string(proj.join(".forge/docs/zh-CN/state.json")).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
    assert_eq!(state["review"], "needs-review");
}

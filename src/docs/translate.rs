//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::constants::{
    DOCS_CONTRACT_VERSION, DOCS_STATE_DIR, STATUS_CURRENT, STATUS_FAILED, STATUS_TRANSLATED,
};
use super::model::{
    CapturedOutput, DocBlock, DocsConfig, LocaleConfigEntry, ReviewStatus, Segment,
    TranslateOutcome, TranslateReport, TranslateRequest, TranslatorConfig, TranslatorWireRequest,
    TranslatorWireResponse,
};
use super::source::{
    default_derivative_path, hash_bytes, load_translation_state, save_translation_state,
    segment_source, state_path_for, validate_output,
};

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem: absolute paths must already sit inside the
/// project, and `..` segments must never escape the root.
/// Symlinks are rechecked canonically at write time.
pub(super) fn lexically_contained(project_dir: &Path, rel: &str) -> bool {
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        return candidate.starts_with(project_dir);
    }
    let mut depth = 0i32;
    for component in candidate.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::Normal(_) => depth += 1,
        }
    }
    true
}

/// Resolve the canonical source file inside the project.
/// `project_dir` must already be canonical.
fn resolve_source_file(project_dir: &Path, source_rel: &str) -> Result<PathBuf, ForgeError> {
    if !lexically_contained(project_dir, source_rel) {
        return Err(ForgeError::DocsInvalid {
            reason: format!(
                "docs source `{source_rel}` resolves outside the project; keep the source inside the project directory"
            ),
        });
    }
    let path = project_dir.join(source_rel);
    if !path.is_file() {
        return Err(ForgeError::DocsInvalid {
            reason: format!(
                "documentation source `{source_rel}` was not found in the project; create it or set `docs.source`"
            ),
        });
    }
    Ok(path)
}

/// Resolve the derivative path inside the project. A
/// derivative that resolves to the source file or outside
/// the project is refused before any write (R1 failure
/// scenario). `project_dir` must already be canonical.
fn resolve_derivative_path(
    project_dir: &Path,
    source_rel: &str,
    derivative_rel: &str,
) -> Result<PathBuf, ForgeError> {
    if !lexically_contained(project_dir, derivative_rel) {
        return Err(ForgeError::DocsInvalid {
            reason: format!(
                "derivative path `{derivative_rel}` resolves outside the project; keep derivatives inside the project directory"
            ),
        });
    }
    let source_path = project_dir.join(source_rel);
    let derivative_path = project_dir.join(derivative_rel);
    // Compare lexically first (cheap), canonically at write
    // time (symlink-safe). Either match refuses the write.
    if derivative_path == source_path {
        return Err(ForgeError::DocsInvalid {
            reason: format!(
                "derivative path `{derivative_rel}` resolves to the canonical source file; derivatives must live in explicit derivative paths"
            ),
        });
    }
    Ok(derivative_path)
}

/// Run the provider with the request JSON on stdin. The
/// process starts via `Command::new(binary)` with an argument
/// array — never through a shell — and runs with the project
/// directory as its working directory so it can only observe
/// the project it was asked to translate.
fn run_with_stdin_timeout(
    binary: &OsString,
    args: &[&str],
    project_dir: &Path,
    stdin_bytes: &[u8],
    timeout: Duration,
) -> Result<CapturedOutput, String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut command = Command::new(binary);
    for arg in args {
        command.arg(arg);
    }
    command.current_dir(project_dir);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                "binary not found on PATH".to_string()
            } else {
                format!("spawn failed: {err}")
            }
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = crate::process::write_request(&mut stdin, stdin_bytes) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("translator stdin write failed: {err}"));
        }
    }
    let start = std::time::SystemTime::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(CapturedOutput {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed().unwrap_or_default() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "translator exceeded timeout of {} seconds",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("wait failed: {err}")),
        }
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    let text = redact_docs_evidence(&String::from_utf8_lossy(stderr));
    let trimmed = text.trim();
    if trimmed.is_empty() {
        "no stderr output".to_string()
    } else {
        const MAX: usize = 200;
        if trimmed.len() > MAX {
            format!("{}…", &trimmed[..MAX])
        } else {
            trimmed.to_string()
        }
    }
}

/// Invoke the translation provider for the changed segments.
/// Returns the translated text keyed by segment id. Any
/// provider failure is a typed `TranslationFailed` error; the
/// caller keeps the prior derivative and state intact.
pub fn run_translator(
    config: &TranslatorConfig,
    project_dir: &Path,
    locale: &str,
    source_language: &str,
    segments: &[Segment],
) -> Result<BTreeMap<String, String>, ForgeError> {
    if segments.is_empty() {
        return Ok(BTreeMap::new());
    }
    let wire = TranslatorWireRequest {
        contract: DOCS_CONTRACT_VERSION.to_string(),
        locale: locale.to_string(),
        source_language: source_language.to_string(),
        segments: segments.to_vec(),
    };
    let stdin_bytes = serde_json::to_vec(&wire).map_err(|err| ForgeError::TranslationFailed {
        reason: format!("cannot serialize translator request: {err}"),
    })?;
    let args = [
        "translate",
        "--locale",
        locale,
        "--source-language",
        source_language,
    ];
    let output = run_with_stdin_timeout(
        &config.binary,
        &args,
        project_dir,
        &stdin_bytes,
        config.timeout,
    )
    .map_err(|reason| ForgeError::TranslationFailed {
        reason: format!("translator invocation failed: {reason}"),
    })?;
    if !output.status.success() {
        return Err(ForgeError::TranslationFailed {
            reason: format!(
                "translator exited with status {}: {}",
                output.status,
                stderr_summary(&output.stderr)
            ),
        });
    }
    let raw = String::from_utf8(output.stdout).map_err(|_| ForgeError::TranslationFailed {
        reason: "translator stdout was not valid UTF-8".to_string(),
    })?;
    let response: TranslatorWireResponse =
        serde_json::from_str(&raw).map_err(|err| ForgeError::TranslationFailed {
            reason: format!(
                "translator output is not parseable as a translation response: {err}; output: {}",
                redact_docs_evidence(&raw.chars().take(200).collect::<String>())
            ),
        })?;
    if response.contract != DOCS_CONTRACT_VERSION {
        return Err(ForgeError::TranslationFailed {
            reason: format!(
                "translator contract mismatch: expected `{}`, got `{}`",
                DOCS_CONTRACT_VERSION, response.contract
            ),
        });
    }
    let mut out = BTreeMap::new();
    for segment in response.segments {
        out.insert(segment.id, segment.text);
    }
    let missing: Vec<String> = segments
        .iter()
        .map(|s| s.id.clone())
        .filter(|id| !out.contains_key(id))
        .collect();
    if !missing.is_empty() {
        return Err(ForgeError::TranslationFailed {
            reason: format!(
                "translator omitted {} segment(s): {}; prior derivative left intact",
                missing.len(),
                missing.join(", ")
            ),
        });
    }
    Ok(out)
}

/// Redact credential-like substrings from a piece of evidence.
/// Delegated to [`crate::policy::redact_credentials`] so the
/// policy and docs contracts share one definition of "secret".
pub fn redact_docs_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Translate one enabled locale. Validation (paths, source
/// presence) runs before any provider contact or write; on a
/// provider failure or a source change during generation the
/// prior derivative and state are preserved and the outcome
/// reports `failed` instead of throwing the run away.
pub fn translate_locale(
    project_dir: &Path,
    config: &DocsConfig,
    entry: &LocaleConfigEntry,
    translator: &TranslatorConfig,
) -> Result<TranslateOutcome, ForgeError> {
    let derivative_rel = entry
        .path
        .clone()
        .unwrap_or_else(|| default_derivative_path(&config.source, &entry.locale));
    let source_path = resolve_source_file(project_dir, &config.source)?;
    let derivative_path = resolve_derivative_path(project_dir, &config.source, &derivative_rel)?;
    let state_path = state_path_for(project_dir, &entry.locale)?;
    let mut state = load_translation_state(&state_path)?;

    let source_bytes = fs::read(&source_path).map_err(|err| ForgeError::DocsInvalid {
        reason: format!(
            "cannot read documentation source `{}`: {err}",
            config.source
        ),
    })?;
    let source_hash = hash_bytes(&source_bytes);
    let source_text = String::from_utf8_lossy(&source_bytes).to_string();

    // Boundary scenario: the recorded hash already matches and
    // the derivative exists, so report `current` without
    // repeating a provider request.
    if state.source_hash == source_hash
        && !state.source_hash.is_empty()
        && derivative_path.is_file()
    {
        let review = state.review.unwrap_or(ReviewStatus::Ok);
        return Ok(TranslateOutcome {
            locale: entry.locale.clone(),
            status: STATUS_CURRENT.to_string(),
            source_hash: Some(source_hash.clone()),
            segments_translated: 0,
            segments_reused: state.segments.len(),
            derivative: Some(derivative_rel),
            review: review.label().to_string(),
            review_reasons: state.review_reasons.clone(),
            note: format!(
                "locale `{}` is current at source hash `{}`; no provider request was made",
                entry.locale,
                short_hash(&source_hash)
            ),
            evidence: vec![format!("source_hash: {source_hash}")],
            recovery: Vec::new(),
        });
    }

    let blocks = segment_source(&source_text);
    // Partition text segments into changed (provider must
    // translate) and reusable (content hash already known).
    // Code blocks bypass the provider entirely.
    let mut changed: Vec<Segment> = Vec::new();
    let mut changed_hashes: Vec<String> = Vec::new();
    let mut reused = 0usize;
    for block in &blocks {
        if let DocBlock::Text { id, text } = block {
            let hash = hash_bytes(text.as_bytes());
            if state.segments.contains_key(&hash) {
                reused += 1;
            } else {
                changed_hashes.push(hash);
                changed.push(Segment {
                    id: id.clone(),
                    text: text.clone(),
                });
            }
        }
    }

    let mut translated: BTreeMap<String, String> = BTreeMap::new();
    if !changed.is_empty() {
        let by_id = match run_translator(
            translator,
            project_dir,
            &entry.locale,
            &config.source_language,
            &changed,
        ) {
            Ok(map) => map,
            // Expected domain failure: keep the typed outcome
            // (prior derivative intact) instead of an opaque
            // error so `--all` can continue with the remaining
            // locales.
            Err(ForgeError::TranslationFailed { reason }) => {
                return Ok(TranslateOutcome {
                    locale: entry.locale.clone(),
                    status: STATUS_FAILED.to_string(),
                    source_hash: None,
                    segments_translated: 0,
                    segments_reused: reused,
                    derivative: None,
                    review: ReviewStatus::Ok.label().to_string(),
                    review_reasons: Vec::new(),
                    note: format!(
                        "translation for locale `{}` failed; prior derivative left intact",
                        entry.locale
                    ),
                    evidence: vec![redact_docs_evidence(&reason)],
                    recovery: vec![
                        "verify the translator binary and re-run the translation".to_string(),
                        "the previous derivative and state were preserved; no rollback is needed"
                            .to_string(),
                    ],
                });
            }
            Err(other) => return Err(other),
        };
        for (segment, hash) in changed.iter().zip(changed_hashes.iter()) {
            if let Some(text) = by_id.get(&segment.id) {
                translated.insert(hash.clone(), text.clone());
            }
        }
    }

    // Failure scenario: the source changed while the provider
    // was running. Refuse to mark the merge current and keep
    // the prior derivative and state intact.
    let recheck_bytes = fs::read(&source_path).map_err(|err| ForgeError::DocsInvalid {
        reason: format!(
            "cannot re-read documentation source `{}`: {err}",
            config.source
        ),
    })?;
    let recheck_hash = hash_bytes(&recheck_bytes);
    if recheck_hash != source_hash {
        return Ok(TranslateOutcome {
            locale: entry.locale.clone(),
            status: STATUS_FAILED.to_string(),
            source_hash: None,
            segments_translated: 0,
            segments_reused: reused,
            derivative: None,
            review: ReviewStatus::Ok.label().to_string(),
            review_reasons: Vec::new(),
            note: format!(
                "source `{}` changed during generation for locale `{}`; prior derivative left intact",
                config.source, entry.locale
            ),
            evidence: vec![format!("source_hash changed from {source_hash}")],
            recovery: vec![format!(
                "re-run `forge docs translate {}` against the settled source",
                entry.locale
            )],
        });
    }

    // Merge in source order: translated text for changed
    // segments, reused translations for unchanged ones,
    // verbatim code everywhere else.
    let mut table = state.segments.clone();
    table.extend(translated);
    let mut merged: Vec<String> = Vec::new();
    for block in &blocks {
        match block {
            DocBlock::Text { text, .. } => {
                let hash = hash_bytes(text.as_bytes());
                match table.get(&hash) {
                    Some(t) => merged.push(t.clone()),
                    None => {
                        return Err(ForgeError::TranslationFailed {
                            reason: format!(
                                "missing translation for a changed segment of locale `{}`; prior derivative left intact",
                                entry.locale
                            ),
                        })
                    }
                }
            }
            DocBlock::Code { text } => merged.push(text.clone()),
        }
    }
    let mut output = merged.join("\n\n");
    output.push('\n');

    let review_reasons = validate_output(&source_text, &output, &config.non_translatable);
    let review = if review_reasons.is_empty() {
        ReviewStatus::Ok
    } else {
        ReviewStatus::NeedsReview
    };

    // Stage, recheck containment canonically (symlink-safe),
    // then publish atomically. The derivative appears only
    // after the source hash was rechecked and the output
    // validated.
    if let Some(parent) = derivative_path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::TranslationFailed {
            reason: format!(
                "cannot create derivative directory {}: {err}",
                parent.display()
            ),
        })?;
        let canonical_parent =
            parent
                .canonicalize()
                .map_err(|err| ForgeError::TranslationFailed {
                    reason: format!(
                        "cannot resolve derivative directory {}: {err}",
                        parent.display()
                    ),
                })?;
        if !canonical_parent.starts_with(project_dir) {
            return Err(ForgeError::DocsInvalid {
                reason: format!(
                    "derivative path `{derivative_rel}` escapes the project; refusing to write"
                ),
            });
        }
        let canonical_source = source_path
            .canonicalize()
            .unwrap_or_else(|_| source_path.clone());
        let canonical_derivative =
            canonical_parent.join(derivative_path.file_name().unwrap_or_default());
        if canonical_derivative == canonical_source {
            return Err(ForgeError::DocsInvalid {
                reason: format!(
                    "derivative path `{derivative_rel}` resolves to the canonical source file; derivatives must live in explicit derivative paths"
                ),
            });
        }
    }
    let tmp = derivative_path.with_extension("md.tmp");
    fs::write(&tmp, output.as_bytes()).map_err(|err| ForgeError::TranslationFailed {
        reason: format!("cannot stage derivative {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, &derivative_path).map_err(|err| ForgeError::TranslationFailed {
        reason: format!(
            "cannot publish derivative {}: {err}",
            derivative_path.display()
        ),
    })?;

    state.contract = DOCS_CONTRACT_VERSION.to_string();
    state.locale = entry.locale.clone();
    state.source = config.source.clone();
    state.path = derivative_rel.clone();
    state.source_hash = source_hash.clone();
    state.source_language = config.source_language.clone();
    state.review = Some(review);
    state.review_reasons = review_reasons.clone();
    state.segments = table;
    state.translated_at = chrono::Utc::now().to_rfc3339();
    save_translation_state(&state_path, &state).map_err(|err| match err {
        ForgeError::DocsInvalid { reason } => ForgeError::TranslationFailed {
            reason: format!(
                "derivative written but state not recorded: {reason}; re-run the translation to reconcile"
            ),
        },
        other => other,
    })?;

    let changed_count = changed.len();
    Ok(TranslateOutcome {
        locale: entry.locale.clone(),
        status: STATUS_TRANSLATED.to_string(),
        source_hash: Some(source_hash.clone()),
        segments_translated: changed_count,
        segments_reused: reused,
        derivative: Some(derivative_rel.clone()),
        review: review.label().to_string(),
        review_reasons,
        note: format!(
            "locale `{}` translated to `{derivative_rel}` at source hash `{}` ({changed_count} segment(s) translated, {reused} reused)",
            entry.locale,
            short_hash(&source_hash)
        ),
        evidence: vec![format!("source_hash: {source_hash}")],
        recovery: Vec::new(),
    })
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(12).collect()
}

/// Run a translate request: one explicit locale or `--all`
/// enabled locales. Unknown locales, disabled explicit
/// locales and an `--all` with no enabled locale are refused
/// before any provider contact or write.
pub fn run_translate(
    project_dir: &Path,
    config: &DocsConfig,
    request: &TranslateRequest,
    translator: &TranslatorConfig,
) -> Result<TranslateReport, ForgeError> {
    request.validate()?;
    let canonical = project_dir
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: project_dir.display().to_string(),
        })?;
    let mut outcomes: Vec<TranslateOutcome> = Vec::new();
    let mut locales: Vec<String> = Vec::new();
    if request.all {
        let enabled = config.enabled_locales();
        if enabled.is_empty() {
            return Err(ForgeError::DocsInvalid {
                reason: "no enabled translation locales; set `enabled: true` under `docs.translations.<locale>`"
                    .to_string(),
            });
        }
        for entry in enabled {
            locales.push(entry.locale.clone());
            outcomes.push(translate_locale(&canonical, config, entry, translator)?);
        }
    } else if let Some(locale) = request.locale.as_deref() {
        match config.find_locale(locale) {
            None => {
                let known: Vec<String> = config.locales.iter().map(|l| l.locale.clone()).collect();
                return Err(ForgeError::DocsInvalid {
                    reason: if known.is_empty() {
                        format!(
                            "unknown locale `{locale}`; the manifest declares no `docs.translations` entries"
                        )
                    } else {
                        format!(
                            "unknown locale `{locale}`; known locales: {}",
                            known.join(", ")
                        )
                    },
                });
            }
            Some(entry) if !entry.enabled => {
                return Err(ForgeError::DocsInvalid {
                    reason: format!(
                        "locale `{locale}` is disabled; set `docs.translations.{locale}.enabled: true` to translate it"
                    ),
                });
            }
            Some(entry) => {
                locales.push(entry.locale.clone());
                outcomes.push(translate_locale(
                    &canonical,
                    config,
                    &entry.clone(),
                    translator,
                )?);
            }
        }
    }
    let translated = outcomes
        .iter()
        .filter(|o| o.status == STATUS_TRANSLATED)
        .count();
    let current = outcomes
        .iter()
        .filter(|o| o.status == STATUS_CURRENT)
        .count();
    let failed = outcomes
        .iter()
        .filter(|o| o.status == STATUS_FAILED)
        .count();
    let note = if failed == 0 {
        format!(
            "docs translate complete for project `{}`: {translated} translated, {current} current",
            request.project_id
        )
    } else {
        format!(
            "docs translate partial for project `{}`: {translated} translated, {current} current, {failed} failed; prior derivatives left intact",
            request.project_id
        )
    };
    let mut report = TranslateReport {
        contract: DOCS_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        source: config.source.clone(),
        source_language: config.source_language.clone(),
        locales,
        all: request.all,
        state_dir: canonical.join(DOCS_STATE_DIR).display().to_string(),
        outcomes,
        note,
        healthy: false,
    };
    report.healthy = report.healthy_check();
    Ok(report)
}

/// Render a [`TranslateReport`] for human output with stable
/// status labels.
pub fn render_report_human(report: &TranslateReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!(
        "source: {} ({})",
        report.source, report.source_language
    ));
    lines.push(format!("locales: {}", report.locales.join(", ")));
    if report.all {
        lines.push("mode: all".to_string());
    }
    lines.push(format!("state: {}", report.state_dir));
    for outcome in &report.outcomes {
        let derivative = outcome
            .derivative
            .clone()
            .map(|d| format!(" derivative={d}"))
            .unwrap_or_default();
        lines.push(format!(
            "  - {locale} {status}{derivative}: {note}",
            locale = outcome.locale,
            status = outcome.status,
            note = outcome.note
        ));
        if !outcome.review_reasons.is_empty() {
            lines.push(format!("      review: {}", outcome.review));
            for reason in &outcome.review_reasons {
                lines.push(format!("      review-reason: {reason}"));
            }
        }
        for line in &outcome.evidence {
            lines.push(format!("      evidence: {line}"));
        }
        for line in &outcome.recovery {
            lines.push(format!("      recovery: {line}"));
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

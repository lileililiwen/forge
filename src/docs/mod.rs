//! Derivative documentation and source-hash freshness
//! (`documentation-translation`).
//!
//! Core owns the typed docs contract. v0.5 supports explicit locale
//! enablement, derivative paths, source-hash tracking and incremental
//! translation with a bundled provider adapter:
//!
//! - The manifest's `docs` section declares the canonical `source`
//!   document (default `README.md`), the `source_language` and one
//!   entry per locale under `translations`. A locale translates
//!   only when it carries `enabled: true`; an omitted or `false`
//!   flag keeps the locale disabled so translation is never
//!   enabled by default.
//! - `translate` splits the source into text segments and fenced
//!   code blocks. Code blocks are never sent to the provider and
//!   are reinserted verbatim; only changed text segments are
//!   requested, keyed by content hash so reordered or inserted
//!   paragraphs do not force a full retranslation.
//! - The provider is an external binary (default
//!   `forge-docs-translator`, overridable via
//!   `FORGE_DOCS_TRANSLATOR_BIN`) invoked with an argument array
//!   — never a shell — receiving the changed segments as JSON on
//!   stdin and returning translated segments as JSON on stdout.
//!   A missing binary, non-zero exit, timeout or unparseable
//!   output is a typed `translation-failed` outcome; the prior
//!   derivative and state are left intact.
//! - Output is merged to a staging file, the source hash is
//!   rechecked, preservation validators run (link destinations,
//!   explicit non-translatable terms, verbatim code blocks) and
//!   only then is the derivative atomically published and the
//!   state recorded with the new source hash plus a review
//!   status (`ok` or `needs-review`). Provider success alone
//!   never claims translation quality.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §28, §34, §43 require
//! derivative documentation with source-hash freshness and
//! incremental translation. The contract stays independent of any
//! real translation provider: a contract fixture proves
//! normalization and error mapping but cannot alone establish a
//! working external integration, matching the existing
//! DriftWatch adapter boundary.
//!
//! ## Persistence
//!
//! [`TranslationState`] is stored under
//! `.forge/docs/<locale>/state.json` so a re-run can reuse
//! unchanged segments and report `current` without contacting
//! the provider. The state is local evidence, not a record of
//! authority: a successful run overwrites the previous entry,
//! a failed run leaves it untouched. The Core registry's
//! `operations` table receives one `docs` row per run with a
//! `done`/`partial` summary that lists the per-locale statuses.
//!
//! ## Risk model
//!
//! AI translation can alter commands or links. Fenced code
//! blocks bypass the provider entirely, link destinations and
//! explicit non-translatable terms are rechecked after every
//! merge, and any violation marks the derivative `needs-review`
//! with the exact reason instead of failing silently. A
//! derivative path that resolves to the source file or outside
//! the project is refused before any write, and a disabled
//! locale is never translated — not even by `--all`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::manifest::{DocsMeta, Manifest};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Contract data version for the docs surface. The provider
/// adapter speaks the same version on stdin/stdout.
pub const DOCS_CONTRACT_VERSION: &str = "0.1.0";

/// Docs state subdirectory inside the project. Each locale owns
/// `<project>/.forge/docs/<locale>/state.json`.
pub const DOCS_STATE_DIR: &str = ".forge/docs";

/// Canonical source document when the manifest's `docs.source`
/// is absent.
pub const DEFAULT_SOURCE: &str = "README.md";

/// Default translator binary. Overridable per run through
/// `FORGE_DOCS_TRANSLATOR_BIN` so contract fixtures can stand
/// in for a real provider.
pub const DEFAULT_TRANSLATOR_BIN: &str = "forge-docs-translator";

/// Environment variable selecting the translator binary.
pub const TRANSLATOR_BIN_ENV: &str = "FORGE_DOCS_TRANSLATOR_BIN";

/// Per-run translator timeout. The adapter uses spawn plus a
/// bounded wait so an unresponsive provider cannot hang the
/// registry.
pub const TRANSLATOR_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable per-locale outcome statuses.
pub const STATUS_TRANSLATED: &str = "translated";
pub const STATUS_CURRENT: &str = "current";
pub const STATUS_FAILED: &str = "failed";

/// Validate a locale tag (`zh-CN`, `en`, `pt-BR`, ...). Tags are
/// treated as data: separators, dots and whitespace are refused
/// so a locale can never smuggle a path into the state layout.
pub fn validate_locale(raw: &str) -> Result<String, ForgeError> {
    if raw.trim().is_empty() {
        return Err(ForgeError::DocsInvalid {
            reason: "locale must not be empty; pass e.g. `zh-CN`".to_string(),
        });
    }
    let mut parts = raw.split('-');
    let first = parts.next().unwrap_or_default();
    if !(2..=8).contains(&first.len()) || !first.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(ForgeError::DocsInvalid {
            reason: format!("invalid locale `{raw}`; expected a language tag like `en` or `zh-CN`"),
        });
    }
    for part in parts {
        if part.is_empty() || part.len() > 8 || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ForgeError::DocsInvalid {
                reason: format!(
                    "invalid locale `{raw}`; expected a language tag like `en` or `zh-CN`"
                ),
            });
        }
    }
    Ok(raw.to_string())
}

/// One configured locale from the manifest. `enabled` defaults
/// to `false` when omitted: translation is opt-in per locale
/// and never enabled by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocaleConfigEntry {
    pub locale: String,
    pub enabled: bool,
    pub path: Option<String>,
}

/// Validated docs config parsed from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocsConfig {
    pub source: String,
    pub source_language: String,
    pub locales: Vec<LocaleConfigEntry>,
    pub non_translatable: Vec<String>,
}

impl DocsConfig {
    /// Parse the raw [`DocsMeta`] into a typed configuration.
    /// Locales are kept in manifest (sorted) order. Unknown or
    /// malformed locale tags and empty non-translatable terms
    /// are refused before any caller acts on the config.
    pub fn from_manifest_meta(meta: &DocsMeta) -> Result<Self, ForgeError> {
        let source = meta
            .source
            .clone()
            .unwrap_or_else(|| DEFAULT_SOURCE.to_string());
        if source.trim().is_empty() {
            return Err(ForgeError::DocsInvalid {
                reason: "docs.source must not be empty".to_string(),
            });
        }
        let mut locales = Vec::new();
        for (locale, entry) in &meta.translations {
            let locale = validate_locale(locale)?;
            if let Some(path) = entry.path.as_deref() {
                if path.trim().is_empty() {
                    return Err(ForgeError::DocsInvalid {
                        reason: format!("docs.translations.{locale}.path must not be empty"),
                    });
                }
            }
            locales.push(LocaleConfigEntry {
                locale,
                enabled: entry.enabled.unwrap_or(false),
                path: entry.path.clone(),
            });
        }
        for term in &meta.non_translatable {
            if term.trim().is_empty() {
                return Err(ForgeError::DocsInvalid {
                    reason: "docs.non_translatable must not contain empty terms".to_string(),
                });
            }
        }
        Ok(DocsConfig {
            source,
            source_language: meta
                .source_language
                .clone()
                .unwrap_or_else(|| "en".to_string()),
            locales,
            non_translatable: meta.non_translatable.clone(),
        })
    }

    /// Enabled locales in declaration order. Disabled locales
    /// are never translated: explicit requests are refused and
    /// `--all` skips them without invoking any provider.
    pub fn enabled_locales(&self) -> Vec<&LocaleConfigEntry> {
        self.locales.iter().filter(|l| l.enabled).collect()
    }

    pub fn find_locale(&self, locale: &str) -> Option<&LocaleConfigEntry> {
        self.locales.iter().find(|l| l.locale == locale)
    }
}

/// Build a [`DocsConfig`] from a manifest's docs section.
/// Convenience for callers that already have the manifest.
pub fn docs_config_from_manifest(manifest: &Manifest) -> Result<DocsConfig, ForgeError> {
    match manifest.docs.as_ref() {
        Some(meta) => DocsConfig::from_manifest_meta(meta),
        None => Err(ForgeError::DocsInvalid {
            reason: format!(
                "project `{}` has no `docs` section; declare `docs.translations` with at least one enabled locale",
                manifest.project.id
            ),
        }),
    }
}

/// Default derivative path for a source + locale pair:
/// `README.md` + `zh-CN` becomes `docs/README.zh-CN.md`,
/// matching the brief's source-of-truth model. The derivative
/// always lives under `docs/` so it can never collide with
/// the canonical source by default.
pub fn default_derivative_path(source: &str, locale: &str) -> String {
    let file = source.rsplit('/').next().unwrap_or(source);
    match file.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => {
            format!("docs/{stem}.{locale}.{ext}")
        }
        _ => format!("docs/{file}.{locale}.md"),
    }
}

/// Hex SHA-256 over bytes. Used for source revisions and
/// segment identity.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// One documentation block. Text blocks are translated;
/// fenced code blocks bypass the provider and are reinserted
/// verbatim on merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocBlock {
    Text { id: String, text: String },
    Code { text: String },
}

/// Split markdown source into translatable text segments and
/// verbatim code blocks. Paragraphs are runs of non-blank
/// lines outside fenced code; fences (``` lines) toggle code
/// mode and belong to the code block. Segment ids (`seg-000`,
/// ...) are positional among text blocks; incremental reuse
/// is keyed by content hash, not by id, so insertions and
/// reorderings do not force a full retranslation.
pub fn segment_source(text: &str) -> Vec<DocBlock> {
    let normalized = text.replace("\r\n", "\n");
    let mut blocks: Vec<DocBlock> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Vec<&str> = Vec::new();
    let mut in_code = false;
    let mut text_index = 0usize;

    let flush_paragraph =
        |paragraph: &mut Vec<&str>, blocks: &mut Vec<DocBlock>, text_index: &mut usize| {
            // Drop whitespace-only paragraphs so empty gaps never
            // become translation segments.
            let joined = paragraph.join("\n");
            if !joined.trim().is_empty() {
                blocks.push(DocBlock::Text {
                    id: format!("seg-{index:03}", index = *text_index),
                    text: joined,
                });
                *text_index += 1;
            }
            paragraph.clear();
        };

    for line in normalized.split('\n') {
        if line.trim_start().starts_with("```") {
            if !in_code {
                flush_paragraph(&mut paragraph, &mut blocks, &mut text_index);
                in_code = true;
                code.push(line);
            } else {
                code.push(line);
                in_code = false;
                blocks.push(DocBlock::Code {
                    text: code.join("\n"),
                });
                code.clear();
            }
            continue;
        }
        if in_code {
            code.push(line);
            continue;
        }
        if line.trim().is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks, &mut text_index);
        } else {
            paragraph.push(line);
        }
    }
    flush_paragraph(&mut paragraph, &mut blocks, &mut text_index);
    if !code.is_empty() {
        // Unclosed fence: the tail is code, preserved verbatim.
        blocks.push(DocBlock::Code {
            text: code.join("\n"),
        });
    }
    blocks
}

/// Extract inline markdown link destinations (`[text](dest)`
/// and `![alt](dest)`). Destinations are preservation-checked
/// after every merge so a translating provider cannot silently
/// rewrite commands or links.
pub fn extract_link_destinations(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == ']' && chars[i + 1] == '(' {
            let mut j = i + 2;
            let mut dest = String::new();
            while j < chars.len() && chars[j] != ')' {
                dest.push(chars[j]);
                j += 1;
            }
            if j < chars.len() {
                let dest = dest.trim().to_string();
                if !dest.is_empty() && !out.contains(&dest) {
                    out.push(dest);
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Recheck preservation after a merge. Returns the list of
/// review reasons; empty means `ok`. Code blocks are
/// reinserted verbatim by construction, so this verifies link
/// destinations and explicit non-translatable terms only.
pub fn validate_output(
    source_text: &str,
    output_text: &str,
    non_translatable: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();
    for dest in extract_link_destinations(source_text) {
        if !output_text.contains(&dest) {
            reasons.push(format!("link destination `{dest}` was altered or dropped"));
        }
    }
    for term in non_translatable {
        if source_text.contains(term) && !output_text.contains(term) {
            reasons.push(format!("non-translatable term `{term}` is missing"));
        }
    }
    reasons
}

/// Review status recorded on the derivative state. `ok` means
/// every preservation check passed; `needs-review` names the
/// exact violation so a human — not provider success — judges
/// translation quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewStatus {
    Ok,
    NeedsReview,
}

impl ReviewStatus {
    pub fn label(self) -> &'static str {
        match self {
            ReviewStatus::Ok => "ok",
            ReviewStatus::NeedsReview => "needs-review",
        }
    }
}

/// Persisted per-locale translation state. `segments` maps
/// source-segment content hashes to their translations so a
/// re-run reuses unchanged segments without contacting the
/// provider. The table is local evidence: a successful run
/// overwrites it, a failed run leaves it untouched.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationState {
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub source_hash: String,
    #[serde(default)]
    pub source_language: String,
    #[serde(default)]
    pub review: Option<ReviewStatus>,
    #[serde(default)]
    pub review_reasons: Vec<String>,
    #[serde(default)]
    pub segments: BTreeMap<String, String>,
    #[serde(default)]
    pub translated_at: String,
}

/// Compute the on-disk state path for a locale. The state is
/// scoped to the project's directory plus the validated
/// locale tag so one locale can never read another's state.
pub fn state_path_for(project_dir: &Path, locale: &str) -> Result<PathBuf, ForgeError> {
    let locale = validate_locale(locale)?;
    Ok(project_dir
        .join(DOCS_STATE_DIR)
        .join(locale)
        .join("state.json"))
}

pub fn load_translation_state(path: &Path) -> Result<TranslationState, ForgeError> {
    if !path.exists() {
        return Ok(TranslationState::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::DocsInvalid {
        reason: format!("cannot read docs state {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(TranslationState::default());
    }
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::DocsInvalid {
        reason: format!("docs state at {} is not valid JSON: {err}", path.display()),
    })
}

pub fn save_translation_state(path: &Path, state: &TranslationState) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::DocsInvalid {
            reason: format!(
                "cannot create docs state directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|err| ForgeError::DocsInvalid {
        reason: format!("cannot serialize docs state: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::DocsInvalid {
        reason: format!("cannot write docs state tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::DocsInvalid {
        reason: format!("cannot rename docs state {}: {err}", path.display()),
    })?;
    Ok(())
}

/// Lexically confine `rel` to `project_dir` without touching
/// the filesystem: absolute paths must already sit inside the
/// project, and `..` segments must never escape the root.
/// Symlinks are rechecked canonically at write time.
fn lexically_contained(project_dir: &Path, rel: &str) -> bool {
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

/// Translation execution request. The CLI builds this from its
/// input args; Core owns the validation and the per-locale
/// execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslateRequest {
    pub project_id: String,
    pub locale: Option<String>,
    pub all: bool,
}

impl TranslateRequest {
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.project_id.trim().is_empty() {
            return Err(ForgeError::DocsInvalid {
                reason: "project id must not be empty".to_string(),
            });
        }
        match (&self.locale, self.all) {
            (Some(_), true) => Err(ForgeError::DocsInvalid {
                reason: "pass either a locale or `--all`, not both".to_string(),
            }),
            (None, false) => Err(ForgeError::DocsInvalid {
                reason: "a locale is required; pass e.g. `zh-CN` or `--all`".to_string(),
            }),
            (Some(locale), false) => {
                validate_locale(locale)?;
                Ok(())
            }
            (None, true) => Ok(()),
        }
    }
}

/// Configuration for the translation provider adapter. The CLI
/// fills this from `FORGE_DOCS_TRANSLATOR_BIN` plus the
/// per-run timeout, then hands it to the translate flow.
#[derive(Debug, Clone)]
pub struct TranslatorConfig {
    pub binary: OsString,
    pub timeout: Duration,
}

impl TranslatorConfig {
    /// Resolve the adapter binary from the environment,
    /// falling back to [`DEFAULT_TRANSLATOR_BIN`].
    /// Empty or whitespace-only values keep the default.
    pub fn from_env() -> Self {
        let binary = std::env::var_os(TRANSLATOR_BIN_ENV)
            .filter(|v| !v.to_string_lossy().trim().is_empty())
            .unwrap_or_else(|| OsString::from(DEFAULT_TRANSLATOR_BIN));
        TranslatorConfig {
            binary,
            timeout: TRANSLATOR_TIMEOUT,
        }
    }
}

/// One translatable segment sent to the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
struct TranslatorWireRequest {
    contract: String,
    locale: String,
    source_language: String,
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Deserialize)]
struct TranslatorWireResponse {
    #[serde(default)]
    contract: String,
    #[serde(default)]
    segments: Vec<Segment>,
}

struct CapturedOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
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
    use std::io::{Read, Write};
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
        if let Err(err) = stdin.write_all(stdin_bytes) {
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

/// Per-locale translation result. Statuses are stable:
/// - `translated`: the derivative was regenerated and the
///   state now records the current source hash plus review
/// - `current`: the recorded source hash already matches and
///   the derivative exists, so no provider was contacted
/// - `failed`: the provider failed or the source changed
///   during generation; prior state is intact
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslateOutcome {
    pub locale: String,
    pub status: String,
    pub source_hash: Option<String>,
    pub segments_translated: usize,
    pub segments_reused: usize,
    pub derivative: Option<String>,
    pub review: String,
    pub review_reasons: Vec<String>,
    pub note: String,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
}

/// Aggregate translate report. The report is the typed outcome
/// the CLI renders; Core owns every field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslateReport {
    pub contract: String,
    pub project_id: String,
    pub source: String,
    pub source_language: String,
    pub locales: Vec<String>,
    pub all: bool,
    pub state_dir: String,
    pub outcomes: Vec<TranslateOutcome>,
    pub note: String,
    /// Aggregate health. The CLI renders this field directly so
    /// a caller can read the verdict without re-evaluating the
    /// per-locale outcomes.
    pub healthy: bool,
}

impl TranslateReport {
    /// `true` when every locale is `translated` or `current`.
    /// A run with no outcomes is not healthy: the request was
    /// meaningless.
    pub fn healthy(&self) -> bool {
        self.healthy_check()
    }

    fn healthy_check(&self) -> bool {
        if self.outcomes.is_empty() {
            return false;
        }
        self.outcomes
            .iter()
            .all(|o| o.status == STATUS_TRANSLATED || o.status == STATUS_CURRENT)
    }
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

/// Freshness of one enabled locale for doctor consumption.
/// Doctor maps these to typed findings; the docs module never
/// constructs doctor findings itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessStatus {
    Current { review: ReviewStatus },
    Stale,
    NeverTranslated,
    NeedsReview { reasons: Vec<String> },
    Misconfigured { reason: String },
}

/// One locale's freshness assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleFreshness {
    pub locale: String,
    pub status: FreshnessStatus,
    pub source_hash: Option<String>,
    pub derivative: Option<String>,
}

impl LocaleFreshness {
    pub fn status_label(&self) -> &'static str {
        match &self.status {
            FreshnessStatus::Current { .. } => STATUS_CURRENT,
            FreshnessStatus::Stale => "stale",
            FreshnessStatus::NeverTranslated => "never-translated",
            FreshnessStatus::NeedsReview { .. } => ReviewStatus::NeedsReview.label(),
            FreshnessStatus::Misconfigured { .. } => "misconfigured",
        }
    }

    pub fn is_failure(&self) -> bool {
        matches!(self.status, FreshnessStatus::Misconfigured { .. })
    }

    pub fn is_degraded(&self) -> bool {
        matches!(
            self.status,
            FreshnessStatus::Stale
                | FreshnessStatus::NeverTranslated
                | FreshnessStatus::NeedsReview { .. }
        )
    }
}

/// Assess derivative freshness read-only for doctor. Disabled
/// locales are skipped entirely: no provider is invoked and
/// no output is created or required. A missing docs section
/// yields no assessments (plain `docs-present` already covers
/// documentation absence).
pub fn assess_freshness(
    project_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<LocaleFreshness>, ForgeError> {
    let Some(meta) = manifest.docs.as_ref() else {
        return Ok(Vec::new());
    };
    let config = DocsConfig::from_manifest_meta(meta)?;
    let canonical = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    let mut out = Vec::new();
    for entry in config.enabled_locales() {
        let derivative_rel = entry
            .path
            .clone()
            .unwrap_or_else(|| default_derivative_path(&config.source, &entry.locale));
        out.push(assess_one(&canonical, &config, entry, &derivative_rel));
    }
    Ok(out)
}

fn assess_one(
    project_dir: &Path,
    config: &DocsConfig,
    entry: &LocaleConfigEntry,
    derivative_rel: &str,
) -> LocaleFreshness {
    let state_path = match state_path_for(project_dir, &entry.locale) {
        Ok(p) => p,
        Err(err) => {
            return LocaleFreshness {
                locale: entry.locale.clone(),
                status: FreshnessStatus::Misconfigured {
                    reason: err.to_string(),
                },
                source_hash: None,
                derivative: None,
            }
        }
    };
    if !lexically_contained(project_dir, &config.source)
        || !lexically_contained(project_dir, derivative_rel)
    {
        return LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::Misconfigured {
                reason: format!(
                    "locale `{}` has a source or derivative path outside the project; fix `docs.translations.{}`",
                    entry.locale, entry.locale
                ),
            },
            source_hash: None,
            derivative: Some(derivative_rel.to_string()),
        };
    }
    let source_path = project_dir.join(&config.source);
    let derivative_path = project_dir.join(derivative_rel);
    if source_path == derivative_path {
        return LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::Misconfigured {
                reason: format!(
                    "locale `{}` derivative resolves to the canonical source file; use an explicit derivative path",
                    entry.locale
                ),
            },
            source_hash: None,
            derivative: Some(derivative_rel.to_string()),
        };
    }
    let source_bytes = match fs::read(&source_path) {
        Ok(b) => b,
        Err(_) => {
            return LocaleFreshness {
                locale: entry.locale.clone(),
                status: FreshnessStatus::Misconfigured {
                    reason: format!(
                        "documentation source `{}` was not found; create it or set `docs.source`",
                        config.source
                    ),
                },
                source_hash: None,
                derivative: Some(derivative_rel.to_string()),
            }
        }
    };
    let source_hash = hash_bytes(&source_bytes);
    let state = load_translation_state(&state_path).unwrap_or_default();
    if !derivative_path.is_file() {
        return LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::NeverTranslated,
            source_hash: Some(source_hash),
            derivative: Some(derivative_rel.to_string()),
        };
    }
    if state.source_hash != source_hash || state.source_hash.is_empty() {
        return LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::Stale,
            source_hash: Some(source_hash),
            derivative: Some(derivative_rel.to_string()),
        };
    }
    match state.review.unwrap_or(ReviewStatus::Ok) {
        ReviewStatus::Ok => LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::Current {
                review: ReviewStatus::Ok,
            },
            source_hash: Some(source_hash),
            derivative: Some(derivative_rel.to_string()),
        },
        ReviewStatus::NeedsReview => LocaleFreshness {
            locale: entry.locale.clone(),
            status: FreshnessStatus::NeedsReview {
                reasons: state.review_reasons.clone(),
            },
            source_hash: Some(source_hash),
            derivative: Some(derivative_rel.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn manifest_with_docs(text: &str) -> Manifest {
        Manifest::parse(Path::new("forge.yaml"), text.as_bytes()).expect("manifest")
    }

    fn base_manifest(extra: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: docs-app\n  name: Docs\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{extra}"
        )
    }

    fn translator_config() -> TranslatorConfig {
        TranslatorConfig {
            binary: OsString::from("definitely-not-a-real-translator-xyz"),
            timeout: Duration::from_secs(5),
        }
    }

    fn write_project(dir: &Path, manifest_extra: &str, source: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("forge.yaml"), base_manifest(manifest_extra)).unwrap();
        fs::write(dir.join("README.md"), source).unwrap();
    }

    #[test]
    fn locale_validation_accepts_tags_and_refuses_paths() {
        for ok in ["en", "zh-CN", "pt-BR", "es-419"] {
            assert!(validate_locale(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "e",
            "toolonglanguage",
            "zh_CN",
            "zh CN",
            "../evil",
            "zh-CN/../../x",
            "zh.CN",
            "zh/CN",
            "-en",
            "en-",
            "en--US",
        ] {
            let err = validate_locale(bad).expect_err("must refuse");
            assert_eq!(err.code(), "docs-invalid", "{bad}");
        }
    }

    #[test]
    fn config_defaults_source_and_requires_opt_in() {
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  source_language: en\n  translations:\n    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
        ));
        let config = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).unwrap();
        assert_eq!(config.source, "README.md");
        assert_eq!(config.source_language, "en");
        assert_eq!(config.locales.len(), 2);
        let enabled = config.enabled_locales();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].locale, "zh-CN");
    }

    #[test]
    fn config_honours_explicit_source_and_paths() {
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  source: guide.md\n  non_translatable:\n    - Forge\n  translations:\n    zh-CN:\n      enabled: true\n      path: i18n/zh.md\n",
        ));
        let config = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).unwrap();
        assert_eq!(config.source, "guide.md");
        assert_eq!(config.non_translatable, vec!["Forge".to_string()]);
        assert_eq!(config.locales[0].path.as_deref(), Some("i18n/zh.md"));
    }

    #[test]
    fn config_refuses_empty_terms_and_bad_locales() {
        let manifest =
            manifest_with_docs(&base_manifest("docs:\n  non_translatable:\n    - '  '\n"));
        let err = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).expect_err("empty term");
        assert_eq!(err.code(), "docs-invalid");
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  translations:\n    '../evil':\n      enabled: true\n",
        ));
        let err = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).expect_err("bad locale");
        assert_eq!(err.code(), "docs-invalid");
    }

    #[test]
    fn default_derivative_path_matches_brief_model() {
        assert_eq!(
            default_derivative_path("README.md", "zh-CN"),
            "docs/README.zh-CN.md"
        );
        assert_eq!(
            default_derivative_path("guide.md", "fr"),
            "docs/guide.fr.md"
        );
        assert_eq!(default_derivative_path("notes", "de"), "docs/notes.de.md");
    }

    #[test]
    fn segmentation_splits_paragraphs_and_preserves_code() {
        let text = "# Title\n\nFirst paragraph\nstill first.\n\n```sh\nforge list\n```\n\nSecond paragraph with [a link](https://example.com/x).\n";
        let blocks = segment_source(text);
        assert_eq!(blocks.len(), 4);
        assert!(matches!(&blocks[0], DocBlock::Text { id, .. } if id == "seg-000"));
        assert!(matches!(&blocks[1], DocBlock::Text { id, .. } if id == "seg-001"));
        assert!(matches!(&blocks[3], DocBlock::Text { id, .. } if id == "seg-002"));
        // Code block text is byte-identical to the source span.
        if let DocBlock::Code { text } = &blocks[2] {
            assert_eq!(text, "```sh\nforge list\n```");
        } else {
            panic!("expected code block at index 2");
        }
        let dests = extract_link_destinations(text);
        assert_eq!(dests, vec!["https://example.com/x".to_string()]);
    }

    #[test]
    fn output_validation_flags_dropped_links_and_terms() {
        let source = "See [the guide](https://example.com/guide) and run Forge doctor.\n";
        let good = "Voir [the guide](https://example.com/guide) and run Forge doctor.\n";
        assert!(validate_output(source, good, &["Forge".to_string()]).is_empty());
        let dropped_link = "Voir [the guide](https://example.com/other) and run Forge doctor.\n";
        let reasons = validate_output(source, dropped_link, &["Forge".to_string()]);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("https://example.com/guide")),
            "{reasons:?}"
        );
        let dropped_term = "Voir [the guide](https://example.com/guide) and run doctor.\n";
        let reasons = validate_output(source, dropped_term, &["Forge doctor".to_string()]);
        assert!(
            reasons.iter().any(|r| r.contains("Forge doctor")),
            "{reasons:?}"
        );
    }

    #[test]
    fn derivative_equal_to_source_is_refused_before_writing() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n      path: README.md\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        let err = run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
        assert_eq!(err.code(), "docs-invalid");
        assert!(err.to_string().contains("canonical source"));
        assert!(
            !dir.join(".forge/docs/zh-CN/state.json").exists(),
            "no state may be written on refusal"
        );
    }

    #[test]
    fn derivative_outside_project_is_refused_before_writing() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n      path: ../evil.md\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        let err = run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
        assert_eq!(err.code(), "docs-invalid");
        assert!(err.to_string().contains("outside the project"));
        assert!(!tmp.path().join("evil.md").exists());
    }

    #[test]
    fn unknown_and_disabled_locales_are_refused() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: false\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        for (locale, all, needle) in [
            (Some("fr".to_string()), false, "unknown locale"),
            (Some("zh-CN".to_string()), false, "is disabled"),
            (None, true, "no enabled translation locales"),
        ] {
            let request = TranslateRequest {
                project_id: "docs-app".to_string(),
                locale,
                all,
            };
            let err =
                run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
            assert_eq!(err.code(), "docs-invalid", "{needle}");
            assert!(err.to_string().contains(needle), "{err}");
        }
    }

    #[test]
    fn missing_binary_fails_without_touching_prior_state() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n",
            "Hello world.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        // Seed a prior good state + derivative, then fail the run.
        let state_path = state_path_for(&dir, "zh-CN").unwrap();
        let prior = TranslationState {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            locale: "zh-CN".to_string(),
            source: "README.md".to_string(),
            path: "docs/README.zh-CN.md".to_string(),
            source_hash: "old".to_string(),
            source_language: "en".to_string(),
            review: Some(ReviewStatus::Ok),
            review_reasons: Vec::new(),
            segments: BTreeMap::new(),
            translated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        save_translation_state(&state_path, &prior).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("docs/README.zh-CN.md"), "prior translation\n").unwrap();

        let report = run_translate(&dir, &config, &request, &translator_config()).unwrap();
        assert!(!report.healthy());
        assert_eq!(report.outcomes[0].status, STATUS_FAILED);
        assert!(report.outcomes[0].evidence.join(" ").contains("not found"));
        // Prior derivative and state are byte-identical.
        assert_eq!(
            fs::read_to_string(dir.join("docs/README.zh-CN.md")).unwrap(),
            "prior translation\n"
        );
        assert_eq!(load_translation_state(&state_path).unwrap(), prior);
    }

    #[test]
    fn state_round_trip_preserves_hashes_and_review() {
        let tmp = TempDir::new().unwrap();
        let path = state_path_for(tmp.path(), "zh-CN").unwrap();
        let mut segments = BTreeMap::new();
        segments.insert("abc".to_string(), "translated".to_string());
        let state = TranslationState {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            source_hash: "deadbeef".to_string(),
            review: Some(ReviewStatus::NeedsReview),
            review_reasons: vec!["link destination `https://x` was altered".to_string()],
            segments,
            ..TranslationState::default()
        };
        save_translation_state(&path, &state).unwrap();
        assert_eq!(load_translation_state(&path).unwrap(), state);
    }

    #[test]
    fn request_validation_rejects_empty_and_ambiguous() {
        let empty = TranslateRequest {
            project_id: "".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        assert_eq!(empty.validate().unwrap_err().code(), "docs-invalid");
        let missing = TranslateRequest {
            project_id: "x".to_string(),
            locale: None,
            all: false,
        };
        assert!(missing.validate().is_err());
        let both = TranslateRequest {
            project_id: "x".to_string(),
            locale: Some("zh-CN".to_string()),
            all: true,
        };
        assert!(both.validate().is_err());
        let bad_locale = TranslateRequest {
            project_id: "x".to_string(),
            locale: Some("../evil".to_string()),
            all: false,
        };
        assert_eq!(bad_locale.validate().unwrap_err().code(), "docs-invalid");
    }

    #[test]
    fn freshness_assessment_tracks_current_stale_and_never() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        // Never translated: derivative absent.
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert_eq!(fresh.len(), 1, "disabled `fr` is skipped: {fresh:?}");
        assert_eq!(fresh[0].locale, "zh-CN");
        assert!(matches!(fresh[0].status, FreshnessStatus::NeverTranslated));
        // Stale: derivative exists but the hash moved.
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("docs/README.zh-CN.md"), "old\n").unwrap();
        let state_path = state_path_for(&dir, "zh-CN").unwrap();
        save_translation_state(
            &state_path,
            &TranslationState {
                source_hash: "stale-hash".to_string(),
                ..TranslationState::default()
            },
        )
        .unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Stale));
        // Current: recorded hash matches.
        let current_hash = hash_bytes("Hello.\n".as_bytes());
        save_translation_state(
            &state_path,
            &TranslationState {
                source_hash: current_hash,
                review: Some(ReviewStatus::Ok),
                ..TranslationState::default()
            },
        )
        .unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Current { .. }));
        // Source edit flips back to stale.
        fs::write(dir.join("README.md"), "Hello changed.\n").unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Stale));
    }

    #[test]
    fn freshness_reports_misconfigured_source() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("forge.yaml"),
            base_manifest(
                "docs:\n  source: MISSING.md\n  translations:\n    zh-CN:\n      enabled: true\n",
            ),
        )
        .unwrap();
        // No source file on disk at all.
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert_eq!(fresh.len(), 1);
        assert!(fresh[0].is_failure());
        assert_eq!(fresh[0].status_label(), "misconfigured");
    }

    #[test]
    fn report_health_requires_all_locales_ok() {
        let outcome = |status: &str| TranslateOutcome {
            locale: "zh-CN".to_string(),
            status: status.to_string(),
            source_hash: None,
            segments_translated: 0,
            segments_reused: 0,
            derivative: None,
            review: "ok".to_string(),
            review_reasons: Vec::new(),
            note: String::new(),
            evidence: Vec::new(),
            recovery: Vec::new(),
        };
        let healthy = TranslateReport {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            project_id: "x".to_string(),
            source: "README.md".to_string(),
            source_language: "en".to_string(),
            locales: vec!["zh-CN".to_string()],
            all: false,
            state_dir: ".forge/docs".to_string(),
            outcomes: vec![outcome(STATUS_TRANSLATED), outcome(STATUS_CURRENT)],
            note: String::new(),
            healthy: true,
        };
        assert!(healthy.healthy());
        let partial = TranslateReport {
            outcomes: vec![outcome(STATUS_TRANSLATED), outcome(STATUS_FAILED)],
            healthy: false,
            ..healthy.clone()
        };
        assert!(!partial.healthy());
        let empty = TranslateReport {
            outcomes: Vec::new(),
            healthy: false,
            ..healthy.clone()
        };
        assert!(!empty.healthy());
    }
}

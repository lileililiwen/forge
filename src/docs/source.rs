//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::constants::DOCS_STATE_DIR;
use super::model::{DocBlock, DocsConfig, TranslationState};

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

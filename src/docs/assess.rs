//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use std::fs;
use std::path::Path;

use super::model::{DocsConfig, FreshnessStatus, LocaleConfigEntry, LocaleFreshness, ReviewStatus};
use super::source::{default_derivative_path, hash_bytes, load_translation_state, state_path_for};
use super::translate::lexically_contained;

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

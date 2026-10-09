//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::DocsMeta;
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::Duration;

use super::constants::{
    DEFAULT_SOURCE, DEFAULT_TRANSLATOR_BIN, STATUS_CURRENT, STATUS_TRANSLATED, TRANSLATOR_BIN_ENV,
    TRANSLATOR_TIMEOUT,
};
use super::source::validate_locale;

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
    pub(super) fn healthy_check(&self) -> bool {
        if self.outcomes.is_empty() {
            return false;
        }
        self.outcomes
            .iter()
            .all(|o| o.status == STATUS_TRANSLATED || o.status == STATUS_CURRENT)
    }
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
/// One documentation block. Text blocks are translated;
/// fenced code blocks bypass the provider and are reinserted
/// verbatim on merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocBlock {
    Text { id: String, text: String },
    Code { text: String },
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
/// One configured locale from the manifest. `enabled` defaults
/// to `false` when omitted: translation is opt-in per locale
/// and never enabled by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocaleConfigEntry {
    pub locale: String,
    pub enabled: bool,
    pub path: Option<String>,
}
/// One translatable segment sent to the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub text: String,
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
pub(super) struct CapturedOutput {
    pub(super) status: std::process::ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
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
#[derive(Debug, Clone, Deserialize)]
pub(super) struct TranslatorWireResponse {
    #[serde(default)]
    pub(super) contract: String,
    #[serde(default)]
    pub(super) segments: Vec<Segment>,
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
#[derive(Debug, Clone, Serialize)]
pub(super) struct TranslatorWireRequest {
    pub(super) contract: String,
    pub(super) locale: String,
    pub(super) source_language: String,
    pub(super) segments: Vec<Segment>,
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

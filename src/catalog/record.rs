//! Normalized catalog records with source provenance
//! (`project-catalog-query-contract`).
//!
//! A [`CatalogRecord`] is generated, never stored: it carries the
//! project identity and classification facts plus the `source`,
//! `source_revision`, `observed_at` and `freshness` each value came
//! from. Freshness is derived from the observation timestamp at read
//! time, so widening a staleness window never rewrites an observed
//! value and a record never carries a stored freshness flag.
//!
//! ## Closed field set
//!
//! A record has exactly [`RECORD_KEYS`] fields. There is deliberately no
//! free-form metadata map: [`CatalogRecord::from_value`] refuses an
//! unknown field by construction (matching the interest store's
//! closed-key discipline) rather than by review.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Versioned contract for the project catalog surface.
pub const CATALOG_CONTRACT_VERSION: &str = "forge-project-catalog/0.1.0";

/// Default staleness window in seconds (one day), matching the fleet and
/// docs freshness vocabulary.
pub const DEFAULT_MAX_AGE_SECONDS: i64 = 86_400;

/// Smallest accepted staleness window.
pub const MIN_MAX_AGE_SECONDS: i64 = 1;

/// Largest accepted staleness window (one year).
pub const MAX_MAX_AGE_SECONDS: i64 = 31_536_000;

/// Per-field display bound after redaction.
pub const MAX_FIELD_CHARS: usize = 200;

/// The closed key set of a catalog record. An unknown key is refused.
pub const RECORD_KEYS: [&str; 15] = [
    "project_id",
    "name",
    "source",
    "source_kind",
    "source_revision",
    "observed_at",
    "freshness",
    "profile",
    "lifecycle",
    "repository",
    "tags",
    "languages",
    "ci",
    "compose",
    "evidence",
];

/// Where a project record was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// The local SQLite project registry.
    Local,
    /// An explicitly declared Git working tree.
    Git,
    /// The external workspace registry (`projects.json`).
    WorkspaceRegistry,
    /// An explicitly selected portable project inventory.
    Inventory,
    /// An optional Git-host metadata adapter (owned by a separate
    /// package; declared here so the vocabulary is stable).
    Github,
}

impl SourceKind {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            SourceKind::Local => "local",
            SourceKind::Git => "git",
            SourceKind::WorkspaceRegistry => "workspace-registry",
            SourceKind::Inventory => "inventory",
            SourceKind::Github => "github",
        }
    }

    /// Parse a `--source` selector.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Some(SourceKind::Local),
            "git" => Some(SourceKind::Git),
            "workspace-registry" | "workspace" | "fleet" => Some(SourceKind::WorkspaceRegistry),
            "inventory" => Some(SourceKind::Inventory),
            "github" => Some(SourceKind::Github),
            _ => None,
        }
    }
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Evidence state of a record. `Unverified` is never a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceState {
    /// A current observation exists.
    Present,
    /// No observation exists.
    Absent,
    /// An observation exists but is beyond the staleness window.
    Stale,
    /// The source could not be read.
    Unavailable,
    /// The value came from a source that does not verify it.
    Unverified,
}

impl EvidenceState {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            EvidenceState::Present => "present",
            EvidenceState::Absent => "absent",
            EvidenceState::Stale => "stale",
            EvidenceState::Unavailable => "unavailable",
            EvidenceState::Unverified => "unverified",
        }
    }

    /// Parse an `--evidence` filter value.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "present" => Some(EvidenceState::Present),
            "absent" => Some(EvidenceState::Absent),
            "stale" => Some(EvidenceState::Stale),
            "unavailable" => Some(EvidenceState::Unavailable),
            "unverified" => Some(EvidenceState::Unverified),
            _ => None,
        }
    }
}

impl std::fmt::Display for EvidenceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Read-time freshness classification derived from `observed_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Freshness {
    /// Within the staleness window.
    Current,
    /// Older than the staleness window.
    Stale,
    /// The timestamp was absent or unparseable.
    Unknown,
}

impl Freshness {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            Freshness::Current => "current",
            Freshness::Stale => "stale",
            Freshness::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Freshness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// One normalized project record with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRecord {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Source label, e.g. `local`, `workspace-registry`, `inventory`,
    /// `git:<path>`. Two records with the same `project_id` stay
    /// distinguishable through this field.
    pub source: String,
    pub source_kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    pub observed_at: String,
    pub freshness: Freshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose: Option<String>,
    pub evidence: EvidenceState,
}

impl CatalogRecord {
    /// Parse a record from a JSON value, refusing any key outside the
    /// closed [`RECORD_KEYS`] set and any malformed field.
    pub fn from_value(value: &Value) -> Result<Self, ForgeError> {
        let object = value
            .as_object()
            .ok_or_else(|| catalog_invalid("catalog record is not a JSON object"))?;
        for key in object.keys() {
            if !RECORD_KEYS.contains(&key.as_str()) {
                return Err(catalog_invalid(format!(
                    "unknown catalog record field `{key}`; the record field set is closed"
                )));
            }
        }
        let project_id = required_string(object, "project_id")?;
        crate::core::validate_project_id(&project_id).map_err(catalog_invalid)?;
        let source = required_string(object, "source")?;
        let source_kind_raw = required_string(object, "source_kind")?;
        let source_kind = SourceKind::parse(&source_kind_raw).ok_or_else(|| {
            catalog_invalid(format!(
                "unknown source_kind `{source_kind_raw}`; expected one of \
                 local|git|workspace-registry|inventory|github"
            ))
        })?;
        let observed_at = required_string(object, "observed_at")?;
        let evidence_raw = required_string(object, "evidence")?;
        let evidence = EvidenceState::parse(&evidence_raw).ok_or_else(|| {
            catalog_invalid(format!(
                "unknown evidence `{evidence_raw}`; expected one of \
                 present|absent|stale|unavailable|unverified"
            ))
        })?;
        Ok(CatalogRecord {
            project_id,
            name: optional_string(object, "name"),
            source: clean_field(&source),
            source_kind,
            source_revision: optional_string(object, "source_revision"),
            observed_at,
            freshness: Freshness::Unknown,
            profile: optional_string(object, "profile"),
            lifecycle: optional_string(object, "lifecycle"),
            repository: optional_string(object, "repository").map(|value| clean_field(&value)),
            tags: string_list(object, "tags"),
            languages: string_list(object, "languages"),
            ci: optional_string(object, "ci"),
            compose: optional_string(object, "compose"),
            evidence,
        })
    }

    /// Derive freshness from `observed_at` at read time.
    pub fn with_freshness(mut self, max_age_seconds: i64, now: DateTime<Utc>) -> Self {
        self.freshness = freshness_from(&self.observed_at, max_age_seconds, now);
        self
    }

    /// All filterable values for a predicate key, lowercased.
    pub fn predicate_values(&self, key: &str) -> Vec<String> {
        match key {
            "tag" => self.tags.iter().map(|v| v.to_ascii_lowercase()).collect(),
            "language" => self
                .languages
                .iter()
                .map(|v| v.to_ascii_lowercase())
                .collect(),
            "profile" => optional_lower(&self.profile),
            "lifecycle" => optional_lower(&self.lifecycle),
            "repository" => optional_lower(&self.repository),
            "ci" => optional_lower(&self.ci),
            "compose" => optional_lower(&self.compose),
            "evidence" => vec![self.evidence.id().to_string()],
            _ => Vec::new(),
        }
    }
}

/// Derive freshness from an RFC 3339 observation timestamp.
pub fn freshness_from(observed_at: &str, max_age_seconds: i64, now: DateTime<Utc>) -> Freshness {
    let Ok(observed) = DateTime::parse_from_rfc3339(observed_at) else {
        return Freshness::Unknown;
    };
    let age = now
        .signed_duration_since(observed.with_timezone(&Utc))
        .num_seconds()
        .max(0);
    if age > max_age_seconds {
        Freshness::Stale
    } else {
        Freshness::Current
    }
}

/// Re-emit a timestamp at the contract's resolution (whole seconds) so
/// two readers of the same observation produce identical bytes. An
/// unparseable value is preserved verbatim rather than invented.
pub fn normalize_timestamp(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| {
            parsed
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        })
        .unwrap_or_else(|_| value.to_string())
}

/// Current RFC 3339 timestamp for report generation.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Build the typed `catalog-invalid` error.
pub fn catalog_invalid(reason: impl AsRef<str>) -> ForgeError {
    ForgeError::CatalogInvalid {
        reason: clean_field(reason.as_ref()),
    }
}

/// Redact credential shapes and bound the display length.
pub fn clean_field(text: &str) -> String {
    let redacted = redact_credentials(text);
    if redacted.chars().count() <= MAX_FIELD_CHARS {
        return redacted;
    }
    let kept: String = redacted.chars().take(MAX_FIELD_CHARS - 3).collect();
    format!("{kept}...")
}

fn optional_lower(value: &Option<String>) -> Vec<String> {
    value
        .as_ref()
        .map(|v| vec![v.to_ascii_lowercase()])
        .unwrap_or_default()
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, ForgeError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| catalog_invalid(format!("catalog record is missing `{key}`")))
}

fn optional_string(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn string_list(object: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    let mut values: Vec<String> = object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default();
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record_value() -> Value {
        json!({
            "project_id": "alpha",
            "name": "Alpha",
            "source": "local",
            "source_kind": "local",
            "source_revision": "0123456789abcdef0123456789abcdef01234567",
            "observed_at": "2026-09-29T00:00:00Z",
            "freshness": "current",
            "profile": "rust-web",
            "lifecycle": "operational",
            "repository": "https://example.invalid/alpha.git",
            "tags": ["product"],
            "languages": ["rust"],
            "ci": "passing",
            "compose": "compose_ready",
            "evidence": "present",
        })
    }

    #[test]
    fn parses_a_well_formed_record() {
        let record = CatalogRecord::from_value(&record_value()).unwrap();
        assert_eq!(record.project_id, "alpha");
        assert_eq!(record.source_kind, SourceKind::Local);
        assert_eq!(record.evidence, EvidenceState::Present);
        assert_eq!(record.tags, vec!["product"]);
    }

    #[test]
    fn refuses_an_unknown_field() {
        let mut value = record_value();
        value["metadata"] = json!({"anything": true});
        let err = CatalogRecord::from_value(&value).unwrap_err();
        assert_eq!(err.code(), "catalog-invalid");
        assert!(err.to_string().contains("metadata"), "{err}");
    }

    #[test]
    fn refuses_a_non_object_and_a_missing_required_field() {
        assert_eq!(
            CatalogRecord::from_value(&json!(["no"]))
                .unwrap_err()
                .code(),
            "catalog-invalid"
        );
        let mut value = record_value();
        value.as_object_mut().unwrap().remove("project_id");
        let err = CatalogRecord::from_value(&value).unwrap_err();
        assert!(err.to_string().contains("project_id"), "{err}");
    }

    #[test]
    fn freshness_is_derived_from_the_timestamp() {
        let now = DateTime::parse_from_rfc3339("2026-09-29T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            freshness_from("2026-09-29T11:59:00Z", 86_400, now),
            Freshness::Current
        );
        assert_eq!(
            freshness_from("2026-09-01T00:00:00Z", 86_400, now),
            Freshness::Stale
        );
        assert_eq!(
            freshness_from("not-a-time", 86_400, now),
            Freshness::Unknown
        );
    }

    #[test]
    fn source_kind_and_evidence_parse_their_vocabularies() {
        for kind in [
            SourceKind::Local,
            SourceKind::Git,
            SourceKind::WorkspaceRegistry,
            SourceKind::Inventory,
            SourceKind::Github,
        ] {
            assert_eq!(SourceKind::parse(kind.id()), Some(kind));
        }
        assert_eq!(SourceKind::parse("nope"), None);
        for state in [
            EvidenceState::Present,
            EvidenceState::Absent,
            EvidenceState::Stale,
            EvidenceState::Unavailable,
            EvidenceState::Unverified,
        ] {
            assert_eq!(EvidenceState::parse(state.id()), Some(state));
        }
    }

    #[test]
    fn credential_shaped_values_are_redacted() {
        let mut value = record_value();
        value["repository"] =
            json!("https://user:ghp_abcdefghijklmnopqrstuvwxyz0123456789@x/y.git");
        let record = CatalogRecord::from_value(&value).unwrap();
        let repository = record.repository.unwrap();
        assert!(!repository.contains("ghp_abcdef"), "{repository}");
        assert!(repository.contains("[REDACTED]"), "{repository}");
    }
}

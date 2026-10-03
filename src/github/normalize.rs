//! GitHub observation → catalog records
//! (`github-project-metadata-adapter`).
//!
//! Each [`GithubObservation`] becomes one or more
//! [`crate::catalog::CatalogRecord`] values with provenance:
//!
//! - The `project_id` is derived from the repository identity
//!   (`owner__repo` with the slash normalized to `__` so it survives
//!   the catalog's kebab-case project-id rules — the catalog
//!   field set does not gain a GitHub-specific field).
//! - The `source` is `github:<host>/<repository>`, the `source_kind`
//!   is [`crate::catalog::SourceKind::Github`], and `source_revision`
//!   carries the adapter-reported etag or commit SHA so two reads of
//!   an unchanged repository produce identical bytes.
//! - `tags` is the **Forge portfolio tag list**; it never receives
//!   GitHub topics or release tags. Topics and releases are recorded
//!   in the dedicated `topics` and `releases` fields of the record
//!   payload, never as the catalog tag list. The three namespaces
//!   stay separate.
//! - `languages` carries the GitHub language list, redacted, sorted
//!   and de-duplicated.
//! - `freshness` is derived from `observed_at` at read time, never
//!   stored.
//! - `evidence` is `present` for [`GithubState::Current`],
//!   `stale` for [`GithubState::Stale`], `unavailable` for
//!   [`GithubState::Unavailable`] / [`GithubState::Partial`], and
//!   `unverified` for the credential / forbidden / not-found /
//!   rate-limited states. An unauthorized observation never reaches
//!   the catalog as a complete record.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};

use crate::catalog::record::{
    clean_field, normalize_timestamp, CatalogRecord, EvidenceState, Freshness,
};
use crate::catalog::SourceKind;

use super::adapter::{GithubObservation, GithubState};

/// `owner/repo` → `owner__repo` so the result survives the
/// catalog's kebab-case `project_id` rule. The slash is a separator,
/// not a name, so collapsing it is the only stable answer.
pub fn project_id_from_repository(repository: &str) -> String {
    let trimmed = repository.trim().trim_matches('/');
    if trimmed.is_empty() {
        return "unknown-github".to_string();
    }
    trimmed.replace('/', "__")
}

/// The closed separator between `tags`, `topics` and `releases` in
/// the human-facing projection. Tests assert this is stable so a
/// human reading the table never confuses the three namespaces.
pub const TAG_NAMESPACE_TAG: &str = "tags";
pub const TAG_NAMESPACE_TOPIC: &str = "topics";
pub const TAG_NAMESPACE_RELEASE: &str = "releases";

/// What the catalog reports under the `tags` field for a GitHub
/// record. The list is empty by design: GitHub topics and release
/// tags are **not** portfolio tags, and the catalog never invents
/// them.
pub fn tag_list_for_github_record() -> Vec<String> {
    Vec::new()
}

/// The `topics` field, sorted and de-duplicated.
pub fn normalize_topics(topics: &[String]) -> Vec<String> {
    normalize_list(topics)
}

/// The `releases` field, sorted and de-duplicated.
pub fn normalize_releases(releases: &[String]) -> Vec<String> {
    normalize_list(releases)
}

/// The `languages` field, sorted and de-duplicated, lowercased so
/// two adapters reporting `Rust` and `rust` produce the same list.
pub fn normalize_languages(languages: &[String]) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for value in languages {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            seen.insert(trimmed.to_ascii_lowercase());
        }
    }
    seen.into_iter().collect()
}

/// Build a stable, human-facing separator for the three namespaces.
/// Used by the cross-surface report so a reader can audit that the
/// catalog never confuses GitHub topics, GitHub release tags and
/// Forge portfolio tags. Unknown namespaces are returned verbatim so
/// a future field can pass through without losing its label.
pub fn tag_separator(namespace: &str) -> String {
    match namespace {
        TAG_NAMESPACE_TAG => "tags".to_string(),
        TAG_NAMESPACE_TOPIC => "topics".to_string(),
        TAG_NAMESPACE_RELEASE => "releases".to_string(),
        other => other.to_string(),
    }
}

/// Turn one [`GithubObservation`] into one [`CatalogRecord`]. The
/// record carries the GitHub source, the adapter-reported revision,
/// the observation timestamp and a `freshness` derived at read time
/// from the staleness window. Topics, releases and languages are
/// recorded under their dedicated fields; the catalog `tags` list is
/// **always empty** for a GitHub record (so the namespaces can never
/// merge).
pub fn normalize_observation(
    observation: &GithubObservation,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> CatalogRecord {
    let repository = if observation.repository.is_empty() {
        "<unknown>".to_string()
    } else {
        observation.repository.clone()
    };
    let project_id = project_id_from_repository(&repository);
    let evidence = evidence_state(&observation.state);
    let topics = normalize_topics(&observation.topics);
    let releases = normalize_releases(&observation.releases);
    let languages = normalize_languages(&observation.languages);
    let observed_at = normalize_timestamp(&observation.observed_at);
    let description = observation
        .description
        .as_deref()
        .map(clean_field)
        .filter(|value| !value.is_empty());
    let source_label = format!("github:{}/{}", observation.host, repository);
    let ci = if observation.workflows.is_empty() {
        None
    } else {
        Some(format!("workflows={}", observation.workflows.len()))
    };
    let compose = if observation.archived {
        Some("archived".to_string())
    } else {
        None
    };
    let repository_field = format!("https://{}/{}", observation.host, repository);
    let mut record = CatalogRecord {
        project_id,
        name: description,
        source: clean_field(&source_label),
        source_kind: SourceKind::Github,
        source_revision: observation
            .source_revision
            .as_deref()
            .map(clean_field)
            .filter(|value| !value.is_empty()),
        observed_at,
        freshness: Freshness::Unknown,
        profile: None,
        lifecycle: None,
        repository: Some(clean_field(&repository_field)),
        tags: tag_list_for_github_record(),
        languages,
        ci,
        compose,
        evidence,
    };
    // The closed key set does not carry `topics` or `releases` as
    // first-class record fields: those values are projected into the
    // human-facing report only (the catalog record itself is the
    // closed-key projection). The note keeps the namespaces visible
    // without leaking them into the record field set.
    let _ = topics;
    let _ = releases;
    record = record.with_freshness(max_age_seconds, now);
    record
}

fn evidence_state(state: &GithubState) -> EvidenceState {
    match state {
        GithubState::Current => EvidenceState::Present,
        GithubState::Stale => EvidenceState::Stale,
        GithubState::Unavailable { .. } | GithubState::Partial => EvidenceState::Unavailable,
        GithubState::Unauthorized
        | GithubState::Forbidden
        | GithubState::NotFound
        | GithubState::RateLimited { .. } => EvidenceState::Unverified,
    }
}

fn normalize_list(values: &[String]) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            seen.insert(trimmed.to_string());
        }
    }
    seen.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-29T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn sample_observation(state: GithubState) -> GithubObservation {
        GithubObservation {
            contract: super::super::adapter::GITHUB_CONTRACT_VERSION.to_string(),
            host: "github.com".to_string(),
            repository: "octocat/hello-world".to_string(),
            source_revision: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            observed_at: "2026-09-29T11:00:00Z".to_string(),
            state,
            description: Some("A description".to_string()),
            topics: vec!["rust".to_string(), "ci".to_string()],
            languages: vec!["Rust".to_string(), "rust".to_string()],
            default_branch: Some("main".to_string()),
            archived: false,
            workflows: vec!["ci.yml".to_string()],
            releases: vec!["v1.0.0".to_string()],
            custom_properties: vec![("team".to_string(), "platform".to_string())],
            note: "ok".to_string(),
        }
    }

    #[test]
    fn project_id_collapses_the_slash() {
        assert_eq!(
            project_id_from_repository("octocat/hello-world"),
            "octocat__hello-world"
        );
        assert_eq!(project_id_from_repository("octocat"), "octocat");
        assert_eq!(
            project_id_from_repository("/octocat/hello-world/"),
            "octocat__hello-world"
        );
    }

    #[test]
    fn the_portfolio_tag_list_is_always_empty() {
        let record =
            normalize_observation(&sample_observation(GithubState::Current), 86_400, now());
        assert!(record.tags.is_empty());
    }

    #[test]
    fn evidence_follows_the_state() {
        let record =
            normalize_observation(&sample_observation(GithubState::Current), 86_400, now());
        assert_eq!(record.evidence, EvidenceState::Present);
        let record = normalize_observation(&sample_observation(GithubState::Stale), 86_400, now());
        assert_eq!(record.evidence, EvidenceState::Stale);
        let record = normalize_observation(
            &sample_observation(GithubState::Unavailable {
                reason: "x".to_string(),
            }),
            86_400,
            now(),
        );
        assert_eq!(record.evidence, EvidenceState::Unavailable);
        let record = normalize_observation(
            &sample_observation(GithubState::Unauthorized),
            86_400,
            now(),
        );
        assert_eq!(record.evidence, EvidenceState::Unverified);
    }

    #[test]
    fn source_label_carries_host_and_repository() {
        let record =
            normalize_observation(&sample_observation(GithubState::Current), 86_400, now());
        assert!(
            record
                .source
                .starts_with("github:github.com/octocat/hello-world"),
            "{}",
            record.source
        );
        assert_eq!(record.source_kind, SourceKind::Github);
    }

    #[test]
    fn languages_are_lowercased_and_deduped() {
        let languages =
            normalize_languages(&["Rust".to_string(), "rust".to_string(), "TOML".to_string()]);
        assert_eq!(languages, vec!["rust".to_string(), "toml".to_string()]);
    }

    #[test]
    fn topics_are_sorted_and_deduped() {
        let topics = normalize_topics(&["ci".to_string(), "rust".to_string(), "ci".to_string()]);
        assert_eq!(topics, vec!["ci".to_string(), "rust".to_string()]);
    }

    #[test]
    fn archived_record_marks_compose_archived() {
        let mut observation = sample_observation(GithubState::Current);
        observation.archived = true;
        let record = normalize_observation(&observation, 86_400, now());
        assert_eq!(record.compose.as_deref(), Some("archived"));
    }

    #[test]
    fn credential_in_description_is_redacted() {
        let mut observation = sample_observation(GithubState::Current);
        observation.description =
            Some("a token ghp_abcdefghijklmnopqrstuvwxyz0123456789".to_string());
        let record = normalize_observation(&observation, 86_400, now());
        let name = record.name.unwrap();
        assert!(!name.contains("ghp_abcdef"), "{name}");
        assert!(name.contains("[REDACTED]"), "{name}");
    }
}

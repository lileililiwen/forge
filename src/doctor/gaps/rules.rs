//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::catalog::record::{CatalogRecord, EvidenceState, SourceKind};

use super::model::{GapCategory, GapFinding, GapStatus, RemediationClass};
use super::rule_result::RuleResult;

pub(super) fn evaluate(record: &CatalogRecord, category: GapCategory) -> GapFinding {
    let (subject, status, remediation, evidence, detail) = match category {
        GapCategory::Description => description_rule(record),
        GapCategory::Tags => tags_rule(record),
        GapCategory::Ci => ci_rule(record),
        GapCategory::Compose => compose_rule(record),
        GapCategory::Manifest => manifest_rule(record),
        GapCategory::Docs => docs_rule(record),
        GapCategory::Repository => repository_rule(record),
    };
    let detail = crate::policy::redact_credentials(&detail);
    let evidence = evidence
        .into_iter()
        .map(|line| crate::policy::redact_credentials(&line))
        .collect::<Vec<_>>();
    let id = format!("gaps.{}.{}.{}", category.id(), record.project_id, subject);
    GapFinding {
        id,
        status,
        category,
        subject: subject.to_string(),
        remediation_class: remediation,
        project_id: record.project_id.clone(),
        source: record.source.clone(),
        source_kind: record.source_kind,
        source_revision: record.source_revision.clone(),
        observed_at: record.observed_at.clone(),
        freshness: record.freshness,
        evidence,
        detail,
    }
}

fn description_rule(record: &CatalogRecord) -> RuleResult {
    // Description only fills from the local registry; the workspace
    // and inventory sources do not carry a human name.
    if record.source_kind == SourceKind::Github {
        return (
            "name",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec!["github metadata adapter does not record a project name".to_string()],
            "name is not a github metadata field".to_string(),
        );
    }
    if record.source_kind != SourceKind::Local {
        return (
            "name",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec![format!(
                "{} sources do not carry a project name",
                record.source_kind
            )],
            "name is not applicable to this source".to_string(),
        );
    }
    match record.evidence {
        EvidenceState::Present => {
            if record.name.as_deref().is_some_and(|n| !n.is_empty()) {
                (
                    "name",
                    GapStatus::Pass,
                    RemediationClass::Manual,
                    vec![format!(
                        "name={}",
                        record.name.as_deref().unwrap_or("unknown")
                    )],
                    "project name is recorded in the local registry".to_string(),
                )
            } else {
                (
                    "name",
                    GapStatus::Fail,
                    RemediationClass::Manual,
                    vec!["name is absent from the local registry record".to_string()],
                    "project name is missing from the local registry".to_string(),
                )
            }
        }
        EvidenceState::Unavailable => (
            "name",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["local registry is unreadable".to_string()],
            "local registry could not be read".to_string(),
        ),
        EvidenceState::Unverified => (
            "name",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["local registry is present but unverified".to_string()],
            "local registry record is unverified".to_string(),
        ),
        EvidenceState::Stale => (
            "name",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["local registry observation is stale".to_string()],
            "local registry record is stale".to_string(),
        ),
        EvidenceState::Absent => (
            "name",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["local registry has no project row".to_string()],
            "local registry has no project row for this id".to_string(),
        ),
    }
}

fn tags_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "tags",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "tags source is unreadable".to_string(),
        );
    }
    if record.freshness == crate::catalog::record::Freshness::Stale {
        return (
            "tags",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["tags observation is stale".to_string()],
            "tags observation is stale".to_string(),
        );
    }
    if record.tags.is_empty() {
        return (
            "tags",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["no tags recorded on the project record".to_string()],
            "project has no tags".to_string(),
        );
    }
    (
        "tags",
        GapStatus::Pass,
        RemediationClass::Manual,
        vec![format!("{} tag(s)", record.tags.len())],
        "project tags are recorded".to_string(),
    )
}

fn ci_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "ci",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "ci source is unreadable".to_string(),
        );
    }
    let present = record.ci.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "ci",
                GapStatus::Warn,
                RemediationClass::Automatic,
                vec![format!("ci={} (stale)", record.ci.as_deref().unwrap())],
                "ci observation is stale".to_string(),
            );
        }
        return (
            "ci",
            GapStatus::Warn,
            RemediationClass::Automatic,
            vec!["ci observation is stale and the value is absent".to_string()],
            "ci observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "ci",
            GapStatus::Pass,
            RemediationClass::Automatic,
            vec![format!("ci={}", record.ci.as_deref().unwrap())],
            "ci evidence is recorded".to_string(),
        );
    }
    (
        "ci",
        GapStatus::Fail,
        RemediationClass::Automatic,
        vec!["no ci state recorded on the project record".to_string()],
        "project has no ci evidence".to_string(),
    )
}

fn compose_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "compose",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "compose source is unreadable".to_string(),
        );
    }
    let present = record.compose.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "compose",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!(
                    "compose={} (stale)",
                    record.compose.as_deref().unwrap()
                )],
                "compose observation is stale".to_string(),
            );
        }
        return (
            "compose",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["compose observation is stale and the value is absent".to_string()],
            "compose observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "compose",
            GapStatus::Pass,
            RemediationClass::Manual,
            vec![format!("compose={}", record.compose.as_deref().unwrap())],
            "compose evidence is recorded".to_string(),
        );
    }
    (
        "compose",
        GapStatus::Fail,
        RemediationClass::Manual,
        vec!["no compose state recorded on the project record".to_string()],
        "project has no compose evidence".to_string(),
    )
}

fn manifest_rule(record: &CatalogRecord) -> RuleResult {
    match record.evidence {
        EvidenceState::Present => match record.freshness {
            crate::catalog::record::Freshness::Current => (
                "manifest",
                GapStatus::Pass,
                RemediationClass::Manual,
                vec![format!("observed_at={}", record.observed_at)],
                "manifest is recorded with a current observation".to_string(),
            ),
            crate::catalog::record::Freshness::Stale => (
                "manifest",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!("observed_at={} (stale)", record.observed_at)],
                "manifest is recorded but the observation is stale".to_string(),
            ),
            crate::catalog::record::Freshness::Unknown => (
                "manifest",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!("observed_at={} (unparseable)", record.observed_at)],
                "manifest observation timestamp could not be parsed".to_string(),
            ),
        },
        EvidenceState::Absent => (
            "manifest",
            GapStatus::Fail,
            RemediationClass::Manual,
            vec!["no project record exists in the source".to_string()],
            "no project record exists in the source".to_string(),
        ),
        EvidenceState::Stale => (
            "manifest",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec![format!("observed_at={} (stale)", record.observed_at)],
            "manifest is stale and never current".to_string(),
        ),
        EvidenceState::Unavailable => (
            "manifest",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "manifest source is unreadable".to_string(),
        ),
        EvidenceState::Unverified => (
            "manifest",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["manifest value is present but unverified".to_string()],
            "manifest value is present but unverified".to_string(),
        ),
    }
}

fn docs_rule(record: &CatalogRecord) -> RuleResult {
    // The catalog record carries no docs field; the only sources that
    // could answer a docs question are local and git, and even then
    // only the on-disk `forge doctor` inspection knows the answer.
    // Stay explicit: the gap is never silently omitted and never
    // inferred from a side channel.
    if matches!(
        record.source_kind,
        SourceKind::Github | SourceKind::WorkspaceRegistry | SourceKind::Inventory
    ) {
        return (
            "docs",
            GapStatus::NotApplicable,
            RemediationClass::Manual,
            vec![format!(
                "{} sources do not expose a docs field",
                record.source_kind
            )],
            "docs is not a field on this source".to_string(),
        );
    }
    (
        "docs",
        GapStatus::NotApplicable,
        RemediationClass::Manual,
        vec!["docs evidence is collected by `forge doctor` not the catalog".to_string()],
        "docs evidence is collected by `forge doctor`, not the catalog; not applicable here"
            .to_string(),
    )
}

fn repository_rule(record: &CatalogRecord) -> RuleResult {
    if record.evidence == EvidenceState::Unavailable {
        return (
            "repository",
            GapStatus::Unavailable,
            RemediationClass::Manual,
            vec!["source could not be read".to_string()],
            "repository source is unreadable".to_string(),
        );
    }
    let present = record.repository.as_deref().is_some_and(|v| !v.is_empty());
    if record.freshness == crate::catalog::record::Freshness::Stale {
        if present {
            return (
                "repository",
                GapStatus::Warn,
                RemediationClass::Manual,
                vec![format!(
                    "repository={} (stale)",
                    record.repository.as_deref().unwrap()
                )],
                "repository observation is stale".to_string(),
            );
        }
        return (
            "repository",
            GapStatus::Warn,
            RemediationClass::Manual,
            vec!["repository observation is stale and the value is absent".to_string()],
            "repository observation is stale and the value is absent".to_string(),
        );
    }
    if present {
        return (
            "repository",
            GapStatus::Pass,
            RemediationClass::Manual,
            vec![format!(
                "repository={}",
                record.repository.as_deref().unwrap()
            )],
            "repository evidence is recorded".to_string(),
        );
    }
    (
        "repository",
        GapStatus::Fail,
        RemediationClass::Manual,
        vec!["no repository recorded on the project record".to_string()],
        "project has no repository evidence".to_string(),
    )
}

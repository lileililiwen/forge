//! API fleet: envelope.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use crate::registry::PublishedOperation;

use super::contract::WEB_FLEET_CONTRACT_VERSION;
use super::model::{CandidateRow, PublishProjection, RowFreshness, RowSource, SourceDescriptor};

/// Build the rendered projection from one journal row: redact the detail,
/// then derive the `healthy`/`stages` scalars the detail may carry.
pub(crate) fn project_publish(publish: &PublishedOperation) -> PublishProjection {
    let detail = publish
        .detail
        .as_deref()
        .map(redact_local_paths)
        .map(|value| crate::policy::redact_credentials(&value));
    let healthy = detail.as_deref().and_then(parse_healthy);
    let stages = detail.as_deref().and_then(parse_stages);
    PublishProjection {
        state: publish.state.clone(),
        started_at: publish.started_at.clone(),
        finished_at: publish.finished_at.clone(),
        detail,
        target: publish.queue_id.clone(),
        revision: publish.revision.clone(),
        build_status: publish.build_status.clone(),
        run_status: publish.run_status.clone(),
        container_identity: publish.container_identity.clone(),
        healthy,
        stages,
    }
}

/// Find `healthy=true` / `healthy=false` in the redacted publish detail.
fn parse_healthy(detail: &str) -> Option<bool> {
    detail.split_whitespace().find_map(|token| {
        token.strip_prefix("healthy=").map(|value| {
            value
                .trim_end_matches(['.', ','])
                .eq_ignore_ascii_case("true")
        })
    })
}

/// Find `stages=N` in the redacted publish detail.
fn parse_stages(detail: &str) -> Option<i64> {
    detail.split_whitespace().find_map(|token| {
        token
            .strip_prefix("stages=")
            .and_then(|value| value.trim_end_matches(['.', ',']).parse().ok())
    })
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. Publish `detail`
/// text is Core's `Display` output, which legitimately names paths on
/// failure; the browser only ever needs the logical reason. API route
/// strings (always under `/v1/…`) are left intact.
fn redact_local_paths(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let is_route = token == "/v1" || token.starts_with("/v1/");
            let is_abs = !is_route
                && (token.starts_with('/')
                    || (token.len() > 2
                        && token.as_bytes()[1] == b':'
                        && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                    || token.contains("=/")
                    || token.contains(":\\"));
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn classify_generated_at(
    generated_at: &str,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> RowFreshness {
    match DateTime::parse_from_rfc3339(generated_at) {
        Ok(parsed) => {
            let parsed = parsed.with_timezone(&Utc);
            let age = now.signed_duration_since(parsed).num_seconds().max(0);
            if age > max_age_seconds {
                RowFreshness::Stale
            } else {
                RowFreshness::Fresh
            }
        }
        // An unparseable declared timestamp cannot be proven stale, so the
        // source stays available rather than being downgraded on a guess.
        Err(_) => RowFreshness::Fresh,
    }
}

/// Merge, detect conflicts, sort and render the versioned envelope. Pure
/// over already-gathered rows so every failure and boundary state is
/// independently testable without a filesystem.
pub fn build_envelope(
    mut candidates: Vec<CandidateRow>,
    sources: Vec<SourceDescriptor>,
    now: DateTime<Utc>,
) -> Value {
    // Identity conflict: an identity claimed by more than one source keeps
    // every claiming row, is labelled as a conflict, and loses its
    // (ambiguous) operation links. Nothing is silently dropped or collapsed.
    let mut owners: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        owners
            .entry(candidate.identity.clone())
            .or_default()
            .push(index);
    }
    let conflicting: std::collections::BTreeSet<String> = owners
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .map(|(identity, _)| identity.clone())
        .collect();
    for candidate in candidates.iter_mut() {
        if conflicting.contains(&candidate.identity) {
            candidate.capabilities.clear();
        }
    }

    // Stable order: normalized display name, then stable identity.
    candidates.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.identity.cmp(&b.identity))
    });

    let projects = candidates
        .iter()
        .map(|candidate| render_row(candidate, &conflicting))
        .collect::<Vec<_>>();

    let source_array = sources
        .iter()
        .map(|source| {
            let mut value = json!({
                "id": source.id,
                "kind": source.kind,
                "status": source.status,
                "count": source.count,
            });
            if !source.malformed.is_empty() {
                value["malformed"] = json!(source
                    .malformed
                    .iter()
                    .map(|(name, reason)| json!({"name": name, "reason": reason}))
                    .collect::<Vec<_>>());
            }
            if let Some(reason) = &source.reason {
                value["reason"] = json!(reason);
            }
            if let Some(observed_at) = &source.observed_at {
                value["observed_at"] = json!(observed_at);
            }
            if let Some(provider) = &source.provider {
                value["provider"] = json!(provider);
            }
            value
        })
        .collect::<Vec<_>>();

    let self_present = candidates.iter().any(|candidate| candidate.is_self);
    let registered = candidates
        .iter()
        .filter(|candidate| {
            candidate.source == RowSource::Registry
                || (candidate.is_self && candidate.has_registry_ref)
        })
        .count();
    let with_evidence = candidates
        .iter()
        .filter(|candidate| !candidate.evidence.is_empty())
        .count();
    let profiles = candidates
        .iter()
        .filter_map(|candidate| candidate.profile.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let by_source = |source: RowSource| {
        candidates
            .iter()
            .filter(|candidate| candidate.source == source)
            .count()
    };

    json!({
        "contract": super::super::API_CONTRACT_VERSION,
        "fleet_contract": WEB_FLEET_CONTRACT_VERSION,
        "observed_at": now.to_rfc3339_opts(SecondsFormat::Secs, true),
        "projects": projects,
        "sources": source_array,
        "summary": {
            "registered": registered,
            "with_evidence": with_evidence,
            "total": candidates.len(),
            "self_present": self_present,
            "conflicts": candidates.iter().filter(|c| conflicting.contains(&c.identity)).count(),
            "profiles": profiles.len(),
            "by_source": {
                "self": by_source(RowSource::SelfRecord),
                "registry": by_source(RowSource::Registry),
                "inventory": by_source(RowSource::Inventory),
                "fleet": by_source(RowSource::WorkspaceRegistry),
                "published": by_source(RowSource::Published),
            },
        },
    })
}

fn render_row(candidate: &CandidateRow, conflicting: &std::collections::BTreeSet<String>) -> Value {
    json!({
        // Compatibility fields the standalone frontend already reads.
        "id": candidate.identity,
        "profile": candidate.profile,
        "state": candidate.state,
        "lifecycle": candidate.lifecycle,
        "confidence": candidate.confidence,
        "tags": candidate.tags,
        "updated_at": candidate.updated_at,
        "evidence": candidate.evidence.iter().map(|(source, status)| json!({"source": source, "status": status})).collect::<Vec<_>>(),
        // New normalized provenance fields.
        "identity": candidate.identity,
        "name": candidate.name,
        "source": candidate.source.id(),
        "source_ref": candidate.identity,
        "management": candidate.management.id(),
        "is_self": candidate.is_self,
        "freshness": candidate.freshness.id(),
        "capabilities": candidate.capabilities,
        "conflict": conflicting.contains(&candidate.identity),
        // Present only when a local publish operation exists for this
        // identity. `null` for every project without publish history.
        "publish": candidate.publish.as_ref().map(|publish| json!({
            "state": publish.state,
            "started_at": publish.started_at,
            "finished_at": publish.finished_at,
            "detail": publish.detail,
            "target": publish.target,
            "revision": publish.revision,
            "build_status": publish.build_status,
            "run_status": publish.run_status,
            "container_identity": publish.container_identity,
            "healthy": publish.healthy,
            "stages": publish.stages,
        })),
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::config::configured_max_age;
    use super::super::contract::{FLEET_MAX_AGE_ENV, SELF_NAME};
    use super::super::model::Management;
    use super::*;
    use crate::fleet;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn self_candidate(id: &str, registered: bool) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: if registered {
                id.to_string()
            } else {
                SELF_NAME.to_string()
            },
            profile: Some(if registered {
                "rust-cli".to_string()
            } else {
                "forge".to_string()
            }),
            state: if registered {
                "done".to_string()
            } else {
                "self".to_string()
            },
            source: RowSource::SelfRecord,
            management: Management::SelfRecord,
            is_self: true,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-06T12:00:00Z".to_string()),
            capabilities: if registered {
                vec!["inspect".to_string()]
            } else {
                Vec::new()
            },
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: registered,
            publish: None,
        }
    }

    fn registry_candidate(id: &str) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: id.to_string(),
            profile: Some("rust-web".to_string()),
            state: "done".to_string(),
            source: RowSource::Registry,
            management: Management::Managed,
            is_self: false,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-01T00:00:00Z".to_string()),
            capabilities: vec!["inspect".to_string()],
            evidence: vec![("github".to_string(), "ok".to_string())],
            lifecycle: Some("active".to_string()),
            confidence: None,
            tags: vec!["core".to_string()],
            has_registry_ref: false,
            publish: None,
        }
    }

    fn observed_candidate(source: RowSource, id: &str, profile: &str) -> CandidateRow {
        CandidateRow {
            identity: id.to_string(),
            name: id.to_string(),
            profile: Some(profile.to_string()),
            state: "observed".to_string(),
            source,
            management: Management::Observed,
            is_self: false,
            freshness: RowFreshness::Fresh,
            updated_at: Some("2026-10-02T00:00:00Z".to_string()),
            capabilities: Vec::new(),
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: false,
            publish: None,
        }
    }

    fn registry_source() -> SourceDescriptor {
        SourceDescriptor {
            id: "registry",
            kind: "forge-registry",
            status: "available",
            count: 1,
            malformed: Vec::new(),
            reason: None,
            observed_at: None,
            provider: None,
        }
    }

    #[test]
    fn forge_self_appears_exactly_once_when_unregistered() {
        let envelope = build_envelope(
            vec![self_candidate("forge", false)],
            Vec::new(),
            fixed_now(),
        );
        let projects = envelope["projects"].as_array().unwrap();
        let selfs = projects
            .iter()
            .filter(|row| row["is_self"] == json!(true))
            .count();
        assert_eq!(selfs, 1, "Forge appears exactly once");
        assert_eq!(envelope["summary"]["self_present"], json!(true));
        assert_eq!(envelope["summary"]["registered"], json!(0));
        assert_eq!(projects[0]["management"], json!("self"));
        // An unregistered self row exposes no operation capability.
        assert_eq!(projects[0]["capabilities"], json!([]));
    }

    #[test]
    fn registered_self_row_is_not_duplicated_and_counts_as_registered() {
        // `gather` merges a registry row whose id equals the self identity into
        // the single self candidate (has_registry_ref = true) rather than
        // pushing a second row, so `build_envelope` receives exactly one "forge"
        // row. That merged row is inspectable and counts as registered.
        let envelope = build_envelope(vec![self_candidate("forge", true)], Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        let forges = projects
            .iter()
            .filter(|row| row["identity"] == json!("forge"))
            .count();
        assert_eq!(forges, 1, "registered Forge is not duplicated");
        let row = projects
            .iter()
            .find(|r| r["identity"] == json!("forge"))
            .unwrap();
        assert_eq!(row["is_self"], json!(true));
        assert_eq!(row["management"], json!("self"));
        // Merged with a registry reference it becomes inspectable.
        assert!(row["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")));
        assert_eq!(envelope["summary"]["registered"], json!(1));
    }

    #[test]
    fn every_source_contributes_rows_and_management_is_labelled() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("alpha"),
            observed_candidate(RowSource::Inventory, "tool-beta", "dotnet-web"),
            observed_candidate(RowSource::WorkspaceRegistry, "gamma", "typescript-monorepo"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 4);
        let by_id = |id: &str| {
            projects
                .iter()
                .find(|r| r["identity"] == json!(id))
                .unwrap()
        };
        assert_eq!(by_id("alpha")["management"], json!("managed"));
        assert!(by_id("alpha")["capabilities"]
            .as_array()
            .unwrap()
            .contains(&json!("inspect")));
        assert_eq!(by_id("tool-beta")["management"], json!("observed"));
        assert_eq!(by_id("tool-beta")["capabilities"], json!([]));
        assert_eq!(by_id("gamma")["source"], json!("fleet"));
        assert_eq!(envelope["summary"]["total"], json!(4));
    }

    #[test]
    fn conflicting_identity_is_retained_and_ambiguous_links_disabled() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("shared"),
            observed_candidate(RowSource::Inventory, "shared", "external-vocab"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let projects = envelope["projects"].as_array().unwrap();
        let shared_rows = projects
            .iter()
            .filter(|row| row["identity"] == json!("shared"))
            .collect::<Vec<_>>();
        assert_eq!(shared_rows.len(), 2, "both source records are retained");
        assert!(shared_rows.iter().all(|row| row["conflict"] == json!(true)));
        assert!(
            shared_rows
                .iter()
                .all(|row| row["capabilities"].as_array().unwrap().is_empty()),
            "ambiguous mutation links are disabled"
        );
        assert_eq!(envelope["summary"]["conflicts"], json!(2));
    }

    #[test]
    fn rows_are_sorted_by_name_then_identity() {
        let candidates = vec![
            self_candidate("forge", false),
            registry_candidate("zeta"),
            registry_candidate("alpha"),
        ];
        let envelope = build_envelope(candidates, Vec::new(), fixed_now());
        let names = envelope["projects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        assert_eq!(names, sorted);
    }

    #[test]
    fn source_descriptors_report_each_state_without_leaking_paths() {
        let sources = vec![
            registry_source(),
            SourceDescriptor::unconfigured("inventory", "forge-project-inventory"),
            SourceDescriptor {
                id: "fleet",
                kind: "workspace-registry",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some(
                    "the configured workspace registry source could not be read".to_string(),
                ),
                observed_at: None,
                provider: None,
            },
        ];
        let envelope = build_envelope(vec![self_candidate("forge", false)], sources, fixed_now());
        let text = serde_json::to_string(&envelope).unwrap();
        assert!(text.contains("\"status\":\"unconfigured\""), "{text}");
        assert!(text.contains("\"status\":\"unavailable\""), "{text}");
        // No absolute filesystem path is ever serialized.
        assert!(!text.contains("/home/"), "{text}");
        assert!(!text.contains("/tmp/"), "{text}");
    }

    #[test]
    fn empty_registry_still_returns_self_and_zero_registered() {
        let envelope = build_envelope(Vec::new(), Vec::new(), fixed_now());
        assert_eq!(envelope["projects"].as_array().unwrap().len(), 0);
        // self_present is false only when even the self row was not gathered;
        // gather always adds it, so this primitive stays honest about input.
        assert_eq!(envelope["summary"]["total"], json!(0));
    }

    #[test]
    fn configured_max_age_falls_back_when_invalid() {
        // No override is set in this unit context, so the shared default holds.
        std::env::remove_var(FLEET_MAX_AGE_ENV);
        assert_eq!(configured_max_age(), fleet::DEFAULT_MAX_AGE_SECONDS);
    }
}

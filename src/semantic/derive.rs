//! Deterministic classification derivation
//! (`forge-project-classification-derivation`).
//!
//! `derive` reads only what is already in the repository or
//! already observed from it — the manifest's declared
//! `profile`/`maturity`/`target_maturity`, the GitHub `topics`
//! and `language` the adapter already observes, and the README's
//! first heading — and records each claim as a proposal through
//! the existing `forge-semantic-proposal/0.1.0` contract. It runs
//! no model and no network call, and it writes no project field:
//! the only bytes it writes are proposal manifests under
//! `.forge/semantic/`.

use std::fs;
use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::github::normalize::normalize_topics;

use super::proposal::{Confidence, ProposalEvidence, ProposalKind, Provider};
use super::review::{suggest, SuggestOutcome, SuggestRequest};

/// Maximum number of portfolio tags recorded in one proposal.
pub const MAX_DERIVE_TAGS: usize = 16;

/// Maximum length of the derived domain string. A README heading
/// longer than this is truncated, not trusted.
pub const MAX_DOMAIN_CHARS: usize = 120;

/// Maximum length of the README heading excerpt kept as evidence.
pub const MAX_HEADING_EXCERPT_CHARS: usize = 200;

/// Derive classification proposals for the project at
/// `project_path` and record them through the existing proposal
/// contract. Returns one outcome per proposal kind that had
/// evidence. Refuses with `ManifestNotFound` when the project has
/// no `forge.yaml`.
pub fn derive(project_path: &Path) -> Result<Vec<SuggestOutcome>, ForgeError> {
    let (manifest, manifest_path) = Manifest::load_from_dir(project_path, None)?;
    let manifest_bytes = fs::read(&manifest_path).map_err(|err| ForgeError::ManifestInvalid {
        path: manifest_path.display().to_string(),
        reason: format!("cannot read manifest: {err}"),
    })?;
    let manifest_revision = short_hash(&manifest_bytes);
    let mut outcomes = Vec::new();

    // profile: declared in the manifest, so high confidence.
    outcomes.push(suggest(&SuggestRequest {
        project_path: project_path.to_path_buf(),
        kind: ProposalKind::Profile,
        current_value: None,
        suggested_value: manifest.project.profile.clone(),
        confidence: Confidence::High,
        provider: Provider::Local,
        evidence: vec![ProposalEvidence::new(
            "forge.yaml",
            manifest_revision.clone(),
            "project.profile",
        )?],
        note: Some("derived from the manifest's declared project.profile".to_string()),
        now: Utc::now(),
    })?);

    // lifecycle: high when current == target, medium on a gap.
    if let Some(maturity) = manifest.project.maturity {
        let confidence = match manifest.project.target_maturity {
            Some(target) if target != maturity => Confidence::Medium,
            _ => Confidence::High,
        };
        let excerpt = match manifest.project.target_maturity {
            Some(target) => {
                format!("project.maturity={maturity}; project.target_maturity={target}")
            }
            None => format!("project.maturity={maturity}"),
        };
        outcomes.push(suggest(&SuggestRequest {
            project_path: project_path.to_path_buf(),
            kind: ProposalKind::Lifecycle,
            current_value: None,
            suggested_value: maturity.to_string(),
            confidence,
            provider: Provider::Local,
            evidence: vec![ProposalEvidence::new(
                "forge.yaml",
                manifest_revision.clone(),
                excerpt,
            )?],
            note: Some(
                "derived from the manifest's declared project.maturity/target_maturity".to_string(),
            ),
            now: Utc::now(),
        })?);
    }

    // portfolio-tags: GitHub topics when observed, else local
    // evidence only — and the evidence says which. No adapter or
    // token means the observation is unavailable, so the local
    // manifest runtime language is the only source and the
    // evidence records that.
    let mut local_tags: Vec<String> = Vec::new();
    if let Some(language) = manifest
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.language.as_deref())
    {
        local_tags.push(language.trim().to_string());
    }
    let mut tags = normalize_topics(&local_tags);
    tags.truncate(MAX_DERIVE_TAGS);
    if !tags.is_empty() {
        let tag_evidence_excerpt = "github observation unavailable (no adapter/token); derived from local evidence only (forge.yaml runtime.language)";
        outcomes.push(suggest(&SuggestRequest {
            project_path: project_path.to_path_buf(),
            kind: ProposalKind::PortfolioTags,
            current_value: None,
            suggested_value: tags.join(", "),
            confidence: Confidence::Medium,
            provider: Provider::Local,
            evidence: vec![ProposalEvidence::new(
                "forge.yaml",
                manifest_revision.clone(),
                tag_evidence_excerpt,
            )?],
            note: Some(
                "derived from the manifest runtime language; GitHub topics are human-set and may be stale"
                    .to_string(),
            ),
            now: Utc::now(),
        })?);
    }

    // domain: README's first `#` heading, bounded; falls back to
    // the manifest name. Low confidence — a heading is a weak
    // signal.
    let readme_path = project_path.join("README.md");
    let readme_bytes = fs::read(&readme_path).ok();
    let heading = readme_bytes
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(first_heading);
    let (domain, domain_evidence_path, domain_excerpt) = match heading {
        Some(heading) => {
            let bounded: String = heading.chars().take(MAX_DOMAIN_CHARS).collect();
            let excerpt: String = heading.chars().take(MAX_HEADING_EXCERPT_CHARS).collect();
            (bounded, "README.md", excerpt)
        }
        None => (
            manifest.project.name.clone(),
            "forge.yaml",
            "project.name".to_string(),
        ),
    };
    let domain_revision = match readme_bytes.as_deref() {
        Some(bytes) => short_hash(bytes),
        None => manifest_revision.clone(),
    };
    outcomes.push(suggest(&SuggestRequest {
        project_path: project_path.to_path_buf(),
        kind: ProposalKind::Domain,
        current_value: None,
        suggested_value: domain,
        confidence: Confidence::Low,
        provider: Provider::Local,
        evidence: vec![ProposalEvidence::new(
            domain_evidence_path,
            domain_revision,
            domain_excerpt,
        )?],
        note: Some(
            "derived from the README's first heading; a heading is a weak signal".to_string(),
        ),
        now: Utc::now(),
    })?);

    Ok(outcomes)
}

fn first_heading(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let heading = rest.trim();
            if !heading.is_empty() {
                return Some(heading.to_string());
            }
        }
    }
    None
}

fn short_hash(bytes: &[u8]) -> String {
    let digest = Sha256::new();
    let mut hasher = digest;
    hasher.update(bytes);
    let hex = format!("{:x}", hasher.finalize());
    hex.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_heading_takes_the_first_level_one_heading() {
        let text = "intro\n\n# Hello World\n\nbody\n# Second\n";
        assert_eq!(first_heading(text).as_deref(), Some("Hello World"));
    }

    #[test]
    fn first_heading_ignores_level_two_headings() {
        let text = "## Sub\n# Real\n";
        assert_eq!(first_heading(text).as_deref(), Some("Real"));
    }

    #[test]
    fn first_heading_returns_none_without_a_heading() {
        assert_eq!(first_heading("no heading\n"), None);
    }

    #[test]
    fn short_hash_is_stable_and_bounded() {
        assert_eq!(short_hash(b"abc"), short_hash(b"abc"));
        assert_eq!(short_hash(b"abc").len(), 12);
    }
}

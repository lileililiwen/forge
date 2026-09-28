//! The public artifact: the contracted document, its canonical bytes
//! and its SHA-256.
//!
//! Determinism is a property of *shape*, not of luck. The hashed part
//! of the manifest is [`ManifestBody`], which carries only the
//! contracted fields; its projects are sorted by stable id and its
//! surfaces by `(label, url)`. The emission time lives on the
//! published [`PublicPortfolioManifest`] instead, so re-exporting an
//! unchanged catalog hashes identically.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::portfolio::share::{ShareFinding, ShareRecord};

use super::validation::{
    looks_like_secret, secret_finding, validate_public_text, validate_public_url,
    validate_status_evidence,
};
use super::{
    ShowcaseStatus, Visibility, MANIFEST_SCHEMA_FAMILY, MANIFEST_SCHEMA_VERSION,
    MAX_CATEGORY_CHARS, MAX_SUMMARY_CHARS, MAX_TITLE_CHARS,
};

// --- public manifest ----------------------------------------------------

/// One project entry of the public manifest. The field set is the
/// consumed contract's field set and nothing else: there is no
/// arbitrary metadata map, no private note, no local path and no
/// analytics payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSurface {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestProject {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub source_url: String,
    pub visibility: String,
    pub showcase_status: String,
    pub featured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub demo_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<ManifestSurface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_evidence: Option<BTreeMap<String, String>>,
}

/// The hashed part of a manifest.
///
/// `generated_at` and `manifest_sha256` are deliberately **not**
/// fields of the body: the hash must depend only on the approved
/// records, so a re-export of an unchanged catalog hashes identically
/// while the published document still carries its own emission time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestBody {
    pub schema_family: String,
    pub schema_version: u32,
    pub manifest_revision: u32,
    pub projects: Vec<ManifestProject>,
}

impl ManifestBody {
    pub fn new(manifest_revision: u32, projects: Vec<ManifestProject>) -> Self {
        Self {
            schema_family: MANIFEST_SCHEMA_FAMILY.to_string(),
            schema_version: MANIFEST_SCHEMA_VERSION,
            manifest_revision,
            projects,
        }
    }

    /// Compact canonical bytes. The struct field order is fixed, the
    /// projects are sorted by stable id and the surfaces by
    /// `(label, url)`, so the same records always produce the same
    /// bytes on every platform.
    pub fn canonical_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Lowercase hex SHA-256 of [`ManifestBody::canonical_json`].
    pub fn sha256(&self) -> String {
        sha256_hex(self.canonical_json().as_bytes())
    }
}

/// The published document: the canonical body plus its emission time
/// and the hash that binds it to an approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPortfolioManifest {
    pub schema_family: String,
    pub schema_version: u32,
    pub generated_at: String,
    pub manifest_revision: u32,
    pub manifest_sha256: String,
    pub projects: Vec<ManifestProject>,
}

/// Lowercase hex SHA-256 of arbitrary bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// A manifest candidate plus the findings that must be cleared before
/// it may be approved.
#[derive(Debug, Clone)]
pub struct ManifestDraft {
    pub body: ManifestBody,
    pub findings: Vec<ShareFinding>,
}

impl ManifestDraft {
    /// Whether the candidate may be approved. A draft carrying any
    /// finding is never approvable: the operator resolves the
    /// offending record first.
    pub fn approvable(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn project_count(&self) -> usize {
        self.body.projects.len()
    }

    pub fn manifest_sha256(&self) -> String {
        self.body.sha256()
    }

    /// The exact document a publisher receives. It embeds the
    /// emission time and the hash the approval was bound to.
    pub fn document(&self, generated_at: &str) -> String {
        let manifest = PublicPortfolioManifest {
            schema_family: self.body.schema_family.clone(),
            schema_version: self.body.schema_version,
            generated_at: generated_at.to_string(),
            manifest_revision: self.body.manifest_revision,
            manifest_sha256: self.manifest_sha256(),
            projects: self.body.projects.clone(),
        };
        let mut text = serde_json::to_string_pretty(&manifest).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// Build the candidate manifest from the stored share records.
///
/// This is the second validation gate: a record was already checked
/// when it was written, and it is checked again here so a row written
/// by an older build, or a record an admin has since broken, can never
/// reach a public artifact. A record that fails is omitted from the
/// catalog and produces a finding; the draft is then not approvable.
pub fn build_manifest(records: &[ShareRecord], manifest_revision: u32) -> ManifestDraft {
    let mut projects = Vec::new();
    let mut findings = Vec::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();

    for record in records {
        if let Err(detail) = check_record_for_publication(record) {
            findings.push(ShareFinding::new(
                &record.project_id,
                "record",
                "share-record-not-publishable",
                detail,
            ));
            continue;
        }
        if !seen_ids.insert(record.project_id.clone()) {
            findings.push(ShareFinding::new(
                &record.project_id,
                "id",
                "share-duplicate-id",
                format!(
                    "project `{}` appears more than once in the share registry; \
                     a public manifest carries each project once",
                    record.project_id
                ),
            ));
            continue;
        }
        match manifest_project(record) {
            Ok(project) => projects.push(project),
            Err((field, detail)) => findings.push(ShareFinding::new(
                &record.project_id,
                &field,
                "share-field-invalid",
                detail,
            )),
        }
    }
    projects.sort_by(|a, b| a.id.cmp(&b.id));
    ManifestDraft {
        body: ManifestBody::new(manifest_revision, projects),
        findings,
    }
}

/// Re-check one stored record before it may enter a public manifest.
fn check_record_for_publication(record: &ShareRecord) -> Result<(), String> {
    if !record.state.is_publishable() {
        return Err(format!(
            "share record is `{}`; a rejected record is never published",
            record.state.label()
        ));
    }
    for (field, value) in [
        ("title", record.title.as_str()),
        ("summary", record.summary.as_str()),
        ("category", record.category.as_str()),
        ("source_url", record.source_url.as_str()),
    ] {
        if looks_like_secret(value) {
            return Err(secret_finding(field));
        }
    }
    if let Some(demo_url) = &record.demo_url {
        validate_public_url("demo_url", demo_url)?;
    }
    validate_public_url("source_url", &record.source_url)?;
    for surface in &record.surfaces {
        surface.validate()?;
    }
    validate_status_evidence(record.status_evidence.as_deref())?;
    Ok(())
}

/// Project one stored record onto the public manifest shape.
fn manifest_project(record: &ShareRecord) -> Result<ManifestProject, (String, String)> {
    let field_error = |field: &str, detail: String| (field.to_string(), detail);
    validate_public_text("title", &record.title, MAX_TITLE_CHARS, false)
        .map_err(|e| field_error("title", e))?;
    validate_public_text("summary", &record.summary, MAX_SUMMARY_CHARS, false)
        .map_err(|e| field_error("summary", e))?;
    validate_public_text("category", &record.category, MAX_CATEGORY_CHARS, false)
        .map_err(|e| field_error("category", e))?;
    validate_public_url("source_url", &record.source_url)
        .map_err(|e| field_error("source_url", e))?;
    let demo_url = record
        .demo_url
        .as_deref()
        .map(|value| validate_public_url("demo_url", value))
        .transpose()
        .map_err(|e| field_error("demo_url", e))?;
    let visibility =
        Visibility::parse(&record.visibility).map_err(|e| field_error("visibility", e))?;
    let showcase_status = ShowcaseStatus::parse(&record.showcase_status)
        .map_err(|e| field_error("showcase_status", e))?;
    let status_evidence = record
        .status_evidence
        .as_deref()
        .and_then(|raw| serde_json::from_str::<BTreeMap<String, String>>(raw).ok())
        .filter(|entries| !entries.is_empty());
    let mut surfaces = Vec::with_capacity(record.surfaces.len());
    for surface in &record.surfaces {
        let validated = surface.validate().map_err(|e| field_error("surfaces", e))?;
        surfaces.push(ManifestSurface {
            label: validated.label,
            url: validated.url,
        });
    }
    surfaces.sort_by(|a, b| (&a.label, &a.url).cmp(&(&b.label, &b.url)));
    Ok(ManifestProject {
        id: record.project_id.clone(),
        title: record.title.clone(),
        summary: record.summary.clone(),
        category: record.category.clone(),
        source_url: record.source_url.clone(),
        visibility: visibility.label().to_string(),
        showcase_status: showcase_status.label().to_string(),
        featured: record.featured,
        demo_url,
        surfaces,
        status_evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::share::{ShareState, ShareSurface};

    fn record(project: &str, url: &str) -> ShareRecord {
        ShareRecord {
            project_id: project.to_string(),
            title: format!("Project {project}"),
            summary: "A public showcase summary.".to_string(),
            category: "platform".to_string(),
            source_url: url.to_string(),
            demo_url: None,
            visibility: "public".to_string(),
            featured: false,
            showcase_status: "demo".to_string(),
            status_evidence: None,
            state: ShareState::Validated,
            revision: 1,
            created_at: "2026-09-29T00:00:00Z".to_string(),
            updated_at: "2026-09-29T00:00:00Z".to_string(),
            surfaces: Vec::new(),
        }
    }

    #[test]
    fn manifest_ordering_and_hash_are_deterministic() {
        let a = record("beta", "https://example.com/beta");
        let alpha = record("alpha", "https://example.com/alpha");
        let forward = build_manifest(&[a.clone(), alpha.clone()], 1);
        let backward = build_manifest(&[alpha, a], 1);
        assert!(forward.approvable());
        assert_eq!(forward.body.projects[0].id, "alpha");
        assert_eq!(
            forward.body.canonical_json(),
            backward.body.canonical_json()
        );
        assert_eq!(forward.manifest_sha256(), backward.manifest_sha256());
        assert_eq!(forward.manifest_sha256().len(), 64);
        assert!(forward
            .manifest_sha256()
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn changing_a_title_changes_the_hash() {
        let base = build_manifest(&[record("alpha", "https://example.com/a")], 1);
        let mut edited = record("alpha", "https://example.com/a");
        edited.title = "Renamed".to_string();
        let changed = build_manifest(&[edited], 1);
        assert_ne!(base.manifest_sha256(), changed.manifest_sha256());
    }

    #[test]
    fn an_empty_catalog_is_valid_and_publishable() {
        let empty = build_manifest(&[], 1);
        assert!(empty.approvable());
        assert_eq!(empty.project_count(), 0);
        assert!(empty
            .document("2026-09-29T00:00:00Z")
            .contains("\"projects\": []"));
    }

    #[test]
    fn a_rejected_record_is_omitted_and_blocks_approval() {
        let mut rejected = record("alpha", "https://example.com/a");
        rejected.state = ShareState::Rejected;
        let draft = build_manifest(&[rejected], 1);
        assert_eq!(draft.project_count(), 0);
        assert!(!draft.approvable());
        assert_eq!(draft.findings[0].code, "share-record-not-publishable");
        assert!(!draft.findings[0].detail.contains("https://"));
    }

    #[test]
    fn a_stored_private_surface_is_caught_by_the_second_gate() {
        let mut leaky = record("alpha", "https://example.com/a");
        leaky.surfaces = vec![ShareSurface {
            label: "Admin".to_string(),
            url: "https://example.com/admin/settings".to_string(),
        }];
        let draft = build_manifest(&[leaky], 1);
        assert_eq!(draft.project_count(), 0);
        assert!(!draft.approvable());
        assert_eq!(draft.findings[0].field, "record");
    }

    #[test]
    fn a_duplicate_project_identity_produces_one_entry_and_a_finding() {
        let first = record("alpha", "https://example.com/a");
        let mut second = record("alpha", "https://example.com/a2");
        second.summary = "A second claim on the same identity.".to_string();
        let draft = build_manifest(&[first, second], 1);
        assert_eq!(draft.project_count(), 1);
        assert_eq!(draft.findings[0].code, "share-duplicate-id");
    }

    #[test]
    fn the_document_embeds_the_hash_and_the_emission_time() {
        let draft = build_manifest(&[record("alpha", "https://example.com/a")], 4);
        let document = draft.document("2026-09-29T00:00:00Z");
        let parsed: PublicPortfolioManifest = serde_json::from_str(&document).expect("document");
        assert_eq!(parsed.schema_family, MANIFEST_SCHEMA_FAMILY);
        assert_eq!(parsed.schema_version, MANIFEST_SCHEMA_VERSION);
        assert_eq!(parsed.manifest_revision, 4);
        assert_eq!(parsed.manifest_sha256, draft.manifest_sha256());
        assert_eq!(parsed.projects[0].id, "alpha");
        assert!(document.ends_with('\n'));
    }

    #[test]
    fn the_manifest_carries_no_private_field() {
        let draft = build_manifest(&[record("alpha", "https://example.com/a")], 1);
        let parsed: PublicPortfolioManifest =
            serde_json::from_str(&draft.document("2026-09-29T00:00:00Z")).expect("document");
        // `serde_json::Value` objects are sorted maps, so this asserts
        // the *set* of public keys, not their order. The published
        // document itself is serialized from a struct, so its byte
        // order is the declaration order and is deterministic.
        let document = serde_json::to_value(&parsed).expect("value");
        let mut keys: Vec<String> = document
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "generated_at",
                "manifest_revision",
                "manifest_sha256",
                "projects",
                "schema_family",
                "schema_version",
            ]
        );
        let project = serde_json::to_value(&parsed.projects[0]).expect("value");
        let mut project_keys: Vec<String> = project
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect();
        project_keys.sort();
        assert_eq!(
            project_keys,
            vec![
                "category",
                "featured",
                "id",
                "showcase_status",
                "source_url",
                "summary",
                "title",
                "visibility",
            ]
        );
    }
}

//! Portfolio share domain: the explicit allowlist that may leave Forge.
//!
//! Everything else in [`crate::portfolio`] is private horizontal
//! project management data. This module is the one door out of it,
//! and it is deliberately narrow:
//!
//! - A project is public **only** because an admin wrote a share
//!   record for it. There is no auto-discovery, no implicit
//!   listing and no fallback that turns a registered project into a
//!   showcase entry.
//! - The public artifact is a versioned
//!   [`MANIFEST_SCHEMA_FAMILY`] document. Its shape is owned by the
//!   `platform-contracts` repository; Forge only *produces* it and
//!   refuses to emit a record that does not validate against that
//!   shape.
//! - Publication requires an exact approval. The approval binds a
//!   SHA-256 hash of the canonical manifest body, so editing a
//!   title or a URL after approval invalidates the approval and the
//!   publication is refused until the new revision is approved.
//!
//! The module stores nothing (persistence lives in
//! [`crate::registry::share`]) and never contacts a public host. The
//! default publisher writes a local file; an external adapter is
//! optional and is handed a manifest, never a query it could use to
//! read private Forge state.
//!
//! The concerns are split so each file answers one question:
//!
//! - this module — what a share record *is*: the closed vocabularies,
//!   the bounds, and the record/surface/finding shapes.
//! - [`validation`] — what may leave Forge: secret detection, public
//!   URL and text rules, the closed evidence block, and the one
//!   `validate_share` gate every transport goes through.
//! - [`manifest`] — what the public artifact *is*: the contracted
//!   document, its canonical bytes and its SHA-256.
//! - [`publish`] — how an approved manifest reaches a target.

pub mod manifest;
pub mod publish;
pub mod validation;

mod audit_types;

pub use audit_types::{validate_actor, validate_operation_key, PublicationAttempt, ShareApproval};
pub use manifest::{
    build_manifest, sha256_hex, wire_manifest_revision, ManifestBody, ManifestDraft,
    ManifestProject, ManifestSurface, PublicPortfolioManifest,
};
pub use publish::{
    publish_timeout, AdapterRequest, AdapterResponse, LocalFilePublisher, PublishContext,
    PublishOutcome, SharePublisher, SubprocessPublisher,
};
pub use validation::{
    looks_like_secret, validate_public_url, validate_share, validate_status_evidence,
};

use serde::Serialize;

use validation::validate_public_text;

use crate::policy::redact_credentials;

pub const SHARE_CONTRACT_VERSION: &str = "forge-portfolio-share/0.1.0";

/// Schema family of the consumed public manifest contract. The
/// serialized shape is owned by `platform-contracts`, whose
/// `public-portfolio-manifest.schema.json` declares this exact
/// family as a closed enum. Forge pins the qualified name and the
/// full version it produces and never invents a field.
pub const MANIFEST_SCHEMA_FAMILY: &str = "platform.public-portfolio-manifest";

/// Full semantic version of the consumed manifest contract, as the
/// contract declares it (`^[0-9]+\.[0-9]+\.[0-9]+$`). Forge supports
/// major 1 and produces no minor or patch variant, so the version it
/// writes is `1.0.0` — a *string*, not the integer major.
pub const MANIFEST_SCHEMA_VERSION: &str = "1.0.0";

/// Contract version of the request/response envelope exchanged with
/// an optional external publication adapter.
pub const SHARE_ADAPTER_CONTRACT: &str = "forge-portfolio-share-adapter/0.1.0";

/// Upper bound on a public title, in characters.
pub const MAX_TITLE_CHARS: usize = 120;

/// Upper bound on a public summary, in characters.
pub const MAX_SUMMARY_CHARS: usize = 500;

/// Upper bound on a public category, in characters.
pub const MAX_CATEGORY_CHARS: usize = 64;

/// Upper bound on a public surface label, in characters.
pub const MAX_SURFACE_LABEL_CHARS: usize = 64;

/// Upper bound on a public URL, in characters.
pub const MAX_URL_CHARS: usize = 2048;

/// Upper bound on the surfaces attached to one record.
pub const MAX_SURFACES: usize = 32;

/// Upper bound on the keys of one `status_evidence` object.
pub const MAX_EVIDENCE_KEYS: usize = 8;

/// Upper bound on a rendered manifest, in bytes. A catalog with an
/// unreasonable number of records is refused before it is written.
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

/// Default bound on one publication attempt, in seconds.
pub const PUBLISH_TIMEOUT_SECS: u64 = 120;

/// Path segments that never belong in a public showcase surface.
/// Matched case-insensitively against a whole segment, so
/// `/administrating` is allowed while `/admin` and `/admin/settings`
/// are refused.
pub const PRIVATE_PATH_SEGMENTS: [&str; 34] = [
    "_admin",
    "account",
    "accounts",
    "admin",
    "administer",
    "administrator",
    "apikey",
    "billing",
    "checkout",
    "console",
    "dashboard",
    "internal",
    "intranet",
    "login",
    "logout",
    "manage",
    "management",
    "password",
    "payment",
    "payments",
    "private",
    "secret",
    "secrets",
    "settings",
    "signin",
    "signup",
    "token",
    "tokens",
    "webhook",
    "webhooks",
    "account-recovery",
    "api",
    "ssh",
    "vpn",
];

/// Suffixes that never belong in a public host name. A public
/// showcase must resolve on the public internet, not on an operator's
/// workstation, cluster or private DNS zone.
pub const PRIVATE_HOST_SUFFIXES: [&str; 10] = [
    "localhost",
    ".localhost",
    ".local",
    ".internal",
    ".intranet",
    ".lan",
    ".home.arpa",
    ".test",
    ".corp",
    ".private",
];

/// Query-string keys that carry a credential. A public surface URL
/// that carries one of them is refused rather than published.
pub const CREDENTIAL_QUERY_KEYS: [&str; 12] = [
    "access_token",
    "api_key",
    "apikey",
    "auth",
    "authorization",
    "credential",
    "key",
    "passwd",
    "password",
    "secret",
    "sig",
    "token",
];

/// Keys a `status_evidence` object may carry. There is no arbitrary
/// metadata map: an evidence block is a short, closed, redaction-safe
/// description of *who observed what and when*.
pub const EVIDENCE_KEYS: [&str; 5] = [
    "observed_at",
    "source_system",
    "source_revision",
    "state",
    "source",
];

// --- share state --------------------------------------------------------

/// Where one share record sits in the publication lifecycle.
///
/// `Validated` is the entry state: Forge validates a record *before*
/// it persists it, so no record is ever stored as an unvalidated
/// draft. A record becomes `Approved` when it is part of an exactly
/// approved manifest, `Published` when that manifest reaches its
/// target, `Superseded` when a later manifest replaces it, and
/// `Rejected` when an attempted edit would have made the public
/// record invalid — a rejected record leaves the manifest rather than
/// silently serving its previous content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShareState {
    Validated,
    Approved,
    Published,
    Superseded,
    Rejected,
}

impl ShareState {
    pub const ALL: [ShareState; 5] = [
        ShareState::Validated,
        ShareState::Approved,
        ShareState::Published,
        ShareState::Superseded,
        ShareState::Rejected,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ShareState::Validated => "validated",
            ShareState::Approved => "approved",
            ShareState::Published => "published",
            ShareState::Superseded => "superseded",
            ShareState::Rejected => "rejected",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown share state `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }

    /// Whether a record in this state may enter a public manifest.
    /// `Rejected` never may: an admin's last attempt to make the
    /// record valid failed, so Forge stops serving the record instead
    /// of keeping stale approved content public.
    pub fn is_publishable(&self) -> bool {
        !matches!(self, ShareState::Rejected)
    }
}

// --- visibility ---------------------------------------------------------

/// How discoverable a shared record is meant to be. This is a
/// presentation hint for the consumer; it never changes who may
/// publish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Public,
    Unlisted,
}

impl Visibility {
    pub const ALL: [Visibility; 2] = [Visibility::Public, Visibility::Unlisted];

    pub fn label(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Unlisted => "unlisted",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown visibility `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- showcase status ----------------------------------------------------

/// The communication status a visitor sees. This is a statement the
/// admin makes about how presentable a project is — deliberately
/// *not* a production-readiness or availability claim, and never
/// derived from a liveness probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShowcaseStatus {
    Planned,
    Demo,
    Beta,
    Stable,
    Archived,
    Unknown,
}

impl ShowcaseStatus {
    pub const ALL: [ShowcaseStatus; 6] = [
        ShowcaseStatus::Planned,
        ShowcaseStatus::Demo,
        ShowcaseStatus::Beta,
        ShowcaseStatus::Stable,
        ShowcaseStatus::Archived,
        ShowcaseStatus::Unknown,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ShowcaseStatus::Planned => "planned",
            ShowcaseStatus::Demo => "demo",
            ShowcaseStatus::Beta => "beta",
            ShowcaseStatus::Stable => "stable",
            ShowcaseStatus::Archived => "archived",
            ShowcaseStatus::Unknown => "unknown",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown showcase status `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- publication status -------------------------------------------------

/// The terminal state of one publication attempt.
///
/// `Unknown` is the honest state for a *partial* external
/// publication: Forge cannot say whether the target accepted the
/// manifest, so it never reports success, and a new revision stays
/// blocked until an operator reconciles the attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublicationStatus {
    Published,
    Failed,
    Unknown,
}

impl PublicationStatus {
    pub const ALL: [PublicationStatus; 3] = [
        PublicationStatus::Published,
        PublicationStatus::Failed,
        PublicationStatus::Unknown,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PublicationStatus::Published => "published",
            PublicationStatus::Failed => "failed",
            PublicationStatus::Unknown => "unknown",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|v| v.label() == raw)
            .ok_or_else(|| {
                format!(
                    "unknown publication status `{raw}`; expected one of {}",
                    Self::labels().join(", ")
                )
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::ALL.into_iter().map(|v| v.label()).collect()
    }
}

// --- records ------------------------------------------------------------

/// One allowlisted public surface: a human label and the HTTPS URL
/// it points at. A surface is the *only* way an operator exposes a
/// specific application path, and it is refused for every private or
/// administrative segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShareSurface {
    pub label: String,
    pub url: String,
}

impl ShareSurface {
    /// Split the `label=url` surface form into an *unvalidated* pair.
    ///
    /// Transports call this rather than validating themselves: the
    /// registry's [`validate_share`] is the single gate, so a surface
    /// a transport cannot parse and one the domain rejects are
    /// recorded the same way, in the same place.
    pub fn split(raw: &str) -> Result<Self, String> {
        let Some((label, url)) = raw.split_once('=') else {
            return Err(format!(
                "surface `{raw}` must be written as `label=https://…`"
            ));
        };
        Ok(Self::new(label, url))
    }

    /// An unvalidated label/URL pair, ready for [`validate_share`].
    pub fn new(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            url: url.into(),
        }
    }

    /// Check the label shape and the public-surface URL rules.
    pub fn validate(&self) -> Result<Self, String> {
        Ok(Self {
            label: validate_public_text(
                "surface label",
                &self.label,
                MAX_SURFACE_LABEL_CHARS,
                false,
            )?,
            url: validate_public_url("surface url", &self.url)?,
        })
    }
}

/// A validated, ready-to-write share record.
#[derive(Debug, Clone)]
pub struct ShareWrite {
    pub title: String,
    pub summary: String,
    pub category: String,
    pub source_url: String,
    pub demo_url: Option<String>,
    pub visibility: Visibility,
    pub featured: bool,
    pub showcase_status: ShowcaseStatus,
    pub status_evidence: Option<String>,
    pub surfaces: Vec<ShareSurface>,
}

/// A persisted share record plus its allowlisted surfaces.
#[derive(Debug, Clone, Serialize)]
pub struct ShareRecord {
    pub project_id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub source_url: String,
    pub demo_url: Option<String>,
    pub visibility: String,
    pub featured: bool,
    pub showcase_status: String,
    pub status_evidence: Option<String>,
    pub state: ShareState,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub surfaces: Vec<ShareSurface>,
}

// --- findings -----------------------------------------------------------

/// One persisted reason a record or manifest was refused. The detail
/// names the field and the rule; it never contains the offending value.
#[derive(Debug, Clone, Serialize)]
pub struct ShareFinding {
    pub project_id: String,
    pub field: String,
    pub code: String,
    pub detail: String,
    pub created_at: String,
}

impl ShareFinding {
    pub fn new(project_id: &str, field: &str, code: &str, detail: impl Into<String>) -> Self {
        Self {
            project_id: project_id.to_string(),
            field: field.to_string(),
            code: code.to_string(),
            detail: redact_credentials(&detail.into()),
            created_at: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_states_are_a_closed_vocabulary_and_rejected_never_publishes() {
        assert_eq!(
            ShareState::labels(),
            vec![
                "validated",
                "approved",
                "published",
                "superseded",
                "rejected"
            ]
        );
        assert!(ShareState::parse("published").is_ok());
        assert!(ShareState::parse("draft").is_err());
        assert!(ShareState::parse("published").unwrap().is_publishable());
        assert!(!ShareState::Rejected.is_publishable());
    }

    #[test]
    fn visibility_and_showcase_status_are_closed_vocabularies() {
        assert_eq!(Visibility::labels(), vec!["public", "unlisted"]);
        assert!(Visibility::parse("unlisted").is_ok());
        assert!(Visibility::parse("secret").is_err());
        assert_eq!(
            ShowcaseStatus::labels(),
            vec!["planned", "demo", "beta", "stable", "archived", "unknown"]
        );
        assert!(ShowcaseStatus::parse("stable").is_ok());
        // `online` is deliberately absent: a public catalog never
        // publishes an availability claim it did not verify.
        assert!(ShowcaseStatus::parse("online").is_err());
    }

    #[test]
    fn publication_status_separates_failure_from_an_unknown_outcome() {
        assert_eq!(
            PublicationStatus::labels(),
            vec!["published", "failed", "unknown"]
        );
        assert!(PublicationStatus::parse("unknown").is_ok());
        assert!(PublicationStatus::parse("succeeded").is_err());
    }

    #[test]
    fn operation_keys_and_actors_are_bounded() {
        assert_eq!(
            validate_operation_key(" publish-1 ").expect("key"),
            "publish-1"
        );
        assert!(validate_operation_key("").is_err());
        assert!(validate_operation_key("has space").is_err());
        assert!(validate_operation_key(&"k".repeat(129)).is_err());
        assert_eq!(validate_actor("ops-admin").expect("actor"), "ops-admin");
        assert!(validate_actor("  ").is_err());
    }
}

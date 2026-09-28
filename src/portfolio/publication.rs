//! Portfolio share publication: the one path from an approved
//! manifest to a public artifact.
//!
//! Every transport — the CLI, the JSON API and (later) a browser —
//! enters through [`publish_approved_manifest`]. Nothing else is
//! allowed to call a [`SharePublisher`]: a publisher that could be
//! reached without an exact approval would make the approval
//! decorative.
//!
//! The sequence is deliberately auditable at every step:
//!
//! 1. Refuse while any attempt is unreconciled (`unknown`) — Forge
//!    cannot say whether the last attempt landed.
//! 2. Refuse unless the newest approval's hash still equals the hash
//!    of the current records. An edit after approval invalidates it.
//! 3. Reserve exactly one publication attempt per operation key.
//! 4. Publish, then record the terminal outcome — including the
//!    honest `unknown` state a partial external publication produces.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::core::ForgeError;
use crate::portfolio::share::{
    self, LocalFilePublisher, PublicationStatus, PublishContext, SharePublisher,
};
use crate::registry::{PublicationReservation, Registry};

/// How one publication should be performed.
#[derive(Debug, Clone)]
pub struct PublishPlan {
    /// Retries name the same operation; a reused key never creates a
    /// second public artifact.
    pub operation_key: String,
    /// Where the manifest is published. For the local publisher this
    /// is the artifact path; for an adapter it is the deployment
    /// target name.
    pub target: String,
    /// Who is publishing. Recorded verbatim in the audit trail.
    pub actor: String,
    /// Optional external adapter. `None` selects the default local
    /// file publisher at `target`.
    pub adapter: Option<PathBuf>,
}

/// What one publication attempt did.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublishReport {
    pub publication_id: i64,
    pub operation_key: String,
    pub manifest_revision: i64,
    pub manifest_sha256: String,
    pub status: PublicationStatus,
    pub target: String,
    pub publisher: String,
    pub project_count: usize,
    pub generated_at: String,
    pub published_revision: Option<String>,
    pub error_code: Option<String>,
    /// True when a retry reconciled an existing attempt instead of
    /// repeating the publication.
    pub already_present: bool,
}

impl PublishReport {
    pub fn status_label(&self) -> &'static str {
        self.status.label()
    }
}

/// Preview the candidate manifest the next approval would bind.
///
/// Purely read-only: an operator can inspect the exact bytes and the
/// hash before anything is approved, and a preview never advances the
/// revision counter.
pub fn preview_manifest(registry: &Registry) -> Result<share::ManifestDraft, ForgeError> {
    let revision = registry.share_next_revision()?;
    registry.share_manifest_draft(revision)
}

/// Publish the currently approved manifest.
///
/// Returns `Err` before any publisher is invoked when the approval is
/// missing, stale, or a previous attempt is unreconciled.
pub fn publish_approved_manifest(
    registry: &Registry,
    plan: &PublishPlan,
    now: DateTime<Utc>,
) -> Result<PublishReport, ForgeError> {
    let operation_key = share::validate_operation_key(&plan.operation_key)
        .map_err(|reason| ForgeError::PortfolioShareInvalid { reason })?;
    let actor = share::validate_actor(&plan.actor)
        .map_err(|reason| ForgeError::PortfolioShareInvalid { reason })?;

    if let Some(pending) = registry.share_unreconciled_publication()? {
        return Err(ForgeError::PortfolioShareConflict {
            reason: format!(
                "publication attempt {} (`{}`) is `unknown`; reconcile it with \
                 `forge portfolio share reconcile` before publishing another revision",
                pending.publication_id, pending.operation_key
            ),
        });
    }

    let approval =
        registry
            .share_latest_approval()?
            .ok_or_else(|| ForgeError::PortfolioShareConflict {
                reason: "no manifest has been approved yet; preview and approve a revision first"
                    .to_string(),
            })?;
    // An already-published revision is reachable here only through a
    // retry of its own operation key; a *new* key for an
    // already-published manifest is refused below, so a revision can
    // never be published twice.
    if approval.state != "approved" && approval.state != "published" {
        return Err(ForgeError::PortfolioShareConflict {
            reason: format!(
                "manifest revision {} is `{}`, not `approved`; approve the current records \
                 before publishing",
                approval.revision, approval.state
            ),
        });
    }

    let current = registry.share_manifest_draft(approval.revision as u32)?;
    if current.manifest_sha256() != approval.manifest_sha256 {
        return Err(ForgeError::PortfolioShareConflict {
            reason: format!(
                "the share records changed after revision {} was approved; the approved hash \
                 is `{}` and the current records hash to `{}`",
                approval.revision,
                approval.manifest_sha256,
                current.manifest_sha256()
            ),
        });
    }

    if approval.state == "published" && registry.share_publication_by_key(&operation_key)?.is_none()
    {
        return Err(ForgeError::PortfolioShareConflict {
            reason: format!(
                "manifest revision {} is already published; a new operation key would publish \
                 it a second time, so only a retry of its own key is accepted",
                approval.revision
            ),
        });
    }

    let reservation = registry.share_reserve_publication(
        &operation_key,
        approval.revision,
        &approval.manifest_sha256,
        &plan.target,
        &actor,
        &now.to_rfc3339(),
    )?;

    let context = PublishContext {
        operation_key: operation_key.clone(),
        manifest_revision: approval.revision,
        manifest_sha256: approval.manifest_sha256.clone(),
        target: plan.target.clone(),
    };
    let publisher = build_publisher(plan);
    let publisher_label = publisher.label();

    // A retry re-renders the bytes the *first* attempt emitted. The
    // emission time belongs to the attempt, not to the moment someone
    // pressed retry, so a retry reconciles the same artifact instead
    // of publishing a document that differs only in its timestamp.
    let (publication_id, generated_at) = match reservation {
        PublicationReservation::Reused {
            publication_id,
            manifest_sha256,
            document_generated_at,
            ..
        } => {
            if manifest_sha256 != approval.manifest_sha256 {
                return Err(ForgeError::PortfolioShareConflict {
                    reason: format!(
                        "operation key `{operation_key}` was already used for a different manifest"
                    ),
                });
            }
            (publication_id, document_generated_at)
        }
        PublicationReservation::Reserved {
            publication_id,
            document_generated_at,
        } => (publication_id, document_generated_at),
    };
    let document = current.document(&generated_at);

    let outcome = match publisher.publish(&document, &context) {
        Ok(outcome) => outcome,
        Err(err) => {
            // The approved manifest is retained and the attempt stays
            // retryable under the same operation key: a transport or
            // adapter failure is a `failed` attempt, never a silent
            // success and never a lost approval.
            let _ = registry.share_finish_publication(
                publication_id,
                PublicationStatus::Failed,
                None,
                Some(&share_failure_code(&err)),
            );
            return Err(err);
        }
    };

    let recorded = registry.share_finish_publication(
        publication_id,
        outcome.status,
        outcome.published_revision.as_deref(),
        outcome.error_code.as_deref(),
    )?;

    Ok(PublishReport {
        publication_id: recorded.publication_id,
        operation_key,
        manifest_revision: approval.revision,
        manifest_sha256: approval.manifest_sha256,
        status: outcome.status,
        target: plan.target.clone(),
        publisher: publisher_label,
        project_count: current.project_count(),
        generated_at: generated_at.clone(),
        published_revision: recorded.published_revision,
        error_code: recorded.error_code,
        already_present: outcome.already_present,
    })
}

/// Select the publisher for one plan: the optional credential-
/// injected adapter, or the default local file export.
fn build_publisher(plan: &PublishPlan) -> Box<dyn SharePublisher> {
    match &plan.adapter {
        Some(command) => Box::new(share::SubprocessPublisher::new(command.clone())),
        None => Box::new(LocalFilePublisher::new(PathBuf::from(&plan.target))),
    }
}

/// A stable, non-echoing failure code for the audit trail.
fn share_failure_code(err: &ForgeError) -> String {
    err.code().to_string()
}

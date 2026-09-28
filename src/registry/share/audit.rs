//! Approval and publication audit for the share allowlist.
//!
//! Everything in this file answers one of two questions about a past
//! publication: *what was it* and *did it land*.
//!
//! - An **approval** pins the canonical manifest bytes and their
//!   SHA-256. It is the only thing that can make a revision
//!   publishable, and it is granted for an exact hash: an edit after
//!   approval invalidates it.
//! - A **publication attempt** pins the operation key, the revision,
//!   the hash, the target and the outcome. Attempts are append-only —
//!   a retry resolves against the existing row rather than creating a
//!   second one — and an attempt whose outcome Forge cannot vouch for
//!   is recorded as `unknown` and blocks the next revision until an
//!   operator reconciles it.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::core::ForgeError;
use crate::portfolio::share::{self, PublicationAttempt, ShareApproval};

use super::{conflict, invalid};

/// Outcome of reserving one publication attempt against an operation
/// key. `Reused` means the audit row already exists: a retry must
/// resolve against that row rather than create a second publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationReservation {
    Reserved {
        publication_id: i64,
        document_generated_at: String,
    },
    Reused {
        publication_id: i64,
        status: String,
        manifest_sha256: String,
        /// The emission time the first attempt used. A retry re-renders
        /// the *same* bytes rather than publishing a document that
        /// differs only in its timestamp.
        document_generated_at: String,
    },
}

impl crate::registry::Registry {
    // --- manifest revision ----------------------------------------------

    /// The revision the *next* approval will receive.
    pub fn share_next_revision(&self) -> Result<u32, ForgeError> {
        let latest: Option<i64> = self
            .registry_conn()
            .query_row(
                "SELECT MAX(revision) FROM portfolio_share_approvals",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(latest.unwrap_or(0).max(0) as u32 + 1)
    }

    /// Build the candidate manifest from every stored share record.
    /// Read-only: a preview never writes, so an operator can inspect
    /// the exact bytes and hash before anything is approved.
    pub fn share_manifest_draft(&self, revision: u32) -> Result<share::ManifestDraft, ForgeError> {
        let records = self.share_records()?;
        Ok(share::build_manifest(&records, revision))
    }

    // --- approvals -------------------------------------------------------

    /// Record one approval of an exact manifest hash.
    ///
    /// The caller must already have confirmed that `manifest_sha256`
    /// is the hash of the current records; this method re-derives the
    /// draft inside the transaction and refuses a stale hash, so a
    /// record edited between preview and approval cannot slip through.
    pub fn share_approve(
        &self,
        manifest_sha256: &str,
        actor: &str,
    ) -> Result<ShareApproval, ForgeError> {
        let actor = share::validate_actor(actor).map_err(invalid)?;
        let next_revision = self.share_next_revision()?;
        let draft = self.share_manifest_draft(next_revision)?;
        if !draft.approvable() {
            let first = draft
                .findings
                .first()
                .map(|f| format!("{} `{}`: {}", f.code, f.field, f.detail))
                .unwrap_or_else(|| "unknown reason".to_string());
            return Err(conflict(format!(
                "candidate manifest carries {} unresolved finding(s); the first is {first}",
                draft.findings.len()
            )));
        }
        if draft.manifest_sha256() != manifest_sha256 {
            return Err(conflict(format!(
                "approved hash `{manifest_sha256}` does not match the current records, \
                 which hash to `{}`; preview and approve the same revision",
                draft.manifest_sha256()
            )));
        }
        let now = Utc::now().to_rfc3339();
        let tx = self.share_transaction()?;
        tx.execute(
            "UPDATE portfolio_share_approvals SET state = 'superseded'
             WHERE state IN ('approved', 'published')",
            [],
        )?;
        tx.execute(
            "INSERT INTO portfolio_share_approvals
                (manifest_sha256, manifest_json, project_count, actor, approved_at, state)
             VALUES (?1, ?2, ?3, ?4, ?5, 'approved')",
            params![
                manifest_sha256,
                draft.body.canonical_json(),
                draft.project_count() as i64,
                actor,
                now
            ],
        )?;
        let revision = tx.last_insert_rowid();
        // Approval is what makes a record publishable: every record
        // that survived the manifest build joins the approved set.
        tx.execute(
            "UPDATE portfolio_share_records SET state = 'approved' WHERE state <> 'rejected'",
            [],
        )?;
        tx.commit()?;
        self.share_approval(revision)?
            .ok_or_else(|| invalid("approval disappeared after it was recorded".to_string()))
    }

    /// Read one approval by revision.
    pub fn share_approval(&self, revision: i64) -> Result<Option<ShareApproval>, ForgeError> {
        self.registry_conn()
            .query_row(
                "SELECT revision, manifest_sha256, manifest_json, project_count,
                        actor, approved_at, state
                 FROM portfolio_share_approvals WHERE revision = ?1",
                params![revision],
                row_to_approval,
            )
            .optional()
            .map_err(Into::into)
    }

    /// The newest approval, whatever its state.
    pub fn share_latest_approval(&self) -> Result<Option<ShareApproval>, ForgeError> {
        self.registry_conn()
            .query_row(
                "SELECT revision, manifest_sha256, manifest_json, project_count,
                        actor, approved_at, state
                 FROM portfolio_share_approvals ORDER BY revision DESC LIMIT 1",
                [],
                row_to_approval,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Read the approval trail, newest first.
    pub fn share_approvals(&self, limit: usize) -> Result<Vec<ShareApproval>, ForgeError> {
        let mut stmt = self.registry_conn().prepare(
            "SELECT revision, manifest_sha256, manifest_json, project_count,
                    actor, approved_at, state
             FROM portfolio_share_approvals ORDER BY revision DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_approval)?;
        let mut approvals = Vec::new();
        for row in rows {
            approvals.push(row?);
        }
        Ok(approvals)
    }

    // --- publication attempts --------------------------------------------

    /// The newest attempt whose outcome Forge cannot vouch for. A
    /// `unknown` attempt blocks a new revision from being marked
    /// published: the previous publication may or may not have landed,
    /// and Forge never guesses.
    pub fn share_unreconciled_publication(&self) -> Result<Option<PublicationAttempt>, ForgeError> {
        self.registry_conn()
            .query_row(
                "SELECT id, operation_key, manifest_revision, manifest_sha256, target,
                        status, published_revision, error_code, actor, attempted_at,
                        finished_at, document_generated_at
                 FROM portfolio_share_publications WHERE status = 'unknown'
                 ORDER BY id LIMIT 1",
                [],
                row_to_publication,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Reserve one publication attempt against its operation key.
    ///
    /// The same key with the same manifest hash resolves to the
    /// existing attempt — that is what makes a retry after a timeout
    /// reconcile instead of publishing a second time. The same key
    /// with a *different* hash is a conflict, never a silent
    /// re-interpretation.
    #[allow(clippy::too_many_arguments)]
    pub fn share_reserve_publication(
        &self,
        operation_key: &str,
        manifest_revision: i64,
        manifest_sha256: &str,
        target: &str,
        actor: &str,
        document_generated_at: &str,
    ) -> Result<PublicationReservation, ForgeError> {
        if let Some(existing) = self.share_publication_by_key(operation_key)? {
            if existing.manifest_sha256 != manifest_sha256 {
                return Err(conflict(format!(
                    "operation key `{operation_key}` was already used for manifest `{}`; \
                     refusing to re-interpret it as a publication of `{manifest_sha256}`",
                    existing.manifest_sha256
                )));
            }
            return Ok(PublicationReservation::Reused {
                publication_id: existing.publication_id,
                status: existing.status,
                manifest_sha256: existing.manifest_sha256,
                document_generated_at: existing.document_generated_at,
            });
        }
        let actor = share::validate_actor(actor).map_err(invalid)?;
        let now = Utc::now().to_rfc3339();
        self.registry_conn().execute(
            "INSERT INTO portfolio_share_publications
                (operation_key, manifest_revision, manifest_sha256, target, status,
                 actor, attempted_at, document_generated_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7)",
            params![
                operation_key,
                manifest_revision,
                manifest_sha256,
                target,
                actor,
                now,
                document_generated_at
            ],
        )?;
        Ok(PublicationReservation::Reserved {
            publication_id: self.registry_conn().last_insert_rowid(),
            document_generated_at: document_generated_at.to_string(),
        })
    }

    /// Close one attempt with its terminal outcome. The row is updated,
    /// never replaced, so the audit trail keeps the attempt history of
    /// one operation key.
    pub fn share_finish_publication(
        &self,
        publication_id: i64,
        status: share::PublicationStatus,
        published_revision: Option<&str>,
        error_code: Option<&str>,
    ) -> Result<PublicationAttempt, ForgeError> {
        let now = Utc::now().to_rfc3339();
        let tx = self.share_transaction()?;
        tx.execute(
            "UPDATE portfolio_share_publications
                SET status = ?1, published_revision = ?2, error_code = ?3, finished_at = ?4
              WHERE id = ?5",
            params![
                status.label(),
                published_revision,
                error_code,
                now,
                publication_id
            ],
        )?;
        if status == share::PublicationStatus::Published {
            tx.execute(
                "UPDATE portfolio_share_approvals SET state = 'published' WHERE revision = (
                    SELECT manifest_revision FROM portfolio_share_publications WHERE id = ?1
                )",
                params![publication_id],
            )?;
            tx.execute(
                "UPDATE portfolio_share_approvals SET state = 'superseded'
                 WHERE revision < (
                    SELECT manifest_revision FROM portfolio_share_publications WHERE id = ?1
                 ) AND state = 'published'",
                params![publication_id],
            )?;
            // Approval set exactly the publishable records to
            // `approved`, so moving `approved` to `published` and the
            // previous `published` set to `superseded` mirrors the
            // approval-to-publication transition record for record.
            tx.execute(
                "UPDATE portfolio_share_records SET state = 'superseded'
                 WHERE state = 'published'",
                [],
            )?;
            tx.execute(
                "UPDATE portfolio_share_records SET state = 'published'
                 WHERE state = 'approved'",
                [],
            )?;
        }
        tx.commit()?;
        self.share_publication(publication_id)?
            .ok_or_else(|| invalid("publication attempt disappeared".to_string()))
    }

    /// Read one publication attempt by id.
    pub fn share_publication(
        &self,
        publication_id: i64,
    ) -> Result<Option<PublicationAttempt>, ForgeError> {
        self.registry_conn()
            .query_row(
                "SELECT id, operation_key, manifest_revision, manifest_sha256, target,
                        status, published_revision, error_code, actor, attempted_at,
                        finished_at, document_generated_at
                 FROM portfolio_share_publications WHERE id = ?1",
                params![publication_id],
                row_to_publication,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Read one publication attempt by its operation key.
    pub fn share_publication_by_key(
        &self,
        operation_key: &str,
    ) -> Result<Option<PublicationAttempt>, ForgeError> {
        self.registry_conn()
            .query_row(
                "SELECT id, operation_key, manifest_revision, manifest_sha256, target,
                        status, published_revision, error_code, actor, attempted_at,
                        finished_at, document_generated_at
                 FROM portfolio_share_publications WHERE operation_key = ?1",
                params![operation_key],
                row_to_publication,
            )
            .optional()
            .map_err(Into::into)
    }

    /// Read the publication audit trail, newest first.
    pub fn share_publications(&self, limit: usize) -> Result<Vec<PublicationAttempt>, ForgeError> {
        let mut stmt = self.registry_conn().prepare(
            "SELECT id, operation_key, manifest_revision, manifest_sha256, target,
                    status, published_revision, error_code, actor, attempted_at,
                    finished_at, document_generated_at
             FROM portfolio_share_publications ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_publication)?;
        let mut attempts = Vec::new();
        for row in rows {
            attempts.push(row?);
        }
        Ok(attempts)
    }

    /// Record one operator reconciliation of a partial publication.
    /// Only an `unknown` attempt can be reconciled, and the outcome is
    /// recorded as the operator's explicit statement rather than
    /// something Forge observed.
    pub fn share_reconcile_publication(
        &self,
        publication_id: i64,
        status: share::PublicationStatus,
        actor: &str,
    ) -> Result<PublicationAttempt, ForgeError> {
        let actor = share::validate_actor(actor).map_err(invalid)?;
        let existing = self.share_publication(publication_id)?.ok_or_else(|| {
            invalid(format!(
                "no publication attempt with id {publication_id} exists"
            ))
        })?;
        if existing.status != share::PublicationStatus::Unknown.label() {
            return Err(conflict(format!(
                "publication attempt {publication_id} is `{}`; only an `unknown` attempt \
                 needs reconciliation",
                existing.status
            )));
        }
        self.share_finish_publication(
            publication_id,
            status,
            existing.published_revision.as_deref(),
            existing.error_code.as_deref(),
        )?;
        self.registry_conn().execute(
            "UPDATE portfolio_share_publications SET actor = ?1 WHERE id = ?2",
            params![actor, publication_id],
        )?;
        self.share_publication(publication_id)?
            .ok_or_else(|| invalid("publication attempt disappeared".to_string()))
    }
}

fn row_to_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShareApproval> {
    Ok(ShareApproval {
        revision: row.get(0)?,
        manifest_sha256: row.get(1)?,
        manifest_json: row.get(2)?,
        project_count: row.get(3)?,
        actor: row.get(4)?,
        approved_at: row.get(5)?,
        state: row.get(6)?,
    })
}

fn row_to_publication(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicationAttempt> {
    Ok(PublicationAttempt {
        publication_id: row.get(0)?,
        operation_key: row.get(1)?,
        manifest_revision: row.get(2)?,
        manifest_sha256: row.get(3)?,
        target: row.get(4)?,
        status: row.get(5)?,
        published_revision: row.get(6)?,
        error_code: row.get(7)?,
        actor: row.get(8)?,
        attempted_at: row.get(9)?,
        finished_at: row.get(10)?,
        document_generated_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::share::ShareState;
    use crate::registry::share::fixtures::{registry_with_projects, tmp, write, STAMP};

    /// Register shareable projects in a disposable registry. The
    /// [`tempfile::TempDir`] is returned so each parallel test owns its
    /// own file rather than racing on a shared path.
    fn seeded(ids: &[&str]) -> (tempfile::TempDir, crate::registry::Registry) {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), ids);
        (dir, registry)
    }

    #[test]
    fn approval_requires_the_exact_current_hash() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let draft = registry.share_manifest_draft(1).expect("draft");
        let err = registry
            .share_approve(&"0".repeat(64), "ops-admin")
            .expect_err("stale hash");
        assert_eq!(err.code(), "portfolio-share-conflict");
        assert!(registry.share_approvals(10).expect("approvals").is_empty());

        let approval = registry
            .share_approve(&draft.manifest_sha256(), "ops-admin")
            .expect("approve");
        assert_eq!(approval.revision, 1);
        assert_eq!(approval.project_count, 1);
        assert_eq!(approval.state, "approved");
        assert_eq!(approval.actor, "ops-admin");
        let record = registry.share_record("alpha").expect("read").expect("row");
        assert_eq!(record.state, ShareState::Approved);
    }

    #[test]
    fn an_edit_after_approval_requires_a_new_approval() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let draft = registry.share_manifest_draft(1).expect("draft");
        let approval = registry
            .share_approve(&draft.manifest_sha256(), "ops-admin")
            .expect("approve");
        let mut edited = write("alpha");
        edited.title = "Alethefy".to_string();
        registry
            .share_upsert_record("alpha", &edited)
            .expect("edit");
        let after = registry.share_manifest_draft(2).expect("draft");
        assert_ne!(after.manifest_sha256(), approval.manifest_sha256);
        let err = registry
            .share_approve(&approval.manifest_sha256, "ops-admin")
            .expect_err("stale");
        assert_eq!(err.code(), "portfolio-share-conflict");
        assert_eq!(registry.share_approvals(10).expect("approvals").len(), 1);
    }

    #[test]
    fn an_empty_catalog_can_still_be_approved() {
        let (_dir, registry) = seeded(&["alpha"]);
        let draft = registry.share_manifest_draft(1).expect("draft");
        assert_eq!(draft.project_count(), 0);
        let approval = registry
            .share_approve(&draft.manifest_sha256(), "ops-admin")
            .expect("approve empty");
        assert_eq!(approval.project_count, 0);
    }

    #[test]
    fn a_second_approval_supersedes_the_first() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let first = registry
            .share_approve(
                &registry.share_manifest_draft(1).unwrap().manifest_sha256(),
                "ops-admin",
            )
            .expect("first");
        let mut edited = write("alpha");
        edited.category = "tooling".to_string();
        registry
            .share_upsert_record("alpha", &edited)
            .expect("edit");
        let second = registry
            .share_approve(
                &registry.share_manifest_draft(2).unwrap().manifest_sha256(),
                "ops-admin",
            )
            .expect("second");
        assert_eq!(second.revision, first.revision + 1);
        let previous = registry
            .share_approval(first.revision)
            .expect("read")
            .expect("row");
        assert_eq!(previous.state, "superseded");
        assert_eq!(registry.share_next_revision().expect("next"), 3);
    }

    #[test]
    fn a_publication_reserves_once_per_operation_key() {
        let (_dir, registry) = seeded(&["alpha"]);
        let reserved = registry
            .share_reserve_publication("op-1", 1, "hash-1", "github-pages", "ops-admin", STAMP)
            .expect("reserve");
        assert!(matches!(reserved, PublicationReservation::Reserved { .. }));
        let reused = registry
            .share_reserve_publication("op-1", 1, "hash-1", "github-pages", "ops-admin", STAMP)
            .expect("reuse");
        match reused {
            PublicationReservation::Reused {
                publication_id,
                manifest_sha256,
                document_generated_at,
                ..
            } => {
                assert_eq!(manifest_sha256, "hash-1");
                assert_eq!(document_generated_at, STAMP);
                assert!(publication_id > 0);
            }
            other => panic!("expected a reuse, got {other:?}"),
        }
        assert_eq!(registry.share_publications(10).expect("rows").len(), 1);
    }

    #[test]
    fn one_operation_key_refuses_a_different_manifest() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_reserve_publication("op-1", 1, "hash-1", "github-pages", "ops-admin", STAMP)
            .expect("reserve");
        let err = registry
            .share_reserve_publication("op-1", 2, "hash-2", "github-pages", "ops-admin", STAMP)
            .expect_err("conflict");
        assert_eq!(err.code(), "portfolio-share-conflict");
        assert_eq!(registry.share_publications(10).expect("rows").len(), 1);
    }

    #[test]
    fn a_published_attempt_marks_the_approval_and_records_published() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let draft = registry.share_manifest_draft(1).expect("draft");
        let approval = registry
            .share_approve(&draft.manifest_sha256(), "ops-admin")
            .expect("approve");
        let reserved = registry
            .share_reserve_publication(
                "op-1",
                approval.revision,
                &approval.manifest_sha256,
                "t",
                "ops-admin",
                STAMP,
            )
            .expect("reserve");
        let PublicationReservation::Reserved { publication_id, .. } = reserved else {
            panic!("expected a fresh reservation");
        };
        let finished = registry
            .share_finish_publication(
                publication_id,
                share::PublicationStatus::Published,
                Some(&approval.manifest_sha256),
                None,
            )
            .expect("finish");
        assert_eq!(finished.status, "published");
        assert!(finished.finished_at.is_some());
        assert_eq!(finished.document_generated_at, STAMP);
        let approval = registry
            .share_approval(approval.revision)
            .expect("read")
            .expect("row");
        assert_eq!(approval.state, "published");
        let record = registry.share_record("alpha").expect("read").expect("row");
        assert_eq!(record.state, ShareState::Published);
    }

    #[test]
    fn a_later_publication_supersedes_the_previous_record_set() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let first = registry
            .share_approve(
                &registry.share_manifest_draft(1).unwrap().manifest_sha256(),
                "ops-admin",
            )
            .expect("approve");
        let PublicationReservation::Reserved { publication_id, .. } = registry
            .share_reserve_publication(
                "op-1",
                first.revision,
                &first.manifest_sha256,
                "t",
                "ops-admin",
                STAMP,
            )
            .expect("reserve")
        else {
            panic!("expected a fresh reservation");
        };
        registry
            .share_finish_publication(
                publication_id,
                share::PublicationStatus::Published,
                None,
                None,
            )
            .expect("finish");
        let mut edited = write("alpha");
        edited.category = "tooling".to_string();
        registry
            .share_upsert_record("alpha", &edited)
            .expect("edit");
        let second = registry
            .share_approve(
                &registry.share_manifest_draft(2).unwrap().manifest_sha256(),
                "ops-admin",
            )
            .expect("approve");
        let PublicationReservation::Reserved { publication_id, .. } = registry
            .share_reserve_publication(
                "op-2",
                second.revision,
                &second.manifest_sha256,
                "t",
                "ops-admin",
                STAMP,
            )
            .expect("reserve")
        else {
            panic!("expected a fresh reservation");
        };
        registry
            .share_finish_publication(
                publication_id,
                share::PublicationStatus::Published,
                None,
                None,
            )
            .expect("finish");
        let record = registry.share_record("alpha").expect("read").expect("row");
        assert_eq!(record.state, ShareState::Published);
        let previous = registry
            .share_approval(first.revision)
            .expect("read")
            .expect("row");
        assert_eq!(previous.state, "superseded");
    }

    #[test]
    fn an_unreconciled_attempt_blocks_a_new_publication() {
        let (_dir, registry) = seeded(&["alpha"]);
        let PublicationReservation::Reserved { publication_id, .. } = registry
            .share_reserve_publication("op-1", 1, "hash-1", "github-pages", "ops-admin", STAMP)
            .expect("reserve")
        else {
            panic!("expected a fresh reservation");
        };
        registry
            .share_finish_publication(
                publication_id,
                share::PublicationStatus::Unknown,
                None,
                None,
            )
            .expect("finish unknown");
        let pending = registry
            .share_unreconciled_publication()
            .expect("read")
            .expect("row");
        assert_eq!(pending.publication_id, publication_id);
        let reconciled = registry
            .share_reconcile_publication(
                publication_id,
                share::PublicationStatus::Published,
                "ops-admin",
            )
            .expect("reconcile");
        assert_eq!(reconciled.status, "published");
        assert!(registry
            .share_unreconciled_publication()
            .expect("read")
            .is_none());
    }

    #[test]
    fn only_an_unknown_attempt_can_be_reconciled() {
        let (_dir, registry) = seeded(&["alpha"]);
        let PublicationReservation::Reserved { publication_id, .. } = registry
            .share_reserve_publication("op-1", 1, "hash-1", "github-pages", "ops-admin", STAMP)
            .expect("reserve")
        else {
            panic!("expected a fresh reservation");
        };
        let err = registry
            .share_reconcile_publication(publication_id, share::PublicationStatus::Published, "ops")
            .expect_err("pending is not unknown");
        assert_eq!(err.code(), "portfolio-share-conflict");
        let err = registry
            .share_reconcile_publication(9999, share::PublicationStatus::Published, "ops")
            .expect_err("unknown id");
        assert_eq!(err.code(), "portfolio-share-invalid");
    }

    #[test]
    fn the_audit_trail_answers_what_was_public_and_by_whom() {
        let (_dir, registry) = seeded(&["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        let draft = registry.share_manifest_draft(1).expect("draft");
        let approval = registry
            .share_approve(&draft.manifest_sha256(), "ops-admin")
            .expect("approve");
        registry
            .share_reserve_publication(
                "op-1",
                approval.revision,
                &approval.manifest_sha256,
                "local",
                "ops-admin",
                STAMP,
            )
            .expect("reserve");
        let approvals = registry.share_approvals(10).expect("approvals");
        let attempts = registry.share_publications(10).expect("attempts");
        assert_eq!(approvals[0].actor, "ops-admin");
        assert_eq!(approvals[0].manifest_sha256.len(), 64);
        assert_eq!(attempts[0].actor, "ops-admin");
        assert_eq!(attempts[0].manifest_revision, approval.revision);
        assert_eq!(attempts[0].manifest_sha256, approval.manifest_sha256);
        assert!(!attempts[0].attempted_at.is_empty());
    }
}

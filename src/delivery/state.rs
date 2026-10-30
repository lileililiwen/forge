//! Closed delivery state machine and idempotency-key helpers.

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;

use super::DeliveryEnvironment;

/// Closed lifecycle of one `(project, revision)` delivery run.
///
/// The state advances through `preflight → stage → production` and
/// is augmented by the optional Hermora child operation. Every value
/// in the enum is a stable machine label rendered as
/// `kebab-case` over JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryPhase {
    Draft,
    Preflighted,
    AwaitingStageConfirmation,
    Staging,
    StageHealthy,
    StageFailed,
    AwaitingProductionApproval,
    Production,
    Healthy,
    Degraded,
    RolledBack,
    HermoraPending,
    HermoraConnected,
    HermoraFailed,
}

impl DeliveryPhase {
    pub const ALL: [DeliveryPhase; 14] = [
        DeliveryPhase::Draft,
        DeliveryPhase::Preflighted,
        DeliveryPhase::AwaitingStageConfirmation,
        DeliveryPhase::Staging,
        DeliveryPhase::StageHealthy,
        DeliveryPhase::StageFailed,
        DeliveryPhase::AwaitingProductionApproval,
        DeliveryPhase::Production,
        DeliveryPhase::Healthy,
        DeliveryPhase::Degraded,
        DeliveryPhase::RolledBack,
        DeliveryPhase::HermoraPending,
        DeliveryPhase::HermoraConnected,
        DeliveryPhase::HermoraFailed,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            DeliveryPhase::Draft => "draft",
            DeliveryPhase::Preflighted => "preflighted",
            DeliveryPhase::AwaitingStageConfirmation => "awaiting-stage-confirmation",
            DeliveryPhase::Staging => "staging",
            DeliveryPhase::StageHealthy => "stage-healthy",
            DeliveryPhase::StageFailed => "stage-failed",
            DeliveryPhase::AwaitingProductionApproval => "awaiting-production-approval",
            DeliveryPhase::Production => "production",
            DeliveryPhase::Healthy => "healthy",
            DeliveryPhase::Degraded => "degraded",
            DeliveryPhase::RolledBack => "rolled-back",
            DeliveryPhase::HermoraPending => "hermora-pending",
            DeliveryPhase::HermoraConnected => "hermora-connected",
            DeliveryPhase::HermoraFailed => "hermora-failed",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        for phase in Self::ALL {
            if phase.label() == raw {
                return Ok(phase);
            }
        }
        Err(format!(
            "unknown delivery phase `{raw}`; expected one of {}",
            Self::ALL
                .iter()
                .map(|p| p.label())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

impl std::fmt::Display for DeliveryPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Closed verb label written into the `operations.kind` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryVerb {
    Preflight,
    Stage,
    Promote,
    Rollback,
    Hermora,
}

/// Which release a rollback restores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackTarget {
    /// A revision the operator named.
    To(String),
    /// The newest healthy release other than the registered one.
    Previous,
}

impl RollbackTarget {
    /// Read the CLI's two selectors. Exactly one is required.
    pub fn from_args(to: Option<String>, previous: bool) -> Result<Self, ForgeError> {
        match (to, previous) {
            (Some(revision), false) => Ok(RollbackTarget::To(revision)),
            (None, true) => Ok(RollbackTarget::Previous),
            (Some(_), true) => Err(ForgeError::DeliveryInvalid {
                reason: "delivery rollback: choose exactly one of `--to <revision>` and \
                         `--previous`; both were supplied"
                    .to_string(),
            }),
            (None, false) => Err(ForgeError::DeliveryInvalid {
                reason: "delivery rollback: choose exactly one of `--to <revision>` and \
                         `--previous`; neither was supplied"
                    .to_string(),
            }),
        }
    }
}

impl DeliveryVerb {
    pub fn label(&self) -> &'static str {
        match self {
            DeliveryVerb::Preflight => "delivery.preflight",
            DeliveryVerb::Stage => "delivery.stage",
            DeliveryVerb::Promote => "delivery.promote",
            DeliveryVerb::Rollback => "delivery.rollback",
            DeliveryVerb::Hermora => "delivery.hermora",
        }
    }
}

/// Compute the deterministic idempotency key for one delivery verb
/// on a `(project, revision)` pair. The 12-character revision prefix
/// matches the canonical `<sha12>` identifier the publish
/// providers already emit in their `container_identity`.
pub fn idempotency_key(
    verb: DeliveryVerb,
    project_id: &str,
    revision: &str,
) -> Result<String, ForgeError> {
    if project_id.is_empty() {
        return Err(ForgeError::DeliveryInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!("revision `{revision}` is not a 40-character hex SHA"),
        });
    }
    let prefix = &revision[..12];
    if verb == DeliveryVerb::Rollback {
        return Err(ForgeError::DeliveryInvalid {
            reason: "delivery rollback: a rollback is keyed by its target revision; \
                     use the rollback idempotency key"
                .to_string(),
        });
    }
    let env_suffix = match verb {
        DeliveryVerb::Preflight => "",
        DeliveryVerb::Stage => ":stage",
        DeliveryVerb::Promote => ":production",
        DeliveryVerb::Rollback | DeliveryVerb::Hermora => ":production",
    };
    Ok(format!(
        "{verb}.{project_id}:{prefix}{env_suffix}",
        verb = match verb {
            DeliveryVerb::Preflight => "delivery.preflight",
            DeliveryVerb::Stage => "delivery.stage",
            DeliveryVerb::Promote => "delivery.promote",
            DeliveryVerb::Rollback => "delivery.rollback",
            DeliveryVerb::Hermora => "delivery.hermora",
        },
    ))
}

/// Compute the deterministic idempotency key for one rollback.
///
/// A rollback is keyed by the release it replaces **and** the release
/// it restores. The per-verb key would collide for two rollbacks from
/// the same broken revision to two different good ones, which are two
/// operations, not a conflict.
pub fn rollback_idempotency_key(
    project_id: &str,
    current_revision: &str,
    target_revision: &str,
) -> Result<String, ForgeError> {
    if project_id.is_empty() {
        return Err(ForgeError::DeliveryInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    for revision in [current_revision, target_revision] {
        if revision.len() != 40 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ForgeError::DeliveryInvalid {
                reason: format!("revision `{revision}` is not a 40-character hex SHA"),
            });
        }
    }
    Ok(format!(
        "delivery.rollback.{project_id}:{}:{target}",
        &current_revision[..12],
        target = &target_revision[..12],
    ))
}

/// Compute the deterministic request hash that pairs with the
/// idempotency key. `confirm_token` is the operator-supplied
/// `--confirm-revision <sha>` (promote) or `--confirm-operation-id
/// <op_id>` (stage) — the same `(verb, project, revision, confirm)`
/// tuple always reserves the same `op_id` and a changed confirmation
/// surfaces as `idempotency-key-conflict` rather than re-using the
/// prior reservation.
pub fn request_hash(
    revision: &str,
    environment: Option<DeliveryEnvironment>,
    confirm_token: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let env = environment.map(|e| e.label()).unwrap_or("");
    let mut hasher = Sha256::new();
    hasher.update(revision.as_bytes());
    hasher.update([0x1f]);
    hasher.update(env.as_bytes());
    hasher.update([0x1f]);
    hasher.update(confirm_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Bounded detail block serialised into `operations.detail`. The
/// evidence and recovery arrays are scrubbed through
/// `policy::redact_credentials` before they land, and the full
/// payload is bounded so a verbose provider cannot widen the
/// journal row.
pub const DETAIL_BYTES_MAX: usize = 4096;

/// Re-serialise one row's `detail` JSON through the redaction
/// pipeline. Empty inputs return an empty string; over-bound
/// inputs are truncated with a trailing `…` so the journal row
/// stays single-page.
pub fn scrub_detail(value: &str) -> String {
    let scrubbed = redact_credentials(value);
    if scrubbed.len() <= DETAIL_BYTES_MAX {
        return scrubbed;
    }
    let mut truncated = scrubbed;
    truncated.truncate(DETAIL_BYTES_MAX.saturating_sub(1));
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_REV: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn the_phase_enum_is_closed_and_round_trips() {
        for phase in DeliveryPhase::ALL {
            let parsed = DeliveryPhase::parse(phase.label()).expect("parses");
            assert_eq!(parsed, phase);
            assert_eq!(phase.to_string(), phase.label());
        }
    }

    #[test]
    fn an_unknown_phase_is_refused() {
        let err = DeliveryPhase::parse("unreachable").unwrap_err();
        assert!(err.contains("unknown delivery phase"));
    }

    #[test]
    fn idempotency_keys_are_deterministic_and_scoped() {
        let preflight =
            idempotency_key(DeliveryVerb::Preflight, "alpha", SAMPLE_REV).expect("preflight");
        let stage = idempotency_key(DeliveryVerb::Stage, "alpha", SAMPLE_REV).expect("stage");
        let promote = idempotency_key(DeliveryVerb::Promote, "alpha", SAMPLE_REV).expect("promote");
        assert_eq!(preflight, "delivery.preflight.alpha:0123456789ab");
        assert_eq!(stage, "delivery.stage.alpha:0123456789ab:stage");
        assert_eq!(promote, "delivery.promote.alpha:0123456789ab:production");
    }

    #[test]
    fn a_rollback_key_names_both_releases_and_refuses_the_per_verb_key() {
        let target = "1111111111111111111111111111111111111111";
        let other = "2222222222222222222222222222222222222222";
        let key = rollback_idempotency_key("alpha", SAMPLE_REV, target).expect("key");
        assert_eq!(key, "delivery.rollback.alpha:0123456789ab:111111111111");
        assert_ne!(
            key,
            rollback_idempotency_key("alpha", SAMPLE_REV, other).expect("other")
        );
        let err = idempotency_key(DeliveryVerb::Rollback, "alpha", SAMPLE_REV).unwrap_err();
        assert!(err.to_string().contains("target revision"));
    }

    #[test]
    fn the_rollback_selectors_are_exactly_one() {
        let revision = SAMPLE_REV.to_string();
        assert_eq!(
            RollbackTarget::from_args(Some(revision.clone()), false).expect("to"),
            RollbackTarget::To(revision)
        );
        assert_eq!(
            RollbackTarget::from_args(None, true).expect("previous"),
            RollbackTarget::Previous
        );
        for (to, previous) in [(Some(SAMPLE_REV.to_string()), true), (None, false)] {
            let err = RollbackTarget::from_args(to, previous).unwrap_err();
            assert!(err.to_string().contains("exactly one"), "{err}");
        }
    }

    #[test]
    fn idempotency_keys_for_different_projects_never_collide() {
        let a = idempotency_key(DeliveryVerb::Stage, "alpha", SAMPLE_REV).expect("a");
        let b = idempotency_key(DeliveryVerb::Stage, "beta", SAMPLE_REV).expect("b");
        assert_ne!(a, b);
    }

    #[test]
    fn non_hex_revisions_are_refused() {
        let err = idempotency_key(DeliveryVerb::Stage, "alpha", "short").unwrap_err();
        assert_eq!(err.code(), "delivery-invalid");
    }

    #[test]
    fn empty_project_id_is_refused() {
        let err = idempotency_key(DeliveryVerb::Stage, "", SAMPLE_REV).unwrap_err();
        assert_eq!(err.code(), "delivery-invalid");
    }

    #[test]
    fn request_hash_changes_when_the_confirmation_changes() {
        let a = request_hash(SAMPLE_REV, Some(DeliveryEnvironment::Production), "tok-a");
        let b = request_hash(SAMPLE_REV, Some(DeliveryEnvironment::Production), "tok-b");
        let c = request_hash(SAMPLE_REV, Some(DeliveryEnvironment::Stage), "tok-a");
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn detail_scrub_redacts_credentials_and_bounds_length() {
        let mut long = "x".repeat(DETAIL_BYTES_MAX + 32);
        long.insert_str(0, "ghp_");
        let scrubbed = scrub_detail(&long);
        assert!(!scrubbed.contains("ghp_"));
        assert!(scrubbed.len() <= DETAIL_BYTES_MAX);
    }
}

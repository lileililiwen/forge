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
    HermoraPending,
    HermoraConnected,
    HermoraFailed,
}

impl DeliveryPhase {
    pub const ALL: [DeliveryPhase; 13] = [
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
    Hermora,
}

impl DeliveryVerb {
    pub fn label(&self) -> &'static str {
        match self {
            DeliveryVerb::Preflight => "delivery.preflight",
            DeliveryVerb::Stage => "delivery.stage",
            DeliveryVerb::Promote => "delivery.promote",
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
    let env_suffix = match verb {
        DeliveryVerb::Preflight => "",
        DeliveryVerb::Stage => ":stage",
        DeliveryVerb::Promote => ":production",
        DeliveryVerb::Hermora => ":production",
    };
    Ok(format!(
        "{verb}.{project_id}:{prefix}{env_suffix}",
        verb = match verb {
            DeliveryVerb::Preflight => "delivery.preflight",
            DeliveryVerb::Stage => "delivery.stage",
            DeliveryVerb::Promote => "delivery.promote",
            DeliveryVerb::Hermora => "delivery.hermora",
        },
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

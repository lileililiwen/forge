//! Thin wrapper around the publish-provider boundary.
//!
//! The workflow reuses `publish::providers::invoke_provider` so the
//! provider contract (`forge-publish-provider/0.1.0`) and the
//! journal shape stay unchanged. This module is the single door the
//! handlers enter; it derives the bounded queue id, normalises the
//! project folder, and wraps provider failures into
//! `DeliveryUnavailable`.

use std::path::Path;

use crate::core::ForgeError;
use crate::publish::providers::{
    self, ProviderEntry, ProviderOperation, PublishProviderRequest, PublishProviderResponse,
};

use super::state::DeliveryVerb;

/// Queue id the publish provider sees when staging or promoting a
/// project. The bounded shape matches the queue-id validator
/// (`1..=128` ASCII alphanumeric/`-`/`_`) and is unique per
/// `(project, environment, revision-12)`.
pub fn delivery_queue_id(verb: DeliveryVerb, project_id: &str, revision: &str) -> String {
    let env = match verb {
        DeliveryVerb::Preflight => "preflight",
        DeliveryVerb::Stage => "stage",
        DeliveryVerb::Promote => "production",
        DeliveryVerb::Rollback => "rollback",
        DeliveryVerb::Hermora => "production",
    };
    let prefix: String = revision.chars().take(12).collect();
    format!("delivery-{env}-{project_id}-{prefix}")
}

/// The provider operation a delivery verb sends.
pub fn operation_for(verb: DeliveryVerb) -> ProviderOperation {
    match verb {
        DeliveryVerb::Preflight => ProviderOperation::Preflight,
        DeliveryVerb::Rollback => ProviderOperation::Rollback,
        DeliveryVerb::Stage | DeliveryVerb::Promote | DeliveryVerb::Hermora => {
            ProviderOperation::Publish
        }
    }
}

/// Build one provider request for the delivery verb.
pub fn build_request(
    verb: DeliveryVerb,
    provider: &ProviderEntry,
    project_id: &str,
    revision: &str,
    folder: &Path,
    operation_id: &str,
) -> PublishProviderRequest {
    PublishProviderRequest {
        contract: providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: operation_for(verb),
        provider: provider.id.clone(),
        project_id: project_id.to_string(),
        revision: revision.to_string(),
        operation_id: operation_id.to_string(),
        folder: Some(folder.display().to_string()),
        dry_run: false,
        queue_id: Some(delivery_queue_id(verb, project_id, revision)),
        target_revision: None,
    }
}

/// Build the provider request for a rollback. `revision` is the
/// registered revision being replaced; `target_revision` is the release
/// being restored.
pub fn build_rollback_request(
    provider: &ProviderEntry,
    project_id: &str,
    revision: &str,
    target_revision: &str,
    folder: &Path,
    operation_id: &str,
) -> PublishProviderRequest {
    PublishProviderRequest {
        contract: providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Rollback,
        provider: provider.id.clone(),
        project_id: project_id.to_string(),
        revision: revision.to_string(),
        operation_id: operation_id.to_string(),
        folder: Some(folder.display().to_string()),
        dry_run: false,
        queue_id: Some(delivery_queue_id(
            DeliveryVerb::Rollback,
            project_id,
            revision,
        )),
        target_revision: Some(target_revision.to_string()),
    }
}

/// Invoke the publish provider for one delivery verb and surface
/// every failure as `DeliveryUnavailable` so the caller can
/// distinguish "provider refused" from "operator's confirmation was
/// stale".
pub fn invoke(
    entry: &ProviderEntry,
    request: &PublishProviderRequest,
    folder: &Path,
) -> Result<PublishProviderResponse, ForgeError> {
    providers::invoke_provider(entry, request, folder).map_err(|err| match err.code() {
        "publish-invalid" | "deploy-target-unavailable" => ForgeError::DeliveryUnavailable {
            reason: format!(
                "publish provider `{}` could not be reached: {}",
                entry.id, err
            ),
        },
        "publish-unauthorized" => ForgeError::DeliveryUnavailable {
            reason: format!(
                "publish provider `{}` refused authorization: {}",
                entry.id, err
            ),
        },
        _ => ForgeError::DeliveryUnavailable {
            reason: format!("publish provider `{}` failed: {}", entry.id, err),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_ids_are_bounded_and_unique_per_environment() {
        let rev = "0123456789abcdef0123456789abcdef01234567";
        let stage = delivery_queue_id(DeliveryVerb::Stage, "alpha", rev);
        let prod = delivery_queue_id(DeliveryVerb::Promote, "alpha", rev);
        assert!(stage.starts_with("delivery-stage-alpha-"));
        assert!(prod.starts_with("delivery-production-alpha-"));
        assert!(providers::validate_queue_id(&stage).is_ok());
        assert!(providers::validate_queue_id(&prod).is_ok());
        assert_ne!(stage, prod);
    }
}

//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::publish::providers::ProviderEntry;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub(in crate::api) struct LoginBody {
    pub(in crate::api) email: String,
    pub(in crate::api) password: String,
}
/// One staged browser delivery operation. The route segment selects the verb;
/// the request body carries only that verb's documented confirmation fields.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::api) enum DeliveryAction {
    Preflight,
    Stage,
    Promote,
    HermoraRetry,
}
impl DeliveryAction {
    pub(in crate::api) fn name(self) -> &'static str {
        match self {
            DeliveryAction::Preflight => "delivery-preflight",
            DeliveryAction::Stage => "delivery-stage",
            DeliveryAction::Promote => "delivery-promote",
            DeliveryAction::HermoraRetry => "delivery-hermora-retry",
        }
    }
}
/// Server-resolved delivery state shared by the status read and every staged
/// mutation: the project directory plus the registered 40-hex revision Core
/// binds all delivery idempotency keys to. Neither value is serialized.
pub(in crate::api) struct DeliveryResolution {
    pub(in crate::api) project_dir: PathBuf,
    pub(in crate::api) revision: String,
}
/// Which handler-backed authoring command a `/v1/admin/projects/{id}/…` write
/// runs. A closed enum — the browser picks one of these by the URL segment, and
/// no free-form command, path or argv ever reaches the handler.
#[derive(Clone, Copy)]
pub(in crate::api) enum Authoring {
    FeatureAdd,
    FeatureRemove,
    FeatureUpgrade,
    SpecGenerate,
    SpecApply,
}
/// Typed staged confirmation parsed from the browser body. Only the verb's
/// documented fields are read; every other key is ignored and can never
/// become a provider, path, argv, host or credential.
pub(in crate::api) enum DeliveryConfirmation {
    None,
    OperationId(i64),
    Revision(String),
    Hermora {
        deployment_url: String,
        secret_ref: String,
    },
}
/// The fully server-resolved publish target for one managed project. The
/// project directory is used only to locate the provider configuration and to
/// run the provider; neither it nor the provider executable path is ever
/// serialized. The revision is the committed `HEAD` the publish binds to.
pub(in crate::api) struct PublishTarget {
    pub(in crate::api) project_dir: PathBuf,
    pub(in crate::api) provider_id: String,
    pub(in crate::api) entry: ProviderEntry,
    pub(in crate::api) revision: String,
}

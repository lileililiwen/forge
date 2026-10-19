//! Project end-user auth setup through replaceable identity provider packs.
//!
//! This is the application-facing path: a project owner asks Forge to prepare the
//! end-user identity of one project. It is separate from Forge's own administrator
//! login (`forge identity`) and from a project's admin OIDC session. A provider pack
//! is an independent process that speaks the `forge-identity-provider/1.0` contract;
//! Core never runs provider code and never falls back to another provider.

pub mod contract;
pub mod packs;
pub mod setup;

pub use contract::{
    PlanChange, Population, ProviderOperation, ProviderRequest, ProviderResponse, ProviderStatus,
    RecoveryAction, CONTRACT_ID,
};
pub use packs::{PackCatalog, ProviderPack};
pub use setup::{AuthSetupOutcome, AuthSetupRequest, AuthSetupRunner};

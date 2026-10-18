//! Authenticated hosted CI evidence ingestion primitives.

mod oidc;
mod store;

pub use oidc::{
    validate_claims_for_envelope, verify_github_actions_token,
    verify_github_actions_token_from_jwks, GitHubActionsClaims, GITHUB_ACTIONS_AUDIENCE,
    GITHUB_ACTIONS_ISSUER,
};
pub use store::{
    bind_repository_id, github_slug, ingest, observations, repository_binding, HostedCiObservation,
    IngestError, IngestOutcome,
};

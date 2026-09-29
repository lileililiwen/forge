//! Optional, external GitHub metadata adapter
//! (`github-project-metadata-adapter`).
//!
//! The package is the **observation and reviewable mutation boundary**
//! between Forge and a Git host. It is deliberately optional: with no
//! adapter binary on the path, no `FORGE_GITHUB_BIN` override, no
//! `FORGE_GITHUB_TOKEN` and no project whose `git_remote` looks like a
//! GitHub URL, the catalog simply reports the GitHub source as
//! `unavailable` and every other Forge command continues unchanged.
//!
//! - [`adapter`] owns the versioned request/response contract
//!   (`forge-github-metadata/0.1.0`), the bounded argument-array
//!   invocation of the external adapter, the closed
//!   [`adapter::GithubState`] mapping and the explicit
//!   [`adapter::MutationMode`] (PR mode is the default; direct mode
//!   requires a separate confirmation).
//! - [`normalize`] turns an [`adapter::GithubObservation`] into one or
//!   more [`crate::catalog::CatalogRecord`] values with provenance,
//!   redacting every credential-shaped substring and keeping GitHub
//!   topics, GitHub release tags and Forge portfolio tags as three
//!   separate namespaces.
//!
//! The package makes no GitHub request itself. All round trips are
//! carried out by the external adapter through `Command::new` plus an
//! argument array, with `LC_ALL=C`, `FORGE_GITHUB_BIN` (or
//! `forge-github-metadata-adapter` on `PATH`) as the binary, the token
//! from `FORGE_GITHUB_TOKEN` (read once, never logged) and a bounded
//! timeout. A missing or non-executable adapter, a malformed payload or
//! any refusal surfaces as a typed
//! [`crate::core::ForgeError::GithubInvalid`] or
//! [`crate::core::ForgeError::GithubAdapterUnavailable`] — never as
//! silently zeroed success.

pub mod adapter;
pub mod normalize;

pub use adapter::{
    AdapterOutput, GithubAdapter, GithubObservation, GithubObservationRequest, GithubState,
    MutationMode, ProposedChange, ProposeOutcome, ProposeRequest, GITHUB_ADAPTER_TIMEOUT,
    GITHUB_BIN_ENV, GITHUB_CONTRACT_VERSION, GITHUB_DEFAULT_HOST, GITHUB_TOKEN_ENV,
    DEFAULT_GITHUB_BIN,
};
pub use normalize::{normalize_observation, normalize_topics, tag_separator};

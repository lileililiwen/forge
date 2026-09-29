//! Optional GitHub adapters and CLI workflow surfaces.
//!
//! The package owns two bounded, optional boundaries between Forge and
//! GitHub:
//!
//! - The **metadata observation and reviewable mutation boundary**
//!   (`github-project-metadata-adapter`) — token-based, read-only by
//!   default, with a separate PR/direct mutation path. Lives in
//!   [`adapter`] and [`normalize`].
//! - The **CLI workflow boundary** (`github-cli-project-workflows`) —
//!   reuses the user's installed `gh` authentication context for
//!   bounded local repository operations. Lives in [`cli`].
//!
//! Both surfaces are deliberately optional: with no adapter binary on
//! the path, no `FORGE_GITHUB_BIN` override, no `FORGE_GITHUB_TOKEN`
//! and no project whose `git_remote` looks like a GitHub URL, the
//! catalog simply reports the GitHub source as `unavailable` and
//! every other Forge command continues unchanged.
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
//! - [`cli`] owns the versioned `forge-github-cli-workflows/0.1.0`
//!   request/response contract, the bounded argument-array invocation
//!   of the installed `gh` CLI, the closed [`cli::GhOutcome`]
//!   mapping, and the four operations
//!   ([`cli::GhOperation::Auth`], [`cli::GhOperation::Clone`],
//!   [`cli::GhOperation::Create`], [`cli::GhOperation::PullRequest`]).
//!
//! The package makes no GitHub request itself. All round trips are
//! carried out by the external adapter or by the installed `gh` CLI
//! through `Command::new` plus an argument array, with `LC_ALL=C` and
//! a bounded per-run timeout. The metadata surface reads the token
//! from `FORGE_GITHUB_TOKEN` (read once, never logged); the CLI
//! surface reads authentication state from `gh auth status` and
//! never reads, persists or forwards the user's `gh` credential.

pub mod adapter;
pub mod cli;
pub mod normalize;

pub use adapter::{
    AdapterOutput, GithubAdapter, GithubObservation, GithubObservationRequest, GithubState,
    MutationMode, ProposeOutcome, ProposeRequest, ProposedChange, DEFAULT_GITHUB_BIN,
    GITHUB_ADAPTER_TIMEOUT, GITHUB_BIN_ENV, GITHUB_CONTRACT_VERSION, GITHUB_DEFAULT_HOST,
    GITHUB_TOKEN_ENV,
};
pub use cli::{
    parse_repository, run_auth, run_clone, run_create, run_pull_request, GhCli, GhOperation,
    GhOutcome, GhResult, GhVisibility, DEFAULT_GITHUB_CLI_BIN, GITHUB_CLI_BIN_ENV,
    GITHUB_CLI_CONTRACT_VERSION, GITHUB_CLI_DEFAULT_HOST, GITHUB_CLI_TIMEOUT, MAX_BODY_BYTES,
    MAX_DESTINATION_BYTES, MAX_TITLE_BYTES,
};
pub use normalize::{normalize_observation, normalize_topics, tag_separator};

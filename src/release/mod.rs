//! Gated resumable releases and publication (`release-publishing`).
//!
//! Core owns the typed release contract. v0.1.0 supports:
//!
//! - `prepare` captures semver, source revision, changelog,
//!   configured documentation and the doctor/test/DriftWatch
//!   evidence into a reviewable plan. No release side effect
//!   runs from `prepare`.
//! - `apply` walks the captured stages (commit, tag, push,
//!   mirror, package, container, docs, notes) once a plan is
//!   verified. Each stage owns a per-stage record so a retry
//!   resumes from the last delivered stage instead of
//!   redoing prior work.
//! - A semver tag, a package version and a container tag are
//!   treated as immutable identities: a second attempt that
//!   points them at a different commit surfaces as
//!   `release-identity-conflict` and refuses to overwrite.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §29, §34, §43 require
//! gated resumable releases with verifiable preparation. The
//! contract is independent of any real provider: a missing
//! package binary, an unavailable DriftWatch adapter, an
//! unparseable changelog or a stale plan all surface as
//! typed `error[...]` responses before any tag, push,
//! package, container or notes file is written.
//!
//! ## Persistence
//!
//! [`ReleaseState`] lives under
//! `.forge/release/<project-id>/<release-id>/state.json` so a
//! retry sees exactly which stages already delivered and which
//! need another attempt. The state is local evidence, not a
//! record of authority: a successful run overwrites the prior
//! entry, a failed run leaves it untouched. The Core
//! registry's `operations` table receives one `release` row
//! per prepare/apply with a `done`/`partial` summary that
//! lists the captured checks and per-stage statuses.
//!
//! ## Risk model
//!
//! Published artifacts and pushed tags are irreversible. The
//! contract refuses to apply a stage without an explicit
//! `--confirm`; the contract refuses to overwrite an existing
//! tag, package version or container tag at a different
//! commit. Credentials embedded in evidence are redacted
//! through [`crate::policy::redact_credentials`].
//!
//! Real provider integration is out of scope for v0.1.0:
//! package/container/notes stages reuse the same
//! adapter-by-environment-variable pattern the docs and
//! policy contracts use, so a fixture binary stands in for a
//! real provider round trip.

pub mod contract;
pub mod engine;
pub mod model;
pub mod prepare;
pub mod semver_display;

// Re-export all types
pub use contract::*;
pub use model::*;
pub use prepare::*;

//! Derivative documentation and source-hash freshness
//! (`documentation-translation`).
//!
//! Core owns the typed docs contract. v0.5 supports explicit locale
//! enablement, derivative paths, source-hash tracking and incremental
//! translation with a bundled provider adapter:
//!
//! - The manifest's `docs` section declares the canonical `source`
//!   document (default `README.md`), the `source_language` and one
//!   entry per locale under `translations`. A locale translates
//!   only when it carries `enabled: true`; an omitted or `false`
//!   flag keeps the locale disabled so translation is never
//!   enabled by default.
//! - `translate` splits the source into text segments and fenced
//!   code blocks. Code blocks are never sent to the provider and
//!   are reinserted verbatim; only changed text segments are
//!   requested, keyed by content hash so reordered or inserted
//!   paragraphs do not force a full retranslation.
//! - The provider is an external binary (default
//!   `forge-docs-translator`, overridable via
//!   `FORGE_DOCS_TRANSLATOR_BIN`) invoked with an argument array
//!   — never a shell — receiving the changed segments as JSON on
//!   stdin and returning translated segments as JSON on stdout.
//!   A missing binary, non-zero exit, timeout or unparseable
//!   output is a typed `translation-failed` outcome; the prior
//!   derivative and state are left intact.
//! - Output is merged to a staging file, the source hash is
//!   rechecked, preservation validators run (link destinations,
//!   explicit non-translatable terms, verbatim code blocks) and
//!   only then is the derivative atomically published and the
//!   state recorded with the new source hash plus a review
//!   status (`ok` or `needs-review`). Provider success alone
//!   never claims translation quality.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §28, §34, §43 require
//! derivative documentation with source-hash freshness and
//! incremental translation. The contract stays independent of any
//! real translation provider: a contract fixture proves
//! normalization and error mapping but cannot alone establish a
//! working external integration, matching the existing
//! DriftWatch adapter boundary.
//!
//! ## Persistence
//!
//! [`TranslationState`] is stored under
//! `.forge/docs/<locale>/state.json` so a re-run can reuse
//! unchanged segments and report `current` without contacting
//! the provider. The state is local evidence, not a record of
//! authority: a successful run overwrites the previous entry,
//! a failed run leaves it untouched. The Core registry's
//! `operations` table receives one `docs` row per run with a
//! `done`/`partial` summary that lists the per-locale statuses.
//!
//! ## Risk model
//!
//! AI translation can alter commands or links. Fenced code
//! blocks bypass the provider entirely, link destinations and
//! explicit non-translatable terms are rechecked after every
//! merge, and any violation marks the derivative `needs-review`
//! with the exact reason instead of failing silently. A
//! derivative path that resolves to the source file or outside
//! the project is refused before any write, and a disabled
//! locale is never translated — not even by `--all`.

pub mod assess;
pub mod constants;
pub mod docs_tests;
pub mod model;
pub mod source;
pub mod translate;

// Re-export all types
pub use assess::*;
pub use constants::*;
pub use model::*;
pub use source::*;
pub use translate::*;

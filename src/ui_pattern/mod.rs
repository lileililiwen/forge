//! Semantic UI patterns across web and Flutter (`semantic-ui-patterns`).
//!
//! Core owns the versioned UI pattern catalog, the contract validator, the
//! quality-aware resolver, and the deterministic installer. Transports
//! render Core outcomes without reinterpreting them.
//!
//! The UI pattern catalog is built on four rules that distinguish a
//! "semantic UI pattern" from a copied screenshot, a programming primitive
//! or a fine-grained component (requirement.md §13, §46, §47):
//!
//! 1. **Semantic intent.** The pattern names a real user-facing surface
//!    (`login`, `crud-table`, `form`, `empty-state`, `dashboard`,
//!    `confirm-dialog`, `settings`, `profile`, `billing`,
//!    `error-page`, `success-page`, `modal`, `file-upload`,
//!    `forgot-password`, `filter-bar`, `register`, `navigation`); a
//!    named primitive such as `if`, `loop`, `try-catch`,
//!    `string-concat`, `addition` is refused. A pattern with no
//!    semantic intent is a placeholder, not a contract.
//! 2. **Explicit state and design contract.** Every pattern declares
//!    the states it must surface (`loading`, `error`, `success`,
//!    `form_validation`, `empty`, `keyboard_focus`) and the design
//!    rules it follows (typography, spacing, responsive, accessibility,
//!    interaction). A pattern without a state contract is a copied
//!    markup fragment and is refused.
//! 3. **Versioned, testable, deterministic install.** Every pattern
//!    pins a version, references a test surface, declares a
//!    deterministic install strategy compatible with the profile
//!    catalog (React/Next.js for `react-web`/`nextjs-web`, Flutter for
//!    `flutter-app`), and ships an ordinary source artifact. A copied
//!    screenshot is not a verified pattern; the artifact and its
//!    test surface are the evidence.
//! 4. **Evidence-backed quality.** A quality classification
//!    (`Experimental` / `Verified` / `Certified` / `Deprecated`) is
//!    attached to verifiable evidence: usage count, test coverage,
//!    last verification timestamp, known issues and a security review
//!    flag. Promotion to `Certified` requires the evidence to be
//!    complete; otherwise the prior quality level is preserved and
//!    the request is refused. A request whose only compatible
//!    candidate is `Deprecated` surfaces a typed
//!    `ui-pattern-quality-conflict` rejection so the planner never
//!    silently selects a deprecated pattern.
//!
//! Per-platform implementations preserve their own implementation while
//! exposing the shared contract (R1 boundary scenario): the same
//! `form` id can be installed for `react-web`, `nextjs-web` and
//! `flutter-app` with three distinct stack-specific patterns sharing
//! one contract surface; a pattern with a `react-web` implementation
//! but no `flutter-app` mapping is reported as `unsupported` for
//! `flutter-app` requests rather than silently substituting copied web
//! markup.
//!
//! ## Why
//!
//! [requirement.md](../../requirement.md) §13, §46, §47 require versioned
//! UI patterns across web and Flutter. The contract stays independent of
//! any framework-specific library: a contract fixture proves the typed
//! state surface and the deterministic install, but a working external
//! integration (a real React/Next.js render or a Flutter build) is a
//! downstream integration step, matching the design decision that
//! contract fixtures supplement but do not replace a real integration
//! run.
//!
//! ## Persistence
//!
//! Installer receipts are stored under `.forge/ui-patterns/<id>/` so
//! the project can audit which patterns are installed, the captured
//! platform and version, and the deterministic install strategy. The
//! receipts are local evidence, not a record of authority: a
//! successful install overwrites the previous entry, a refused
//! install leaves the prior state untouched. The Core registry's
//! `operations` table receives one `ui_pattern` row per resolve or
//! install run with a `done`/`rejected`/`blocked` summary.
//!
//! ## Risk model
//!
//! A copied screenshot or HTML fragment is not a verified pattern; each
//! adapter needs state and interaction evidence. Public requests carry
//! explicit project/asset identity and validated configuration; Core
//! returns typed outcomes; transports render, never reinterpret.
//! Configuration and observations are separate, with provenance for
//! any asserted verification. The install refuses to overwrite a
//! customized file with a typed `ui-pattern-ownership-conflict` so a
//! user edit is never silently clobbered.

pub mod constants;
pub mod install;
pub mod model;
pub mod patterns;
pub mod resolve;
pub mod ui_tests;
pub mod validate;

// Re-export all types
pub use constants::*;
pub use install::*;
pub use model::*;
pub use patterns::*;
pub use resolve::*;
pub use validate::*;

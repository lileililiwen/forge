//! The runtime-kind vocabulary and the host model every kind declares.
//!
//! Forge Core owns one vocabulary of runtime kinds — `web`, `worker`,
//! `job` and `library` — and every kind declares how its artifact is
//! hosted. An own-process kind runs as a process of its own, with its
//! own container and its own deploy target. A shared-host kind is a
//! module loaded into a host another project owns, and several modules
//! can share one host.
//!
//! The `library` row is the shared-host row: a library is a module, and
//! the project that adopts it owns its lifecycle. A surface that starts
//! workloads must judge a shared-host entry by that host model, not by
//! the Compose file the entry declares.

pub mod kind;
pub mod kind_tests;

pub use kind::{
    parse_runtime_kind, HostModel, RuntimeKind, RuntimeKindRow, RUNTIME_KINDS, RUNTIME_KIND_NAMES,
    SHARED_HOST_REASON,
};

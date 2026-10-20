//! The runtime-kind vocabulary and the host model every kind declares.
//!
//! Forge Core owns one vocabulary of runtime kinds — `web`, `worker`,
//! `job` and `library` — and every kind declares how its artifact is
//! hosted. An own-process kind runs as a process of its own, with its
//! own container and its own deploy target. A shared-host kind is a
//! module loaded into a host another project owns, and several modules
//! can share one host.
//!
//! The spellings are the ones the inventory contract already carries,
//! so the vocabulary moves owner and nothing else moves.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

/// The reason a shared-host artifact carries.
///
/// A module is hosted by the project that adopts it, so the fleet
/// never starts one. One wording, owned by the vocabulary, so the
/// publish lane and the observe lane say the same thing.
pub const SHARED_HOST_REASON: &str =
    "a library is a module; it is hosted by the project that adopts it, so the fleet never starts it";

/// How a project's artifact is hosted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostModel {
    /// The artifact is a process of its own, with its own container
    /// and its own deploy target.
    OwnProcess,
    /// The artifact is a module loaded into a host another project
    /// owns. Several modules can share one host.
    SharedHost,
}

impl HostModel {
    /// The canonical spelling of this host model.
    pub const fn as_str(self) -> &'static str {
        match self {
            HostModel::OwnProcess => "own_process",
            HostModel::SharedHost => "shared_host",
        }
    }

    /// The reason this host model holds an artifact back, or `None`
    /// when the artifact is a process of its own and needs no host
    /// but its own.
    pub const fn hold_back_reason(self) -> Option<&'static str> {
        match self {
            HostModel::OwnProcess => None,
            HostModel::SharedHost => Some(SHARED_HOST_REASON),
        }
    }
}

impl fmt::Display for HostModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of runtime a project's artifact is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// A public HTTP service.
    Web,
    /// A long-running background worker.
    Worker,
    /// A short-lived job.
    Job,
    /// A module loaded into a host another project owns.
    Library,
}

impl RuntimeKind {
    /// The canonical spelling of this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            RuntimeKind::Web => "web",
            RuntimeKind::Worker => "worker",
            RuntimeKind::Job => "job",
            RuntimeKind::Library => "library",
        }
    }

    /// How this kind's artifact is hosted.
    ///
    /// The `library` kind is the shared-host kind; every other kind is
    /// an own-process kind.
    pub const fn host_model(self) -> HostModel {
        match self {
            RuntimeKind::Web | RuntimeKind::Worker | RuntimeKind::Job => HostModel::OwnProcess,
            RuntimeKind::Library => HostModel::SharedHost,
        }
    }

    /// What this kind is, in one sentence.
    pub const fn summary(self) -> &'static str {
        match self {
            RuntimeKind::Web => "A public HTTP service with a host process of its own.",
            RuntimeKind::Worker => {
                "A long-running background worker with a host process of its own."
            }
            RuntimeKind::Job => "A short-lived job with a host process of its own.",
            RuntimeKind::Library => {
                "A module loaded into a host another project owns. Several modules can share one host."
            }
        }
    }

    /// Read one declared runtime value.
    ///
    /// The value is trimmed and read case-insensitively. A value that
    /// is not one of the four is refused.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" => Some(RuntimeKind::Web),
            "worker" => Some(RuntimeKind::Worker),
            "job" => Some(RuntimeKind::Job),
            "library" => Some(RuntimeKind::Library),
            _ => None,
        }
    }
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of the runtime-kind table: the kind, how it is hosted and
/// what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKindRow {
    /// The kind this row describes.
    pub kind: RuntimeKind,
    /// How the kind's artifact is hosted.
    pub host_model: HostModel,
    /// What the kind is, in one sentence.
    pub summary: &'static str,
}

/// The runtime-kind table, one row per kind, in stable order.
pub const RUNTIME_KINDS: &[RuntimeKindRow] = &[
    RuntimeKindRow {
        kind: RuntimeKind::Web,
        host_model: RuntimeKind::Web.host_model(),
        summary: RuntimeKind::Web.summary(),
    },
    RuntimeKindRow {
        kind: RuntimeKind::Worker,
        host_model: RuntimeKind::Worker.host_model(),
        summary: RuntimeKind::Worker.summary(),
    },
    RuntimeKindRow {
        kind: RuntimeKind::Job,
        host_model: RuntimeKind::Job.host_model(),
        summary: RuntimeKind::Job.summary(),
    },
    RuntimeKindRow {
        kind: RuntimeKind::Library,
        host_model: RuntimeKind::Library.host_model(),
        summary: RuntimeKind::Library.summary(),
    },
];

/// The canonical runtime-kind spellings, in stable order.
pub const RUNTIME_KIND_NAMES: &[&str] = &[
    RuntimeKind::Web.as_str(),
    RuntimeKind::Worker.as_str(),
    RuntimeKind::Job.as_str(),
    RuntimeKind::Library.as_str(),
];

/// Read one declared runtime value into the shared kind.
///
/// The value is trimmed and read case-insensitively. A value that is
/// not one of the four is refused by name, and the refusal names the
/// accepted values.
pub fn parse_runtime_kind(value: &str) -> Result<RuntimeKind, ForgeError> {
    RuntimeKind::parse(value).ok_or_else(|| ForgeError::BuildInvalid {
        reason: format!(
            "unknown runtime kind `{value}`; accepted values are {}",
            accepted_values(RUNTIME_KIND_NAMES)
        ),
    })
}

/// The accepted values of one vocabulary, rendered for a refusal.
fn accepted_values(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

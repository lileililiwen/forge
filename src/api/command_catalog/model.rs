//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Web,
    CliOnly,
    ProviderRequired,
    ProjectCapabilityRequired,
    Disabled,
    NotYetWeb,
}
impl Availability {
    pub(super) fn id(self) -> &'static str {
        match self {
            Availability::Web => "web",
            Availability::CliOnly => "cli_only",
            Availability::ProviderRequired => "provider_required",
            Availability::ProjectCapabilityRequired => "project_capability_required",
            Availability::Disabled => "disabled",
            Availability::NotYetWeb => "not_yet_web",
        }
    }
}
/// One catalog row: `{id,parent_id,label,summary,category,scope,risk,
/// availability,route,cli_invocation,reason,capabilities,execution}`.
#[derive(Clone, Debug, Serialize)]
pub struct CommandRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub label: String,
    pub summary: &'static str,
    pub category: &'static str,
    pub scope: &'static str,
    pub risk: &'static str,
    pub availability: &'static str,
    pub route: Option<&'static str>,
    pub cli_invocation: String,
    pub reason: Option<&'static str>,
    pub capabilities: &'static [&'static str],
    /// Present (Some) only for `web` rows that are executable from the
    /// portal; every other row serializes this as `null`.
    pub execution: Option<CommandExecution>,
}
/// One typed parameter an executable (`web`) row accepts: a `name`, a closed
/// scalar `kind` (`string`, `string_array` or `boolean`) and whether it is
/// `required`. The browser renders one control per parameter and never a
/// free-text shell/path/argv field, so this is the entire request surface of
/// the row's route — no field here is ever interpreted as a command or path.
#[derive(Clone, Debug, Serialize)]
pub struct ExecParameter {
    pub name: String,
    pub kind: &'static str,
    pub required: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Workspace,
    Project,
    Forge,
    Profile,
    Provider,
}
impl Scope {
    pub(super) fn id(self) -> &'static str {
        match self {
            Scope::Workspace => "workspace",
            Scope::Project => "project",
            Scope::Forge => "forge",
            Scope::Profile => "profile",
            Scope::Provider => "provider",
        }
    }
}
/// The machine-readable execution contract carried only by `web` rows so the
/// frontend can render a runnable, confirm-gated control directly from the
/// catalog with no bespoke per-command wiring. `route` is the exact admin
/// route the router registers (always one of [`IMPLEMENTED_WEB_ROUTES`]);
/// `method` is its HTTP verb; `risk` mirrors the row's risk label;
/// `confirm_required` and `digest_bound` are always true for these session-
/// gated mutating lifecycle actions (preview writes nothing; the confirmed
/// run must echo the matching `plan_digest`). Non-executable rows carry
/// `None`.
#[derive(Clone, Debug, Serialize)]
pub struct CommandExecution {
    pub route: &'static str,
    pub method: &'static str,
    pub risk: &'static str,
    pub confirm_required: bool,
    pub digest_bound: bool,
    pub parameters: Vec<ExecParameter>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    Read,
    LocalWrite,
    RemoteWrite,
    SessionAdmin,
}
impl Risk {
    pub(super) fn id(self) -> &'static str {
        match self {
            Risk::Read => "read",
            Risk::LocalWrite => "local_write",
            Risk::RemoteWrite => "remote_write",
            Risk::SessionAdmin => "session_admin",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Registry,
    Creation,
    Quality,
    Release,
    Delivery,
    Portfolio,
    Identity,
    Transports,
    Reference,
}
impl Category {
    pub(super) fn id(self) -> &'static str {
        match self {
            Category::Registry => "registry",
            Category::Creation => "creation",
            Category::Quality => "quality",
            Category::Release => "release",
            Category::Delivery => "delivery",
            Category::Portfolio => "portfolio",
            Category::Identity => "identity",
            Category::Transports => "transports",
            Category::Reference => "reference",
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Category::Registry => "Registry",
            Category::Creation => "Creation",
            Category::Quality => "Quality",
            Category::Release => "Release",
            Category::Delivery => "Delivery",
            Category::Portfolio => "Portfolio",
            Category::Identity => "Identity",
            Category::Transports => "Transports",
            Category::Reference => "Reference",
        }
    }
}

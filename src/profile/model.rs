//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

/// Catalog support status for one profile id. A descriptor marked
/// [`ProfileSupportStatus::Supported`] is selectable through every
/// owning-domain entry point (resolution, preflight, generation);
/// [`ProfileSupportStatus::Planned`] is reserved on the roadmap and
/// inspectable, but operations that require generation evidence fail
/// before any mutation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileSupportStatus {
    /// Fully implemented, has a tested template and resolves like the MVP.
    Supported,
    /// Reserved on the roadmap; no tested template, not selectable.
    Planned,
}
/// Versioned descriptor for one profile. The `support_status` field
/// disambiguates a `Supported` profile (selectable, has a tested template)
/// from a `Planned` candidate (discoverable, not yet implemented).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDescriptor {
    pub id: String,
    pub version: String,
    #[serde(default = "crate::profile::catalog::default_support_status")]
    pub support_status: ProfileSupportStatus,
    pub adapter: String,
    pub language: String,
    pub toolchain: String,
    #[serde(default)]
    pub toolchain_version: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub layout: Vec<String>,
    #[serde(default)]
    pub conventions: Vec<String>,
    pub build_command: String,
    pub test_command: String,
    #[serde(default)]
    pub deployment_type: Option<String>,
    #[serde(default)]
    pub deployment_target: Option<String>,
    #[serde(default)]
    pub quality_policies: Vec<String>,
    #[serde(default)]
    pub requires_database: bool,
    #[serde(default)]
    pub description: Option<String>,
    /// Workspace Governance metadata mapping (`.project.json` emission).
    /// `None` is the no-mapping sentinel: generation omits the file and
    /// prints a note, and never guesses a governance profile.
    #[serde(default)]
    pub workspace: Option<WorkspaceMapping>,
    /// Shared-layer kit reference (`scaffold-prewires-shared-layer`).
    /// `None` is the pre-kit sentinel: the profile predates kits and renders
    /// exactly the prior output. A profile with no registered kit for its
    /// ecosystem still declares a **zero** reference, so the absence is
    /// recorded with its reason rather than left implicit.
    #[serde(default)]
    pub kit: Option<crate::kit::registry::KitReference>,
}
/// One capability a profile descriptor implements for generated
/// projects. Only the name and owner are descriptor data; the emitted
/// evidence state is always `declared`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceCapability {
    /// Capability name from the governance vocabulary (e.g. `identity`).
    pub name: String,
    /// Capability owner (`platform`, `product`, `shared`, `external`,
    /// or a project id).
    pub owner: String,
}
/// Successful resolution: exact pinned version plus owning adapter identity.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedProfile {
    pub id: String,
    pub version: String,
    pub adapter: String,
    pub language: String,
    pub toolchain: String,
}
/// Explicit per-profile Workspace Governance mapping used by generation to
/// emit an honest initial `.project.json` (sibling `schema_version: 1`).
/// The governance profile string must exist in the sibling's vocabulary;
/// the mapping lives in the descriptor so each profile decides its own
/// adoption shape (or opts out with no mapping).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMapping {
    /// Governance profile vocabulary value (e.g. `rust-product`).
    pub governance_profile: String,
    /// Declaration kind (e.g. `product`; `control-plane` when declared).
    pub kind: String,
    /// Gate runtime name, emitted only when the profile declares one.
    /// No supported profile declares a gate runtime today.
    #[serde(default)]
    pub gate_runtime: Option<String>,
    /// Capability names the profile descriptor really implements. A
    /// generated project starts with intent but no observed evidence, so
    /// entries emit as `declared` (the state that asserts intent without
    /// claiming evidence). No supported descriptor declares any today,
    /// so generated declarations carry no `capabilities` key at all.
    /// Forge never copies its own capability set into generated
    /// projects and never invents a speculative entry.
    #[serde(default)]
    pub capabilities: Vec<WorkspaceCapability>,
}
/// Toolchain preflight outcome. A present toolchain is a prerequisite, not
/// proof: it never claims the profile was tested.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PreflightReport {
    pub id: String,
    pub version: String,
    pub toolchain: String,
    pub available: bool,
}

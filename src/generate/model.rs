//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use serde::Serialize;
use std::path::PathBuf;

/// Outcome of [`generate`]: the registered record plus what was rendered.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedProject {
    pub record: crate::registry::ProjectRecord,
    pub files: Vec<String>,
    pub native_verified: bool,
    pub native_note: String,
    /// Honest omission notes (e.g. a profile without a governance
    /// mapping). Empty in normal runs, so rendered output stays
    /// byte-identical to pre-change releases.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Shared-layer floor outcome that an operator must see: a declared
    /// zero or a recorded exception.
    ///
    /// Deliberately a *separate* field from `notes`. `notes` is documented as
    /// omission notes and is asserted empty for a fully mapped profile, so a
    /// kit warning there would change an established transport contract. This
    /// is additive and skipped when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kit_warning: Option<String>,
}
/// One normalized creation request: the single shape both explicit flags
/// and interactive answers produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationRequest {
    pub profile: String,
    pub id: String,
    pub name: String,
    pub features: Vec<String>,
    pub destination: PathBuf,
    /// Emit the Workspace Governance `.project.json` declaration
    /// (`--no-workspace-metadata` opts out; transports default to `true`).
    pub workspace_metadata: bool,
    /// Explicitly selected `<pack>@<version>` whose `.standard/` snapshot is
    /// rendered alongside the project (`--standard-pack`). `None` renders
    /// exactly the prior output: no pack is ever selected implicitly.
    pub standard_pack: Option<String>,
    /// Operator-written reason for bypassing an unmet shared-layer
    /// consumption floor (`--kit-exception <reason>`). The reason is
    /// mandatory and is recorded visibly and dated; Forge never infers,
    /// defaults or generates one.
    pub kit_exception: Option<String>,
}
/// Native verification outcome. Rendering alone never yields `verified`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReport {
    pub profile: String,
    pub build_command: String,
    pub test_command: String,
    pub verified: bool,
}
/// A profile's shared-layer kit, resolved and gated.
///
/// Resolution, the ecosystem check, the floor decision and the digest
/// verification all happen here — before a single file is staged — so a
/// refusal never leaves a half-wired directory or a registered project row.
pub(super) struct KitContext {
    pub(super) descriptor: crate::kit::registry::KitDescriptor,
    pub(super) decision: crate::kit::floor::FloorDecision,
}
impl KitContext {
    /// Resolve the kit a profile declares.
    ///
    /// `None` means the profile predates kits: generation renders exactly the
    /// prior output and never guesses a kit.
    pub(super) fn resolve(
        request: &CreationRequest,
        profile: &crate::profile::ProfileDescriptor,
        generated_at: &str,
    ) -> Result<Option<Self>, ForgeError> {
        let Some(reference) = profile.kit.as_ref() else {
            return Ok(None);
        };
        let descriptor =
            crate::kit::registry::kit_for_profile(&request.profile, reference, &profile.toolchain)?;
        let decision = crate::kit::floor::check_floor(
            &request.profile,
            &descriptor,
            request.kit_exception.as_deref(),
            generated_at,
        )?;
        Ok(Some(Self {
            descriptor,
            decision,
        }))
    }
    pub(super) fn reference(&self) -> &crate::kit::registry::KitReference {
        &self.descriptor.reference
    }
    /// The `WARN` line to surface, if this state warrants one.
    pub(super) fn warning(&self, profile: &str) -> Option<String> {
        self.decision.warning(profile)
    }
}

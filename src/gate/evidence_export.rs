//! Optional producer resolution for the separate release-evidence export.

use std::path::PathBuf;

/// `FORGE_GATE_BIN` remains an explicit override for this export verb only;
/// it never selects the implementation of `forge gate` itself.
pub(super) fn resolve_evidence_export_binary() -> Option<PathBuf> {
    for key in [super::GATE_BIN_ENV, "FORGE_DRIFTWATCH_BIN"] {
        if let Some(binary) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            return Some(PathBuf::from(binary));
        }
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    crate::policy::first_binary_on_path(&path, crate::policy::DRIFTWATCH_BINARY_CANDIDATES)
}

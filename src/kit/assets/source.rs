//! Kit assets: source.

use crate::core::ForgeError;
use std::fs;
use std::path::PathBuf;

use super::model::KitManifest;

/// Directory holding the checked-in vendored assets.
pub fn kits_dir() -> PathBuf {
    if let Some(dir) =
        std::env::var_os("FORGE_KITS_DIR").filter(|v| !v.to_string_lossy().trim().is_empty())
    {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kits")
}

pub(super) fn load_manifest() -> Result<KitManifest, ForgeError> {
    let path = kits_dir().join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kits/manifest.json missing or unreadable at {}: {err}; \
             nothing was staged and no project was registered",
            path.display()
        ),
    })?;
    serde_json::from_str(&text).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kits/manifest.json is not a valid kit manifest: {err}; \
             nothing was staged and no project was registered"
        ),
    })
}

pub(crate) fn digest_of(bytes: &[u8]) -> String {
    crate::standard::sha256_hex(bytes)
}

//! Kit assets: receipt.

use crate::core::ForgeError;
use std::fs;
use std::path::Path;

use super::model::{KitReceipt, KitReceiptFile};
use super::paths::{PLATFORM_RECEIPT_PATH, PLATFORM_RECEIPT_SCHEMA};

use super::source::digest_of;

/// Build the receipt for the files a kit owns. Pure: identical inputs give
/// identical bytes, including the receipt itself.
pub fn render_receipt(
    id: &str,
    profile: &str,
    descriptor: &crate::kit::registry::KitDescriptor,
    owned: &[(String, String)],
) -> KitReceipt {
    let mut files: Vec<KitReceiptFile> = owned
        .iter()
        .map(|(path, content)| KitReceiptFile {
            path: path.clone(),
            digest: digest_of(content.as_bytes()),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    KitReceipt {
        schema: PLATFORM_RECEIPT_SCHEMA,
        generator: format!("forge@{}", env!("CARGO_PKG_VERSION")),
        project: id.to_string(),
        profile: profile.to_string(),
        kit: descriptor.reference.id.clone(),
        version: descriptor.reference.version.clone(),
        files,
    }
}

/// Serialize a kit receipt as its canonical document.
pub fn receipt_text(receipt: &KitReceipt) -> Result<String, ForgeError> {
    let text =
        serde_json::to_string_pretty(receipt).map_err(|err| ForgeError::KitDigestMismatch {
            reason: format!("kit receipt is not serializable: {err}"),
        })?;
    Ok(format!("{text}\n"))
}

pub(crate) fn read_receipt(dir: &Path) -> Result<Option<KitReceipt>, ForgeError> {
    let path = dir.join(PLATFORM_RECEIPT_PATH);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(ForgeError::KitDigestMismatch {
                reason: format!("cannot read kit receipt '{}': {err}", path.display()),
            })
        }
    };
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::KitDigestMismatch {
        reason: format!(
            "kit receipt '{}' is not a valid receipt: {err}",
            path.display()
        ),
    })
}

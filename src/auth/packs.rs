//! Provider pack discovery.
//!
//! A pack is an independent executable that speaks the provider contract. Core
//! discovers packs from descriptor files; it never runs a pack it did not find in
//! the descriptor directory, and it never falls back to another provider.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One discovered provider pack.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderPack {
    /// The pack identifier.
    pub id: String,
    /// A human readable description.
    pub description: String,
    /// The modes the pack supports.
    pub modes: Vec<String>,
    /// The executable that answers the provider contract.
    pub executable: PathBuf,
    /// Whether the owner enabled the pack.
    pub enabled: bool,
    /// Whether the executable is present on disk.
    pub available: bool,
}

/// The pack descriptor file.
#[derive(Debug, Clone, Deserialize)]
struct PackDescriptor {
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    modes: Vec<String>,
    executable: String,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// The packs Core found.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PackCatalog {
    /// The directory the descriptors were read from.
    pub directory: PathBuf,
    /// The discovered packs, sorted by id.
    pub packs: Vec<ProviderPack>,
}

impl PackCatalog {
    /// Finds one pack by id.
    pub fn find(&self, id: &str) -> Option<&ProviderPack> {
        self.packs.iter().find(|pack| pack.id == id)
    }

    /// The packs an owner may select: enabled and present.
    pub fn selectable(&self) -> Vec<&ProviderPack> {
        self.packs
            .iter()
            .filter(|pack| pack.enabled && pack.available)
            .collect()
    }
}

/// Reads the pack descriptors in a directory.
///
/// A missing directory yields an empty catalog, never an error: a checkout without
/// provider packs simply has no provider to select.
pub fn discover(directory: &Path) -> PackCatalog {
    let mut packs = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            return PackCatalog {
                directory: directory.to_path_buf(),
                packs,
            }
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let descriptor: PackDescriptor = match serde_json::from_slice(&bytes) {
            Ok(descriptor) => descriptor,
            Err(_) => continue,
        };
        let executable = directory.join(&descriptor.executable);
        let available = executable.is_file();
        packs.push(ProviderPack {
            id: descriptor.id,
            description: descriptor.description,
            modes: descriptor.modes,
            executable,
            enabled: descriptor.enabled,
            available,
        });
    }

    packs.sort_by(|left, right| left.id.cmp(&right.id));
    PackCatalog {
        directory: directory.to_path_buf(),
        packs,
    }
}

/// The default directory that holds the pack descriptors.
pub fn default_pack_directory(forge_root: &Path) -> PathBuf {
    forge_root.join("modules/identity-providers/packs")
}

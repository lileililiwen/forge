//! Shared-layer kit contract (`scaffold-prewires-shared-layer`).
//!
//! Covers the change's requirements end to end through the built binary plus
//! the compiled-in library: the declared kit renders into the native
//! manifest, the floor refuses or records an explicit exception, a declared
//! zero stays a distinct state, the feed is named rather than machine
//! specific, the token source is vendored and receipted, rendering stays
//! deterministic, digest drift is caught, and the generated project operates
//! through its own toolchain with Forge absent.
//!
//! The target was a single 2,376-line file; it is now a directory of focused
//! submodules, each owning one concern. The shared helpers below stay
//! reachable to every submodule through `super::`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::kit;

mod classification;
mod feed;
mod manifest;
mod prewiring;
mod registration;
mod tampering;
mod tokens_and_assets;
mod upgrade;

pub(crate) fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

pub(crate) fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    // The rejected mechanism. Removed from every test process so no test can
    // accidentally pass because the variable is set.
    cmd.env_remove("NUGET_PLATFORM_FEED");
    cmd.env_remove("NUGET_PACKAGES");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

pub(crate) fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

pub(crate) fn run_json(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

pub(crate) fn run_with_stdin(
    db: &Path,
    args: &[&str],
    stdin_bytes: &[&str],
) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn forge");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for line in stdin_bytes {
            writeln!(stdin, "{line}").expect("write stdin");
        }
    }
    child.wait_with_output().expect("wait forge")
}

pub(crate) fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

pub(crate) fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_into(root, &mut out);
    out.sort();
    out
}

pub(crate) fn collect_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        out.push(path.clone());
        if path.is_dir() {
            collect_into(&path, out);
        }
    }
}

pub(crate) fn file_bytes(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for path in collect_files(root) {
        if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, fs::read(&path).unwrap()));
        }
    }
    out.sort();
    out
}

/// Parse the generated `forge.yaml` into a plain map so assertions read as
/// statements about the manifest rather than about string matching.
pub(crate) fn parse_yaml(path: &Path) -> serde_yaml::Value {
    let text = fs::read_to_string(path).expect("forge.yaml");
    serde_yaml::from_str(&text).expect("generated forge.yaml parses")
}

pub(crate) fn scaffold(db: &Path, dir: &Path, profile: &str) -> std::process::Output {
    run(
        db,
        &["new", &dir.display().to_string(), "--profile", profile],
    )
}

/// A `#` followed by three or six hex digits is a literal colour.
///
/// This has to parse the text. A substring search for the *string*
/// `#[0-9a-fA-F]{6}` matches nothing ever, because it is a literal and not a
/// pattern — the check it replaced could not have failed.
pub(crate) fn has_literal_hex_colour(text: &str) -> bool {
    for (index, byte) in text.bytes().enumerate() {
        if byte != b'#' {
            continue;
        }
        let rest = &text[index + 1..];
        for width in [3usize, 6] {
            if let Some(candidate) = rest.get(..width) {
                if candidate.chars().all(|c| c.is_ascii_hexdigit()) && candidate.len() == width {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create dir");
    for entry in fs::read_dir(from).expect("read dir").flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// Every file under a directory as `relative path -> bytes`.
pub(crate) fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if let Ok(bytes) = fs::read(&path) {
                    out.insert(rel, bytes);
                }
            }
        }
    }
    out
}

pub(crate) fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// Restores the committed feed when a pack test ends, however it ends.
///
/// `forge kit pack` rewrites `kits/` by design, so a test that repacks has to
/// put the reviewed bytes back. A plain trailing statement is not enough: an
/// assertion panic would leave the repository carrying a feed nobody reviewed.
pub(crate) struct FeedRestore {
    pub(crate) backup: PathBuf,
}

impl Drop for FeedRestore {
    fn drop(&mut self) {
        let kits = kit::assets::kits_dir();
        let _ = fs::remove_dir_all(&kits);
        copy_dir(&self.backup, &kits);
    }
}

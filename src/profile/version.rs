//! The profile toolchain version rule.
//!
//! One module owns the whole rule: how a runtime's version is read from its
//! banner, how a declared version is compared with an observed one, which
//! binary carries an ecosystem's runtime and how a runtime's version is
//! probed on a host. The profile preflight and the readiness matrix judge
//! the same declaration through this module, so a host is never judged by
//! two different rules.
//!
//! A declared version is a requirement on the **runtime** that executes a
//! profile's commands, not on the toolchain's name: the Node ecosystem's
//! runtime is `node`, so `react-web` (toolchain `npm`) and `nextjs-web`
//! (toolchain `node`) declare and check the same node version.
//!
//! A version that cannot be read is unknown. Unknown is never read as a
//! mismatch: the rule judges what the host reports and says nothing about
//! what it does not.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::process::Command;

use crate::kit::registry::Ecosystem;

/// The numeric version in a toolchain's `--version` banner.
///
/// The version is the first dotted-numeric token of the banner: a maximal
/// run of digits joined by single dots (`1.99.1`). A product name, a
/// leading `v`, a channel label, a build hash and a URL all carry digits
/// too, and none of them is the version.
///
/// ```text
/// v22.23.3                                  -> 22.23.3
/// Python 3.14.4                             -> 3.14.4
/// Flutter 3.47.1 • channel stable • ...      -> 3.47.1
/// cargo 1.99.1 (5f94df478 2026-08-27)        -> 1.99.1
/// ```
pub fn extract_version(banner: &str) -> Option<String> {
    let bytes = banner.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        // A maximal run of digits joined by single dots: a dot that is not
        // followed by a digit ends the token.
        while index < bytes.len() {
            if bytes[index].is_ascii_digit() {
                index += 1;
                continue;
            }
            if bytes[index] == b'.' {
                let mut lookahead = index + 1;
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_digit() {
                    lookahead += 1;
                }
                if lookahead == index + 1 {
                    break;
                }
                index = lookahead;
                continue;
            }
            break;
        }
        return Some(banner[start..index].to_string());
    }
    None
}

/// Whether the observed version satisfies the declared requirement.
///
/// The declared version names the components that must match: `24` is a
/// major version, `3.12` is a major and a minor, `10.0` is a major and a
/// minor. The comparison is component-wise and numeric, never a string
/// prefix: `3.1` does not satisfy `3.12`, `2` does not satisfy `24` and
/// `3.9` does not satisfy `3.12`. A component the observed version does
/// not carry cannot be confirmed, so it does not satisfy the requirement.
pub fn version_satisfies(declared: &str, observed: &str) -> bool {
    let required = match version_components(declared) {
        Some(components) if !components.is_empty() => components,
        _ => return false,
    };
    let found = match version_components(observed) {
        Some(components) => components,
        None => return false,
    };
    if found.len() < required.len() {
        return false;
    }
    required
        .iter()
        .zip(found.iter())
        .all(|(want, have)| want == have)
}

/// Whether a declaration names a version the rule can judge.
///
/// A declaration that is not a version — a moving channel such as
/// `stable` — is not a version requirement: a channel cannot be pinned and
/// is never refused on a version. A profile that declares one is judged on
/// presence only.
pub fn is_version_requirement(declaration: &str) -> bool {
    !declaration.is_empty() && version_components(declaration).is_some()
}

/// Judge a declaration against an observed version.
///
/// `Some(true)` when the observation satisfies the declaration,
/// `Some(false)` when it does not, and `None` when there is nothing to
/// judge: the profile declares no version, declares a channel rather than
/// a version, or the host's version could not be read. Unknown is never a
/// mismatch.
pub fn judge(declaration: Option<&str>, observed: Option<&str>) -> Option<bool> {
    let declaration = declaration?;
    if !is_version_requirement(declaration) {
        return None;
    }
    let observed = observed?;
    Some(version_satisfies(declaration, observed))
}

/// The numeric components of a version string.
///
/// `None` when a component is not a number: a version that cannot be read
/// is unknown, and unknown never satisfies a requirement.
fn version_components(version: &str) -> Option<Vec<u64>> {
    let mut components = Vec::new();
    for part in version.split('.') {
        if part.is_empty() {
            return None;
        }
        match part.parse::<u64>() {
            Ok(value) => components.push(value),
            Err(_) => return None,
        }
    }
    Some(components)
}

/// The binary whose version carries the ecosystem's runtime.
///
/// The Node ecosystem's runtime is `node`: npm and node ship together and
/// Forge already treats them as one toolchain, so the version a
/// Node-ecosystem profile declares is node's version. A toolchain with no
/// registered ecosystem is its own runtime.
pub fn runtime_binary(toolchain: &str) -> &str {
    match Ecosystem::for_toolchain(toolchain) {
        Ecosystem::Npm => "node",
        Ecosystem::Dotnet => "dotnet",
        Ecosystem::Cargo => "cargo",
        Ecosystem::Pub => "flutter",
        Ecosystem::None => toolchain,
    }
}

/// The runtime's version under the given `PATH`.
///
/// Runs `<runtime> --version` and reads the version from its banner.
/// Standard output is read first; standard error is the fallback for the
/// tools that wrote their version there (older `python --version`).
///
/// `None` means the version could not be read: the command did not start,
/// it failed, or its output carries no version.
pub fn probe_version(runtime: &str, path: &OsStr) -> Option<String> {
    let output = Command::new(runtime)
        .arg("--version")
        .env("PATH", path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(version) = extract_version(&stdout) {
        return Some(version);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    extract_version(&stderr)
}

/// One entry of a host's toolchain inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainEntry<'a> {
    /// The toolchain's name (`node`).
    pub name: &'a str,
    /// The version the host reports for it, when the entry carries one.
    pub version: Option<&'a str>,
}

/// Read one entry of a host's toolchain inventory.
///
/// An entry is a toolchain name, optionally followed by `@` and the version
/// the host reports for it: `node` says "node is present, its version is
/// unknown", `node@22.23.3` says "node is present and reports 22.23.3".
/// The versioned form lets a caller judge a declaration against a host it
/// describes instead of probing one.
pub fn inventory_entry(entry: &str) -> ToolchainEntry<'_> {
    match entry.split_once('@') {
        Some((name, version)) if !version.is_empty() => ToolchainEntry {
            name,
            version: Some(version),
        },
        _ => ToolchainEntry {
            name: entry,
            version: None,
        },
    }
}

/// The inventory's entry for `toolchain`, if the inventory has one.
///
/// An entry that carries a version wins over a bare name: the inventory
/// then states the version the host reports instead of leaving it unknown.
pub fn inventory_lookup<'a>(
    inventory: &'a HashSet<String>,
    toolchain: &str,
) -> Option<ToolchainEntry<'a>> {
    let mut bare: Option<ToolchainEntry<'a>> = None;
    for entry in inventory.iter() {
        let parsed = inventory_entry(entry.as_str());
        if parsed.name != toolchain {
            continue;
        }
        if parsed.version.is_some() {
            return Some(parsed);
        }
        bare = Some(parsed);
    }
    bare
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_banner_yields_its_first_dotted_numeric_token() {
        assert_eq!(extract_version("v22.23.3"), Some("22.23.3".to_string()));
        assert_eq!(extract_version("10.9.9"), Some("10.9.9".to_string()));
        assert_eq!(extract_version("Python 3.14.4"), Some("3.14.4".to_string()));
        assert_eq!(extract_version("10.0.400"), Some("10.0.400".to_string()));
        assert_eq!(
            extract_version(
                "Flutter 3.47.1 • channel stable • https://github.com/flutter/flutter.git"
            ),
            Some("3.47.1".to_string())
        );
        assert_eq!(
            extract_version("cargo 1.99.1 (5f94df478 2026-08-27)"),
            Some("1.99.1".to_string())
        );
    }

    #[test]
    fn a_banner_without_a_version_yields_nothing() {
        assert_eq!(extract_version(""), None);
        assert_eq!(extract_version("command not found"), None);
        assert_eq!(extract_version("no numeric token here"), None);
    }

    #[test]
    fn a_banner_reads_the_first_line_of_a_multiline_banner() {
        let banner = "Flutter 3.47.1 • channel stable • https://github.com/flutter/flutter.git\nTools • Dart 3.11.2 • DevTools 2.51.0";
        assert_eq!(extract_version(banner), Some("3.47.1".to_string()));
    }

    #[test]
    fn the_declared_component_count_decides_what_must_match() {
        assert!(version_satisfies("24", "24.5.0"));
        assert!(!version_satisfies("24", "22.23.3"));
        assert!(version_satisfies("3.12", "3.12.4"));
        assert!(!version_satisfies("3.12", "3.14.4"));
        assert!(version_satisfies("3.12", "3.12"));
        assert!(version_satisfies("10.0", "10.0.400"));
        assert!(!version_satisfies("10.0", "10.1.0"));
    }

    #[test]
    fn the_comparison_is_numeric_and_never_a_prefix() {
        assert!(!version_satisfies("3.1", "3.12.4"));
        assert!(!version_satisfies("2", "24"));
        assert!(!version_satisfies("3.9", "3.12.4"));
        assert!(version_satisfies("3.9", "3.9.1"));
    }

    #[test]
    fn a_component_the_observed_version_lacks_cannot_be_confirmed() {
        assert!(!version_satisfies("3.12.1", "3.12"));
        assert!(version_satisfies("3.12", "3.12.4"));
    }

    #[test]
    fn an_unreadable_version_never_satisfies_a_requirement() {
        assert!(!version_satisfies("24", "unknown"));
        assert!(!version_satisfies("24", ""));
        assert!(!version_satisfies("", "24.5.0"));
    }

    #[test]
    fn a_channel_is_not_a_version_requirement() {
        assert!(is_version_requirement("24"));
        assert!(is_version_requirement("3.12"));
        assert!(!is_version_requirement("stable"));
        assert!(!is_version_requirement(""));
        assert_eq!(judge(Some("stable"), Some("1.99.1")), None);
        assert_eq!(judge(Some("24"), Some("24.5.0")), Some(true));
        assert_eq!(judge(Some("24"), Some("22.23.3")), Some(false));
        assert_eq!(judge(Some("24"), None), None);
        assert_eq!(judge(None, Some("24.5.0")), None);
    }

    #[test]
    fn the_runtime_is_the_ecosystems_binary() {
        assert_eq!(runtime_binary("npm"), "node");
        assert_eq!(runtime_binary("node"), "node");
        assert_eq!(runtime_binary("dotnet"), "dotnet");
        assert_eq!(runtime_binary("cargo"), "cargo");
        assert_eq!(runtime_binary("flutter"), "flutter");
        assert_eq!(runtime_binary("python3"), "python3");
    }

    #[test]
    fn an_inventory_entry_states_a_version_when_it_carries_one() {
        let plain = inventory_entry("node");
        assert_eq!(plain.name, "node");
        assert_eq!(plain.version, None);
        let versioned = inventory_entry("node@22.23.3");
        assert_eq!(versioned.name, "node");
        assert_eq!(versioned.version, Some("22.23.3"));
    }

    #[test]
    fn an_inventory_lookup_prefers_the_versioned_entry() {
        let inventory: HashSet<String> = ["node".to_string(), "node@22.23.3".to_string()]
            .into_iter()
            .collect();
        let found = inventory_lookup(&inventory, "node").expect("node is in the inventory");
        assert_eq!(found.version, Some("22.23.3"));
        assert!(inventory_lookup(&inventory, "npm").is_none());
    }

    #[cfg(unix)]
    fn fake_runtime(dir: &std::path::Path, name: &str, script: &str) -> std::ffi::OsString {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, script).expect("write the fake runtime");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make the fake runtime executable");
        std::env::join_paths([dir]).expect("join the fake runtime path")
    }

    #[cfg(unix)]
    #[test]
    fn a_probe_reads_the_version_the_runtime_reports() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = fake_runtime(dir.path(), "faketool", "#!/bin/sh\necho 'faketool 4.5.6'\n");
        assert_eq!(probe_version("faketool", &path), Some("4.5.6".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn a_probe_falls_back_to_stderr_for_a_tool_that_writes_there() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = fake_runtime(
            dir.path(),
            "oldpython",
            "#!/bin/sh\necho 'Python 3.9.0' >&2\n",
        );
        assert_eq!(probe_version("oldpython", &path), Some("3.9.0".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn a_probe_of_a_runtime_that_cannot_start_yields_nothing() {
        assert_eq!(
            probe_version("forge-runtime-that-does-not-exist", OsStr::new("")),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_probe_of_a_failing_runtime_yields_nothing() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = fake_runtime(dir.path(), "failingtool", "#!/bin/sh\nexit 1\n");
        assert_eq!(probe_version("failingtool", &path), None);
    }
}

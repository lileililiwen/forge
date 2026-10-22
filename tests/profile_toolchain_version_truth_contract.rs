//! Profile toolchain version truth contract (`profile-toolchain-version-truth`).
//!
//! A profile declares a required toolchain version. The declaration must
//! be true: it must be a version the profile's runtime can have, it must
//! match the versions the repository's own qualification provisions, and
//! Forge must check it — a host of the wrong version is refused by the
//! preflight and unverified by the readiness matrix, never reported as
//! missing and never reported as passing.
//!
//! Four claims are pinned here:
//!
//! 1. The catalogue. Every supported profile declares the version the
//!    repository qualifies against, and the CI workflow provisions the Node
//!    version the Node-ecosystem profiles declare. The declarations cannot
//!    drift away from the qualification.
//! 2. The rule. A declared version names the components that must match:
//!    `24` is a major version, `3.12` a major and a minor. The comparison
//!    is numeric and component-wise, never a string prefix.
//! 3. The banner. A runtime's version is read from its `--version` banner:
//!    a product name, a leading `v`, a channel label, a build hash and a
//!    URL are not the version.
//! 4. The check. The preflight refuses a host whose version does not
//!    satisfy the declaration with its own code, and the readiness matrix
//!    reports such a host as `unverified` with the mismatch named.
//!
//! The behaviour is driven through the public API. The preflight and the
//! matrix take an explicit host inventory so the check is deterministic: an
//! inventory entry is a toolchain name, optionally followed by `@` and the
//! version the host reports for it (`node@22.23.3`).

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use forge::profile::version::{
    extract_version, is_version_requirement, runtime_binary, version_satisfies,
};
use forge::profile::{list_profiles, preflight_profile, ProfileSupportStatus};
use forge::readiness::{run_profile_row, ReadinessStatus};

/// The declared toolchain version of every supported profile, with the
/// runtime the declaration is judged against.
///
/// The Node-ecosystem profiles declare the Node version CI provisions
/// (`node-version: "24"`), not an npm version: npm has no version 24 and
/// Forge treats npm and node as one toolchain.
const DECLARED: [(&str, Option<&str>, &str); 6] = [
    ("aspnet-web", Some("10.0"), "dotnet"),
    ("flutter-app", Some("3.22"), "flutter"),
    ("nextjs-web", Some("24"), "node"),
    ("python-service", Some("3.12"), "python3"),
    ("react-web", Some("24"), "node"),
    ("rust-web", None, "cargo"),
];

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// A host inventory: a toolchain name, optionally with the version the
/// host reports for it.
fn host(entries: &[&str]) -> HashSet<String> {
    entries.iter().map(|entry| entry.to_string()).collect()
}

#[test]
fn every_supported_profile_declares_the_qualified_version() {
    // The map is the version requirement each profile states. A profile
    // that names a moving channel instead of a version (`rust-web` tracks
    // the `stable` channel) states no version requirement: a channel
    // cannot be pinned and is never refused on a version.
    let declared: BTreeMap<String, Option<String>> = list_profiles()
        .into_iter()
        .filter(|p| p.support_status == ProfileSupportStatus::Supported)
        .map(|p| {
            (
                p.id.clone(),
                p.toolchain_version
                    .clone()
                    .filter(|version| is_version_requirement(version)),
            )
        })
        .collect();
    let expected: BTreeMap<String, Option<String>> = DECLARED
        .iter()
        .map(|(id, version, _runtime)| (id.to_string(), version.map(str::to_string)))
        .collect();
    assert_eq!(
        declared, expected,
        "the catalogue must declare the qualified versions"
    );
}

#[test]
fn the_ci_workflow_provisions_the_qualified_node_version() {
    let workflow = std::fs::read_to_string(repo_path(".github/workflows/ci.yml"))
        .expect("the CI workflow must be readable");
    assert!(
        workflow.contains("node-version: \"24\""),
        "the CI workflow must provision Node 24: the Node-ecosystem profiles declare it"
    );
}

#[test]
fn every_declared_version_is_a_version_its_runtime_can_have() {
    let toolchains: BTreeMap<String, String> = list_profiles()
        .into_iter()
        .map(|p| (p.id.clone(), p.toolchain))
        .collect();
    for (id, version, runtime) in DECLARED {
        let declared = match version {
            Some(version) => version,
            None => continue,
        };
        let toolchain = toolchains
            .get(id)
            .unwrap_or_else(|| panic!("'{id}' must be in the catalogue"));
        assert_eq!(
            runtime_binary(toolchain),
            runtime,
            "the declared version of '{id}' is judged against the runtime's own version"
        );
        assert!(
            version_satisfies(declared, &format!("{declared}.0")),
            "'{declared}' must name a version line the runtime can have"
        );
    }
}

#[test]
fn the_rule_judges_the_declared_components() {
    assert!(version_satisfies("24", "24.5.0"));
    assert!(!version_satisfies("24", "22.23.3"));
    assert!(version_satisfies("3.12", "3.12.4"));
    assert!(!version_satisfies("3.12", "3.14.4"));
    assert!(version_satisfies("10.0", "10.0.400"));
    assert!(!version_satisfies("3.1", "3.12.4"));
    assert!(!version_satisfies("2", "24"));
}

#[test]
fn the_banner_yields_the_version() {
    assert_eq!(extract_version("v22.23.3").as_deref(), Some("22.23.3"));
    assert_eq!(extract_version("Python 3.14.4").as_deref(), Some("3.14.4"));
    assert_eq!(
        extract_version("Flutter 3.47.1 • channel stable • https://github.com/flutter/flutter.git")
            .as_deref(),
        Some("3.47.1")
    );
    assert_eq!(
        extract_version("cargo 1.99.1 (5f94df478 2026-08-27)").as_deref(),
        Some("1.99.1")
    );
}

#[test]
fn the_node_ecosystem_runs_on_node() {
    assert_eq!(runtime_binary("npm"), "node");
    assert_eq!(runtime_binary("node"), "node");
    assert_eq!(runtime_binary("python3"), "python3");
}

#[test]
fn a_host_of_the_wrong_version_is_refused_with_the_mismatch_code() {
    let host = host(&["node@22.23.3"]);
    let err = preflight_profile("nextjs-web", Some(&host))
        .expect_err("node 22 does not satisfy the declared 24");
    assert_eq!(err.code(), "toolchain-version-mismatch");
    assert_ne!(
        err.code(),
        "toolchain-missing",
        "a present toolchain of the wrong version is never reported as missing"
    );
    let message = err.to_string();
    assert!(
        message.contains("nextjs-web"),
        "the refusal names the profile: {message}"
    );
    assert!(
        message.contains("24"),
        "the refusal names the declared version: {message}"
    );
    assert!(
        message.contains("22.23.3"),
        "the refusal names the observed version: {message}"
    );
}

#[test]
fn a_host_of_the_declared_version_passes_and_names_what_it_checked() {
    let host = host(&["node@24.5.0"]);
    let report =
        preflight_profile("nextjs-web", Some(&host)).expect("node 24 satisfies the declared 24");
    assert!(report.available);
    assert_eq!(report.toolchain_version.as_deref(), Some("24.5.0"));
    assert_eq!(report.toolchain_version_declared.as_deref(), Some("24"));
}

#[test]
fn a_profile_without_a_declaration_is_judged_on_presence_only() {
    let host = host(&["cargo@1.99.1"]);
    let report = preflight_profile("rust-web", Some(&host))
        .expect("rust-web declares no version, so presence is the only condition");
    assert!(report.available);
    assert_eq!(report.toolchain_version_declared, None);
}

#[test]
fn a_row_on_a_mismatched_version_is_unverified_and_runs_no_native_command() {
    let host = host(&["node@22.23.3"]);
    let row = run_profile_row("nextjs-web", Some(&host)).expect("the row is returned");
    assert_eq!(row.result, ReadinessStatus::Unverified);
    assert!(row.build.is_none(), "no native build may run");
    assert!(row.test.is_none(), "no native test may run");
    assert_eq!(row.toolchain_runtime, "node");
    assert_eq!(row.toolchain_version.as_deref(), Some("22.23.3"));
    assert_eq!(row.toolchain_version_declared.as_deref(), Some("24"));
    assert_eq!(row.toolchain_version_satisfied, Some(false));
    assert!(
        row.reason.contains("nextjs-web"),
        "the row names the profile: {}",
        row.reason
    );
    assert!(
        row.reason.contains("24"),
        "the row names the declared version: {}",
        row.reason
    );
    assert!(
        row.reason.contains("22.23.3"),
        "the row names the observed version: {}",
        row.reason
    );
}

#[test]
fn a_row_of_the_npm_ecosystem_names_the_runtime_it_probed() {
    let host = host(&["npm", "node@22.23.3"]);
    let row = run_profile_row("react-web", Some(&host)).expect("the row is returned");
    assert_eq!(row.toolchain, "npm");
    assert_eq!(row.toolchain_runtime, "node");
    assert_eq!(row.toolchain_version.as_deref(), Some("22.23.3"));
    assert_eq!(row.toolchain_version_declared.as_deref(), Some("24"));
    assert_eq!(row.toolchain_version_satisfied, Some(false));
    assert_eq!(row.result, ReadinessStatus::Unverified);
}

#[test]
fn a_row_without_a_declaration_records_no_requirement() {
    // The toolchain is absent, so the row is unverified before any native
    // command runs. A profile without a declaration has no requirement to
    // satisfy.
    let host = host(&[]);
    let row = run_profile_row("rust-web", Some(&host)).expect("the row is returned");
    assert_eq!(row.toolchain_runtime, "cargo");
    assert_eq!(row.toolchain_version_declared, None);
    assert_eq!(
        row.toolchain_version_satisfied, None,
        "without a declaration there is nothing to satisfy"
    );
    assert_eq!(row.result, ReadinessStatus::Unverified);
}

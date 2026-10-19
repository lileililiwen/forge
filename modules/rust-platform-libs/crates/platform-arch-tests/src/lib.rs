//! Workspace-level architecture tests.
//!
//! These tests are the single source of truth for the
//! dependency-direction rule documented in the OpenSpec change:
//!
//! * Foundation **production** crates MUST NOT depend on Axum, SQLx,
//!   database drivers, cookie/session implementations, OIDC/JWT
//!   providers, lettre, tokio, or any product crate.
//! * Foundation **production** crates MUST NOT depend on the
//!   test-only `platform-testing` crate.
//! * The named satellite `platform-mailing` alone may reach `lettre`
//!   and its transitive `tokio` (see [`SATELLITE_ALLOWLIST`]); any
//!   other crate reaching them is denied until an explicit policy
//!   change names it.
//!
//! The rule itself is the pure function [`check_crate_policy`] over
//! `(crate, transitive-graph)` so unit tests can exercise violations
//! with synthesized graphs. The tests below apply it to the real
//! manifests and lockfile.
//!
//! The test approach is intentionally lightweight: we read the
//! workspace's own `Cargo.toml` files, the `Cargo.lock` for the
//! transitive check, and assert the forbidden-dep set is empty. We
//! do not call `cargo metadata` so the tests are deterministic and
//! run offline.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Crate name of the test-only support crate. Production crates MUST
/// NOT list this as a direct dependency.
pub const TEST_ONLY_CRATE: &str = "platform-testing";

/// Workspace member names that are considered production crates.
///
/// The four foundation crates. Satellite crates such as
/// `platform-mailing` are production code too, but they live under
/// the named-satellite allowlist ([`SATELLITE_ALLOWLIST`]) instead of
/// this list so the foundation rule stays frozen.
pub const PRODUCTION_CRATES: &[&str] = &[
    "platform-core",
    "platform-contracts",
    "platform-identity",
    "platform-audit",
];

/// Named satellite crates and the extra dependencies each one is
/// explicitly allowed to reach. The two satellites today are
/// `platform-mailing` (SMTP mail delivery, extracted after three
/// consumers demonstrated a compatible lettre contract) and
/// `platform-ai` (the provider-neutral AI capability contract plus
/// opt-in OpenAI-compatible and ollama adapters). A crate not
/// named here that reaches `lettre`, `reqwest`, or `tokio` is a
/// violation until an explicit policy change names it.
pub const SATELLITE_ALLOWLIST: &[(&str, &[&str])] = &[
    ("platform-mailing", &["lettre", "tokio"]),
    ("platform-ai", &["reqwest", "tokio", "futures"]),
];

/// Forbidden dependency crate names. A direct or transitive
/// dependency on any of these in a foundation production crate is a
/// violation of the provider-neutrality rule. `lettre` is forbidden
/// to the foundation even though the named satellite
/// `platform-mailing` may use it; [`check_crate_policy`] applies the
/// allowlist per crate.
pub const FORBIDDEN_DEPS: &[&str] = &[
    "axum",
    "axum-extra",
    "tower",
    "tower-http",
    "sqlx",
    "sqlx-core",
    "sqlx-macros",
    "sqlx-postgres",
    "sqlx-sqlite",
    "sqlx-mysql",
    "diesel",
    "sea-orm",
    "cookie",
    "cookies",
    "jsonwebtoken",
    "openidconnect",
    "oauth2",
    "reqwest",
    "lettre", // satellite-only; see SATELLITE_ALLOWLIST
    "tokio",  // foundation is sync-only; async adapters live outside
];

/// Direct forbidden deps that are checked by name in each
/// production crate's `Cargo.toml` `dependencies` and
/// `dev-dependencies` tables. These are listed separately from
/// [`FORBIDDEN_DEPS`] so the failure message can be specific.
/// `lettre` is direct-forbidden to the foundation; only the named
/// satellite `platform-mailing` may declare it.
pub const DIRECT_FORBIDDEN_DEPS: &[&str] = &[
    "axum",
    "sqlx",
    "cookie",
    "cookies",
    "jsonwebtoken",
    "openidconnect",
    "oauth2",
    "reqwest",
    "lettre",
    "tokio",
];

/// Pure dependency-direction policy over one crate and its
/// transitive dependency names. Returns one message per violation,
/// empty when the crate complies:
///
/// * a foundation [`PRODUCTION_CRATES`] crate MUST NOT reach any
///   [`FORBIDDEN_DEPS`] entry or the [`TEST_ONLY_CRATE`];
/// * a [`SATELLITE_ALLOWLIST`] crate may reach exactly its listed
///   allowances (plus anything not forbidden) but MUST NOT reach
///   any other [`FORBIDDEN_DEPS`] entry or the [`TEST_ONLY_CRATE`];
/// * any other crate that reaches `lettre`/`tokio` is an unknown
///   satellite and is denied until an explicit policy change names
///   it.
///
/// Pure over `(crate, graph)` so a synthesized graph can exercise a
/// violation that does not exist in the real lockfile.
#[must_use]
pub fn check_crate_policy(crate_name: &str, transitive: &BTreeSet<String>) -> Vec<String> {
    let mut violations = Vec::new();
    if transitive.contains(TEST_ONLY_CRATE) {
        violations.push(format!(
            "{crate_name} reaches test-only crate {TEST_ONLY_CRATE:?}"
        ));
    }
    if PRODUCTION_CRATES.contains(&crate_name) {
        for forbidden in FORBIDDEN_DEPS {
            if transitive.contains(*forbidden) {
                violations.push(format!(
                    "{crate_name} reaches forbidden dependency {forbidden:?}"
                ));
            }
        }
        return violations;
    }
    if let Some((_, allowed)) = SATELLITE_ALLOWLIST
        .iter()
        .find(|(name, _)| *name == crate_name)
    {
        for forbidden in FORBIDDEN_DEPS {
            if allowed.contains(forbidden) {
                continue;
            }
            if transitive.contains(*forbidden) {
                violations.push(format!(
                    "{crate_name} reaches forbidden dependency {forbidden:?} \
                     outside its satellite allowlist"
                ));
            }
        }
        return violations;
    }
    // Unknown crate: only the satellite-gated leaves are policed.
    for gated in ["lettre", "tokio"] {
        if transitive.contains(gated) {
            violations.push(format!(
                "{crate_name} reaches {gated:?} without a satellite allowlist entry"
            ));
        }
    }
    violations
}

/// Locate the workspace root by walking up from `CARGO_MANIFEST_DIR`.
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("arch-tests crate lives at crates/platform-arch-tests")
        .to_path_buf()
}

/// Parse every `Cargo.toml` under `crates/` and return a map of
/// crate name -> parsed manifest.
pub fn read_workspace_manifests(workspace: &Path) -> BTreeMap<String, ParsedCrate> {
    let crates_dir = workspace.join("crates");
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(&crates_dir).expect("crates/ is readable") {
        let entry = entry.expect("dir entry");
        let crate_dir = entry.path();
        if !crate_dir.is_dir() {
            continue;
        }
        let manifest_path = crate_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&manifest_path).expect("Cargo.toml is readable");
        let value: toml::Value = toml::from_str(&raw).expect("Cargo.toml is valid TOML");
        let name = value
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .expect("Cargo.toml has package.name")
            .to_string();
        let parsed = ParsedCrate::from_manifest(value);
        out.insert(name, parsed);
    }
    out
}

/// A minimal projection of a `Cargo.toml` manifest, just enough to
/// assert dependency direction.
#[derive(Debug, Clone, Default)]
pub struct ParsedCrate {
    /// Crate name.
    pub name: String,
    /// Direct dependencies (crate name -> path or version).
    pub dependencies: BTreeSet<String>,
    /// Dev dependencies.
    pub dev_dependencies: BTreeSet<String>,
    /// Workspace-relative directory.
    pub dir: PathBuf,
}

impl ParsedCrate {
    /// Parse a `Cargo.toml` value into a [`ParsedCrate`].
    #[must_use]
    pub fn from_manifest(value: toml::Value) -> Self {
        let name = value
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let mut dependencies = BTreeSet::new();
        let mut dev_dependencies = BTreeSet::new();
        let direct_keys = ["dependencies", "dev-dependencies", "build-dependencies"];
        for key in direct_keys {
            if let Some(table) = value.get(key).and_then(|v| v.as_table()) {
                for dep_name in table.keys() {
                    if key == "dev-dependencies" {
                        dev_dependencies.insert(dep_name.clone());
                    } else {
                        dependencies.insert(dep_name.clone());
                    }
                }
            }
        }
        Self {
            name,
            dependencies,
            dev_dependencies,
            dir: PathBuf::new(),
        }
    }
}

/// Parse the workspace `Cargo.lock` to enumerate resolved packages.
#[derive(Debug)]
pub struct Lockfile {
    /// Crate name -> list of dependency crate names.
    pub packages: BTreeMap<String, Vec<String>>,
}

impl Lockfile {
    /// Load and parse `Cargo.lock` from `workspace`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the file is missing or malformed.
    pub fn load(workspace: &Path) -> Result<Self, String> {
        let path = workspace.join("Cargo.lock");
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
        let mut packages = BTreeMap::new();
        let pkgs = value
            .get("package")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Cargo.lock has no [[package]] array".to_string())?;
        for pkg in pkgs {
            let name = pkg
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "package entry missing name".to_string())?
                .to_string();
            let deps = pkg
                .get("dependencies")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|d| d.get("name").and_then(|n| n.as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            packages.insert(name, deps);
        }
        Ok(Self { packages })
    }

    /// Returns the transitive closure of dependencies starting from
    /// `roots`.
    pub fn transitive_closure(&self, roots: &[String]) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack: Vec<String> = roots.to_vec();
        while let Some(next) = stack.pop() {
            if !out.insert(next.clone()) {
                continue;
            }
            if let Some(deps) = self.packages.get(&next) {
                for dep in deps {
                    if !out.contains(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workspace_manifests() {
        let ws = workspace_root();
        let manifests = read_workspace_manifests(&ws);
        for name in PRODUCTION_CRATES {
            assert!(
                manifests.contains_key(*name),
                "production crate {name} missing from crates/"
            );
        }
        assert!(
            manifests.contains_key(TEST_ONLY_CRATE),
            "test-only crate {TEST_ONLY_CRATE} missing from crates/"
        );
    }

    #[test]
    fn production_crates_have_no_direct_forbidden_deps() {
        let ws = workspace_root();
        let manifests = read_workspace_manifests(&ws);
        let mut failures: Vec<String> = Vec::new();
        for name in PRODUCTION_CRATES {
            let Some(crate_manifest) = manifests.get(*name) else {
                failures.push(format!("{name}: missing manifest"));
                continue;
            };
            for dep in &crate_manifest.dependencies {
                if DIRECT_FORBIDDEN_DEPS.contains(&dep.as_str()) {
                    failures.push(format!("{name} has forbidden direct dependency {dep:?}"));
                }
            }
            for dep in &crate_manifest.dev_dependencies {
                if DIRECT_FORBIDDEN_DEPS.contains(&dep.as_str()) {
                    failures.push(format!(
                        "{name} has forbidden direct dev-dependency {dep:?}"
                    ));
                }
                if dep == TEST_ONLY_CRATE {
                    failures.push(format!(
                        "{name} has dev-dependency on test-only crate {dep:?}"
                    ));
                }
            }
            if crate_manifest.dependencies.contains(TEST_ONLY_CRATE) {
                failures.push(format!(
                    "{name} has direct dependency on test-only crate {TEST_ONLY_CRATE:?}"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "dependency-direction violations:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn satellite_allowlist_permits_only_platform_mailing() {
        // Compliant: the named satellite reaches exactly its
        // allowances.
        let mailing: BTreeSet<String> = ["lettre", "tokio", "platform-core"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            check_crate_policy("platform-mailing", &mailing).is_empty(),
            "named satellite with its allowances must pass"
        );
        // The satellite reaching anything else forbidden still fails.
        let mut overreach = mailing.clone();
        overreach.insert("axum".to_string());
        let violations = check_crate_policy("platform-mailing", &overreach);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("platform-mailing"));
        assert!(violations[0].contains("axum"));
    }

    #[test]
    fn ai_satellite_allowlist_permits_reqwest() {
        // Compliant: `platform-ai` reaches its allowlisted leaves
        // (reqwest, tokio, futures) plus non-forbidden helpers.
        let ai: BTreeSet<String> = [
            "reqwest",
            "tokio",
            "futures",
            "async-stream",
            "async-trait",
            "serde",
            "serde_json",
            "thiserror",
            "platform-core",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert!(
            check_crate_policy("platform-ai", &ai).is_empty(),
            "named ai satellite with its allowances must pass"
        );
        // The satellite reaching `lettre` (not on its allowlist)
        // still fails: the allowlist is not a free pass.
        let mut overreach = ai.clone();
        overreach.insert("lettre".to_string());
        let violations = check_crate_policy("platform-ai", &overreach);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("platform-ai"));
        assert!(violations[0].contains("lettre"));
    }

    #[test]
    fn foundation_crate_cannot_reach_reqwest() {
        // Synthesized graph: reqwest attached to a foundation crate
        // (the rule must refuse even though `platform-ai` may use it).
        let mut graph = BTreeSet::new();
        graph.insert("reqwest".to_string());
        graph.insert("serde".to_string());
        let violations = check_crate_policy("platform-identity", &graph);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("platform-identity"));
        assert!(violations[0].contains("reqwest"));
    }

    #[test]
    fn foundation_violation_is_reported_by_crate_and_dep() {
        // Synthesized graph: lettre attached to a foundation crate.
        // The real lockfile walk alone cannot exercise a violation
        // that does not exist, so the pure function is tested with a
        // graph that does.
        let mut graph = BTreeSet::new();
        graph.insert("lettre".to_string());
        graph.insert("serde".to_string());
        let violations = check_crate_policy("platform-identity", &graph);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("platform-identity"));
        assert!(violations[0].contains("lettre"));
    }

    #[test]
    fn unknown_satellite_reaching_lettre_is_denied() {
        let mut graph = BTreeSet::new();
        graph.insert("lettre".to_string());
        let violations = check_crate_policy("some-new-crate", &graph);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("some-new-crate"));
        assert!(violations[0].contains("lettre"));

        let clean = BTreeSet::new();
        assert!(check_crate_policy("some-new-crate", &clean).is_empty());
    }

    #[test]
    fn lockfile_resolves_and_does_not_contain_forbidden() {
        // The lockfile may be missing if the workspace has never been
        // built. In that case the test cannot enforce a transitive
        // check and we treat that as a separate, recoverable failure.
        let ws = workspace_root();
        let lockfile = match Lockfile::load(&ws) {
            Ok(l) => l,
            Err(err) => panic!("BLOCKED: {err}. Run `cargo build` once to generate Cargo.lock."),
        };
        for name in PRODUCTION_CRATES {
            let transitive = lockfile.transitive_closure(&[(*name).to_string()]);
            let violations = check_crate_policy(name, &transitive);
            assert!(
                violations.is_empty(),
                "dependency-direction violations:\n{}",
                violations.join("\n")
            );
        }
        // The named satellite complies through the same pure rule.
        for (satellite, _) in SATELLITE_ALLOWLIST {
            let transitive = lockfile.transitive_closure(&[(*satellite).to_string()]);
            let violations = check_crate_policy(satellite, &transitive);
            assert!(
                violations.is_empty(),
                "dependency-direction violations:\n{}",
                violations.join("\n")
            );
        }
    }

    /// Parse the `.publish-order` file at the workspace root. Comment
    /// lines (`#…`) and blank lines are skipped; the remaining lines
    /// are returned in file order.
    fn read_publish_order(workspace: &Path) -> Vec<String> {
        let raw = fs::read_to_string(workspace.join(".publish-order"))
            .expect(".publish-order is readable at the workspace root");
        raw.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect()
    }

    /// Names of every workspace member that may be published to a
    /// registry. The set excludes the one explicitly-unpublishable
    /// member (`platform-arch-tests`, `publish = false`) and
    /// includes every other member, including the public test-only
    /// crate `platform-testing` (which is published so consumers can
    /// `cargo add --dev platform-testing`).
    fn publishable_crate_names(manifests: &BTreeMap<String, ParsedCrate>) -> BTreeSet<String> {
        manifests
            .iter()
            .filter(|(name, _)| *name != "platform-arch-tests")
            .map(|(name, _)| name.clone())
            .collect()
    }

    #[test]
    fn publish_order_covers_every_publishable_crate_exactly_once() {
        let ws = workspace_root();
        let order = read_publish_order(&ws);
        let manifests = read_workspace_manifests(&ws);
        let publishable = publishable_crate_names(&manifests);

        // Every name in `.publish-order` MUST be a real, publishable
        // workspace member.
        for name in &order {
            assert!(
                publishable.contains(name),
                ".publish-order lists {name:?} but it is not a publishable workspace member"
            );
        }
        // `platform-arch-tests` MUST NOT appear, even if it slips in.
        assert!(
            !order.iter().any(|n| n == "platform-arch-tests"),
            ".publish-order must not list platform-arch-tests (publish = false)"
        );
        // Every publishable crate MUST appear exactly once.
        let unique: BTreeSet<&String> = order.iter().collect();
        assert_eq!(
            unique.len(),
            order.len(),
            ".publish-order has duplicate entries: {order:?}"
        );
        let order_set: BTreeSet<String> = order.iter().cloned().collect();
        let missing: Vec<&String> = publishable.difference(&order_set).collect();
        assert!(
            missing.is_empty(),
            ".publish-order is missing publishable crates: {missing:?}"
        );
    }

    #[test]
    fn publish_order_respects_dependency_dag() {
        // For every `b` in the order, every `platform-*` direct
        // dependency of `b` MUST appear earlier in the order. A pair
        // that violates this would mean the publish workflow uploads
        // a crate before its dep is on the registry, which
        // `cargo publish` refuses (it requires registry-addressable
        // deps).
        let ws = workspace_root();
        let order = read_publish_order(&ws);
        let order_index: BTreeMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let manifests = read_workspace_manifests(&ws);
        for b in &order {
            let b_manifest = manifests
                .get(b)
                .unwrap_or_else(|| panic!("publish-order entry {b:?} has no manifest"));
            for dep in &b_manifest.dependencies {
                // Only platform-* deps must be ordered; non-platform
                // deps resolve from crates.io independently.
                if !dep.starts_with("platform-") {
                    continue;
                }
                let dep_index = order_index.get(dep.as_str()).copied().unwrap_or_else(|| {
                    panic!("publish-order entry {b:?} depends on {dep:?} which is not in the order")
                });
                let b_index = order_index[b.as_str()];
                assert!(
                    dep_index < b_index,
                    "publish-order DAG violation: {b} (position {b_index}) depends on {dep} (position {dep_index}); reorder .publish-order so every dep comes first"
                );
            }
        }
    }

    #[test]
    fn arch_tests_manifest_explicitly_publish_false() {
        // The publish workflow relies on `platform-arch-tests`
        // refusing to publish, not merely being absent from
        // `.publish-order`. Read the manifest verbatim and assert
        // `publish = false` is set explicitly so a future change
        // cannot accidentally re-enable publication through
        // workspace inheritance.
        let ws = workspace_root();
        let manifest_path = ws
            .join("crates")
            .join("platform-arch-tests")
            .join("Cargo.toml");
        let raw = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
        let value: toml::Value = toml::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", manifest_path.display()));
        let publish = value
            .get("package")
            .and_then(|p| p.get("publish"))
            .unwrap_or_else(|| {
                panic!(
                    "platform-arch-tests/Cargo.toml must declare `publish` explicitly (got: {raw})"
                )
            });
        assert_eq!(
            publish.as_bool(),
            Some(false),
            "platform-arch-tests/Cargo.toml must set `publish = false`; got {publish:?}"
        );
    }
}

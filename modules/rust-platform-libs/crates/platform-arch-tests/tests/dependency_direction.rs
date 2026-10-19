//! Top-level integration test that runs the architecture checks
//! against the real manifests and lockfile.

use platform_arch_tests::{
    check_crate_policy, Lockfile, ParsedCrate, PRODUCTION_CRATES, SATELLITE_ALLOWLIST,
    TEST_ONLY_CRATE,
};

#[test]
fn production_crates_listed() {
    for name in PRODUCTION_CRATES {
        assert!(!name.is_empty());
    }
    assert_eq!(TEST_ONLY_CRATE, "platform-testing");
}

#[test]
fn production_crates_do_not_depend_on_test_only() {
    // The unit tests in `lib.rs` already enforce the strict rule
    // against forbidden direct deps. This integration test asserts
    // the broader invariant: a production crate can never reach the
    // test-only crate through any path, direct or transitive.
    let ws = platform_arch_tests::workspace_root();
    let manifests = read_manifests(&ws);
    for name in PRODUCTION_CRATES {
        let manifest = manifests
            .iter()
            .find(|m| m.name == *name)
            .unwrap_or_else(|| panic!("missing manifest for {name}"));
        assert!(
            !manifest.dependencies.contains(TEST_ONLY_CRATE),
            "{name} has direct dependency on {TEST_ONLY_CRATE}"
        );
        assert!(
            !manifest.dev_dependencies.contains(TEST_ONLY_CRATE),
            "{name} has dev-dependency on {TEST_ONLY_CRATE}"
        );
    }
}

#[test]
fn no_production_crate_depends_on_axum() {
    let ws = platform_arch_tests::workspace_root();
    let manifests = read_manifests(&ws);
    for name in PRODUCTION_CRATES {
        let manifest = manifests
            .iter()
            .find(|m| m.name == *name)
            .unwrap_or_else(|| panic!("missing manifest for {name}"));
        for dep in manifest
            .dependencies
            .iter()
            .chain(manifest.dev_dependencies.iter())
        {
            assert_ne!(dep, "axum", "{name} depends on axum");
            assert_ne!(dep, "sqlx", "{name} depends on sqlx");
            assert_ne!(dep, "cookie", "{name} depends on cookie");
            assert_ne!(dep, "jsonwebtoken", "{name} depends on jsonwebtoken");
            assert_ne!(dep, "openidconnect", "{name} depends on openidconnect");
        }
    }
}

#[test]
fn lockfile_transitive_closure_is_clean() {
    let ws = platform_arch_tests::workspace_root();
    let lockfile = match Lockfile::load(&ws) {
        Ok(l) => l,
        Err(err) => panic!("BLOCKED: {err}. Run `cargo build` once."),
    };
    for name in PRODUCTION_CRATES {
        let closure = lockfile.transitive_closure(&[name.to_string()]);
        let violations = check_crate_policy(name, &closure);
        assert!(
            violations.is_empty(),
            "dependency-direction violations:\n{}",
            violations.join("\n")
        );
        for forbidden in &[
            "axum",
            "axum-core",
            "axum-extra",
            "tower",
            "tower-http",
            "sqlx",
            "sqlx-core",
            "sqlx-macros",
            "diesel",
            "sea-orm",
            "cookie",
            "cookies",
            "jsonwebtoken",
            "openidconnect",
            "oauth2",
            "reqwest",
            "lettre",
            "tokio",
        ] {
            assert!(
                !closure.contains(*forbidden),
                "production crate {name} transitively depends on {forbidden:?}"
            );
        }
    }
    // The named satellites comply through the same pure rule: only
    // their allowlisted leaves may appear.
    for (satellite, _) in SATELLITE_ALLOWLIST {
        let closure = lockfile.transitive_closure(&[satellite.to_string()]);
        let violations = check_crate_policy(satellite, &closure);
        assert!(
            violations.is_empty(),
            "dependency-direction violations:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn only_allowlisted_satellites_directly_depend_on_lettre() {
    let ws = platform_arch_tests::workspace_root();
    let manifests = read_manifests(&ws);
    for manifest in &manifests {
        let reaches_lettre = manifest.dependencies.contains("lettre")
            || manifest.dev_dependencies.contains("lettre");
        if !reaches_lettre {
            continue;
        }
        let allowed = SATELLITE_ALLOWLIST
            .iter()
            .any(|(name, _)| *name == manifest.name);
        assert!(
            allowed,
            "{} reaches lettre without a satellite allowlist entry",
            manifest.name
        );
    }
    // The allowlist names a crate that actually exists and uses
    // at least one of its allowlisted leaves, so the rule cannot
    // rot into a vacuous pass.
    for (satellite, allowed) in SATELLITE_ALLOWLIST {
        let manifest = manifests
            .iter()
            .find(|m| m.name == *satellite)
            .unwrap_or_else(|| panic!("allowlisted satellite {satellite} missing"));
        let uses_allowance = allowed.iter().any(|leaf| {
            manifest.dependencies.contains(*leaf) || manifest.dev_dependencies.contains(*leaf)
        });
        assert!(
            uses_allowance,
            "allowlisted satellite {satellite} no longer uses any of its allowlisted leaves {allowed:?}"
        );
    }
}

#[test]
fn ai_satellite_actually_depends_on_its_allowlisted_leaves() {
    // Vacuous-pass guard: every allowance named in
    // SATELLITE_ALLOWLIST for `platform-ai` must be a real direct
    // dependency, so removing the rule does not silently leave the
    // satellite on the list with nothing to enforce.
    let ws = platform_arch_tests::workspace_root();
    let manifests = read_manifests(&ws);
    let ai = manifests
        .iter()
        .find(|m| m.name == "platform-ai")
        .expect("platform-ai manifest present");
    for leaf in ["reqwest", "tokio"] {
        assert!(
            ai.dependencies.contains(leaf) || ai.dev_dependencies.contains(leaf),
            "platform-ai must depend on {leaf} (allowlisted satellite leaf rot check)"
        );
    }
}

fn read_manifests(ws: &std::path::Path) -> Vec<ParsedCrate> {
    platform_arch_tests::read_workspace_manifests(ws)
        .into_values()
        .collect()
}

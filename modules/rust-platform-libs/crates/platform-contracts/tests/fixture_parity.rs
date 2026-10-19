//! Cross-repository fixture parity tests.
//!
//! These tests load Forge's canonical contract tree by default and retain
//! support for the legacy platform-contracts layout. The path is resolved in
//! this order:
//!
//! 1. `PLATFORM_CONTRACTS_DIR` environment variable (exact path).
//! 2. `contracts/` at the Forge repository root, derived from this nested
//!    Cargo workspace.
//!
//! Forge-layout parity validates its six supported fixture pairs and proves
//! four unsupported families remain explicitly unsupported. An override with
//! the legacy `schemas/registry.json` layout retains the nine-family manifest
//! walker. Missing evidence never passes silently.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use platform_contracts::validate::{validate_document, ValidationResult};
use platform_contracts::ContractRegistry;
use serde_json::Value;

/// Resolve the canonical platform contract root.
fn platform_contracts_dir() -> PathBuf {
    if let Ok(env_path) = env::var("PLATFORM_CONTRACTS_DIR") {
        return PathBuf::from(env_path);
    }
    // `CARGO_MANIFEST_DIR` for an integration test in
    // `crates/platform-contracts/tests/` is
    // `.../forge/modules/rust-platform-libs/crates/platform-contracts`.
    // Two levels up is the module root; two more reach Forge's root.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .expect("crate path is at least two levels deep");
    let forge_root = workspace
        .parent()
        .and_then(Path::parent)
        .expect("module path has a Forge repository parent");
    forge_root.join("contracts")
}

fn load_registry_or_fail(dir: &Path) -> ContractRegistry {
    let result = if dir.join("registry.json").is_file() && dir.join("family-support.json").is_file()
    {
        fs::read_to_string(dir.join("registry.json"))
            .map_err(platform_contracts::RegistryError::Io)
            .and_then(|raw| serde_json::from_str::<Value>(&raw).map_err(Into::into))
            .and_then(|value| ContractRegistry::from_value(dir.to_path_buf(), value))
    } else {
        ContractRegistry::load(dir)
    };
    result.unwrap_or_else(|err| {
        panic!(
            "BLOCKED: cannot load a supported contract registry from {}: {err}. \
             Set PLATFORM_CONTRACTS_DIR to Forge's contracts root or a legacy \
             platform-contracts root.",
            dir.display()
        )
    })
}

fn is_forge_layout(dir: &Path) -> bool {
    dir.join("registry.json").is_file() && dir.join("family-support.json").is_file()
}

fn assert_forge_family_unsupported(family: &str, dir: &Path, registry: &ContractRegistry) {
    assert!(
        registry.get(family).is_none(),
        "Forge registry unexpectedly owns {family}"
    );
    let support: Value = serde_json::from_slice(
        &fs::read(dir.join("family-support.json")).expect("Forge family-support.json exists"),
    )
    .expect("Forge family-support.json is valid JSON");
    let entry = support["families"]
        .as_array()
        .expect("family-support.json contains a families array")
        .iter()
        .find(|entry| entry["family"] == family)
        .unwrap_or_else(|| panic!("Forge family-support.json omits {family}"));
    assert_eq!(
        entry["status"], "unsupported",
        "Forge support status changed for {family}"
    );
    assert!(entry["reason"]
        .as_str()
        .is_some_and(|reason| !reason.is_empty()));
}

fn check_forge_fixture_pair(family: &str, dir: &Path, registry: &ContractRegistry) -> Vec<String> {
    let Some(entry) = registry.get(family) else {
        assert_forge_family_unsupported(family, dir, registry);
        return Vec::new();
    };
    let short = family.strip_prefix("platform.").unwrap_or(family);
    let fixture_dir = dir.join("fixtures").join(short);
    let mut failures = Vec::new();
    for (case, expected_valid) in [("valid", true), ("invalid", false)] {
        let path = fixture_dir.join(format!("{case}.json"));
        let payload: Value = match fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|error| format!("invalid JSON at {}: {error}", path.display()))
            }) {
            Ok(payload) => payload,
            Err(error) => {
                failures.push(error);
                continue;
            }
        };
        let document = serde_json::json!({
            "contract": format!("{family}/{}", entry.current_version),
            "generated_at": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        match validate_document(&document, registry) {
            Ok(result) if result.valid == expected_valid => {}
            Ok(result) => failures.push(format!(
                "{} fixture expected valid={expected_valid}, got valid={} ({:?})",
                path.display(),
                result.valid,
                result.errors
            )),
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }
    failures
}

#[derive(Debug)]
struct FixtureCase {
    file: String,
    #[allow(dead_code)]
    category: String,
    description: String,
    expected: String,
    expected_error_codes: Vec<String>,
    expected_warning_codes: Vec<String>,
}

fn load_manifest(family_dir: &Path) -> Vec<FixtureCase> {
    let manifest_path = family_dir.join("manifest.json");
    let raw = fs::read_to_string(&manifest_path).unwrap_or_else(|err| {
        panic!(
            "BLOCKED: missing manifest at {}: {err}",
            manifest_path.display()
        )
    });
    let value: Value = serde_json::from_str(&raw).unwrap_or_else(|err| {
        panic!(
            "manifest at {} is not valid JSON: {err}",
            manifest_path.display()
        )
    });
    let cases = value
        .get("fixtures")
        .and_then(Value::as_array)
        .expect("manifest must contain a 'fixtures' array");
    cases
        .iter()
        .map(|case| FixtureCase {
            file: case
                .get("file")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            category: case
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            description: case
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            expected: case
                .get("expected")
                .and_then(Value::as_str)
                .unwrap_or("valid")
                .to_string(),
            expected_error_codes: case
                .get("expected_error_codes")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            expected_warning_codes: case
                .get("expected_warning_codes")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn check_one(
    family: &str,
    family_dir: &Path,
    registry: &ContractRegistry,
    case: &FixtureCase,
) -> Result<(), String> {
    let path = family_dir.join(&case.file);
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("{} is not valid JSON: {e}", path.display()))?;
    let result: ValidationResult = validate_document(&value, registry)
        .map_err(|e| format!("validator failure on {}: {e}", path.display()))?;

    let ok = result.valid;
    let want_valid = case.expected == "valid";

    if ok != want_valid {
        return Err(format!(
            "{}/{}: expected={} got valid={} errors={:?} warnings={:?} description={}",
            family,
            case.file,
            case.expected,
            ok,
            result
                .errors
                .iter()
                .map(|e| (&e.code, &e.path))
                .collect::<Vec<_>>(),
            result
                .warnings
                .iter()
                .map(|w| (&w.code, &w.path))
                .collect::<Vec<_>>(),
            case.description,
        ));
    }

    for code in &case.expected_error_codes {
        if !result.errors.iter().any(|e| &e.code == code) {
            return Err(format!(
                "{}/{}: expected error code {code:?}, got {:?}",
                family,
                case.file,
                result
                    .errors
                    .iter()
                    .map(|e| e.code.clone())
                    .collect::<Vec<_>>(),
            ));
        }
    }
    for code in &case.expected_warning_codes {
        if !result.warnings.iter().any(|w| &w.code == code) {
            return Err(format!(
                "{}/{}: expected warning code {code:?}, got {:?}",
                family,
                case.file,
                result
                    .warnings
                    .iter()
                    .map(|w| w.code.clone())
                    .collect::<Vec<_>>(),
            ));
        }
    }
    Ok(())
}

fn walk_family(family: &str, dir: &Path, registry: &ContractRegistry) -> Vec<String> {
    if is_forge_layout(dir) {
        return check_forge_fixture_pair(&format!("platform.{family}"), dir, registry);
    }
    let family_dir = dir.join("fixtures").join(family);
    if !family_dir.is_dir() {
        return vec![format!(
            "BLOCKED: missing fixtures directory for {family} at {}",
            family_dir.display()
        )];
    }
    let cases = load_manifest(&family_dir);
    let mut failures = Vec::new();
    for case in &cases {
        if let Err(err) = check_one(family, &family_dir, registry, case) {
            failures.push(err);
        }
    }
    failures
}

#[test]
fn forge_owned_six_family_fixture_matrix_is_explicit() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    if !is_forge_layout(&dir) {
        assert!(
            dir.join("schemas").join("registry.json").is_file(),
            "selected contract tree is neither Forge nor legacy layout: {}",
            dir.display()
        );
        return;
    }
    for family in [
        "platform.audit-event",
        "platform.capability",
        "platform.gate-result",
        "platform.readiness",
        "platform.release-evidence",
    ] {
        let errors = check_forge_fixture_pair(family, &dir, &registry);
        assert!(
            errors.is_empty(),
            "{family} Forge parity failed:\n{}",
            errors.join("\n")
        );
    }

    let portfolio_schema_path = dir.join("schemas/public-portfolio-manifest.schema.json");
    let portfolio_schema: Value = serde_json::from_slice(
        &fs::read(&portfolio_schema_path).expect("Forge portfolio schema exists"),
    )
    .expect("Forge portfolio schema is valid JSON");
    let portfolio_validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&portfolio_schema)
        .expect("Forge portfolio schema compiles");
    let portfolio_fixtures = dir.join("fixtures/public-portfolio-manifest");
    for (case, expected_valid) in [("valid", true), ("invalid", false)] {
        let path = portfolio_fixtures.join(format!("{case}.json"));
        let payload: Value = serde_json::from_slice(
            &fs::read(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("invalid JSON at {}: {error}", path.display()));
        let is_valid = portfolio_validator.iter_errors(&payload).next().is_none();
        assert_eq!(is_valid, expected_valid, "Forge portfolio {case} fixture");
    }

    for family in [
        "platform.identity-subject",
        "platform.job-outcome",
        "platform.permission-decision",
        "platform.tenant-context",
    ] {
        assert_forge_family_unsupported(family, &dir, &registry);
    }
}

#[test]
fn registry_loads_from_live_repo() {
    let dir = platform_contracts_dir();
    assert!(
        is_forge_layout(&dir) || dir.join("schemas").join("registry.json").is_file(),
        "BLOCKED: {} is neither Forge's contracts root nor a legacy platform-contracts root",
        dir.display()
    );
    let _registry = load_registry_or_fail(&dir);
}

#[test]
fn identity_subject_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("identity-subject", &dir, &registry);
    assert!(
        failures.is_empty(),
        "identity-subject fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn permission_decision_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("permission-decision", &dir, &registry);
    assert!(
        failures.is_empty(),
        "permission-decision fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn tenant_context_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("tenant-context", &dir, &registry);
    assert!(
        failures.is_empty(),
        "tenant-context fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn audit_event_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("audit-event", &dir, &registry);
    assert!(
        failures.is_empty(),
        "audit-event fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn readiness_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("readiness", &dir, &registry);
    assert!(
        failures.is_empty(),
        "readiness fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn gate_result_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("gate-result", &dir, &registry);
    assert!(
        failures.is_empty(),
        "gate-result fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn job_outcome_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("job-outcome", &dir, &registry);
    assert!(
        failures.is_empty(),
        "job-outcome fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn release_evidence_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("release-evidence", &dir, &registry);
    assert!(
        failures.is_empty(),
        "release-evidence fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn capability_fixtures_match() {
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let failures = walk_family("capability", &dir, &registry);
    assert!(
        failures.is_empty(),
        "capability fixture mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn readme_documented_families_all_covered() {
    // Smoke test: every family in the registry that has a fixtures
    // directory must be reachable. This catches registry drift.
    let dir = platform_contracts_dir();
    let registry = load_registry_or_fail(&dir);
    let fixtures_root = dir.join("fixtures");
    for family in registry.contract_ids() {
        let short = family.strip_prefix("platform.").unwrap_or(family);
        let family_dir = fixtures_root.join(short);
        assert!(
            family_dir.is_dir(),
            "BLOCKED: contract family {family} is registered but {} is missing",
            family_dir.display()
        );
    }
}

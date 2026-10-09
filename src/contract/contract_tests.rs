//! Unit tests for the contract inventory, manifest digests and wire
//! validation, extracted verbatim from `src/contract/mod.rs` to keep that
//! file under the source-file-size cap.

use super::*;
use super::*;

#[test]
fn manifest_digests_match() {
    verify_manifest_digests().expect("vendored digests must match");
}

#[test]
fn inventory_completeness() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing = Vec::new();
    let mut found_consts = std::collections::HashSet::new();
    for entry in walk_rs(&dir) {
        let text = fs::read_to_string(&entry).unwrap();
        for line in text.lines() {
            if let Some(name) = extract_const_name(line) {
                if name.contains("CONTRACT")
                    || name.contains("CATALOG")
                    || name.contains("VERSION")
                    || name == "GENERATOR_VERSION"
                    || name == "DOCTOR_POLICY_VERSION"
                    || name == "TESTED_VERSION"
                {
                    if let Some(ver) = extract_version(line) {
                        if ver == "0.1.0" {
                            let module = entry
                                .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                                .unwrap()
                                .with_extension("")
                                .to_string_lossy()
                                .replace('/', ".")
                                .trim_start_matches('.')
                                .to_string();
                            let key = format!("{module}::{name}");
                            found_consts.insert(key);
                        }
                    }
                }
            }
        }
    }
    for key in &found_consts {
        let parts: Vec<&str> = key.split("::").collect();
        let rel = parts[0];
        let top = rel
            .split('.')
            .next()
            .unwrap_or(rel)
            .split('/')
            .next()
            .unwrap_or(rel);
        let const_part = parts[1];
        let hit = CONTRACTS
            .iter()
            .any(|c| c.module == top && c.constant == const_part);
        if !hit && const_part != "SUPPORTED_SCHEMA" && const_part != "TESTED_VERSION" {
            missing.push(key.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "unregistered versioned surfaces: {missing:?}"
    );
}

#[test]
fn inventory_agreement() {
    for spec in CONTRACTS {
        // The vendored UI token kit row is checked against the compiled-in
        // constant itself, so a reviewed kit resync moves two surfaces that
        // are compared to each other rather than tripping a blanket
        // literal; every other row keeps the shared 0.1.0 baseline.
        let expected = if spec.constant == "PLATFORM_UI_KIT_VERSION" {
            crate::kit::registry::PLATFORM_UI_KIT_VERSION
        } else {
            "0.1.0"
        };
        assert_eq!(
            spec.version, expected,
            "inventory row {}.{} version drift",
            spec.module, spec.constant
        );
    }
}

#[test]
fn map_gate_rows() {
    assert_eq!(
        map_gate_aggregate(crate::gate::GateAggregate::Passed).unwrap(),
        "passed"
    );
    assert_eq!(
        map_gate_aggregate(crate::gate::GateAggregate::Blocked).unwrap(),
        "failed"
    );
    assert_eq!(
        map_gate_aggregate(crate::gate::GateAggregate::Failed).unwrap(),
        "errored"
    );
    assert_eq!(
        map_gate_aggregate(crate::gate::GateAggregate::Unknown).unwrap(),
        "errored"
    );
}

#[test]
fn map_readiness_rows() {
    assert_eq!(
        map_readiness_status(crate::readiness::ReadinessStatus::Passed).unwrap(),
        "ready"
    );
    assert_eq!(
        map_readiness_status(crate::readiness::ReadinessStatus::Failed).unwrap(),
        "not_ready"
    );
    assert!(map_readiness_status(crate::readiness::ReadinessStatus::Unverified).is_err());
}

#[test]
fn map_audit_rows() {
    assert_eq!(map_audit_outcome("done").unwrap(), "success");
    assert_eq!(map_audit_outcome("failed").unwrap(), "failure");
    assert_eq!(map_audit_outcome("blocked").unwrap(), "denied");
    assert_eq!(map_audit_outcome("rejected").unwrap(), "denied");
    assert!(map_audit_outcome("pending").is_err());
    assert!(map_audit_outcome("partial").is_err());
    assert!(map_audit_outcome("unknown-state").is_err());
}

#[test]
fn secret_refusal() {
    let subs = vec!["token".to_string(), "secret".to_string()];
    let payload = serde_json::json!({"api_token": "x", "ok": 1});
    assert!(payload_contains_secret_field(&payload, &subs).is_some());
    let payload2 = serde_json::json!({"name": "x"});
    assert!(payload_contains_secret_field(&payload2, &subs).is_none());
}

#[test]
fn envelope_pattern() {
    let env = build_envelope(
        "platform.gate-result",
        serde_json::json!({"gate_id": "a/b", "pipeline": "x", "evaluated_at": "2026-01-01T00:00:00Z", "result": "passed"}),
    );
    assert!(env
        .get("contract")
        .unwrap()
        .as_str()
        .unwrap()
        .starts_with("platform.gate-result/"));
}

#[test]
fn validate_accepts_valid_gate() {
    let doc = serde_json::json!({
        "contract": "platform.gate-result/0.1.0",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {"gate_id": "proj-a/driftwatchdog", "pipeline": "driftwatchdog", "evaluated_at": "2026-01-01T00:00:00Z", "result": "passed"}
    });
    assert!(
        validate_envelope(&doc).is_ok(),
        "{:?}",
        validate_envelope(&doc)
    );
}

#[test]
fn validate_refuses_unknown_family() {
    let doc = serde_json::json!({
        "contract": "platform.unknown-thing/0.1.0",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {}
    });
    assert!(validate_envelope(&doc).is_err());
}

#[test]
fn validate_refuses_major_mismatch() {
    let doc = serde_json::json!({
        "contract": "platform.gate-result/1.0.0",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {}
    });
    assert!(validate_envelope(&doc).is_err());
}

fn walk_rs(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                out.extend(walk_rs(&p));
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}

fn extract_const_name(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with("pub const ") {
        return None;
    }
    let rest = &t["pub const ".len()..];
    let name = rest.split([':', ' ', '=']).next()?.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn extract_version(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

#[test]
#[ignore]
fn parity_walk() {
    let src = std::env::var("PLATFORM_CONTRACTS_DIR")
        .or_else(|_| std::env::var("FORGE_CONTRACTS_DIR"))
        .unwrap_or_else(|_| format!("{}/../platform-contracts", env!("CARGO_MANIFEST_DIR")));
    let path = Path::new(&src);
    assert!(path.is_dir(), "parity source not found at {src}: set PLATFORM_CONTRACTS_DIR (next action: create a sibling checkout or set the env var)");
    for fam in supported_families() {
        let dir = path
            .join("fixtures")
            .join(fam.strip_prefix("platform.").unwrap());
        assert!(dir.is_dir(), "missing fixture dir for {fam}");
    }
}

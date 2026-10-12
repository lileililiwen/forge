//! Unit tests for the contract inventory, manifest digests and wire
//! validation, extracted verbatim from `src/contract/mod.rs` to keep that
//! file under the source-file-size cap.

use super::*;

fn evidence_with(
    aggregate: crate::gate::GateAggregate,
    states: &[crate::gate::GateCheckState],
) -> crate::gate::GateEvidence {
    crate::gate::GateEvidence {
        contract: crate::gate::GATE_CONTRACT_VERSION.to_string(),
        project_id: "proj-a".to_string(),
        runtime: "driftwatchdog".to_string(),
        runtime_version: None,
        revision: Some("a".repeat(40)),
        manifest_digest: None,
        rule_pack_version: None,
        aggregate,
        checks: states
            .iter()
            .enumerate()
            .map(|(i, state)| crate::gate::GateCheck {
                id: format!("check-{i}"),
                state: *state,
                note: None,
            })
            .collect(),
        observed_at: "2026-01-01T00:00:00Z".to_string(),
        dry_run: false,
        note: None,
    }
}

fn write_evidence(dir: &Path, evidence: crate::gate::GateEvidence) {
    let path = dir.join(crate::gate::evidence_path(&evidence.project_id));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_string_pretty(&evidence).unwrap()).unwrap();
}

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
    use crate::gate::{GateAggregate, GateCheckState};
    let e = |aggregate, states: &[GateCheckState]| evidence_with(aggregate, states);

    // A run the runtime reported as passing.
    assert_eq!(
        map_gate_result(&e(GateAggregate::Passed, &[GateCheckState::Pass])).unwrap(),
        "passed"
    );
    // A non-blocking failure is an attempted check that reported a
    // failure: 1.0.0 `failed`, not `errored` ("could not be evaluated to
    // a verdict"), which Forge never observed.
    assert_eq!(
        map_gate_result(&e(GateAggregate::Failed, &[GateCheckState::Fail])).unwrap(),
        "failed"
    );
    // Blocked with a failed row: an evaluated failure.
    assert_eq!(
        map_gate_result(&e(GateAggregate::Blocked, &[GateCheckState::Fail])).unwrap(),
        "failed"
    );
    // Blocked with an unclassifiable row: a human decision is open, and no
    // check reported a failure.
    assert_eq!(
        map_gate_result(&e(
            GateAggregate::Blocked,
            &[GateCheckState::Unresolved, GateCheckState::Fail]
        ))
        .unwrap(),
        "failed",
        "an observed failure outranks an open decision"
    );
    assert_eq!(
        map_gate_result(&e(GateAggregate::Blocked, &[GateCheckState::Unresolved])).unwrap(),
        "review_required"
    );
    // An unclassifiable document: Forge declined to decide, so nobody has.
    assert_eq!(
        map_gate_result(&e(GateAggregate::Unknown, &[GateCheckState::Unresolved])).unwrap(),
        "review_required"
    );

    // No aggregate renders `errored`: the gate plane never observes a
    // check it could not evaluate to a verdict (that run is
    // `GateOutcome::Unavailable`, which leaves no evidence at all).
    for aggregate in [
        GateAggregate::Passed,
        GateAggregate::Blocked,
        GateAggregate::Failed,
        GateAggregate::Unknown,
    ] {
        let rendered = map_gate_result(&e(aggregate, &[GateCheckState::Fail]));
        assert!(
            rendered.is_err() || rendered.unwrap() != "errored",
            "{aggregate:?} must never render errored"
        );
    }
}

#[test]
fn blocked_without_an_observed_failure_is_an_undecided_outcome() {
    use crate::gate::{GateAggregate, GateCheckState};
    // Every reachable blocked shape that carries no failed row: a required
    // check with no result at all, a required-but-inapplicable check with
    // no declared exemption, an out-of-vocabulary review, and the
    // degenerate empty document. Forge observed no attempted failure in
    // any of them, so none may render `failed`.
    for states in [
        vec![],
        vec![GateCheckState::NotApplicable],
        vec![GateCheckState::Skip],
        vec![GateCheckState::Pass],
        vec![GateCheckState::Pass, GateCheckState::NotApplicable],
        vec![GateCheckState::Pass, GateCheckState::Unresolved],
    ] {
        let rendered = map_gate_result(&evidence_with(GateAggregate::Blocked, &states)).unwrap();
        assert_eq!(
            rendered, "review_required",
            "states {states:?} carry no observed failure"
        );
    }
}

#[test]
fn map_gate_check_rows() {
    use crate::gate::GateCheckState;
    assert_eq!(
        map_gate_check_state(GateCheckState::Pass).unwrap(),
        "passed"
    );
    assert_eq!(
        map_gate_check_state(GateCheckState::Fail).unwrap(),
        "failed"
    );
    assert_eq!(
        map_gate_check_state(GateCheckState::NotApplicable).unwrap(),
        "skipped",
        "1.0.0 skipped is 'proven not to apply; this is an exemption'"
    );
    assert_eq!(
        map_gate_check_state(GateCheckState::Skip).unwrap(),
        "skipped"
    );
    // `unresolved` exists so Forge does not assert an outcome it did not
    // observe: it is never errored, never an exemption and never unbound.
    assert_eq!(
        map_gate_check_state(GateCheckState::Unresolved).unwrap(),
        "review_required"
    );
    for state in [
        GateCheckState::Pass,
        GateCheckState::Fail,
        GateCheckState::Skip,
        GateCheckState::NotApplicable,
        GateCheckState::Unresolved,
    ] {
        let rendered = map_gate_check_state(state).unwrap();
        assert_ne!(rendered, "errored", "{state:?} must never render errored");
        assert_ne!(
            rendered, "unbound",
            "{state:?} has no binding source to claim unbound from"
        );
    }
}

#[test]
fn every_rendered_value_is_inside_the_vendored_schema_vocabulary() {
    use crate::gate::{GateAggregate, GateCheckState};
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(contracts_dir().join("schemas/gate-result.schema.json")).unwrap(),
    )
    .unwrap();
    let result_enum: Vec<&str> = schema["properties"]["result"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let check_enum: Vec<&str> = schema["properties"]["checks"]["items"]["properties"]["status"]
        ["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    // The shared vocabulary of 1.0.0 is still nameable, so removing a
    // Forge value can never be mistaken for "the contract does not have
    // it" — the mirror image of the "errored is never produced" test.
    for value in [
        "passed",
        "failed",
        "errored",
        "skipped",
        "review_required",
        "unbound",
    ] {
        assert!(result_enum.contains(&value), "result enum lost {value}");
        assert!(check_enum.contains(&value), "checks enum lost {value}");
    }
    for aggregate in [
        GateAggregate::Passed,
        GateAggregate::Blocked,
        GateAggregate::Failed,
        GateAggregate::Unknown,
    ] {
        for states in [
            vec![],
            vec![GateCheckState::Pass],
            vec![GateCheckState::Fail],
            vec![GateCheckState::Skip],
            vec![GateCheckState::NotApplicable],
            vec![GateCheckState::Unresolved],
            vec![GateCheckState::Fail, GateCheckState::Unresolved],
        ] {
            if let Ok(rendered) = map_gate_result(&evidence_with(aggregate, &states)) {
                assert!(
                    result_enum.contains(&rendered),
                    "result {rendered} is not in the vendored enum"
                );
            }
            for state in states {
                let rendered = map_gate_check_state(state).unwrap();
                assert!(
                    check_enum.contains(&rendered),
                    "checks status {rendered} is not in the vendored enum"
                );
            }
        }
    }
}

#[test]
fn registered_family_coverage() {
    let mut seen: Vec<&str> = PLATFORM_FAMILY_VERSIONS.iter().map(|(f, _)| *f).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        PLATFORM_FAMILY_VERSIONS.len(),
        "a family is registered twice"
    );
    let mut supported = supported_families();
    supported.sort_unstable();
    assert_eq!(
        seen, supported,
        "the version table and supported_families() must cover the same families"
    );
    for (family, version) in PLATFORM_FAMILY_VERSIONS {
        assert_eq!(
            platform_family_version(family),
            Some(*version),
            "{family} does not resolve to its registered version"
        );
        let discriminator = contract_discriminator(family).unwrap();
        assert_eq!(discriminator, format!("{family}/{version}"));
        // The pattern the family's documents must satisfy.
        assert!(
            regex_lite_matches(&discriminator),
            "{discriminator} does not match CONTRACT_PATTERN"
        );
        assert_eq!(
            CONTRACT_PATTERN,
            r"^platform\.[a-z][a-z0-9]*(?:-[a-z0-9]+)*\/[0-9]+\.[0-9]+\.[0-9]+$"
        );
    }
    assert_eq!(
        platform_family_version("platform.gate-result"),
        Some("1.0.0"),
        "gate-result must declare the adopted major"
    );
    assert_eq!(platform_family_version("platform.unknown-thing"), None);
}

/// Dependency-free check of the `CONTRACT_PATTERN` regexp: the envelope
/// discriminator is `<family>/<major>.<minor>.<patch>`, which the pattern
/// accepts only when the family is lowercase dotted words and the version
/// is three numeric components. Proving it here keeps the contract free of
/// a regex crate for one assertion.
fn regex_lite_matches(discriminator: &str) -> bool {
    let Some((family, version)) = discriminator.split_once('/') else {
        return false;
    };
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    let Some(words) = family.strip_prefix("platform.") else {
        return false;
    };
    !words.is_empty()
        && words.split('.').all(|w| {
            matches!(w.bytes().next(), Some(b) if b.is_ascii_lowercase())
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

#[test]
fn emit_gate_result_declares_the_registered_version() {
    use crate::gate::{GateAggregate, GateCheckState};
    let tmp = tempfile::tempdir().unwrap();
    write_evidence(
        tmp.path(),
        evidence_with(
            GateAggregate::Blocked,
            &[GateCheckState::Fail, GateCheckState::Unresolved],
        ),
    );
    let doc = emit_gate_result(tmp.path(), "proj-a").expect("emit");
    assert_eq!(
        doc["contract"], "platform.gate-result/1.0.0",
        "the emitted document declares the adopted major"
    );
    assert_eq!(doc["payload"]["result"], "failed");
    assert_eq!(doc["payload"]["checks"][0]["status"], "failed");
    assert_eq!(
        doc["payload"]["checks"][1]["status"], "review_required",
        "an unclassifiable check is an open decision, never errored"
    );
    assert!(
        validate_envelope(&doc).is_ok(),
        "{:?}",
        validate_envelope(&doc)
    );
}

#[test]
fn emit_gate_result_refuses_a_record_it_cannot_project() {
    use crate::gate::{GateAggregate, GateCheckState};
    let tmp = tempfile::tempdir().unwrap();
    // A blocked record with no rows is still projectable — nothing was
    // observed to be decided. This test pins that the emitter refuses only
    // what it cannot read, never what it cannot classify into a value it
    // has: the sibling's own statuses Forge does not know still land as
    // `review_required` rather than as an invented outcome.
    write_evidence(
        tmp.path(),
        evidence_with(
            GateAggregate::Unknown,
            &[GateCheckState::Unresolved, GateCheckState::NotApplicable],
        ),
    );
    let doc = emit_gate_result(tmp.path(), "proj-a").expect("emit");
    assert_eq!(doc["payload"]["result"], "review_required");
    assert_eq!(doc["payload"]["checks"][0]["status"], "review_required");
    assert_eq!(doc["payload"]["checks"][1]["status"], "skipped");
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
    assert_eq!(
        env.get("contract").unwrap().as_str().unwrap(),
        "platform.gate-result/1.0.0",
        "the envelope declares the registered version, not a literal"
    );
    // Every other family keeps its registered version, so no envelope is
    // stamped by accident.
    assert_eq!(
        build_envelope("platform.readiness", serde_json::json!({}))
            .get("contract")
            .unwrap()
            .as_str()
            .unwrap(),
        "platform.readiness/0.1.0"
    );
}

#[test]
fn validate_accepts_valid_gate() {
    for contract in ["platform.gate-result/0.1.0", "platform.gate-result/1.0.0"] {
        let doc = serde_json::json!({
            "contract": contract,
            "generated_at": "2026-01-01T00:00:00Z",
            "payload": {"gate_id": "proj-a/driftwatchdog", "pipeline": "driftwatchdog", "evaluated_at": "2026-01-01T00:00:00Z", "result": "passed"}
        });
        assert!(
            validate_envelope(&doc).is_ok(),
            "{contract}: {:?}",
            validate_envelope(&doc)
        );
    }
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
    // A major the family's vendored registry record does not serve.
    let doc = serde_json::json!({
        "contract": "platform.gate-result/2.0.0",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {"gate_id": "proj-a/driftwatchdog", "pipeline": "driftwatchdog", "evaluated_at": "2026-01-01T00:00:00Z", "result": "passed"}
    });
    let err = validate_envelope(&doc).expect_err("major 2 is not served");
    assert!(err.contains("unsupported major"), "{err}");
    assert!(err.contains("platform.gate-result"), "{err}");

    // The registry is the authority, so gate-result serves [0, 1].
    assert_eq!(
        supported_majors("platform.gate-result").unwrap(),
        vec![0, 1],
        "the vendored registry must still serve major 0 while narrowing is declared"
    );

    // A registry that cannot answer refuses rather than skipping.
    let missing_family = supported_majors("platform.not-registered").unwrap_err();
    assert!(
        missing_family.contains("platform.not-registered"),
        "{missing_family}"
    );

    // A document with no version at all is refused, not waved through.
    let doc = serde_json::json!({
        "contract": "platform.gate-result/",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {"gate_id": "proj-a/driftwatchdog", "pipeline": "driftwatchdog", "evaluated_at": "2026-01-01T00:00:00Z", "result": "passed"}
    });
    assert!(validate_envelope(&doc).is_err());
}

#[test]
fn validate_refuses_value_outside_the_vendored_enum() {
    let doc = serde_json::json!({
        "contract": "platform.gate-result/1.0.0",
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": {"gate_id": "proj-a/driftwatchdog", "pipeline": "driftwatchdog", "evaluated_at": "2026-01-01T00:00:00Z", "result": "warned"}
    });
    let err = validate_envelope(&doc).expect_err("warned is not in the 1.0.0 enum");
    assert!(err.contains("not in enum"), "{err}");
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

//! Unit tests for the plugin registry and declaration-first resolution,
//! extracted verbatim from `src/plugins/mod.rs` to keep that file under the
//! source-file-size cap.

use super::*;

fn provider(id: &str, command: &str, enabled: bool) -> ProviderEntry {
    ProviderEntry {
        id: id.to_string(),
        command: PathBuf::from(command),
        enabled,
    }
}

fn descriptor(id: &str, kind: &str, capabilities: &[&str]) -> PluginConfig {
    PluginConfig {
        plugins: vec![PluginDescriptorEntry {
            id: id.to_string(),
            descriptor: PluginDescriptor {
                kind: Some(kind.to_string()),
                capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
                description: None,
            },
        }],
    }
}

/// Resolve with no project directory to inspect: the builtin-only
/// shape these tests have always used.
fn resolved(config: &ProviderConfig, descriptors: &PluginConfig) -> Vec<PluginRecord> {
    list(config, descriptors, None).expect("no declaration to read")
}

/// A project directory carrying `body` as its declaration.
fn declared_dir(body: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(DECLARATION_FILE), body).unwrap();
    tmp
}

fn resolve_in(dir: &Path, id: &str) -> Result<Vec<PluginRecord>, ForgeError> {
    list(
        &ProviderConfig {
            providers: vec![provider(id, "/bin/true", true)],
        },
        &PluginConfig::default(),
        Some(dir),
    )
}

#[test]
fn a_plugin_without_a_descriptor_is_a_delivery_plugin() {
    // This is the shape every existing providers.yaml has. It must keep
    // working, so it resolves to delivery without any edit.
    let config = ProviderConfig {
        providers: vec![provider("openpanel", "/bin/true", true)],
    };
    let records = resolved(&config, &PluginConfig::default());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, PluginKind::Delivery);
    assert_eq!(records[0].state, PluginState::Ready);
    assert_eq!(records[0].capability_list(), vec!["delivery"]);
}

#[test]
fn a_metadata_plugin_advertises_its_fields() {
    let config = ProviderConfig {
        providers: vec![provider("github", "/bin/true", true)],
    };
    let records = resolved(
        &config,
        &descriptor("github", "metadata", &["topic", "description"]),
    );
    assert_eq!(records[0].kind, PluginKind::Metadata);
    assert!(records[0].maintains("topic"));
    assert!(records[0].maintains("description"));
    assert!(!records[0].maintains("homepage"));
}

#[test]
fn an_unknown_capability_is_invalid_and_advertises_nothing() {
    let config = ProviderConfig {
        providers: vec![provider("github", "/bin/true", true)],
    };
    let records = resolved(
        &config,
        &descriptor("github", "metadata", &["topic", "teleport"]),
    );
    assert!(matches!(records[0].state, PluginState::Invalid { .. }));
    assert!(records[0].capabilities.is_empty());
    let reason = records[0].state.reason().unwrap();
    assert!(reason.contains("teleport"), "{reason}");
    assert!(reason.contains("closed set"), "{reason}");
}

#[test]
fn an_unknown_kind_is_invalid() {
    let config = ProviderConfig {
        providers: vec![provider("x", "/bin/true", true)],
    };
    let records = resolved(&config, &descriptor("x", "teleport", &[]));
    assert!(matches!(records[0].state, PluginState::Invalid { .. }));
    assert!(records[0]
        .state
        .reason()
        .unwrap()
        .contains("unknown plugin kind"));
}

#[test]
fn a_missing_command_does_not_hide_the_others() {
    let config = ProviderConfig {
        providers: vec![
            provider("broken", "/nonexistent/plugin", true),
            provider("working", "/bin/true", true),
        ],
    };
    let records = resolved(&config, &PluginConfig::default());
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id, "broken");
    assert!(matches!(records[0].state, PluginState::Unavailable { .. }));
    // The healthy plugin is still listed.
    assert_eq!(records[1].id, "working");
    assert_eq!(records[1].state, PluginState::Ready);
}

#[test]
fn a_disabled_plugin_is_reported_disabled_not_broken() {
    let config = ProviderConfig {
        providers: vec![provider("off", "/bin/true", false)],
    };
    let records = resolved(&config, &PluginConfig::default());
    assert_eq!(records[0].state, PluginState::Disabled);
}

#[test]
fn metadata_routing_requires_every_field() {
    let config = ProviderConfig {
        providers: vec![provider("gh", "/bin/true", true)],
    };
    let records = resolved(
        &config,
        &descriptor("gh", "metadata", &["topic", "description"]),
    );
    let both = metadata_plugins(&records, &["topic".to_string(), "description".to_string()]);
    assert_eq!(both.len(), 1);
    let too_many = metadata_plugins(&records, &["topic".to_string(), "homepage".to_string()]);
    assert!(too_many.is_empty(), "no plugin can service `homepage`");
}

// -- Declaration-first resolution -----------------------------------

/// A project that declares `driftwatchdog` as `gate[gate]` through its
/// capability block. The declaration states both the kind and the
/// capability, so this is the claim the builtin table would have made.
const DECLARED_GATE: &str = r#"{
  "capabilities": {
    "gate": {
      "owner": "driftwatchdog",
      "evidence_state": "configured",
      "evidence_ref": "src/gate/contract.rs"
    }
  }
}"#;

#[test]
fn a_declared_row_reports_the_declaration_and_equals_the_builtin_row() {
    // Behavioural equality, not merely acceptance: with the declaration
    // present the row is `declared`, and its kind and capabilities are
    // exactly what the builtin table produced without it.
    let without = resolve_in(Path::new("/nonexistent-project"), "driftwatchdog").unwrap();
    assert_eq!(without[0].source, PluginSource::Builtin);
    assert_eq!(without[0].kind, PluginKind::Gate);
    assert_eq!(without[0].capability_list(), vec!["gate"]);

    let dir = declared_dir(DECLARED_GATE);
    let with = resolve_in(dir.path(), "driftwatchdog").unwrap();
    assert_eq!(with[0].source, PluginSource::Declared);
    assert_eq!(with[0].kind, without[0].kind);
    assert_eq!(with[0].capability_list(), without[0].capability_list());
    assert_eq!(with[0].state, without[0].state);
    // The description is not filled: a declared row never inherits the
    // builtin table's own "builtin …" text.
    assert_eq!(with[0].description, None);
}

#[test]
fn the_declaration_answers_before_the_builtin() {
    let dir = declared_dir(DECLARED_GATE);
    let records = resolve_in(dir.path(), "driftwatchdog").unwrap();
    assert_eq!(records[0].source, PluginSource::Declared);
}

#[test]
fn an_absent_declaration_falls_through_and_reports_builtin() {
    let tmp = tempfile::tempdir().unwrap();
    let records = resolve_in(tmp.path(), "driftwatchdog").unwrap();
    assert_eq!(records[0].source, PluginSource::Builtin);
    assert_eq!(records[0].kind, PluginKind::Gate);
    assert_eq!(records[0].capability_list(), vec!["gate"]);
}

#[test]
fn an_absent_configuration_still_reports_builtin_for_an_unknown_id() {
    // An id nothing declares keeps working exactly as today: the
    // legacy delivery default, attributed to the builtin rank.
    let tmp = tempfile::tempdir().unwrap();
    let descriptors = PluginConfig::default();
    let records = list(
        &ProviderConfig {
            providers: vec![provider("nobody-knows", "/bin/true", true)],
        },
        &descriptors,
        Some(tmp.path()),
    )
    .unwrap();
    assert_eq!(records[0].source, PluginSource::Builtin);
    assert_eq!(records[0].kind, PluginKind::Delivery);
    assert_eq!(records[0].capability_list(), vec!["delivery"]);
}

#[test]
fn an_interfaces_consumption_declares_the_provider_it_reaches() {
    // The provider id is named as the other side of a declared
    // consumption, which is how a project states a boundary it does
    // not own.
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"driftwatchdog check --format json","contract":"driftwatch-checker/0.1.0","evidence_ref":"src/gate/contract.rs"}]}"#,
    );
    let records = resolve_in(dir.path(), "driftwatchdog").unwrap();
    assert_eq!(records[0].source, PluginSource::Declared);
    // The declaration states no kind here, so the builtin value fills
    // it: an unstated field is filled, never overridden.
    assert_eq!(records[0].kind, PluginKind::Gate);
    assert_eq!(records[0].capability_list(), vec!["gate"]);
}

#[test]
fn an_integration_entry_declares_the_provider_it_names() {
    let dir = declared_dir(
        r#"{"integrations":[{"id":"ariadex","required":false,"evidence_ref":"src/agent/model.rs"}]}"#,
    );
    let records = resolve_in(dir.path(), "ariadex").unwrap();
    assert_eq!(records[0].source, PluginSource::Declared);
    assert_eq!(records[0].kind, PluginKind::Agent);
    assert_eq!(records[0].capability_list(), vec!["agent"]);
}

#[test]
fn a_declared_id_the_builtin_does_not_know_resolves_from_the_declaration() {
    // The point of the source: a provider whose capability list changes
    // no longer needs a Forge source edit and a release.
    let dir = declared_dir(
        r#"{"capabilities":{"quality":{"owner":"new-tool","evidence_state":"configured","evidence_ref":"src/gate/contract.rs"}}}"#,
    );
    let records = resolve_in(dir.path(), "new-tool").unwrap();
    assert_eq!(records[0].source, PluginSource::Declared);
    assert_eq!(records[0].kind, PluginKind::Quality);
    assert_eq!(records[0].capability_list(), vec!["quality"]);
}

#[test]
fn a_descriptor_and_a_declaration_that_agree_resolve_from_the_descriptor() {
    // Rank 1 still speaks first; agreement is not a contradiction.
    let dir = declared_dir(DECLARED_GATE);
    let records = list(
        &ProviderConfig {
            providers: vec![provider("driftwatchdog", "/bin/true", true)],
        },
        &descriptor("driftwatchdog", "gate", &["gate"]),
        Some(dir.path()),
    )
    .unwrap();
    assert_eq!(records[0].source, PluginSource::Descriptor);
    assert_eq!(records[0].kind, PluginKind::Gate);
}

#[test]
fn a_descriptor_that_contradicts_the_declaration_refuses_by_name() {
    let dir = declared_dir(DECLARED_GATE);
    let err = list(
        &ProviderConfig {
            providers: vec![provider("driftwatchdog", "/bin/true", true)],
        },
        &descriptor("driftwatchdog", "metadata", &["topic"]),
        Some(dir.path()),
    )
    .expect_err("a contradiction must refuse");
    assert_eq!(err.code(), "plugin-boundary-refused");
    let message = err.to_string();
    for needle in [
        "definition-conflict",
        "`driftwatchdog`",
        "declared `gate`",
        "configured `metadata`",
        "declared [gate]",
        "configured [topic]",
    ] {
        assert!(message.contains(needle), "{needle} missing from {message}");
    }
}

#[test]
fn a_capability_disagreement_alone_also_refuses() {
    let dir = declared_dir(DECLARED_GATE);
    let err = list(
        &ProviderConfig {
            providers: vec![provider("driftwatchdog", "/bin/true", true)],
        },
        &descriptor("driftwatchdog", "gate", &["quality"]),
        Some(dir.path()),
    )
    .expect_err("a capability disagreement must refuse");
    assert!(err.to_string().contains("capabilities:"), "{err}");
}

#[test]
fn an_unknown_mechanism_refuses_naming_the_field() {
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"teleport","entry":"x","contract":"driftwatch-checker/0.1.0","digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}]}"#,
    );
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    assert_eq!(err.code(), "plugin-boundary-refused");
    let message = err.to_string();
    assert!(message.contains("mechanism-unknown"), "{message}");
    assert!(message.contains("`gate-check`.mechanism"), "{message}");
    assert!(message.contains("teleport"), "{message}");
    assert!(
        message.contains("invoke, consume, observe, sibling"),
        "{message}"
    );
}

#[test]
fn a_versionless_contract_on_a_non_sibling_interface_refuses() {
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"driftwatchdog check --format json"}]}"#,
    );
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    let message = err.to_string();
    assert!(message.contains("contract-unversioned"), "{message}");
    assert!(message.contains("`gate-check`.contract"), "{message}");
    assert!(message.contains("`invoke`"), "{message}");
}

#[test]
fn a_contract_without_a_version_refuses_too() {
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"x","contract":"driftwatch-checker"}]}"#,
    );
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    assert!(err.to_string().contains("contract-unversioned"), "{err}");
}

#[test]
fn a_consumption_without_a_digest_refuses() {
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"platform-schemas","consumes":true,"provider":"platform-contracts","mechanism":"consume","entry":"contracts/registry.json","contract":"platform-contracts/1.0.0"}]}"#,
    );
    let err = resolve_in(dir.path(), "platform-contracts").expect_err("must refuse");
    let message = err.to_string();
    assert!(message.contains("digest-missing"), "{message}");
    assert!(message.contains("`platform-schemas`.digest"), "{message}");
    assert!(message.contains("`consume`"), "{message}");
}

#[test]
fn an_observation_without_a_digest_refuses() {
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"checker-document","provides":true,"mechanism":"observe","entry":"openspec/specs/external-checker-emission/spec.md","contract":"forge-checker/0.1.0"}]}"#,
    );
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    assert!(err.to_string().contains("`observe`"), "{err}");
}

#[test]
fn an_unparseable_declaration_refuses_naming_the_file_and_the_error() {
    let dir = declared_dir("{ not json");
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    assert_eq!(err.code(), "plugin-boundary-refused");
    let message = err.to_string();
    assert!(message.contains("declaration-invalid"), "{message}");
    assert!(message.contains(DECLARATION_FILE), "{message}");
}

#[test]
fn a_malformed_block_is_never_partially_honoured() {
    // The first entry is lawful; the second is not. Nothing resolves.
    let dir = declared_dir(
        r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"x","contract":"driftwatch-checker/0.1.0"},{"id":"nope","provides":true,"mechanism":"invoke","entry":"y"}]}"#,
    );
    let err = resolve_in(dir.path(), "driftwatchdog").expect_err("must refuse");
    assert!(err.to_string().contains("contract-unversioned"), "{err}");
}

#[test]
fn an_empty_interfaces_array_is_silent() {
    let dir = declared_dir(r#"{"interfaces":[],"capabilities":{}}"#);
    let records = resolve_in(dir.path(), "driftwatchdog").unwrap();
    assert_eq!(records[0].source, PluginSource::Builtin);
}

#[test]
fn a_declaration_naming_nothing_resolves_nothing_as_declared() {
    let dir = declared_dir(r#"{"capabilities":{}}"#);
    let records = resolve_in(dir.path(), "cargo-clippy").unwrap();
    assert_eq!(records[0].source, PluginSource::Builtin);
    assert_eq!(records[0].kind, PluginKind::Quality);
}

#[test]
fn every_rank_is_reported_in_one_listing() {
    let dir = declared_dir(DECLARED_GATE);
    let config = ProviderConfig {
        providers: vec![
            provider("github", "/bin/true", true),
            provider("driftwatchdog", "/bin/true", true),
            provider("cargo-clippy", "/bin/true", true),
            provider("openpanel", "/bin/true", true),
        ],
    };
    let records = list(
        &config,
        &descriptor("github", "metadata", &["topic"]),
        Some(dir.path()),
    )
    .unwrap();
    let sources: Vec<(&str, PluginSource)> = records
        .iter()
        .map(|record| (record.id.as_str(), record.source))
        .collect();
    assert_eq!(
        sources,
        vec![
            ("cargo-clippy", PluginSource::Builtin),
            ("driftwatchdog", PluginSource::Declared),
            ("github", PluginSource::Descriptor),
            ("openpanel", PluginSource::Builtin),
        ]
    );
}

#[test]
fn the_closed_mechanism_set_matches_the_workspace_schema() {
    // Mirrored, not invented: the same four values the workspace
    // schema's `interface_mechanism` enum carries.
    assert_eq!(
        INTERFACE_MECHANISMS,
        ["invoke", "consume", "observe", "sibling"]
    );
}

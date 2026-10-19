//! Declaration-first plugin resolution contract (`forge-plugins/0.1.0`).
//!
//! The workspace already owns a language for the boundaries between its
//! projects (`workspace-governance` `schemas/project.schema.json`). This
//! suite pins how Forge *reads* it and what it refuses:
//!
//! - a project's declaration answers for an id its own file names, and the
//!   row says so (`source: declared`) with the **same kind and
//!   capabilities** the builtin table would have produced — behavioural
//!   equality, not merely acceptance;
//! - the operator's descriptor still speaks first, and refuses by name
//!   when it *contradicts* the declaration instead of choosing silently;
//! - a declaration that cannot be read is refused field by field, never
//!   half-read and never treated as an absent one;
//! - an id nothing declares falls back and reports `builtin`, exactly as
//!   it did before this source existed;
//! - Forge's own `.project.json` declares its boundaries with contract
//!   versions derived from the versioned-surface inventory rather than
//!   restated, and accounts for every retained copy.
//!
//! The declaration is read from the project directory being inspected and
//! nowhere else: every case here runs with no sibling checkout present.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

use forge::contract::CONTRACTS;
use forge::plugins::{PluginSource, INTERFACE_MECHANISMS};

const CONTRACT: &str = "forge-plugins/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(dir);
    cmd.output().expect("run forge")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn run_json(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    assert!(out.status.success(), "{:?} failed: {}", args, stderr(&out));
    serde_json::from_slice(&out.stdout).expect("plugins json")
}

/// A workspace whose `.forge/providers.yaml` carries `providers`, whose
/// declaration is `declaration` (absent when `None`), and whose kits
/// manifest is `kits` (absent when `None`).
fn workspace(
    tmp: &Path,
    providers: &str,
    declaration: Option<&str>,
    kits: Option<&str>,
) -> PathBuf {
    let dir = tmp.to_path_buf();
    fs::create_dir_all(dir.join(".forge")).unwrap();
    fs::write(dir.join(".forge/providers.yaml"), providers).unwrap();
    if let Some(declaration) = declaration {
        fs::write(dir.join(".project.json"), declaration).unwrap();
    }
    if let Some(kits) = kits {
        fs::create_dir_all(dir.join("kits")).unwrap();
        fs::write(dir.join("kits/manifest.json"), kits).unwrap();
    }
    dir
}

fn plugin<'a>(page: &'a Value, id: &str) -> &'a Value {
    page["plugins"]
        .as_array()
        .expect("plugins array")
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("no plugin `{id}` in {page}"))
}

fn plugin_json(dir: &Path) -> Value {
    run_json(dir, &["plugins", "list", ".", "--format", "json"])
}

const DRIFTWATCHDOG_PROVIDER: &str =
    "providers:\n  - id: driftwatchdog\n    command: /bin/true\n    enabled: true\n";

/// A declaration that answers for `driftwatchdog` as `gate[gate]`, the
/// same claim the builtin table makes.
const DECLARED_GATE: &str = r#"{
  "capabilities": {
    "gate": {
      "owner": "driftwatchdog",
      "evidence_state": "configured",
      "evidence_ref": "src/gate/mod.rs"
    }
  }
}"#;

// -- the declaration answers, and the row says so ------------------------

#[test]
fn a_declaration_answers_before_the_builtin_and_reports_declared() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(DECLARED_GATE),
        None,
    );

    let page = plugin_json(&dir);
    assert_eq!(page["contract"], CONTRACT);
    let entry = plugin(&page, "driftwatchdog");
    assert_eq!(entry["source"], "declared");
    assert_eq!(entry["kind"], "gate");
    assert_eq!(entry["capabilities"], serde_json::json!(["gate"]));
    assert_eq!(entry["state"], "ready");

    // And the human render names the provenance, so a reviewer can tell a
    // declaration from a compiled-in guess.
    let text = stdout(&run(&dir, &["plugins", "list", "."]));
    assert!(text.contains("driftwatchdog  gate  ready"), "{text}");
    assert!(text.contains("source: declared"), "{text}");
}

#[test]
fn with_no_declaration_the_row_reports_builtin_and_is_behaviourally_equal() {
    let declared_tmp = tempfile::tempdir().unwrap();
    let declared = workspace(
        declared_tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(DECLARED_GATE),
        None,
    );
    let builtin_tmp = tempfile::tempdir().unwrap();
    let builtin = workspace(builtin_tmp.path(), DRIFTWATCHDOG_PROVIDER, None, None);

    let declared_row = plugin_json(&declared)["plugins"][0].clone();
    let builtin_row = plugin_json(&builtin)["plugins"][0].clone();

    assert_eq!(declared_row["source"], "declared");
    assert_eq!(builtin_row["source"], "builtin");
    // Behavioural equality: the declaration path loses no capability and
    // invents none. The only difference is provenance.
    for field in ["id", "kind", "capabilities", "state", "enabled", "reason"] {
        assert_eq!(
            declared_row[field], builtin_row[field],
            "declared and builtin rows differ on `{field}`"
        );
    }
}

#[test]
fn an_empty_or_absent_declaration_is_silent() {
    let tmp = tempfile::tempdir().unwrap();
    let empty = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(r#"{"interfaces":[],"capabilities":{}}"#),
        None,
    );
    let page = plugin_json(&empty);
    assert_eq!(plugin(&page, "driftwatchdog")["source"], "builtin");

    let text = stdout(&run(&empty, &["plugins", "list", "."]));
    assert!(text.contains("driftwatchdog  gate  ready"), "{text}");
}

#[test]
fn an_interfaces_consumption_declares_the_provider_it_reaches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(
            r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"driftwatchdog check --format json","contract":"driftwatch-checker/0.1.0","evidence_ref":"src/gate/mod.rs"}]}"#,
        ),
        None,
    );
    let entry = plugin_json(&dir)["plugins"][0].clone();
    assert_eq!(entry["source"], "declared");
    // The declaration states no kind, so the builtin value fills it rather
    // than being overridden by the declaration.
    assert_eq!(entry["kind"], "gate");
    assert_eq!(entry["capabilities"], serde_json::json!(["gate"]));
}

// -- rank 1: the descriptor, and its refusal -----------------------------

#[test]
fn the_descriptor_still_speaks_first_when_it_agrees() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        "providers:\n  - id: driftwatchdog\n    command: /bin/true\n    enabled: true\nplugins:\n  - id: driftwatchdog\n    kind: gate\n    capabilities: [gate]\n",
        Some(DECLARED_GATE),
        None,
    );
    let entry = plugin_json(&dir)["plugins"][0].clone();
    assert_eq!(entry["source"], "descriptor");
    assert_eq!(entry["kind"], "gate");
}

#[test]
fn a_contradicting_descriptor_and_declaration_refuse_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        "providers:\n  - id: driftwatchdog\n    command: /bin/true\n    enabled: true\nplugins:\n  - id: driftwatchdog\n    kind: metadata\n    capabilities: [topic]\n",
        Some(DECLARED_GATE),
        None,
    );
    let out = run(&dir, &["plugins", "list", "."]);
    assert!(!out.status.success(), "must refuse, not choose");
    let text = stderr(&out);
    for needle in [
        "definition-conflict",
        "`driftwatchdog`",
        "declared `gate`",
        "configured `metadata`",
        "declared [gate]",
        "configured [topic]",
    ] {
        assert!(text.contains(needle), "{needle} missing from {text}");
    }
    // Nothing is resolved from that declaration: no row at all.
    assert!(!text.contains("driftwatchdog  gate  ready"), "{text}");

    // The machine-readable form carries the stable code, so a caller does
    // not have to parse prose.
    let json = run(&dir, &["plugins", "list", ".", "--format", "json"]);
    let line = stderr(&json)
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .expect("an error document")
        .to_string();
    let body: Value = serde_json::from_str(&line).expect("error json");
    assert_eq!(body["error"]["code"], "plugin-boundary-refused");
    assert_eq!(out.status.code(), Some(1));
}

// -- a declaration is readable or refused --------------------------------

#[test]
fn an_unparseable_declaration_refuses_naming_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(tmp.path(), DRIFTWATCHDOG_PROVIDER, Some("{ not json"), None);
    let out = run(&dir, &["plugins", "list", "."]);
    assert!(!out.status.success());
    let text = stderr(&out);
    assert!(text.contains("declaration-invalid"), "{text}");
    assert!(text.contains(".project.json"), "{text}");
}

#[test]
fn an_unknown_mechanism_refuses_naming_the_field_and_the_accepted_set() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(
            r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"teleport","entry":"x","contract":"driftwatch-checker/0.1.0","digest":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}]}"#,
        ),
        None,
    );
    let text = stderr(&run(&dir, &["plugins", "list", "."]));
    assert!(text.contains("mechanism-unknown"), "{text}");
    assert!(text.contains("`gate-check`.mechanism"), "{text}");
    assert!(text.contains("teleport"), "{text}");
    assert!(text.contains("invoke, consume, observe, sibling"), "{text}");
}

#[test]
fn a_versionless_contract_on_a_non_sibling_interface_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(
            r#"{"interfaces":[{"id":"gate-check","consumes":true,"provider":"driftwatchdog","mechanism":"invoke","entry":"driftwatchdog check --format json"}]}"#,
        ),
        None,
    );
    let text = stderr(&run(&dir, &["plugins", "list", "."]));
    assert!(text.contains("contract-unversioned"), "{text}");
    assert!(text.contains("`gate-check`.contract"), "{text}");
    assert!(text.contains("`invoke`"), "{text}");
}

#[test]
fn a_consumption_without_a_digest_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(
            r#"{"interfaces":[{"id":"platform-schemas","consumes":true,"provider":"platform-contracts","mechanism":"consume","entry":"contracts/registry.json","contract":"platform-contracts/1.0.0"}]}"#,
        ),
        None,
    );
    let text = stderr(&run(&dir, &["plugins", "list", "."]));
    assert!(text.contains("digest-missing"), "{text}");
    assert!(text.contains("`platform-schemas`.digest"), "{text}");
}

// -- rank 3: the bundled kit manifest ------------------------------------

#[test]
fn the_kit_manifest_answers_when_nothing_else_does() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        "providers:\n  - id: kit-built\n    command: /bin/true\n    enabled: true\n",
        None,
        Some(
            r#"{"schema_version":1,"source":"test-kits","version":"0.0.1","plugins":[{"id":"kit-built","kind":"quality","capabilities":["quality"],"description":"from the kit feed"}]}"#,
        ),
    );
    let out = run(&dir, &["plugins", "list", ".", "--format", "json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let page: Value = serde_json::from_slice(&out.stdout).unwrap();
    let entry = plugin(&page, "kit-built");
    assert_eq!(entry["source"], "manifest");
    assert_eq!(entry["kind"], "quality");
    assert_eq!(entry["description"], "from the kit feed");

    // A declaration naming the same id answers first — the manifest is
    // rank 3, the declaration rank 2.
    let tmp2 = tempfile::tempdir().unwrap();
    let declared = workspace(
        tmp2.path(),
        "providers:\n  - id: kit-built\n    command: /bin/true\n    enabled: true\n",
        Some(
            r#"{"capabilities":{"quality":{"owner":"kit-built","evidence_state":"configured","evidence_ref":"src/gate/mod.rs"}}}"#,
        ),
        None,
    );
    let page = plugin_json(&declared);
    assert_eq!(plugin(&page, "kit-built")["source"], "declared");
}

// -- cap counts and provenance -------------------------------------------

#[test]
fn cap_counts_are_unchanged_and_cap_inspect_names_the_provenance() {
    let declared_tmp = tempfile::tempdir().unwrap();
    let declared = workspace(
        declared_tmp.path(),
        DRIFTWATCHDOG_PROVIDER,
        Some(DECLARED_GATE),
        None,
    );
    let builtin_tmp = tempfile::tempdir().unwrap();
    let builtin = workspace(builtin_tmp.path(), DRIFTWATCHDOG_PROVIDER, None, None);

    let with = run_json(&declared, &["cap", "list", ".", "--format", "json"]);
    let without = run_json(&builtin, &["cap", "list", ".", "--format", "json"]);
    // The id set and the Ready counts are the same either way: this change
    // adds a source, it removes no capability.
    let counts = |page: &Value| -> BTreeMap<String, i64> {
        page["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    row["capability"].as_str().unwrap().to_string(),
                    row["ready_plugins"].as_i64().unwrap(),
                )
            })
            .collect()
    };
    assert_eq!(counts(&with), counts(&without));

    let text = stdout(&run(&declared, &["cap", "inspect", "gate", "."]));
    assert!(text.contains("driftwatchdog (gate, declared)"), "{text}");
}

// -- Forge's own declaration ---------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("cannot parse {}: {e}", path.display()))
}

fn forge_declaration() -> Value {
    read_json(&repo_root().join(".project.json"))
}

/// A bare `schema_version: 1` written in the form the workspace schema
/// requires: `<major>.<minor>.<patch>`.
fn render_schema_version(version: u64) -> String {
    format!("{version}.0.0")
}

/// The contract a `CONTRACTS` row registers, in the declaration's own
/// `"<name>/<major>.<minor>.<patch>"` form: the row's wire discriminator
/// when it has one, the platform family's registered version when it has
/// one, and otherwise the row's canonical spec id with the row's version.
fn inventory_contract(module: &str, constant: &str) -> String {
    let spec = CONTRACTS
        .iter()
        .find(|spec| spec.module == module && spec.constant == constant)
        .unwrap_or_else(|| panic!("no CONTRACTS row for {module}/{constant}"));
    if !spec.discriminator.is_empty() {
        return spec.discriminator.to_string();
    }
    if let Some(family) = spec.platform_family {
        return forge::contract::contract_discriminator(family)
            .unwrap_or_else(|| panic!("{family} has no registered version"));
    }
    let spec_id = spec
        .doc
        .rsplit('/')
        .nth(1)
        .unwrap_or_else(|| panic!("{} names no canonical spec", spec.constant));
    format!("{spec_id}/{}", spec.version)
}

/// The versioned-surface inventory this repository exposes, mapped to each
/// declared interface id. The shipped declaration is compared with this
/// derivation, so the two cannot drift and no version is restated by hand.
fn derived_contracts() -> BTreeMap<String, String> {
    let mut derived: BTreeMap<String, String> = BTreeMap::new();

    // Boundaries whose version the inventory registers.
    for (id, (module, constant)) in [
        ("gate-result", ("gate", "GATE_CONTRACT_VERSION")),
        (
            "contract-envelope",
            ("contract", "CONTRACT_CONTRACT_VERSION"),
        ),
        ("mcp-surface", ("mcp", "MCP_CONTRACT_VERSION")),
        (
            "policy-checker-report",
            ("policy", "POLICY_CONTRACT_VERSION"),
        ),
        (
            "governance-observation",
            ("governance", "GOVERNANCE_CONTRACT_VERSION"),
        ),
    ] {
        derived.insert(id.to_string(), inventory_contract(module, constant));
    }
    // Adapter contract constants, which already carry name and version.
    derived.insert(
        "publish-provider".to_string(),
        forge::publish::providers::contract::PUBLISH_PROVIDER_CONTRACT.to_string(),
    );
    derived.insert(
        "deploy-executor".to_string(),
        forge::deploy::contract::DEPLOY_EXECUTOR_CONTRACT.to_string(),
    );
    derived.insert(
        "checker-document".to_string(),
        forge::checker::CHECKER_SCHEMA_ID.to_string(),
    );

    // Retained copies. A copy whose pin document records a release version
    // is named after its provider; one whose artifact records only its own
    // schema version is named after the declared interface id and that
    // version, rendered in the required form.
    let kits = read_json(&repo_root().join("kits/manifest.json"));
    derived.insert(
        "platform-kit-feed".to_string(),
        format!(
            "{}/{}",
            kits["source"].as_str().expect("kit feed source"),
            kits["version"].as_str().expect("kit feed version")
        ),
    );
    let contracts_manifest = read_json(&repo_root().join("contracts/manifest.json"));
    let pin_version = render_schema_version(
        contracts_manifest["schema_version"]
            .as_u64()
            .expect("contracts manifest schema_version"),
    );
    derived.insert(
        "secret-field-vocabulary".to_string(),
        format!("secret-field-vocabulary/{pin_version}"),
    );

    // The governance copies: the checker declares its own report schema
    // version, and the vocabulary declares `schema_version` — the same
    // value Forge itself accepts.
    let checker =
        fs::read_to_string(repo_root().join("scripts/governance/product_code_quality.py"))
            .expect("vendored checker ships");
    let checker_version = checker
        .lines()
        .find_map(|line| line.strip_prefix("SCHEMA_VERSION = "))
        .map(|value| value.trim().parse::<u64>().expect("checker schema version"))
        .expect("the checker declares SCHEMA_VERSION");
    derived.insert(
        "governance-checker".to_string(),
        format!(
            "governance-checker/{}",
            render_schema_version(checker_version)
        ),
    );
    let vocabulary = read_json(&repo_root().join("scripts/vocabulary.json"));
    let vocabulary_version = vocabulary["schema_version"]
        .as_u64()
        .expect("vocabulary schema_version");
    assert_eq!(
        vocabulary_version,
        forge::vocabulary::VOCABULARY_SCHEMA_VERSION,
        "the vendored vocabulary and Forge's accepted schema version disagree"
    );
    let rendered = render_schema_version(vocabulary_version);
    for id in [
        "governance-vocabulary",
        "governance-checker-vocabulary",
        "vendored-governance-vocabulary",
    ] {
        derived.insert(id.to_string(), format!("governance-vocabulary/{rendered}"));
    }
    derived
}

#[test]
fn every_declared_contract_version_is_derived_from_the_inventory() {
    let declaration = forge_declaration();
    let mut declared: BTreeMap<String, String> = BTreeMap::new();
    for entry in declaration["interfaces"]
        .as_array()
        .expect("interfaces array")
    {
        let id = entry["id"].as_str().expect("interface id").to_string();
        if let Some(contract) = entry["contract"].as_str() {
            declared.insert(id, contract.to_string());
        }
    }
    let derived = derived_contracts();
    // Every derived boundary is declared identically...
    for (id, contract) in &derived {
        assert_eq!(
            declared.get(id),
            Some(contract),
            "declared `{}` does not match the inventory's `{}`",
            declared.get(id).map(String::as_str).unwrap_or("<absent>"),
            contract
        );
    }
    // ...and no declared contract is absent from the derivation: a version
    // the inventory does not register is absent from the declaration.
    for (id, contract) in &declared {
        assert!(
            derived.contains_key(id),
            "interface `{id}` declares contract `{contract}`, which no inventory entry supplies"
        );
    }
    assert_eq!(declared.len(), derived.len());
}

#[test]
fn every_declared_digest_pins_the_bytes_it_names() {
    let declaration = forge_declaration();
    let mut pinned = 0;
    for entry in declaration["interfaces"]
        .as_array()
        .expect("interfaces array")
    {
        let Some(digest) = entry["digest"].as_str() else {
            continue;
        };
        let path = repo_root().join(entry["entry"].as_str().expect("entry"));
        let bytes =
            fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let actual = format!("sha256:{}", hex(&Sha256::digest(&bytes)));
        assert_eq!(
            digest, actual,
            "interface `{}` pins `{}` with a digest that is not those bytes",
            entry["id"], entry["entry"]
        );
        pinned += 1;
    }
    assert!(pinned >= 5, "only {pinned} digests are pinned");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn no_retained_copy_is_unaccounted_for() {
    let declaration = forge_declaration();
    let mut named: BTreeSet<String> = BTreeSet::new();
    for entry in declaration["interfaces"]
        .as_array()
        .expect("interfaces array")
    {
        if entry["consumes"] != true {
            continue;
        }
        named.insert(entry["entry"].as_str().expect("entry").to_string());
    }
    // An external copy is accounted for when a declared entry names it, or
    // when a declared entry names the pin document that records its sha256.
    let mut recorded: BTreeSet<String> = BTreeSet::new();
    for pin in ["contracts/manifest.json", "kits/manifest.json"] {
        if pin == "kits/manifest.json" && !named.contains(pin) {
            continue;
        }
        let manifest = read_json(&repo_root().join(pin));
        let base = Path::new(pin).parent().expect("pin document parent");
        for file in manifest["files"].as_array().expect("manifest files") {
            recorded.insert(
                base.join(file["path"].as_str().expect("file path"))
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    for tree in ["contracts", "kits", "scripts/governance"] {
        for relative in walk(&repo_root().join(tree)) {
            let relative = relative
                .strip_prefix(repo_root())
                .expect("inside the repository")
                .to_string_lossy()
                .to_string();
            // Runtime templates are now Forge-owned source with their own
            // checked asset manifest; they are not a retained sibling copy.
            if relative.starts_with("scripts/governance/runtime-templates/") {
                continue;
            }
            assert!(
                named.contains(&relative) || recorded.contains(&relative),
                "retained copy `{relative}` is neither declared nor recorded in a declared manifest"
            );
        }
    }
    // The three copies of the secret-field vocabulary: Forge holds one, and
    // it is declared rather than unaccounted for.
    assert!(named.contains("contracts/vocabulary/secret-field-substrings.json"));
}

/// Every file under `root`, relative to it, skipping build output and
/// caches the repository does not own.
fn walk(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "__pycache__" || name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                // A pin document records the files it pins, not itself.
                if matches!(
                    name.as_str(),
                    "manifest.json" if path.parent() == Some(root)
                ) {
                    continue;
                }
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn forge_declares_its_own_boundaries_and_invents_no_version() {
    let declaration = forge_declaration();
    let interfaces = declaration["interfaces"]
        .as_array()
        .expect("interfaces array");
    assert!(!interfaces.is_empty());

    // Every entry is lawful under the workspace schema's closed set, and
    // the declaration is one Forge itself can read.
    for entry in interfaces {
        let mechanism = entry["mechanism"].as_str().expect("mechanism");
        assert!(
            INTERFACE_MECHANISMS.contains(&mechanism),
            "mechanism `{mechanism}` is outside the closed set"
        );
        assert!(entry.get("contract").is_some() || mechanism == "sibling");
        if matches!(mechanism, "consume" | "observe" | "sibling") {
            assert!(
                entry.get("digest").is_some(),
                "`{}` is digest-pinned",
                entry["id"]
            );
        }
        if entry["consumes"] == true {
            assert!(entry.get("provider").is_some(), "a consumption names it");
            assert!(
                entry.get("evidence_ref").is_some(),
                "`{}` declares its own use of the edge",
                entry["id"]
            );
        }
    }

    let ids: BTreeSet<&str> = interfaces.iter().filter_map(|e| e["id"].as_str()).collect();
    assert_eq!(ids.len(), interfaces.len(), "interface ids are unique");

    // The two agent runtimes have no contract version on either side, so
    // they are integrations, never interfaces with an invented version.
    let integrations: BTreeSet<&str> = declaration["integrations"]
        .as_array()
        .expect("integrations array")
        .iter()
        .filter_map(|e| e["id"].as_str())
        .collect();
    for runtime in ["ariadex", "sisyphusfy"] {
        assert!(integrations.contains(runtime), "{runtime} is not declared");
        assert!(!ids.contains(runtime), "{runtime} must not be an interface");
    }

    // Forge reads its own declaration with nothing refused.
    let declaration_read = forge::plugins::read_declaration(&repo_root())
        .expect("Forge's own declaration is readable");
    assert_eq!(declaration_read.interfaces.len(), interfaces.len());
}

#[test]
fn every_plugin_source_is_reported_by_its_own_name() {
    // The four ranks, as the renderer names them.
    assert_eq!(PluginSource::Descriptor.as_str(), "descriptor");
    assert_eq!(PluginSource::Declared.as_str(), "declared");
    assert_eq!(PluginSource::Manifest.as_str(), "manifest");
    assert_eq!(PluginSource::Builtin.as_str(), "builtin");
}

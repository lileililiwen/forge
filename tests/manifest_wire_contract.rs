//! The published manifest is validated against the *consumed* contract.
//!
//! Forge produces a document that a separate static site reads. That site's
//! build fails unless the document validates against
//! `platform-contracts`' `schemas/public-portfolio-manifest.schema.json`. Forge
//! used to emit three fields the schema rejects — a bare family name, an
//! integer `schema_version` and an integer `manifest_revision` — and nothing
//! inside this repository could catch it, because the mirror under `contracts/`
//! does not contain that schema file at all.
//!
//! Two tests, failing differently:
//!
//! - `a_published_manifest_file_carries_the_contract_wire_shape` runs in the
//!   ordinary suite. It drives the real CLI end to end, reads the file the
//!   publisher actually wrote, and pins the three fields. It needs no sibling
//!   checkout, so the wire shape cannot drift unnoticed.
//! - `a_published_manifest_validates_against_the_pinned_schema` is the
//!   decisive check and is `#[ignore]`d, because it needs the sibling
//!   `platform-contracts` checkout and a Python with `jsonschema`. Run it with
//!
//!   ```sh
//!   cargo test --test manifest_wire_contract -- --ignored
//!   ```
//!
//!   It is ignored rather than silently skipped: a test that returns early when
//!   its oracle is missing is a green result that never ran the check.

#[path = "support/share.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use support::{fleet, run_json, share_set};

/// Publish one approved manifest through the real CLI and return the bytes the
/// local file publisher wrote.
fn publish_one_manifest(tmp: &Path) -> String {
    let (db, _token) = fleet(tmp);
    share_set(&db, "alethefy", "Alethefy");
    let preview = run_json(&db, &["portfolio", "share", "preview"]);
    let hash = preview["preview"]["manifest_sha256"]
        .as_str()
        .expect("manifest_sha256")
        .to_string();
    run_json(&db, &["portfolio", "share", "approve", "--hash", &hash]);
    let target = tmp.join("public/portfolio-manifest.json");
    run_json(
        &db,
        &[
            "portfolio",
            "share",
            "publish",
            "--target",
            target.to_str().unwrap(),
            "--operation-key",
            "op-wire-shape",
        ],
    );
    fs::read_to_string(&target).expect("published manifest file")
}

#[test]
fn a_published_manifest_file_carries_the_contract_wire_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let document = publish_one_manifest(tmp.path());
    let value: Value = serde_json::from_str(&document).expect("published document");

    // The contract's family is a closed enum and its name is qualified.
    assert_eq!(value["schema_family"], "platform.public-portfolio-manifest");
    // The version is a full semantic version, as a string.
    assert_eq!(value["schema_version"], "1.0.0");
    // The revision is an opaque producer-side tag, as a string.
    assert_eq!(value["manifest_revision"], "rev_1");
    // The three failures that broke the consumer's build were all JSON type
    // errors, so assert the *types*, not only the values.
    for field in ["schema_family", "schema_version", "manifest_revision"] {
        assert!(
            value[field].is_string(),
            "`{field}` must be a JSON string, got {:?}",
            value[field]
        );
    }
    // The hash stays lowercase hex and the catalog is non-empty, so this test
    // cannot pass by validating an empty document.
    let hash = value["manifest_sha256"].as_str().expect("hash");
    assert!(hash.len() == 64, "`{hash}` is not a 64-character hash");
    assert_eq!(value["projects"][0]["id"], "alethefy");
}

/// The decisive acceptance test: a real Forge-produced document, validated by
/// `jsonschema` against the pinned contract schema.
#[test]
#[ignore = "needs the sibling platform-contracts checkout and python jsonschema"]
fn a_published_manifest_validates_against_the_pinned_schema() {
    let schema = pinned_schema_path();
    let tmp = tempfile::tempdir().unwrap();
    let document = publish_one_manifest(tmp.path());
    let manifest = tmp.path().join("manifest.json");
    fs::write(&manifest, &document).expect("write manifest");

    // The consumer's own oracle: `jsonschema` against the pinned schema. No
    // `FormatChecker` is passed, so `format` keywords stay annotations — the
    // same semantics the consumer's `validate_manifest.py` uses. The three
    // fields at issue are `enum`/`pattern` constraints and are enforced either
    // way.
    let script = r#"
import json, sys
from jsonschema import Draft202012Validator

schema = json.load(open(sys.argv[1]))
document = json.load(open(sys.argv[2]))
Draft202012Validator.check_schema(schema)
errors = sorted(
    Draft202012Validator(schema).iter_errors(document),
    key=lambda e: list(e.absolute_path),
)
if errors:
    for err in errors:
        path = "/".join(str(p) for p in err.absolute_path) or "<root>"
        print(f"{path}: {err.message}", file=sys.stderr)
    raise SystemExit(1)
print(f"valid: {len(document['projects'])} project(s), revision "
      f"{document['manifest_revision']!r}, schema_version "
      f"{document['schema_version']!r}")
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&schema)
        .arg(&manifest)
        .output()
        .expect("run python3; install it with the `jsonschema` package");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the published manifest does not satisfy {}\nstdout: {stdout}\nstderr: {stderr}",
        schema.display()
    );
    println!("jsonschema accepted the Forge-produced manifest: {stdout}");
}

/// Locate the pinned contract schema, or say exactly what is missing.
fn pinned_schema_path() -> PathBuf {
    let relative = "schemas/public-portfolio-manifest.schema.json";
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("PLATFORM_CONTRACTS_DIR") {
        roots.push(PathBuf::from(dir));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cursor: Option<&Path> = Some(manifest_dir.as_path());
    while let Some(dir) = cursor {
        roots.push(dir.join("platform-contracts"));
        cursor = dir.parent();
    }
    for root in &roots {
        let candidate = root.join(relative);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "the pinned contract schema was not found. Check out platform-contracts beside \
         this repository or set PLATFORM_CONTRACTS_DIR; looked for {relative} under {roots:?}"
    );
}

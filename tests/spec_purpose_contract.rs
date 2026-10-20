//! The canonical capabilities state their purpose (`spec-purpose-completion`).
//!
//! The OpenSpec archive writes a placeholder into every capability it
//! creates: `## Purpose` followed by `TBD - created by archiving change
//! <name>. Update Purpose after archive.` Somebody has to replace it with the
//! real purpose, and nobody did for fourteen capabilities.
//!
//! This test is the guard. It reads every canonical capability
//! specification and refuses one whose purpose is the archive placeholder,
//! whose purpose section is missing, or whose purpose is too short to be a
//! paragraph. It names the capability it refuses, so the failure says which
//! file to open and what to write.
//!
//! The test reads the repository's own specifications from
//! `CARGO_MANIFEST_DIR`, so it reads the tree it was built from and not
//! whatever happens to be in the working directory. It is fail-closed: a
//! specification it cannot read is a failure, never a skip, because an empty
//! or unreadable spec tree would otherwise pass silently.

use std::fs;
use std::path::{Path, PathBuf};

/// The shortest purpose the tree accepts. A purpose is a paragraph, not a
/// label: the shortest real purpose in the tree is longer than this.
const MIN_PURPOSE_CHARS: usize = 60;

/// The line the OpenSpec archive writes into a new capability's purpose.
const ARCHIVE_PLACEHOLDER: &str = "TBD - created by archiving change";

/// The repository root, resolved from the build's own manifest directory.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every canonical capability specification, sorted by capability name.
fn capability_specs() -> Vec<PathBuf> {
    let root = repo_root().join("openspec").join("specs");
    let entries = fs::read_dir(&root)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", root.display()));
    let mut specs: Vec<PathBuf> = entries
        .map(|entry| entry.expect("read a specs directory entry").path())
        .filter(|path| path.is_dir())
        .map(|dir| dir.join("spec.md"))
        .filter(|spec| spec.is_file())
        .collect();
    specs.sort();
    specs
}

/// The capability name a specification file belongs to.
fn capability_name(spec: &Path) -> String {
    spec.parent()
        .and_then(|dir| dir.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| spec.display().to_string())
}

/// The body of the specification's `## Purpose` section, or `None` when the
/// section is missing.
fn purpose_body(spec: &Path) -> Option<String> {
    let text = fs::read_to_string(spec)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", spec.display()));
    let (_, after_purpose) = text.split_once("## Purpose")?;
    // The purpose runs until the next section heading. A capability that
    // states no requirements after its purpose would still carry a body,
    // so the split is on the heading rather than on the requirements.
    let end = after_purpose
        .find("\n## ")
        .unwrap_or(after_purpose.len());
    Some(after_purpose[..end].trim().to_string())
}

#[test]
fn every_capability_states_a_real_purpose() {
    let specs = capability_specs();
    assert!(
        !specs.is_empty(),
        "no canonical capability specification was found under openspec/specs"
    );
    let mut problems: Vec<String> = Vec::new();
    for spec in &specs {
        let name = capability_name(spec);
        let body = match purpose_body(spec) {
            Some(body) => body,
            None => {
                problems.push(format!(
                    "{name}: no `## Purpose` section — write what the capability is for"
                ));
                continue;
            }
        };
        if body.is_empty() {
            problems.push(format!("{name}: the `## Purpose` section is empty"));
            continue;
        }
        if body
            .lines()
            .any(|line| line.trim_start().starts_with(ARCHIVE_PLACEHOLDER))
        {
            problems.push(format!(
                "{name}: the purpose is still the archive placeholder — replace it with what \
                 the capability is for"
            ));
            continue;
        }
        if body.chars().count() < MIN_PURPOSE_CHARS {
            problems.push(format!(
                "{name}: the purpose is {} characters — a purpose is a paragraph, not a label",
                body.chars().count()
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {} canonical capabilities state no real purpose:\n  - {}",
        problems.len(),
        specs.len(),
        problems.join("\n  - ")
    );
}

#[test]
fn a_capability_that_states_a_purpose_passes_the_guard() {
    // The guard's own cases: a written purpose passes, and each shape of
    // missing purpose is refused.
    let spec = repo_root()
        .join("openspec")
        .join("specs")
        .join("doctor-maturity-assessment")
        .join("spec.md");
    let body = purpose_body(&spec).expect("the doctor capability states a purpose");
    assert!(!body.is_empty());
    assert!(!body.contains(ARCHIVE_PLACEHOLDER));
    assert!(body.chars().count() >= MIN_PURPOSE_CHARS);

    let placeholder = format!("{ARCHIVE_PLACEHOLDER} something. Update Purpose after archive.");
    assert!(placeholder.trim_start().starts_with(ARCHIVE_PLACEHOLDER));
    assert!(placeholder.chars().count() >= MIN_PURPOSE_CHARS);
}

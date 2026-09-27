//! Consumed governance vocabulary (`governance-vocabulary-consumption`).
//!
//! Workspace Governance owns the canonical `kind`, `profile` and word
//! lists (`vocabulary.json`, `schema_version: 1`); Forge consumes a
//! digest-pinned copy at `contracts/vocabulary/governance-vocabulary.json`
//! and never re-declares those values locally. Resolution order is an
//! explicit path, then `FORGE_GOVERNANCE_VOCABULARY`, then the vendored
//! copy — the same explicit-beats-env shape the gate plane already uses
//! for `FORGE_GATE_BIN`.
//!
//! Failure contract, by tier:
//!
//! - An explicitly named file (flag or environment) that is missing or
//!   does not parse is a typed refusal naming that file. There is never
//!   a silent fall-through to the vendored copy.
//! - The vendored tier degrades to `unavailable`: every consulting
//!   surface behaves exactly as it did before this vocabulary existed
//!   and reports `vocabulary-unavailable`. Absence never blocks
//!   generation, import, doctor, check, fleet or gate, and nothing is
//!   fetched from the network or guessed locally as a substitute.
//!
//! The consumed document is untrusted data: reads are size-bounded and
//! entries carrying path separators or traversal tokens are refused.

use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use serde::Deserialize;

/// Process-wide serialization around [`load`]: tests across the crate
/// mutate the `FORGE_GOVERNANCE_VOCABULARY` environment to assert
/// behaviour, and the loader reads the env at call time. Without the
/// guard two concurrent doctor tests could see each other's env and
/// produce an `unavailable` finding when the test expects a `warn`.
pub static VOCABULARY_ENV_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static VOCABULARY_ENV_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// RAII guard: sets a thread-local vocabulary env override for the
/// lifetime of the guard, and restores None on drop. Each test thread
/// gets its own isolated override, so concurrent tests never race on
/// the global env.
pub struct WithVocabularyOverride<'a> {
    _thread: thread::ThreadId,
    previous: Option<String>,
    _life: &'a (), // prevents multiple guards stacking
}

impl WithVocabularyOverride<'_> {
    /// Set the thread-local override to `env_value` for the duration
    /// of the returned guard. Dropping the guard restores the previous
    /// value (None).
    pub fn new(env_value: Option<&str>) -> WithVocabularyOverride<'static> {
        let previous = VOCABULARY_ENV_OVERRIDE.with(|cell| cell.borrow().clone());
        if let Some(value) = env_value {
            VOCABULARY_ENV_OVERRIDE.with(|cell| *cell.borrow_mut() = Some(value.to_string()));
        } else {
            VOCABULARY_ENV_OVERRIDE.with(|cell| *cell.borrow_mut() = None);
        }
        WithVocabularyOverride {
            _thread: thread::current().id(),
            previous,
            _life: &(),
        }
    }
}

impl Drop for WithVocabularyOverride<'_> {
    fn drop(&mut self) {
        VOCABULARY_ENV_OVERRIDE.with(|cell| *cell.borrow_mut() = self.previous.clone());
    }
}

/// Environment override for the consumed vocabulary file.
pub const VOCABULARY_ENV: &str = "FORGE_GOVERNANCE_VOCABULARY";

/// Vendored copy, relative to [`crate::contract::contracts_dir`].
pub const VENDORED_REL_PATH: &str = "vocabulary/governance-vocabulary.json";

/// Bound on any consumed vocabulary document (mirrors the fleet
/// registry's 1 MiB bound for consumed sibling documents).
pub const MAX_VOCABULARY_BYTES: usize = 1024 * 1024;

/// Vocabulary schema version this consumer understands.
pub const VOCABULARY_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Deserialize)]
struct VocabularyDocument {
    #[serde(default)]
    schema_version: u64,
    #[serde(default)]
    profiles: Vec<String>,
    #[serde(default)]
    kinds: Vec<String>,
    #[serde(default)]
    placeholder_markers: Vec<String>,
    #[serde(default)]
    secret_field_substrings: Vec<String>,
}

/// Consumed canonical sets plus provenance.
#[derive(Debug, Clone)]
pub struct GovernanceVocabulary {
    /// Canonical governance profile values, verbatim from the source.
    pub profiles: HashSet<String>,
    /// Canonical declaration kind values, verbatim from the source.
    pub kinds: HashSet<String>,
    /// Repository-check marker words (informational; the quality checker
    /// owns counting).
    pub placeholder_markers: Vec<String>,
    /// Secret field-name fragments (informational; captured-output
    /// redaction stays in `policy::redact_credentials`).
    pub secret_field_substrings: Vec<String>,
    /// Where this copy was resolved from.
    pub source: ResolvedSource,
}

/// Provenance of one consumed copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// `explicit` (flag), `env`, or `vendored`.
    pub tier: &'static str,
    /// Absolute path that was read.
    pub path: PathBuf,
    /// Pinned source revision when the tier carries one (vendored tier
    /// reads the contracts manifest entry).
    pub revision: Option<String>,
}

/// Loader outcome: either a vocabulary or a typed failure. Callers match
/// on [`VocabularyError`] to decide between refusal and the
/// `vocabulary-unavailable` fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyError {
    /// An explicitly named file (flag or environment) cannot be used.
    /// The request is refused naming this file; never fall through.
    Refused { path: String, reason: String },
    /// No explicit file was named and the vendored copy cannot be
    /// trusted (missing, unreadable, digest mismatch, or unparseable).
    /// Behave as before and report `vocabulary-unavailable`.
    Unavailable { reason: String },
}

impl std::fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VocabularyError::Refused { path, reason } => {
                write!(f, "governance vocabulary refused at '{path}': {reason}")
            }
            VocabularyError::Unavailable { reason } => {
                write!(f, "vocabulary-unavailable: {reason}")
            }
        }
    }
}

/// Resolve which file would be read. Pure in `env` so tests stay
/// deterministic without touching the process environment.
pub fn resolve_with(explicit: Option<&Path>, env: Option<&OsStr>) -> (PathBuf, &'static str) {
    if let Some(path) = explicit {
        return (path.to_path_buf(), "explicit");
    }
    if let Some(dir) = env.filter(|v| !v.is_empty()) {
        return (PathBuf::from(dir), "env");
    }
    (
        crate::contract::contracts_dir().join(VENDORED_REL_PATH),
        "vendored",
    )
}

/// Resolve using the live process environment.
pub fn resolve_source(explicit: Option<&Path>) -> (PathBuf, &'static str) {
    // Check thread-local override first (set by run_doctor_with_vocabulary_env
    // in tests), then fall back to the global env.
    let env_override = VOCABULARY_ENV_OVERRIDE
        .with(|cell| cell.borrow().clone())
        .filter(|v| !v.is_empty());
    if let Some(ref v) = env_override {
        let env: &OsStr = OsStr::new(v);
        return resolve_with(explicit, Some(env));
    }
    // No thread-local override; fall back to global env.
    let env = std::env::var_os(VOCABULARY_ENV).filter(|v| !v.is_empty());
    resolve_with(explicit, env.as_deref())
}

fn entry_clean(value: &str) -> bool {
    !(value.is_empty()
        || value == ".."
        || value.starts_with('/')
        || value.contains("../")
        || value.contains("/..")
        || value.contains('\\'))
}

/// Parse and validate one document's bytes. Shared by every tier so an
/// explicit file and the vendored copy meet the same bar.
fn parse_document(bytes: &[u8], origin: &str) -> Result<GovernanceVocabulary, VocabularyError> {
    if bytes.len() > MAX_VOCABULARY_BYTES {
        return Err(VocabularyError::Refused {
            path: origin.to_string(),
            reason: format!(
                "document is {} bytes, over the {}-byte bound",
                bytes.len(),
                MAX_VOCABULARY_BYTES
            ),
        });
    }
    let doc: VocabularyDocument =
        serde_json::from_slice(bytes).map_err(|err| VocabularyError::Refused {
            path: origin.to_string(),
            reason: format!("unparseable governance vocabulary: {err}"),
        })?;
    if doc.schema_version != VOCABULARY_SCHEMA_VERSION {
        return Err(VocabularyError::Refused {
            path: origin.to_string(),
            reason: format!(
                "unsupported schema_version {}, expected {VOCABULARY_SCHEMA_VERSION}",
                doc.schema_version
            ),
        });
    }
    if doc.profiles.is_empty() || doc.kinds.is_empty() {
        return Err(VocabularyError::Refused {
            path: origin.to_string(),
            reason: "document carries no profiles or no kinds".to_string(),
        });
    }
    for value in doc
        .profiles
        .iter()
        .chain(doc.kinds.iter())
        .chain(doc.placeholder_markers.iter())
        .chain(doc.secret_field_substrings.iter())
    {
        if !entry_clean(value) {
            return Err(VocabularyError::Refused {
                path: origin.to_string(),
                reason: format!("entry {value:?} carries a path separator or traversal token"),
            });
        }
    }
    Ok(GovernanceVocabulary {
        profiles: doc.profiles.into_iter().collect(),
        kinds: doc.kinds.into_iter().collect(),
        placeholder_markers: doc.placeholder_markers,
        secret_field_substrings: doc.secret_field_substrings,
        source: ResolvedSource {
            tier: "parsed",
            path: PathBuf::from(origin),
            revision: None,
        },
    })
}

fn read_bounded(path: &Path, origin: &str) -> Result<Vec<u8>, VocabularyError> {
    let meta = std::fs::metadata(path).map_err(|err| VocabularyError::Refused {
        path: origin.to_string(),
        reason: format!("cannot read: {err}"),
    })?;
    if !meta.is_file() {
        return Err(VocabularyError::Refused {
            path: origin.to_string(),
            reason: "not a regular file".to_string(),
        });
    }
    if meta.len() > MAX_VOCABULARY_BYTES as u64 {
        return Err(VocabularyError::Refused {
            path: origin.to_string(),
            reason: format!(
                "document is {} bytes, over the {MAX_VOCABULARY_BYTES}-byte bound",
                meta.len()
            ),
        });
    }
    std::fs::read(path).map_err(|err| VocabularyError::Refused {
        path: origin.to_string(),
        reason: format!("cannot read: {err}"),
    })
}

/// Manifest revision pinned for the vendored copy, when the contracts
/// manifest is readable and names it.
fn vendored_revision() -> Option<String> {
    let manifest = crate::contract::load_manifest().ok()?;
    let entry = manifest
        .files
        .iter()
        .find(|f| f.path == VENDORED_REL_PATH)?;
    let bytes = std::fs::read(crate::contract::contracts_dir().join(&entry.path)).ok()?;
    let actual = crate::release::hash_bytes(&bytes);
    if actual != entry.sha256 {
        return None;
    }
    entry.revision.clone()
}

/// Load the vocabulary through the documented resolution order.
///
/// - `explicit` (`--vocabulary PATH`) or the environment tier naming a
///   file that cannot be used yields [`VocabularyError::Refused`]:
///   refuse the request naming the file.
/// - The vendored tier failing (missing, digest mismatch, unparseable)
///   yields [`VocabularyError::Unavailable`]: behave as before and
///   report `vocabulary-unavailable`.
pub fn load(explicit: Option<&Path>) -> Result<GovernanceVocabulary, VocabularyError> {
    // Acquire the lock only long enough to capture the env var and
    // resolve the path. File I/O (read_bounded / std::fs::read) happens
    // without the lock so callers that hold the lock
    // (e.g. run_doctor_with_vocabulary_env in tests) are not blocked
    // for the duration of file access. The captured path is then used
    // outside the lock.
    let (path, tier, origin) = {
        let _guard = VOCABULARY_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let (path, tier) = resolve_source(explicit);
        let origin = path.display().to_string();
        (path, tier, origin)
    };
    match tier {
        "explicit" | "env" => {
            let bytes = read_bounded(&path, &origin)?;
            let mut vocab = parse_document(&bytes, &origin)?;
            vocab.source = ResolvedSource {
                tier,
                path,
                revision: None,
            };
            Ok(vocab)
        }
        _ => {
            let bytes = std::fs::read(&path).map_err(|_| VocabularyError::Unavailable {
                reason: format!("vendored copy missing or unreadable at {}", path.display()),
            })?;
            if bytes.len() > MAX_VOCABULARY_BYTES {
                return Err(VocabularyError::Unavailable {
                    reason: format!(
                        "vendored copy is {} bytes, over the {MAX_VOCABULARY_BYTES}-byte bound",
                        bytes.len()
                    ),
                });
            }
            match parse_document(&bytes, &origin) {
                Ok(mut vocab) => match vendored_revision() {
                    Some(revision) => {
                        vocab.source = ResolvedSource {
                            tier: "vendored",
                            path,
                            revision: Some(revision),
                        };
                        Ok(vocab)
                    }
                    None => Err(VocabularyError::Unavailable {
                        reason: format!(
                            "vendored copy at {} fails its manifest digest or the manifest is unreadable",
                            vocab.source.path.display()
                        ),
                    }),
                },
                Err(VocabularyError::Refused { reason, .. }) => {
                    Err(VocabularyError::Unavailable { reason })
                }
                Err(other) => Err(other),
            }
        }
    }
}

impl GovernanceVocabulary {
    /// True when `value` is in the consumed canonical profile set.
    /// Foreign values are never coerced; callers refuse them by name.
    pub fn is_canonical_profile(&self, value: &str) -> bool {
        self.profiles.contains(value)
    }

    /// True when `value` is in the consumed canonical kind set.
    pub fn is_canonical_kind(&self, value: &str) -> bool {
        self.kinds.contains(value)
    }

    /// Short provenance line for evidence strings
    /// (`vocabulary workspace-governance@<rev>` or the tier + path).
    pub fn provenance(&self) -> String {
        match &self.source.revision {
            Some(rev) => format!("vocabulary workspace-governance@{rev}"),
            None => format!(
                "vocabulary {}@{}",
                self.source.tier,
                self.source.path.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const FIXTURE: &str = r#"{
  "schema_version": 1,
  "profiles": ["rust-product", "product"],
  "kinds": ["product", "platform"],
  "placeholder_markers": ["fixture-marker"],
  "secret_field_substrings": ["password"]
}"#;

    fn write_tmp(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn resolution_order_is_explicit_then_env_then_vendored() {
        let (path, tier) = resolve_with(
            Some(Path::new("/tmp/x.json")),
            Some(OsStr::new("/tmp/y.json")),
        );
        assert_eq!(tier, "explicit");
        assert_eq!(path, PathBuf::from("/tmp/x.json"));
        let (path, tier) = resolve_with(None, Some(OsStr::new("/tmp/y.json")));
        assert_eq!(tier, "env");
        assert_eq!(path, PathBuf::from("/tmp/y.json"));
        // Blank env values are treated as absent (never a lookup of "").
        let (path, tier) = resolve_with(None, Some(OsStr::new("")));
        assert_eq!(tier, "vendored");
        assert_eq!(
            path,
            crate::contract::contracts_dir().join(VENDORED_REL_PATH)
        );
        let (path, tier) = resolve_with(None, None);
        assert_eq!(tier, "vendored");
        assert_eq!(
            path,
            crate::contract::contracts_dir().join(VENDORED_REL_PATH)
        );
    }

    #[test]
    fn explicit_file_loads_and_reports_tier() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = write_tmp(tmp.path(), "vocab.json", FIXTURE.as_bytes());
        let vocab = load(Some(&path)).unwrap();
        assert!(vocab.is_canonical_profile("rust-product"));
        assert!(!vocab.is_canonical_profile("flutter-product"));
        assert!(vocab.is_canonical_kind("platform"));
        assert!(!vocab.is_canonical_kind("control-plane"));
        assert_eq!(vocab.source.tier, "explicit");
    }

    #[test]
    fn explicit_missing_file_refuses_without_fallthrough() {
        let missing = PathBuf::from("/nonexistent/vocabulary.json");
        match load(Some(&missing)) {
            Err(VocabularyError::Refused { path, .. }) => {
                assert!(path.contains("nonexistent"), "{path}")
            }
            other => panic!("explicit missing file must refuse, got {other:?}"),
        }
    }

    #[test]
    fn explicit_unparseable_file_refuses_naming_the_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = write_tmp(tmp.path(), "vocab.json", b"{not json");
        match load(Some(&path)) {
            Err(VocabularyError::Refused { path: p, reason }) => {
                assert!(p.contains("vocab.json"), "{p}");
                assert!(reason.contains("unparseable"), "{reason}");
            }
            other => panic!("unparseable file must refuse, got {other:?}"),
        }
    }

    #[test]
    fn wrong_schema_version_refuses() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = write_tmp(
            tmp.path(),
            "vocab.json",
            b"{\"schema_version\": 2, \"profiles\": [\"a\"], \"kinds\": [\"b\"]}",
        );
        match load(Some(&path)) {
            Err(VocabularyError::Refused { reason, .. }) => {
                assert!(reason.contains("schema_version"), "{reason}")
            }
            other => panic!("wrong schema must refuse, got {other:?}"),
        }
    }

    #[test]
    fn traversal_entries_refuse_the_document() {
        let tmp = tempfile::TempDir::new().unwrap();
        let body =
            "{\"schema_version\": 1, \"profiles\": [\"../escape\"], \"kinds\": [\"product\"]}";
        let path = write_tmp(tmp.path(), "vocab.json", body.as_bytes());
        match load(Some(&path)) {
            Err(VocabularyError::Refused { reason, .. }) => {
                assert!(reason.contains("traversal"), "{reason}")
            }
            other => panic!("traversal entry must refuse, got {other:?}"),
        }
    }

    #[test]
    fn vendored_copy_loads_with_pinned_revision() {
        // The vendored copy ships with this checkout; the manifest names
        // its digest, so the tier resolves with provenance. Other test
        // modules mutate `FORGE_GOVERNANCE_VOCABULARY` to assert their
        // own boundaries; we restore it here so the vendored tier is
        // the one we actually test.
        let previous = std::env::var_os(VOCABULARY_ENV);
        unsafe {
            std::env::remove_var(VOCABULARY_ENV);
        }
        let vocab = load(None).unwrap();
        let result = {
            assert!(vocab.is_canonical_profile("rust-product"));
            assert!(vocab.is_canonical_kind("platform"));
            assert_eq!(vocab.source.tier, "vendored");
            let revision = vocab
                .source
                .revision
                .clone()
                .expect("vendored tier pins a revision");
            assert_eq!(revision.len(), 40, "{revision}");
            revision
        };
        unsafe {
            if let Some(value) = previous {
                std::env::set_var(VOCABULARY_ENV, value);
            }
        }
        assert!(vocab.provenance().contains(&result[..12]));
    }

    #[test]
    fn directory_candidate_refuses_as_non_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        match load(Some(tmp.path())) {
            Err(VocabularyError::Refused { reason, .. }) => {
                assert!(reason.contains("regular file"), "{reason}")
            }
            other => panic!("directory must refuse, got {other:?}"),
        }
    }
}

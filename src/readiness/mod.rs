//! Reproducible native profile evidence and release-gate artifact checks.
//!
//! The v0.1 acceptance boundary requires native generation/build/test
//! evidence for every supported profile, plus reproducible CI/local release
//! gates. Contract fixtures alone do not establish that a generated project
//! builds through its native toolchain, and local `cargo build` success is
//! not by itself release evidence.
//!
//! The matrix generates every supported profile descriptor into a disposable
//! directory (never the caller's tree, never the registry), removes Forge
//! from `PATH` for the native phase so a generated project proves it builds
//! without Forge, captures toolchain versions, commands, exit statuses, the
//! source identity hash and a timestamp, and classifies each row as
//! `passed`, `failed` or `unverified`. A missing toolchain is `unverified`
//! with the missing prerequisite named; it can never satisfy a release gate.
//! A failed native command is `failed` with the profile and command named.
//!
//! The artifact surface reports the platform-native Forge binary produced by
//! Cargo (path, SHA-256, `forge --version` smoke) that CI and local release
//! checks share. The `check` gate passes only when every selected matrix row
//! is `passed` and the artifact smoke succeeds; anything else blocks with
//! the exact failed or unavailable check named.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::ForgeError;
use crate::generate::{normalize_explicit, render_feed_files, render_files};
use crate::profile::{inspect_profile, list_profiles, ProfileDescriptor, ProfileSupportStatus};

/// Versioned contract for the readiness surface.
pub const READINESS_CONTRACT_VERSION: &str = "0.1.0";

/// Native evidence classification for one matrix row.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReadinessStatus {
    Passed,
    Failed,
    Unverified,
}

impl ReadinessStatus {
    pub fn id(&self) -> &'static str {
        match self {
            ReadinessStatus::Passed => "passed",
            ReadinessStatus::Failed => "failed",
            ReadinessStatus::Unverified => "unverified",
        }
    }
}

/// Outcome of one native command (build or test) inside a matrix row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeCommandOutcome {
    pub command: String,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub evidence: String,
}

/// Reproducible native evidence row for one supported profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixRow {
    pub contract: String,
    pub profile: String,
    pub adapter: String,
    pub language: String,
    pub toolchain: String,
    /// The version the host's runtime reported, when it could be read.
    pub toolchain_version: Option<String>,
    /// The binary whose version was probed (`node` for the npm ecosystem).
    pub toolchain_runtime: String,
    /// The profile's declared toolchain version requirement.
    pub toolchain_version_declared: Option<String>,
    /// Whether the observed version satisfies the declaration. `None` when
    /// there is no declaration or no observation: unknown is never a
    /// mismatch.
    pub toolchain_version_satisfied: Option<bool>,
    pub build_command: String,
    pub test_command: String,
    pub source_hash: String,
    pub generated_at: String,
    pub forge_absent_from_path: bool,
    pub result: ReadinessStatus,
    pub reason: String,
    pub build: Option<NativeCommandOutcome>,
    pub test: Option<NativeCommandOutcome>,
}

/// Matrix report over the selected supported profiles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixReport {
    pub contract: String,
    pub rows: Vec<MatrixRow>,
    pub passed: usize,
    pub failed: usize,
    pub unverified: usize,
    pub ready: bool,
}

/// Verifiable evidence for the platform-native Forge binary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEvidence {
    pub contract: String,
    pub binary_path: String,
    pub sha256: String,
    pub version: String,
    pub version_smoke: String,
}

/// Combined release-gate outcome: selected matrix rows plus artifact smoke.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateReport {
    pub contract: String,
    pub matrix: MatrixReport,
    pub artifact: ArtifactEvidence,
    pub ready: bool,
    pub blocking: Vec<String>,
}

/// Supported profile ids in stable catalog order. The descriptor catalog is
/// the source of truth; planned candidates are never matrix rows.
pub fn matrix_profile_ids() -> Vec<String> {
    list_profiles()
        .iter()
        .filter(|p| p.support_status == ProfileSupportStatus::Supported)
        .map(|p| p.id.clone())
        .collect()
}

/// Remove the directory containing the running Forge binary from a
/// `PATH`-style value so the native phase proves the generated project
/// builds without Forge. Only the exact directory is removed; every other
/// entry (including the toolchain directories) is preserved byte-for-byte.
pub fn strip_forge_from_path(path_var: &OsStr, forge_dir: &Path) -> OsString {
    let kept: Vec<PathBuf> = std::env::split_paths(path_var)
        .filter(|dir| dir != forge_dir)
        .collect();
    std::env::join_paths(kept).unwrap_or_default()
}

/// `PATH` for the native phase: the host `PATH` minus the directory holding
/// the running Forge binary. Returns the filtered value and whether a
/// `forge` executable still resolves on it (false means the native phase
/// provably runs without Forge).
pub fn native_path_without_forge() -> (OsString, bool) {
    let raw = std::env::var_os("PATH").unwrap_or_default();
    let forge_dir: Option<PathBuf> = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let filtered = match forge_dir {
        Some(dir) => strip_forge_from_path(&raw, &dir),
        None => raw,
    };
    let absent = !forge_resolves_on_path(&filtered);
    (filtered, absent)
}

/// True when a `forge` executable resolves on the given `PATH`-style value.
pub fn forge_resolves_on_path(path_var: &OsStr) -> bool {
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(format!("forge{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Stable source identity over rendered fixture files: SHA-256 over the
/// sorted `rel + 0x00 + contents + 0x00` stream.
pub fn source_hash_for(files: &[(String, String)]) -> String {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, contents) in &sorted {
        hasher.update(rel.as_bytes());
        hasher.update([0u8]);
        hasher.update(contents.as_bytes());
        hasher.update([0u8]);
    }
    format!("{:x}", hasher.finalize())
}

fn now_rfc3339() -> String {
    chrono::DateTime::<chrono::Utc>::from(SystemTime::now()).to_rfc3339()
}

/// The toolchain version evidence one matrix row records: the runtime that
/// was probed, the version the host reports, the profile's declaration and
/// the verdict on the declaration.
struct ToolchainEvidence {
    /// The binary whose version was probed.
    runtime: String,
    /// The version the host reports, when it could be read.
    observed: Option<String>,
    /// The profile's declared requirement.
    declared: Option<String>,
    /// Whether the observation satisfies the declaration.
    satisfied: Option<bool>,
}

impl ToolchainEvidence {
    /// Judge the profile's declaration against the host's observation.
    ///
    /// The runtime carries the version: the Node ecosystem's runtime is
    /// `node`, so `react-web` (toolchain `npm`) is judged on node's
    /// version. A declaration without an observation is unknown, and
    /// unknown is never a mismatch.
    fn judge(descriptor: &ProfileDescriptor, observed: Option<String>) -> Self {
        let declared = descriptor
            .toolchain_version
            .clone()
            .filter(|declaration| crate::profile::version::is_version_requirement(declaration));
        let satisfied = crate::profile::version::judge(
            descriptor.toolchain_version.as_deref(),
            observed.as_deref(),
        );
        Self {
            runtime: crate::profile::version::runtime_binary(&descriptor.toolchain).to_string(),
            observed,
            declared,
            satisfied,
        }
    }
}

fn run_native_command(command: &str, dir: &Path, path_var: &OsStr) -> NativeCommandResult {
    let (program, args) = match crate::native::split_command(command) {
        Ok(split) => split,
        Err(err) => {
            return NativeCommandResult {
                outcome: NativeCommandOutcome {
                    command: command.to_string(),
                    exit_code: None,
                    success: false,
                    evidence: format!("refused to run: {err}"),
                },
                timed_out: false,
            };
        }
    };
    let env = vec![("PATH".to_string(), path_var.to_string_lossy().into_owned())];
    let outcome = match crate::native::run_native(dir, &program, &args, &env) {
        Ok(outcome) => outcome,
        Err(err) => {
            return NativeCommandResult {
                outcome: NativeCommandOutcome {
                    command: command.to_string(),
                    exit_code: None,
                    success: false,
                    evidence: format!("native command '{command}' could not start: {err}"),
                },
                timed_out: false,
            };
        }
    };
    if let Some(start_error) = outcome.start_error {
        return NativeCommandResult {
            outcome: NativeCommandOutcome {
                command: command.to_string(),
                exit_code: None,
                success: false,
                evidence: format!("native command '{command}' could not start: {start_error}"),
            },
            timed_out: false,
        };
    }
    if outcome.timed_out {
        return NativeCommandResult {
            outcome: NativeCommandOutcome {
                command: command.to_string(),
                exit_code: None,
                success: false,
                evidence: format!(
                    "native command '{command}' exceeded the {} timeout; the command's process \
                     group was killed",
                    crate::native::NATIVE_COMMAND_TIMEOUT.as_secs()
                ),
            },
            timed_out: true,
        };
    }
    NativeCommandResult {
        outcome: NativeCommandOutcome {
            command: command.to_string(),
            exit_code: outcome.exit_code,
            success: outcome.success,
            evidence: outcome.tail(2000),
        },
        timed_out: false,
    }
}

/// One native command's result inside a matrix row.
///
/// The timeout flag is kept beside the outcome so the row's reason can
/// name a timeout; the row's serialized shape does not change.
struct NativeCommandResult {
    outcome: NativeCommandOutcome,
    timed_out: bool,
}

/// The reason a matrix row carries for a native command that did not
/// pass: a timeout is named as a timeout, an ordinary failure as a
/// failure.
fn native_failure_reason(command: &str, profile: &str, timed_out: bool) -> String {
    if timed_out {
        format!(
            "native command '{command}' exceeded the {} timeout for profile '{profile}'",
            crate::native::NATIVE_COMMAND_TIMEOUT.as_secs()
        )
    } else {
        format!("native command '{command}' failed for profile '{profile}'")
    }
}

fn unverified_row(
    profile_id: &str,
    toolchain: &str,
    source_hash: String,
    forge_absent: bool,
    reason: String,
    evidence: &ToolchainEvidence,
) -> MatrixRow {
    let descriptor = inspect_profile(profile_id).ok();
    MatrixRow {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        profile: profile_id.to_string(),
        adapter: descriptor
            .as_ref()
            .map(|d| d.adapter.clone())
            .unwrap_or_default(),
        language: descriptor
            .as_ref()
            .map(|d| d.language.clone())
            .unwrap_or_default(),
        toolchain: toolchain.to_string(),
        toolchain_version: evidence.observed.clone(),
        toolchain_runtime: evidence.runtime.clone(),
        toolchain_version_declared: evidence.declared.clone(),
        toolchain_version_satisfied: evidence.satisfied,
        build_command: descriptor
            .as_ref()
            .map(|d| d.build_command.clone())
            .unwrap_or_default(),
        test_command: descriptor
            .as_ref()
            .map(|d| d.test_command.clone())
            .unwrap_or_default(),
        source_hash,
        generated_at: now_rfc3339(),
        forge_absent_from_path: forge_absent,
        result: ReadinessStatus::Unverified,
        reason,
        build: None,
        test: None,
    }
}

/// Run the matrix row for one profile.
///
/// `available` overrides the host `PATH` toolchain probe with a fixed
/// inventory (deterministic for tests); `None` probes the host. An unknown
/// id fails with `unknown-profile` and a planned id with
/// `unsupported-profile` before any directory is created. A missing
/// toolchain yields an `unverified` row, never a pass. A runtime whose
/// version does not satisfy the profile's declared toolchain version is
/// treated like a missing toolchain: the row is `unverified`, the mismatch
/// is named and no native command runs. The fixture is rendered into a
/// disposable directory and the registry is never contacted, so the matrix
/// invents no project and leaves the caller's tree untouched.
pub fn run_profile_row(
    profile_id: &str,
    available: Option<&HashSet<String>>,
) -> Result<MatrixRow, ForgeError> {
    let descriptor = inspect_profile(profile_id)?;
    if descriptor.support_status == ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{profile_id}' is reserved on the catalog as a planned candidate \
                 with no tested template; promote it to a versioned supported \
                 descriptor before selection; no files were changed"
            ),
        });
    }
    let (native_path, forge_absent) = native_path_without_forge();
    let toolchain_present = match available {
        Some(set) => {
            crate::profile::version::inventory_lookup(set, &descriptor.toolchain).is_some()
        }
        None => crate::generate::toolchain_present(&descriptor.toolchain, None),
    };

    let fixture_id = format!("readiness-{}", descriptor.id);
    let staging = tempfile::TempDir::new().map_err(|err| ForgeError::GenerationFailed {
        reason: format!("readiness staging directory could not be created: {err}"),
    })?;
    let target = staging.path().join(&fixture_id);
    let request = normalize_explicit(
        Some(&descriptor.id),
        Some(&fixture_id),
        None,
        &[],
        &target,
        None,
    )?;
    let files = render_files(&request)?;
    let feed_files = render_feed_files(&request)?;
    let source_hash = source_hash_for(&files);

    let runtime = crate::profile::version::runtime_binary(&descriptor.toolchain);

    if !toolchain_present {
        // The toolchain is absent: its version is unknown and no native
        // command runs.
        let evidence = ToolchainEvidence::judge(&descriptor, None);
        return Ok(unverified_row(
            &descriptor.id,
            &descriptor.toolchain,
            source_hash,
            forge_absent,
            format!(
                "toolchain '{}' required by profile '{}' is unavailable locally; profile was not tested",
                descriptor.toolchain, descriptor.id
            ),
            &evidence,
        ));
    }

    // The runtime carries the version: the Node ecosystem's runtime is
    // `node`, so `react-web` (toolchain `npm`) is judged on node's version.
    let observed = match available {
        Some(set) => crate::profile::version::inventory_lookup(set, runtime)
            .and_then(|entry| entry.version.map(str::to_string)),
        None => crate::profile::version::probe_version(runtime, &native_path),
    };
    let evidence = ToolchainEvidence::judge(&descriptor, observed);

    // A runtime of the wrong version is not evidence for this profile: the
    // row is unverified, the mismatch is named and no native command runs.
    if evidence.satisfied == Some(false) {
        let observed = evidence.observed.clone().unwrap_or_default();
        let declared = evidence.declared.clone().unwrap_or_default();
        return Ok(unverified_row(
            &descriptor.id,
            &descriptor.toolchain,
            source_hash,
            forge_absent,
            format!(
                "toolchain '{runtime}' for profile '{}' is version {observed}; the profile \
                 declares {runtime}@{declared}; the profile was not tested",
                descriptor.id
            ),
            &evidence,
        ));
    }

    fs::create_dir_all(&target).map_err(|err| ForgeError::GenerationFailed {
        reason: format!(
            "readiness fixture directory '{}' is unwritable: {err}",
            target.display()
        ),
    })?;
    for (rel, contents) in &files {
        let path = target.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("readiness fixture directory is unwritable: {err}"),
            })?;
        }
        fs::write(&path, contents).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("readiness fixture file '{rel}' is unwritable: {err}"),
        })?;
    }
    for (rel, bytes) in &feed_files {
        let path = target.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("readiness fixture feed directory is unwritable: {err}"),
            })?;
        }
        fs::write(&path, bytes).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("readiness fixture feed package '{rel}' is unwritable: {err}"),
        })?;
    }

    let build = run_native_command(&descriptor.build_command, &target, &native_path);
    if !build.outcome.success {
        return Ok(MatrixRow {
            contract: READINESS_CONTRACT_VERSION.to_string(),
            profile: descriptor.id.clone(),
            adapter: descriptor.adapter.clone(),
            language: descriptor.language.clone(),
            toolchain: descriptor.toolchain.clone(),
            toolchain_version: evidence.observed.clone(),
            toolchain_runtime: evidence.runtime.clone(),
            toolchain_version_declared: evidence.declared.clone(),
            toolchain_version_satisfied: evidence.satisfied,
            build_command: descriptor.build_command.clone(),
            test_command: descriptor.test_command.clone(),
            source_hash,
            generated_at: now_rfc3339(),
            forge_absent_from_path: forge_absent,
            result: ReadinessStatus::Failed,
            reason: native_failure_reason(
                &descriptor.build_command,
                &descriptor.id,
                build.timed_out,
            ),
            build: Some(build.outcome),
            test: None,
        });
    }
    let test = run_native_command(&descriptor.test_command, &target, &native_path);
    if !test.outcome.success {
        return Ok(MatrixRow {
            contract: READINESS_CONTRACT_VERSION.to_string(),
            profile: descriptor.id.clone(),
            adapter: descriptor.adapter.clone(),
            language: descriptor.language.clone(),
            toolchain: descriptor.toolchain.clone(),
            toolchain_version: evidence.observed.clone(),
            toolchain_runtime: evidence.runtime.clone(),
            toolchain_version_declared: evidence.declared.clone(),
            toolchain_version_satisfied: evidence.satisfied,
            build_command: descriptor.build_command.clone(),
            test_command: descriptor.test_command.clone(),
            source_hash,
            generated_at: now_rfc3339(),
            forge_absent_from_path: forge_absent,
            result: ReadinessStatus::Failed,
            reason: native_failure_reason(&descriptor.test_command, &descriptor.id, test.timed_out),
            build: Some(build.outcome),
            test: Some(test.outcome),
        });
    }
    Ok(MatrixRow {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        profile: descriptor.id.clone(),
        adapter: descriptor.adapter.clone(),
        language: descriptor.language.clone(),
        toolchain: descriptor.toolchain.clone(),
        toolchain_version: evidence.observed.clone(),
        toolchain_runtime: evidence.runtime.clone(),
        toolchain_version_declared: evidence.declared.clone(),
        toolchain_version_satisfied: evidence.satisfied,
        build_command: descriptor.build_command.clone(),
        test_command: descriptor.test_command.clone(),
        source_hash,
        generated_at: now_rfc3339(),
        forge_absent_from_path: forge_absent,
        result: ReadinessStatus::Passed,
        reason: format!(
            "native commands '{}' and '{}' passed for profile '{}' without Forge on PATH",
            descriptor.build_command, descriptor.test_command, descriptor.id
        ),
        build: Some(build.outcome),
        test: Some(test.outcome),
    })
}

/// Run the matrix over the selected supported profiles. An empty selection
/// covers every supported profile in stable catalog order. Unknown or
/// planned ids are refused before any fixture is generated.
pub fn run_matrix(selected: &[String]) -> Result<MatrixReport, ForgeError> {
    let ids = if selected.is_empty() {
        matrix_profile_ids()
    } else {
        let mut checked = Vec::with_capacity(selected.len());
        for id in selected {
            let descriptor = inspect_profile(id)?;
            if descriptor.support_status == ProfileSupportStatus::Planned {
                return Err(ForgeError::UnsupportedProfile {
                    reason: format!(
                        "profile '{id}' is reserved on the catalog as a planned candidate \
                         with no tested template; promote it to a versioned supported \
                         descriptor before selection; no files were changed"
                    ),
                });
            }
            if !checked.contains(&descriptor.id) {
                checked.push(descriptor.id);
            }
        }
        checked
    };
    let mut rows = Vec::with_capacity(ids.len());
    for id in &ids {
        rows.push(run_profile_row(id, None)?);
    }
    Ok(summarize_matrix(rows))
}

fn summarize_matrix(rows: Vec<MatrixRow>) -> MatrixReport {
    let passed = rows
        .iter()
        .filter(|r| r.result == ReadinessStatus::Passed)
        .count();
    let failed = rows
        .iter()
        .filter(|r| r.result == ReadinessStatus::Failed)
        .count();
    let unverified = rows
        .iter()
        .filter(|r| r.result == ReadinessStatus::Unverified)
        .count();
    let ready = !rows.is_empty() && failed == 0 && unverified == 0;
    MatrixReport {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        rows,
        passed,
        failed,
        unverified,
        ready,
    }
}

/// Verifiable evidence for the platform-native Forge binary: the running
/// executable's path, its SHA-256, the crate version and a
/// `forge --version` smoke run. A binary that cannot be read or that fails
/// its own version smoke is `readiness-invalid`, never release evidence.
pub fn artifact_evidence() -> Result<ArtifactEvidence, ForgeError> {
    let binary = std::env::current_exe().map_err(|err| ForgeError::ReadinessInvalid {
        reason: format!("readiness artifact binary path is unavailable: {err}"),
    })?;
    artifact_evidence_for(&binary)
}

/// Artifact evidence for an explicit binary path. The CLI uses the running
/// executable; tests target the built `forge` binary so the harness runner
/// itself is never mistaken for the artifact.
pub fn artifact_evidence_for(binary: &Path) -> Result<ArtifactEvidence, ForgeError> {
    let bytes = fs::read(binary).map_err(|err| ForgeError::ReadinessInvalid {
        reason: format!(
            "readiness artifact binary '{}' is unreadable: {err}",
            binary.display()
        ),
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|err| ForgeError::ReadinessInvalid {
            reason: format!("readiness artifact version smoke could not start: {err}"),
        })?;
    if !output.status.success() {
        return Err(ForgeError::ReadinessInvalid {
            reason: "readiness artifact version smoke failed; the binary is not release evidence"
                .to_string(),
        });
    }
    let smoke = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(ArtifactEvidence {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        binary_path: binary.display().to_string(),
        sha256,
        version: env!("CARGO_PKG_VERSION").to_string(),
        version_smoke: smoke,
    })
}

/// Evaluate the release gate over the selected profiles plus the artifact
/// smoke. The report is always returned for stdout rendering so a blocked
/// gate stays observable; the result carries `readiness-not-ready` with the
/// exact failed or unavailable check named, so `check` owns its exit code
/// without hiding the partial report.
pub fn evaluate_gate(selected: &[String]) -> (GateReport, Result<(), ForgeError>) {
    let matrix = match run_matrix(selected) {
        Ok(matrix) => matrix,
        Err(err) => return (empty_gate(), Err(err)),
    };
    let artifact = match artifact_evidence() {
        Ok(artifact) => artifact,
        Err(err) => {
            return (
                GateReport {
                    contract: READINESS_CONTRACT_VERSION.to_string(),
                    matrix,
                    artifact: ArtifactEvidence {
                        contract: READINESS_CONTRACT_VERSION.to_string(),
                        binary_path: String::new(),
                        sha256: String::new(),
                        version: String::new(),
                        version_smoke: String::new(),
                    },
                    ready: false,
                    blocking: vec!["artifact: unreadable".to_string()],
                },
                Err(err),
            );
        }
    };
    let mut blocking: Vec<String> = matrix
        .rows
        .iter()
        .filter(|r| r.result != ReadinessStatus::Passed)
        .map(|r| format!("{}: {} ({})", r.profile, r.result.id(), r.reason))
        .collect();
    if artifact.version_smoke.trim().is_empty() {
        blocking.push("artifact: empty version smoke".to_string());
    }
    let ready = matrix.ready && blocking.is_empty();
    let report = GateReport {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        matrix,
        artifact,
        ready,
        blocking: blocking.clone(),
    };
    if ready {
        (report, Ok(()))
    } else {
        (
            report,
            Err(ForgeError::ReadinessNotReady {
                reason: format!(
                    "release readiness blocked: {}; rerun after the failed or unavailable check passes",
                    blocking.join("; ")
                ),
            }),
        )
    }
}

fn empty_gate() -> GateReport {
    GateReport {
        contract: READINESS_CONTRACT_VERSION.to_string(),
        matrix: MatrixReport {
            contract: READINESS_CONTRACT_VERSION.to_string(),
            rows: Vec::new(),
            passed: 0,
            failed: 0,
            unverified: 0,
            ready: false,
        },
        artifact: ArtifactEvidence {
            contract: READINESS_CONTRACT_VERSION.to_string(),
            binary_path: String::new(),
            sha256: String::new(),
            version: String::new(),
            version_smoke: String::new(),
        },
        ready: false,
        blocking: Vec::new(),
    }
}

/// Human rendering of one matrix row.
pub fn render_row_human(row: &MatrixRow) -> String {
    let mut out = format!(
        "profile={} result={} toolchain={}",
        row.profile,
        row.result.id(),
        row.toolchain,
    );
    // The runtime carries the version: name it when it differs from the
    // profile's toolchain, so the reader knows which binary was probed.
    if row.toolchain_runtime != row.toolchain {
        out.push_str(&format!(" runtime={}", row.toolchain_runtime));
    }
    if let Some(version) = &row.toolchain_version {
        out.push_str(&format!(" version={version}"));
    }
    if let Some(declared) = &row.toolchain_version_declared {
        out.push_str(&format!(" declared={declared}"));
    }
    out.push_str(&format!(
        " source={} forge_absent={}",
        &row.source_hash[..12.min(row.source_hash.len())],
        row.forge_absent_from_path,
    ));
    out.push_str(&format!("\n  reason: {}", row.reason));
    if let Some(build) = &row.build {
        out.push_str(&format!(
            "\n  build: `{}` exit={} success={}",
            build.command,
            build
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            build.success,
        ));
        if !build.evidence.is_empty() && !build.success {
            out.push_str(&format!("\n    evidence: {}", build.evidence));
        }
    }
    if let Some(test) = &row.test {
        out.push_str(&format!(
            "\n  test: `{}` exit={} success={}",
            test.command,
            test.exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string()),
            test.success,
        ));
        if !test.evidence.is_empty() && !test.success {
            out.push_str(&format!("\n    evidence: {}", test.evidence));
        }
    }
    out
}

/// Human rendering of the matrix report.
pub fn render_matrix_human(report: &MatrixReport) -> String {
    let mut lines = vec![format!(
        "readiness matrix (contract {}): passed={} failed={} unverified={} ready={}",
        report.contract, report.passed, report.failed, report.unverified, report.ready,
    )];
    for row in &report.rows {
        lines.push(render_row_human(row));
    }
    lines.join("\n")
}

/// Human rendering of the artifact evidence.
pub fn render_artifact_human(evidence: &ArtifactEvidence) -> String {
    format!(
        "readiness artifact (contract {}):\n  binary: {}\n  sha256: {}\n  version: {}\n  smoke: {}",
        evidence.contract,
        evidence.binary_path,
        evidence.sha256,
        evidence.version,
        evidence.version_smoke,
    )
}

/// Human rendering of the gate report.
pub fn render_gate_human(report: &GateReport) -> String {
    let mut out = render_matrix_human(&report.matrix);
    out.push('\n');
    out.push_str(&render_artifact_human(&report.artifact));
    out.push_str(&format!("\ngate ready={}", report.ready));
    for item in &report.blocking {
        out.push_str(&format!("\n  blocked: {item}"));
    }
    out
}

#[cfg(test)]
mod readiness_tests;

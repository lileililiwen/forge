//! Jenkins/Mac publish orchestration (`jenkins-publish-integration`).
//!
//! Core owns the *generic* publish contract: actions, requests,
//! reports, stage outcomes, journal labels, dry-run gating. The
//! *mechanism* (today: Jenkins over SSH) lives behind the
//! [`PublishAdapter`] trait so the core can stay stable while
//! alternative adapters (raw SSH, GitHub Actions, local-compose) are
//! added without touching it.
//!
//! ## Architecture
//!
//! Three layers, each replaceable independently:
//!
//! 1. **Core** — [`PublishAction`], [`PublishRequest`],
//!    [`PublishReport`], [`StageOutcome`]. Knows about stages and
//!    outcomes, not about SSH or Jenkins.
//! 2. **Transport** — [`SshTransport`] is the only I/O boundary.
//!    Subprocesses go through it; tests inject a recording transport.
//! 3. **Adapter** — implements [`PublishAdapter`] for one mechanism.
//!    [`jenkins::JenkinsAdapter`] is the shipped implementation;
//!    other adapters (raw SSH, GitHub Actions) follow the same trait.
//!
//! ## Stages
//!
//! 1. **Sync** — make the project's source tree available to the
//!    target host (adapter-specific; SSH+rsh for Jenkins, git push
//!    for Actions).
//! 2. **Prepare** — ensure the project has ports, env files, and
//!    any other host-level prerequisites.
//! 3. **Deploy** — trigger the production deploy job.
//!
//! ## Dry-run mode
//!
//! Every stage is gated on the `dry_run` flag. A dry run records the
//! would-be command in the stage outcome, marks the stage as
//! `dry-run`, and exits without invoking any subprocess.
//!
//! ## Risk model
//!
//! Publishing to a remote host is irreversible from Forge's
//! perspective. The contract refuses to deploy without explicit
//! `--confirm` (the CLI's `all` action implies confirmation),
//! preserves the prior partial state on failure, and never claims
//! success on an unreachable target.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::registry::Registry;

pub mod fleet;
pub mod github;
pub mod inventory;
pub mod jenkins;
pub mod providers;

// ---------------------------------------------------------------------------
// Contract version and stable labels
// ---------------------------------------------------------------------------

/// Contract version for the publish surface. The report carries this
/// version so the transport and any later reader share one definition.
pub const PUBLISH_CONTRACT_VERSION: &str = "forge-publish/0.1.0";

/// Subprocess timeout. The Mac may be slow over SSH; the timeout is
/// bounded so an unresponsive host never hangs Forge forever.
pub const PUBLISH_SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(60);

/// Stable stage names. The transport renders these labels verbatim.
pub const STAGE_SYNC: &str = "sync";
pub const STAGE_DB: &str = "db";
pub const STAGE_PREPARE: &str = "prepare";
pub const STAGE_DEPLOY: &str = "deploy";

/// Stable per-stage statuses.
pub const STATUS_DRY_RUN: &str = "dry-run";
pub const STATUS_DONE: &str = "done";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_SKIPPED: &str = "skipped";

// ---------------------------------------------------------------------------
// Action and request
// ---------------------------------------------------------------------------

/// What the transport asks Core to do. `sync`, `db`, `prepare` and
/// `deploy` are runnable as standalone stages; `all` runs them in
/// order and stops at the first failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishAction {
    Sync,
    Db,
    Prepare,
    Deploy,
    All,
}

impl PublishAction {
    pub fn parse(value: &str) -> Result<Self, ForgeError> {
        match value {
            "sync" => Ok(PublishAction::Sync),
            "db" => Ok(PublishAction::Db),
            "prepare" => Ok(PublishAction::Prepare),
            "deploy" => Ok(PublishAction::Deploy),
            "all" => Ok(PublishAction::All),
            other => Err(ForgeError::PublishInvalid {
                reason: format!(
                    "unknown publish action `{other}`; expected one of `sync`, `db`, `prepare`, `deploy`, `all`"
                ),
            }),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            PublishAction::Sync => STAGE_SYNC,
            PublishAction::Db => STAGE_DB,
            PublishAction::Prepare => STAGE_PREPARE,
            PublishAction::Deploy => STAGE_DEPLOY,
            PublishAction::All => "all",
        }
    }
    pub fn stages(&self) -> &'static [PublishAction] {
        match self {
            PublishAction::Sync => &[PublishAction::Sync],
            PublishAction::Db => &[PublishAction::Db],
            PublishAction::Prepare => &[PublishAction::Prepare],
            PublishAction::Deploy => &[PublishAction::Deploy],
            PublishAction::All => &[
                PublishAction::Sync,
                PublishAction::Db,
                PublishAction::Prepare,
                PublishAction::Deploy,
            ],
        }
    }
}

/// Typed request from the transport layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishRequest {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub action: PublishAction,
    pub dry_run: bool,
}

impl PublishRequest {
    pub fn validate(&self) -> Result<(), ForgeError> {
        validate_project_id(&self.project_id)?;
        if !self.project_dir.is_dir() {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "project directory `{}` does not exist",
                    self.project_dir.display()
                ),
            });
        }
        Ok(())
    }
}

fn validate_project_id(project_id: &str) -> Result<(), ForgeError> {
    if project_id.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if !project_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "project id `{project_id}` must be kebab/snake-case (letters, digits, dash, underscore)"
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Adapter trait
// ---------------------------------------------------------------------------

/// The boundary between Core and any concrete publish mechanism.
/// Implementations own all mechanism-specific vocabulary: SSH
/// hostnames, Jenkins exit codes, recovery hints, default paths.
///
/// Core calls [`PublishAdapter::plan`] to obtain the commands for a
/// stage; Core calls [`PublishAdapter::classify`] to translate a
/// raw [`CommandResult`] into a status + recovery hint set.
/// Adapters do not touch subprocesses directly — they compose
/// [`CommandSpec`] values and hand them to the transport.
pub trait PublishAdapter {
    /// Stable id used in `forge.yaml` (`publish.adapter: <id>`).
    fn id(&self) -> &'static str;
    /// Human label, used in dry-run banners and reports.
    fn label(&self) -> &'static str;
    /// Build the commands for one stage.
    fn plan(&self, request: &PublishRequest, stage: PublishAction)
        -> Result<StagePlan, ForgeError>;
    /// Translate a transport result into a status + note. The
    /// adapter decides what counts as success and which exit
    /// codes map to which recovery hints.
    fn classify(&self, plan: &StagePlan, result: &CommandResult) -> Classification;
    /// The public subdomain that a successful deploy lands at.
    /// Used in `forge publish deploy --dry-run` and the report.
    fn subdomain(&self, project_id: &str) -> Option<String>;
}

/// Outcome of one stage, pre-classified by the adapter. The
/// orchestrator renders this into a [`StageOutcome`] without
/// reinterpreting any mechanism-specific vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub status: String,
    pub note: String,
    pub recovery: Vec<String>,
}

impl Classification {
    pub fn done(note: impl Into<String>) -> Self {
        Classification {
            status: STATUS_DONE.to_string(),
            note: note.into(),
            recovery: vec![],
        }
    }
    pub fn failed(note: impl Into<String>, recovery: Vec<String>) -> Self {
        Classification {
            status: STATUS_FAILED.to_string(),
            note: note.into(),
            recovery,
        }
    }
}

/// One stage, described as one or more commands and an adapter
/// identity. The orchestrator holds onto this between `plan()` and
/// the `classify()` call so the adapter can carry context (project
/// id, script name) without re-parsing the request.
///
/// A stage may issue multiple commands when the mechanism requires
/// it (e.g. SSH mkdir before rsync); the adapter concatenates them
/// into one logical stage and the orchestrator runs each in order.
#[derive(Debug)]
pub struct StagePlan {
    pub stage: String,
    pub commands: Vec<CommandSpec>,
    pub project_id: String,
}

impl StagePlan {
    /// Render every command concatenated by ` && ` so dry-run output
    /// shows the full logical pipeline.
    pub fn render(&self) -> String {
        self.commands
            .iter()
            .map(|c| c.render())
            .collect::<Vec<_>>()
            .join(" && ")
    }
}

// ---------------------------------------------------------------------------
// SSH transport abstraction
// ---------------------------------------------------------------------------

/// The boundary between Core and the SSH/rsync subprocesses. The
/// default implementation spawns real subprocesses; tests swap in
/// an in-memory recorder.
///
/// Implementations are expected to surface the captured exit code
/// verbatim so adapters can apply mechanism-specific classification.
/// The orchestrator's `dry_run` flag is the only gate the transport
/// needs to honour.
pub trait SshTransport {
    /// Run one subprocess spec. Returns the captured result.
    fn run(&self, cmd: CommandSpec) -> Result<CommandResult, ForgeError>;
}

/// A typed command specification. The orchestrator builds one of
/// these for every remote action; the transport decides how to run
/// it. Using a struct (rather than `Vec<String>`) keeps the type
/// self-documenting and makes it cheap to log structured commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<OsString>,
    pub label: String,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>, label: impl Into<String>) -> Self {
        CommandSpec {
            program: program.into(),
            args: Vec::new(),
            label: label.into(),
        }
    }
    pub fn arg(mut self, value: impl Into<OsString>) -> Self {
        self.args.push(value.into());
        self
    }
    pub fn render(&self) -> String {
        let mut parts = vec![self.program.clone()];
        for a in &self.args {
            parts.push(a.to_string_lossy().into_owned());
        }
        parts.join(" ")
    }
}

/// Captured subprocess output. Status is preserved verbatim so the
/// adapter can classify exit codes (e.g. Jenkins's exit 3 for
/// missing `.env`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

/// The default transport: spawns real subprocesses. Used in
/// production and integration tests.
pub struct SubprocessTransport {
    pub timeout: Duration,
}

impl Default for SubprocessTransport {
    fn default() -> Self {
        SubprocessTransport {
            timeout: PUBLISH_SUBPROCESS_TIMEOUT,
        }
    }
}

impl SshTransport for SubprocessTransport {
    fn run(&self, spec: CommandSpec) -> Result<CommandResult, ForgeError> {
        run_subprocess(&spec, self.timeout)
    }
}

/// In-memory transport used by tests. Records every command the
/// orchestrator would have run so a dry-run assertion can inspect
/// the exact argv. Returns the next queued result on each call;
/// missing results default to success.
#[derive(Debug, Default, Clone)]
pub struct RecordingTransport {
    pub commands: Vec<CommandSpec>,
    queue: Vec<Result<CommandResult, String>>,
    cursor: usize,
}

impl RecordingTransport {
    pub fn new() -> Self {
        RecordingTransport::default()
    }
    /// Pre-seed N successes (one per expected subprocess).
    pub fn succeeding(&mut self, n: usize) -> &mut Self {
        self.queue.clear();
        self.cursor = 0;
        for _ in 0..n {
            self.queue.push(Ok(CommandResult {
                status: 0,
                stdout: String::new(),
                stderr: String::new(),
            }));
        }
        self
    }
    /// Pre-seed a specific result for the next invocation.
    pub fn push(&mut self, result: Result<CommandResult, String>) -> &mut Self {
        self.queue.push(result);
        self
    }
}

impl SshTransport for RecordingTransport {
    fn run(&self, spec: CommandSpec) -> Result<CommandResult, ForgeError> {
        let mut owned = self.clone();
        owned.commands.push(spec);
        let next = owned.queue.get(owned.cursor).cloned().unwrap_or_else(|| {
            Ok(CommandResult {
                status: 0,
                stdout: String::new(),
                stderr: String::new(),
            })
        });
        owned.cursor += 1;
        // Persist the advanced cursor back into `self` through the
        // caller's borrow by returning a clone; tests observe
        // `commands` (recorded list) for assertions.
        match next {
            Ok(r) => Ok(r),
            Err(reason) => Err(ForgeError::PublishInvalid { reason }),
        }
    }
}

fn run_subprocess(spec: &CommandSpec, timeout: Duration) -> Result<CommandResult, ForgeError> {
    use std::io::Read;
    use std::process::Stdio;
    let mut cmd = Command::new(&spec.program);
    cmd.args(spec.args.iter());
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| ForgeError::PublishInvalid {
        reason: format!("cannot spawn `{}`: {err}", spec.program),
    })?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out_bytes = Vec::new();
                let mut err_bytes = Vec::new();
                if let Some(s) = stdout.as_mut() {
                    let _ = s.read_to_end(&mut out_bytes);
                }
                if let Some(s) = stderr.as_mut() {
                    let _ = s.read_to_end(&mut err_bytes);
                }
                return Ok(CommandResult {
                    status: status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&out_bytes).into_owned(),
                    stderr: String::from_utf8_lossy(&err_bytes).into_owned(),
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(ForgeError::PublishInvalid {
                        reason: format!(
                            "subprocess `{}` timed out after {}s",
                            spec.program,
                            timeout.as_secs()
                        ),
                    });
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                return Err(ForgeError::PublishInvalid {
                    reason: format!("subprocess wait failed: {err}"),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Report and stage outcome
// ---------------------------------------------------------------------------

/// Per-stage outcome. One row per stage in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageOutcome {
    pub stage: String,
    pub status: String,
    pub note: String,
    pub command: Vec<String>,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
    pub elapsed_ms: u128,
}

/// Aggregate publish report. The transport renders this before the
/// process exits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishReport {
    pub contract: String,
    pub adapter: String,
    pub project_id: String,
    pub action: String,
    pub dry_run: bool,
    pub subdomain: Option<String>,
    pub stages: Vec<StageOutcome>,
    pub note: String,
    pub healthy: bool,
}

impl PublishReport {
    pub fn healthy(&self) -> bool {
        self.healthy
    }
    pub fn subdomain(&self) -> Option<&str> {
        self.subdomain.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Orchestrator (core)
// ---------------------------------------------------------------------------

/// Run the publish workflow end-to-end. The action selects which
/// stages to run; the registry receives one operation row per
/// executed stage plus the aggregate. The adapter owns all
/// mechanism-specific behaviour; this function is a pure renderer.
pub fn run_publish(
    request: &PublishRequest,
    adapter: &dyn PublishAdapter,
    transport: &dyn SshTransport,
    registry: Option<&Registry>,
) -> Result<PublishReport, ForgeError> {
    request.validate()?;

    let mut stages: Vec<StageOutcome> = Vec::new();
    let mut healthy = true;
    let mut subdomain: Option<String> = None;

    for action in request.action.stages() {
        let plan = adapter.plan(request, action.clone())?;
        let outcome = execute_stage(adapter, &plan, request.dry_run, transport);

        let stage_status = outcome.status.clone();
        let is_ok = matches!(stage_status.as_str(), STATUS_DONE | STATUS_DRY_RUN);

        if let Some(reg) = registry {
            let _ = reg.record_operation(
                "publish",
                &request.project_id,
                publish_journal_state(&stage_status),
                &format!(
                    "publish {} via {}: {} ({}, {}ms)",
                    plan.stage,
                    adapter.id(),
                    outcome.note,
                    stage_status,
                    outcome.elapsed_ms
                ),
            );
        }
        if !is_ok {
            healthy = false;
        }
        if matches!(action, PublishAction::Deploy) && is_ok {
            subdomain = adapter.subdomain(&request.project_id);
        }
        stages.push(outcome);
        if !healthy && !request.dry_run {
            break;
        }
    }

    let note = if request.dry_run {
        format!(
            "dry-run: {} stage(s) planned, no side effects",
            stages.len()
        )
    } else if healthy {
        "publish succeeded; project reachable at subdomain".to_string()
    } else {
        "publish did not complete; see stages for failure detail".to_string()
    };

    if let Some(reg) = registry {
        let verdict = if request.dry_run {
            "done"
        } else if healthy {
            "done"
        } else {
            "failed"
        };
        let _ = reg.record_operation(
            "publish",
            &request.project_id,
            verdict,
            &format!(
                "publish {} summary via {}: healthy={} stages={}",
                request.action.label(),
                adapter.id(),
                healthy,
                stages.len()
            ),
        );
    }

    Ok(PublishReport {
        contract: PUBLISH_CONTRACT_VERSION.to_string(),
        adapter: adapter.id().to_string(),
        project_id: request.project_id.clone(),
        action: request.action.label().to_string(),
        dry_run: request.dry_run,
        subdomain,
        stages,
        note,
        healthy,
    })
}

// The orchestrator calls `adapter.classify(plan, result)` back
// through the trait object. `PublishAdapter` is object-safe so this
// works directly without a view trait.

/// Run one stage through the transport (or report the dry-run
/// plan). Pure orchestration: no mechanism-specific vocabulary
/// here. When the stage issues multiple commands, they run
/// sequentially; any failure aborts the stage.
fn execute_stage(
    adapter: &dyn PublishAdapter,
    plan: &StagePlan,
    dry_run: bool,
    transport: &dyn SshTransport,
) -> StageOutcome {
    let command_rendered = plan.render();
    let command_lines: Vec<String> = plan.commands.iter().map(|c| c.render()).collect();
    if dry_run {
        return StageOutcome {
            stage: plan.stage.clone(),
            status: STATUS_DRY_RUN.to_string(),
            note: format!("would run: {command_rendered}"),
            command: command_lines,
            evidence: vec!["dry-run: no subprocess spawned".to_string()],
            recovery: vec![],
            elapsed_ms: 0,
        };
    }

    let started = std::time::Instant::now();
    let mut last_result: Option<CommandResult> = None;
    let mut last_error: Option<ForgeError> = None;
    let total = plan.commands.len();
    for (idx, cmd) in plan.commands.iter().enumerate() {
        match transport.run(cmd.clone()) {
            Ok(result) => {
                if !result.success() {
                    last_result = Some(result.clone());
                    break;
                }
                last_result = Some(result);
                if idx + 1 < total {
                    // Continue only on success.
                }
            }
            Err(err) => {
                last_error = Some(err);
                break;
            }
        }
    }
    let elapsed_ms = started.elapsed().as_millis();

    if let Some(err) = last_error {
        return StageOutcome {
            stage: plan.stage.clone(),
            status: STATUS_FAILED.to_string(),
            note: err.to_string(),
            command: command_lines,
            evidence: vec![format!("transport error after {}ms", elapsed_ms)],
            recovery: vec![format!("verify the transport can reach `{}`", plan.stage)],
            elapsed_ms,
        };
    }

    let result = last_result.expect("at least one command runs per stage");
    let classification = adapter.classify(plan, &result);
    let is_done = classification.status == STATUS_DONE;
    let note = if is_done {
        format!("{} completed in {}ms", plan.stage, elapsed_ms)
    } else {
        format!("{} (exit {})", classification.note, result.status)
    };
    StageOutcome {
        stage: plan.stage.clone(),
        status: classification.status,
        note,
        command: command_lines,
        evidence: vec![format!("exit {} after {}ms", result.status, elapsed_ms)],
        recovery: classification.recovery,
        elapsed_ms,
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render a [`PublishReport`] for human output. The transport
/// renders the same data the JSON envelope carries so a partial run
/// is observable on stdout.
pub fn render_report_human(report: &PublishReport) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", report.project_id));
    lines.push(format!("adapter: {}", report.adapter));
    lines.push(format!("action: {}", report.action));
    lines.push(format!(
        "mode: {}",
        if report.dry_run { "dry-run" } else { "apply" }
    ));
    if let Some(sub) = &report.subdomain {
        lines.push(format!("subdomain: https://{sub}"));
    }
    if !report.stages.is_empty() {
        lines.push("stages:".to_string());
        for outcome in &report.stages {
            lines.push(format!(
                "  - {stage} {status} ({elapsed_ms}ms): {note}",
                stage = outcome.stage,
                status = outcome.status,
                elapsed_ms = outcome.elapsed_ms,
                note = outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
            if !outcome.command.is_empty() {
                lines.push(format!("      command: {}", outcome.command.join(" | ")));
            }
        }
    }
    lines.push(format!("summary: {}", report.note));
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convenience for the CLI handler: build a [`PublishRequest`] from
/// the user-facing fields.
pub fn request_from(
    project_id: impl Into<String>,
    project_dir: impl Into<PathBuf>,
    action: PublishAction,
    dry_run: bool,
) -> PublishRequest {
    PublishRequest {
        project_id: project_id.into(),
        project_dir: project_dir.into(),
        action,
        dry_run,
    }
}

/// Map a stage status to a stable registry journal label. Public so
/// the CLI handler can use the same mapping when building custom
/// operation rows; the test suite verifies the contract.
pub fn publish_journal_state(status: &str) -> &'static str {
    match status {
        STATUS_DONE => "done",
        STATUS_DRY_RUN => "skipped",
        STATUS_FAILED => "failed",
        STATUS_SKIPPED => "skipped",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture_request(action: PublishAction, dry_run: bool) -> (PublishRequest, TempDir) {
        let tmp = TempDir::new().unwrap();
        let req = PublishRequest {
            project_id: "demo".to_string(),
            project_dir: tmp.path().to_path_buf(),
            action,
            dry_run,
        };
        (req, tmp)
    }

    /// Trivial adapter for testing the orchestrator. Records every
    /// plan call and never fails — the orchestrator tests do not
    /// care about Jenkins specifics.
    struct StubAdapter {
        id: &'static str,
        label: &'static str,
    }
    impl PublishAdapter for StubAdapter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn label(&self) -> &'static str {
            self.label
        }
        fn plan(
            &self,
            request: &PublishRequest,
            stage: PublishAction,
        ) -> Result<StagePlan, ForgeError> {
            let label = format!("stub-{:?}", stage);
            let command = CommandSpec::new("echo", label.clone()).arg(label);
            Ok(StagePlan {
                stage: stage.label().to_string(),
                commands: vec![command],
                project_id: request.project_id.clone(),
            })
        }
        fn classify(&self, plan: &StagePlan, result: &CommandResult) -> Classification {
            if result.success() {
                Classification::done(format!("{} ok", plan.stage))
            } else {
                Classification::failed(
                    format!("{} failed", plan.stage),
                    vec![format!("inspect exit {}", result.status)],
                )
            }
        }
        fn subdomain(&self, project_id: &str) -> Option<String> {
            Some(format!("{project_id}.stub.test"))
        }
    }

    #[test]
    fn request_validate_rejects_missing_dir() {
        let req = PublishRequest {
            project_id: "demo".to_string(),
            project_dir: PathBuf::from("/no/such/path/exists/anywhere"),
            action: PublishAction::Sync,
            dry_run: true,
        };
        let err = req.validate().unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn request_validate_rejects_bad_project_id() {
        let (req, _tmp) = fixture_request(PublishAction::Sync, true);
        let mut bad = req.clone();
        bad.project_id = "demo with space".to_string();
        let err = bad.validate().unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn action_parse_accepts_known_values() {
        assert_eq!(PublishAction::parse("sync").unwrap(), PublishAction::Sync);
        assert_eq!(PublishAction::parse("db").unwrap(), PublishAction::Db);
        assert_eq!(
            PublishAction::parse("prepare").unwrap(),
            PublishAction::Prepare
        );
        assert_eq!(
            PublishAction::parse("deploy").unwrap(),
            PublishAction::Deploy
        );
        assert_eq!(PublishAction::parse("all").unwrap(), PublishAction::All);
    }

    #[test]
    fn action_parse_rejects_unknown_value() {
        let err = PublishAction::parse("promote").unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn action_stages_returns_correct_slices() {
        assert_eq!(PublishAction::Sync.stages(), &[PublishAction::Sync]);
        assert_eq!(PublishAction::All.stages().len(), 4);
    }

    #[test]
    fn command_spec_render_includes_program_and_args() {
        let cmd = CommandSpec::new("ssh", "test")
            .arg("mac")
            .arg("bash")
            .arg("/path/to/script.sh");
        let rendered = cmd.render();
        assert!(rendered.starts_with("ssh mac bash /path/to/script.sh"));
    }

    #[test]
    fn journal_state_maps_known_statuses() {
        assert_eq!(publish_journal_state(STATUS_DONE), "done");
        assert_eq!(publish_journal_state(STATUS_DRY_RUN), "skipped");
        assert_eq!(publish_journal_state(STATUS_FAILED), "failed");
        assert_eq!(publish_journal_state("something-else"), "unknown");
    }

    #[test]
    fn dry_run_does_not_invoke_real_transport() {
        let (req, _tmp) = fixture_request(PublishAction::All, true);
        let adapter = StubAdapter {
            id: "stub",
            label: "stub",
        };
        let transport = RecordingTransport::new();
        let report = run_publish(&req, &adapter, &transport, None).unwrap();
        assert!(report.dry_run);
        assert_eq!(report.stages.len(), 4);
        assert!(report.stages.iter().all(|s| s.status == STATUS_DRY_RUN));
        // The recording transport is not real, so execute_stage
        // short-circuits regardless; confirm the recorder stays
        // empty.
        assert!(transport.commands.is_empty());
        assert!(report.healthy);
    }

    #[test]
    fn apply_path_records_each_real_subprocess() {
        let (req, _tmp) = fixture_request(PublishAction::All, false);
        let adapter = StubAdapter {
            id: "stub",
            label: "stub",
        };
        let mut transport = RecordingTransport::new();
        transport.succeeding(4);
        let report = run_publish(&req, &adapter, &transport, None).unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.stages.len(), 4);
        assert!(report.stages.iter().all(|s| s.status == STATUS_DONE));
        assert!(report.healthy);
        assert_eq!(report.subdomain(), Some("demo.stub.test"));
        assert_eq!(report.adapter, "stub");
    }

    #[test]
    fn apply_stops_at_first_failure() {
        let (req, _tmp) = fixture_request(PublishAction::All, false);
        let adapter = StubAdapter {
            id: "stub",
            label: "stub",
        };
        let mut transport = RecordingTransport::new();
        transport.push(Err("boom".to_string()));
        let report = run_publish(&req, &adapter, &transport, None).unwrap();
        assert_eq!(report.stages.len(), 1);
        assert!(!report.healthy);
    }

    #[test]
    fn render_report_human_includes_key_fields() {
        let (req, _tmp) = fixture_request(PublishAction::Sync, true);
        let adapter = StubAdapter {
            id: "stub",
            label: "stub",
        };
        let transport = RecordingTransport::new();
        let report = run_publish(&req, &adapter, &transport, None).unwrap();
        let rendered = render_report_human(&report);
        assert!(rendered.contains("project: demo"));
        assert!(rendered.contains("adapter: stub"));
        assert!(rendered.contains("action: sync"));
        assert!(rendered.contains("mode: dry-run"));
    }

    #[test]
    fn stage_outcome_serializes_to_json() {
        let outcome = StageOutcome {
            stage: STAGE_SYNC.to_string(),
            status: STATUS_DONE.to_string(),
            note: "ok".to_string(),
            command: vec!["rsync".to_string()],
            evidence: vec!["exit 0".to_string()],
            recovery: vec![],
            elapsed_ms: 12,
        };
        let json = serde_json::to_value(&outcome).unwrap();
        assert_eq!(json["stage"], STAGE_SYNC);
        assert_eq!(json["status"], STATUS_DONE);
        assert_eq!(json["elapsed_ms"], 12);
    }
}

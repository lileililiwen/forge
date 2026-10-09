//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use super::contract::{
    STAGE_DB, STAGE_DEPLOY, STAGE_PREPARE, STAGE_SYNC, STATUS_DONE, STATUS_FAILED,
};
use super::pipeline::validate_project_id;

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
            other => {
                Err(ForgeError::PublishInvalid {
                    reason: format!(
                        "unknown publish action `{other}`; expected one of `sync`, `db`, `prepare`, `deploy`, `all`"
                    ),
                })
            }
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
/// In-memory transport used by tests. Records every command the
/// orchestrator would have run so a dry-run assertion can inspect
/// the exact argv, and hands out the next queued result on each call;
/// missing results default to success.
#[derive(Debug, Default)]
pub struct RecordingTransport {
    pub(super) recorded: RefCell<Vec<CommandSpec>>,
    pub(super) queue: Vec<Result<CommandResult, String>>,
    pub(super) cursor: Cell<usize>,
}
impl RecordingTransport {
    pub fn new() -> Self {
        RecordingTransport::default()
    }
    /// Pre-seed N successes (one per expected subprocess).
    pub fn succeeding(&mut self, n: usize) -> &mut Self {
        self.queue.clear();
        self.cursor.set(0);
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
    /// Every command the orchestrator handed to this transport, in
    /// invocation order.
    pub fn commands(&self) -> Vec<CommandSpec> {
        self.recorded.borrow().clone()
    }
    pub fn command_count(&self) -> usize {
        self.recorded.borrow().len()
    }
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
/// The default transport: spawns real subprocesses. Used in
/// production and integration tests.
pub struct SubprocessTransport {
    pub timeout: Duration,
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
    /// Per-command timeout override. `None` means the transport's
    /// default bound; adapters set it for stages whose runtime
    /// legitimately exceeds the interactive default (container
    /// builds). The timeout never renders into dry-run plans.
    pub timeout: Option<Duration>,
    /// Serialise this command against every other exclusive command
    /// in the process. Used for a shared single-container mutation
    /// (the platform router reload) that two fleet workers must
    /// never run at once (`fleet-live-rollout` live evidence
    /// 2026-09-30). Never renders into dry-run plans.
    pub exclusive: bool,
}
impl CommandSpec {
    pub fn new(program: impl Into<String>, label: impl Into<String>) -> Self {
        CommandSpec {
            program: program.into(),
            args: Vec::new(),
            label: label.into(),
            timeout: None,
            exclusive: false,
        }
    }
    pub fn arg(mut self, value: impl Into<OsString>) -> Self {
        self.args.push(value.into());
        self
    }
    /// Append several arguments in one call. `arg` consumes the spec,
    /// so a long argv reads better as a single call.
    pub fn with_args<I, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<OsString>,
    {
        self.args
            .extend(values.into_iter().map(|value| value.into()));
        self
    }
    /// Override the transport's default subprocess bound for this
    /// command (container builds). The value never renders into
    /// dry-run output.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    /// Mark the command exclusive: the transport runs at most one
    /// exclusive command at a time across all threads in the
    /// process. See [`CommandSpec::exclusive`].
    pub fn with_exclusive(mut self) -> Self {
        self.exclusive = true;
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

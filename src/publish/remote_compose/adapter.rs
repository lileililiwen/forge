//! # `RemoteComposeAdapter` - Trait Implementations
//!
//! This module contains trait implementations for `RemoteComposeAdapter`.
//!
//! ## Implemented Traits
//!
//! - `PublishAdapter`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::contract::{STAGE_DB, STAGE_DEPLOY, STAGE_PREPARE, STAGE_SYNC};
use super::super::{
    Classification, CommandResult, PublishAction, PublishAdapter, PublishRequest, SshTransport,
    StagePlan,
};
use crate::core::ForgeError;

use super::constants::{ADAPTER_ID, COMPOSE_CANDIDATES};
use super::model::{RemoteComposeAdapter, RemoteState};
use super::stages::{
    classify_db, classify_deploy, classify_generic, classify_prepare, classify_sync,
    compose_profiles, local_compose_file,
};

impl PublishAdapter for RemoteComposeAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn label(&self) -> &'static str {
        "Remote compose (generic Docker host)"
    }

    fn materialize(
        &self,
        request: &PublishRequest,
        transport: &dyn SshTransport,
        dry_run: bool,
    ) -> Result<(), ForgeError> {
        if !request.action.stages().contains(&PublishAction::Prepare) {
            return Ok(());
        }
        self.config.validate()?;
        let project_id = request.project_id.clone();
        let mut state = RemoteState {
            compose_file: local_compose_file(&request.project_dir),
            ..RemoteState::default()
        };
        // Profiles come from the local checkout (the exact tree Sync
        // ships), so both dry-run previews and real runs render the
        // same `--profile` selection without another target round
        // trip. Absent or unreadable means no profiles, as before.
        if let Some(file) = state.compose_file.clone() {
            state.profiles = compose_profiles(&request.project_dir, &file);
            if !state.profiles.is_empty() {
                state.notes.push(format!(
                    "compose declares profile(s) {}; deploys enable all of them",
                    state.profiles.join(", ")
                ));
            }
        }

        if !dry_run {
            state.registry = self.read_registry(transport, &mut state)?;
            if state.compose_file.is_none() {
                match self.resolve_compose_file(transport, &project_id) {
                    Ok(candidate) => state.compose_file = Some(candidate),
                    Err(error) => {
                        return Err(ForgeError::DeployTargetUnavailable {
                            reason: format!(
                                "{error}; no local Compose file either (looked for {})",
                                COMPOSE_CANDIDATES.join(", ")
                            ),
                        });
                    }
                }
            }
            state.compose =
                self.read_compose_config(transport, &mut state, &project_id, &request.project_dir)?;
            state.unavailable = self.read_unavailable_ports(transport, &mut state)?;
            state.shared_db_env =
                self.probe_target_file(transport, &self.config.shared_db_env_path(&project_id));
            state.project_env =
                self.probe_target_file(transport, &self.config.project_env_path(&project_id));
        }
        state.notes.push(if dry_run {
            "dry-run: target registry and Compose config were not read; \
             the rendered documents preview an empty registry"
                .to_string()
        } else {
            format!(
                "target registry {} carries {} project(s)",
                self.config.registry_path(),
                state.registry.projects.len()
            )
        });

        self.render_artifacts(&mut state, &project_id, dry_run)?;
        self.state.replace(state);
        Ok(())
    }

    fn plan(
        &self,
        request: &PublishRequest,
        stage: PublishAction,
    ) -> Result<StagePlan, ForgeError> {
        match stage {
            PublishAction::Sync => Ok(self.plan_sync(request)?),
            PublishAction::Db => Ok(self.plan_db(request)?),
            PublishAction::Prepare => Ok(self.plan_prepare(request)?),
            PublishAction::Deploy => Ok(self.plan_deploy(request)?),
            PublishAction::All => Err(ForgeError::PublishInvalid {
                reason: "PublishAction::All cannot be planned; it expands via the orchestrator"
                    .to_string(),
            }),
        }
    }

    fn classify(&self, plan: &StagePlan, result: &CommandResult) -> Classification {
        match plan.stage.as_str() {
            STAGE_SYNC => classify_sync(result, &self.config),
            STAGE_DB => classify_db(result),
            STAGE_PREPARE => classify_prepare(result, &self.config),
            STAGE_DEPLOY => classify_deploy(result, &self.config),
            _ => classify_generic(result),
        }
    }

    fn subdomain(&self, project_id: &str) -> Option<String> {
        Some(format!("{project_id}.{}", self.config.domain))
    }
}

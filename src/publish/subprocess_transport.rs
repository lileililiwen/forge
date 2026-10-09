//! # `SubprocessTransport` - Trait Implementations
//!
//! This module contains trait implementations for `SubprocessTransport`.
//!
//! ## Implemented Traits
//!
//! - `Default`
//! - `SshTransport`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;

use super::contract::PUBLISH_SUBPROCESS_TIMEOUT;
use super::model::{CommandResult, CommandSpec, SubprocessTransport};
use super::pipeline::{run_subprocess, SshTransport};

impl Default for SubprocessTransport {
    fn default() -> Self {
        SubprocessTransport {
            timeout: PUBLISH_SUBPROCESS_TIMEOUT,
        }
    }
}
impl SshTransport for SubprocessTransport {
    fn run(&self, spec: CommandSpec) -> Result<CommandResult, ForgeError> {
        let timeout = spec.timeout.unwrap_or(self.timeout);
        if !spec.exclusive {
            return run_subprocess(&spec, timeout);
        }
        // Exclusive commands mutate a shared single resource (the
        // platform router container), so fleet worker threads must
        // run them one at a time: concurrent
        // `docker compose up -d --force-recreate` calls race and one
        // fails with `Conflict. The container name … is already in
        // use` (`fleet-live-rollout` live evidence 2026-09-30). The
        // lock is process-global and never held across a
        // non-exclusive command, so parallel builds stay parallel.
        static EXCLUSIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = EXCLUSIVE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        run_subprocess(&spec, timeout)
    }
}

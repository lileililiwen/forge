//! # `RecordingTransport` - Trait Implementations
//!
//! This module contains trait implementations for `RecordingTransport`.
//!
//! ## Implemented Traits
//!
//! - `SshTransport`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;

use super::model::{CommandResult, CommandSpec, RecordingTransport};
use super::pipeline::SshTransport;

impl SshTransport for RecordingTransport {
    fn run(&self, spec: CommandSpec) -> Result<CommandResult, ForgeError> {
        self.recorded.borrow_mut().push(spec);
        let cursor = self.cursor.get();
        self.cursor.set(cursor + 1);
        match self.queue.get(cursor).cloned() {
            Some(Ok(result)) => Ok(result),
            Some(Err(reason)) => Err(ForgeError::PublishInvalid { reason }),
            None => Ok(CommandResult {
                status: 0,
                stdout: String::new(),
                stderr: String::new(),
            }),
        }
    }
}

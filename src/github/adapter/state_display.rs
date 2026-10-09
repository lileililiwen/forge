//! # `GithubState` - Trait Implementations
//!
//! This module contains trait implementations for `GithubState`.
//!
//! ## Implemented Traits
//!
//! - `Display`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::GithubState;

impl std::fmt::Display for GithubState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

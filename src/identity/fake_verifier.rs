//! # `FakeBrowserAuthVerifier` - Trait Implementations
//!
//! This module contains trait implementations for `FakeBrowserAuthVerifier`.
//!
//! ## Implemented Traits
//!
//! - `Default`
//! - `BrowserAuthVerifier`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use chrono::{DateTime, Utc};

use super::browser_auth::BrowserAuthVerifier;
use super::model::{
    AuthCallback, AuthChallenge, FakeBrowserAuthVerifier, IdentityConfig, ProviderClaims,
};

impl Default for FakeBrowserAuthVerifier {
    fn default() -> Self {
        Self::new()
    }
}
impl BrowserAuthVerifier for FakeBrowserAuthVerifier {
    fn exchange_and_verify(
        &self,
        _callback: &AuthCallback,
        _challenge: &AuthChallenge,
        _config: &IdentityConfig,
        _now: DateTime<Utc>,
    ) -> Result<ProviderClaims, ForgeError> {
        let mut queue = self.queue.lock().expect("fake verifier mutex");
        match queue.pop() {
            Some(outcome) => outcome,
            None => Err(ForgeError::IdentityAuthFailed {
                reason: "FakeBrowserAuthVerifier has no queued outcome; queue a \
                         success or error before driving the callback"
                    .to_string(),
            }),
        }
    }
}

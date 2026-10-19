//! Clock abstraction.
//!
//! [`Clock::now`] returns the current instant. [`SystemClock`] is the
//! production implementation (delegates to [`time::OffsetDateTime::now_utc`]).
//! [`FixedClock`] is for tests; it returns a configured instant and is
//! `Send + Sync` so it can live behind an `Arc` in concurrent fixtures.

use std::sync::Mutex;
use std::time::Duration;

use time::OffsetDateTime;

/// A source of "now" values.
///
/// Implementations MUST return an `OffsetDateTime` expressed in UTC.
/// Returning local-time values would make every log line and audit
/// event ambiguous.
pub trait Clock: Send + Sync {
    /// Current instant in UTC.
    fn now(&self) -> OffsetDateTime;
}

/// Production clock backed by the OS clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Test clock. Holds an instant under a `Mutex` so it is `Send + Sync`
/// and can be shared across threads in parallel test runs.
#[derive(Debug)]
pub struct FixedClock {
    inner: Mutex<OffsetDateTime>,
}

impl FixedClock {
    /// Construct a clock that always returns `start`.
    #[must_use]
    pub fn new(start: OffsetDateTime) -> Self {
        Self {
            inner: Mutex::new(start),
        }
    }

    /// Construct a clock initialised at the Unix epoch.
    #[must_use]
    pub fn at_epoch() -> Self {
        Self::new(OffsetDateTime::UNIX_EPOCH)
    }

    /// Replace the configured instant.
    pub fn set(&self, instant: OffsetDateTime) {
        *self.inner.lock().expect("FixedClock mutex poisoned") = instant;
    }

    /// Advance the configured instant by `delta`.
    pub fn advance(&self, delta: Duration) {
        let mut guard = self.inner.lock().expect("FixedClock mutex poisoned");
        *guard += time::Duration::try_from(delta).expect("duration in range");
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        *self.inner.lock().expect("FixedClock mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_configured_instant() {
        let instant = OffsetDateTime::UNIX_EPOCH;
        let clock = FixedClock::new(instant);
        assert_eq!(clock.now(), instant);
    }

    #[test]
    fn fixed_clock_advances() {
        let clock = FixedClock::at_epoch();
        clock.advance(Duration::from_secs(60));
        assert_eq!(
            clock.now(),
            OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(60)
        );
    }

    #[test]
    fn fixed_clock_set_overrides() {
        let clock = FixedClock::at_epoch();
        let next = OffsetDateTime::UNIX_EPOCH + time::Duration::days(1);
        clock.set(next);
        assert_eq!(clock.now(), next);
    }
}

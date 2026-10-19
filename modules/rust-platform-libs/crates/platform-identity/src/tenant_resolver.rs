//! Tenant resolver trait and reference implementations.
//!
//! The application owns the tenant store and the policy that
//! decides which `TenantContext` a `Subject` should receive. The
//! platform exposes only the trait shape and a `NoTenantResolver`
//! placeholder that returns `IdentityError::Misconfigured` for
//! every input. The placeholder is useful for wiring, for tests
//! that exercise the failure path, and as a documented signal
//! that the application has not yet committed to a real store.
//!
//! All implementations are synchronous. Asynchronous adapters wrap
//! the call in a runtime of their choice.

use platform_core::clock::Clock;
use platform_core::correlation::CorrelationId;

use crate::error::{IdentityError, IdentityFailure};
use crate::outcome::IdentityOutcome;
use crate::subject::Subject;
use crate::tenant::{Environment, IsolationLevel, TenantContext, TenantId};
use time::OffsetDateTime;

/// Application-implemented tenant resolver.
///
/// The resolver maps a resolved [`Subject`] to a [`TenantContext`]
/// for the current request. The application decides the policy
/// (header-driven, claim-driven, default-per-subject, ...) and
/// the store (database, cache, file, ...).
///
/// The trait receives the [`Clock`] so the resolver can stamp
/// `active_at` with the application's "now" policy rather than a
/// fresh `OffsetDateTime::now_utc()` at the call site.
pub trait TenantResolver: Send + Sync {
    /// Resolve the tenant context for `subject`.
    fn resolve(
        &self,
        subject: &Subject,
        clock: &dyn Clock,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<TenantContext>;
}

/// Resolver that always reports
/// `IdentityError::Misconfigured`. Useful as a placeholder during
/// wiring and as the explicit "no resolver" signal in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoTenantResolver;

impl TenantResolver for NoTenantResolver {
    fn resolve(
        &self,
        _subject: &Subject,
        _clock: &dyn Clock,
        correlation_id: CorrelationId,
    ) -> IdentityOutcome<TenantContext> {
        IdentityOutcome::Failed(
            IdentityError::new(
                IdentityFailure::Misconfigured,
                "tenant resolver is not configured",
            )
            .with_correlation(correlation_id),
        )
    }
}

/// Static resolver that returns the same [`TenantContext`] for
/// every subject. Useful in single-tenant applications, in
/// migration periods, and in tests that need a working resolver
/// without configuring a store.
#[derive(Debug, Clone)]
pub struct StaticTenantResolver {
    tenant_id: TenantId,
    environment: Environment,
    isolation_level: IsolationLevel,
}

impl StaticTenantResolver {
    /// Construct a static resolver.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        environment: Environment,
        isolation_level: IsolationLevel,
    ) -> Self {
        Self {
            tenant_id,
            environment,
            isolation_level,
        }
    }
}

impl TenantResolver for StaticTenantResolver {
    fn resolve(
        &self,
        _subject: &Subject,
        clock: &dyn Clock,
        _correlation_id: CorrelationId,
    ) -> IdentityOutcome<TenantContext> {
        IdentityOutcome::Authenticated(TenantContext::new(
            self.tenant_id.clone(),
            self.environment,
            self.isolation_level,
            OffsetDateTime::from_unix_timestamp(0).unwrap_or_else(|_| clock.now()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subject::{Subject, SubjectId, SubjectType};
    use crate::tenant::TenantTag;
    use platform_core::clock::SystemClock;
    use platform_core::id::Id;

    fn subject() -> Subject {
        let tenant = Id::<TenantTag>::new("tnt_test");
        Subject::new(SubjectId::new("usr_1"), SubjectType::User, tenant)
    }

    #[test]
    fn no_tenant_resolver_returns_misconfigured() {
        let resolver = NoTenantResolver;
        let outcome = resolver.resolve(&subject(), &SystemClock, CorrelationId::new());
        match outcome {
            IdentityOutcome::Failed(err) => {
                assert_eq!(err.failure(), IdentityFailure::Misconfigured);
                let s = format!("{err}");
                assert!(!s.contains("hunter2"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn static_resolver_returns_configured_tenant() {
        let tenant = Id::<TenantTag>::new("tnt_main");
        let resolver = StaticTenantResolver::new(
            tenant.clone(),
            Environment::Production,
            IsolationLevel::Dedicated,
        );
        let outcome = resolver.resolve(&subject(), &SystemClock, CorrelationId::new());
        match outcome {
            IdentityOutcome::Authenticated(ctx) => {
                assert_eq!(ctx.tenant_id(), &tenant);
                assert_eq!(ctx.environment(), Environment::Production);
                assert_eq!(ctx.isolation_level(), IsolationLevel::Dedicated);
            }
            other => panic!("expected Authenticated, got {other:?}"),
        }
    }
}

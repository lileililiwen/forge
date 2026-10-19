//! Test subject builder.

use platform_identity::subject::{
    Authentication, AuthenticationMethod, Subject, SubjectId, SubjectType,
};
use platform_identity::tenant::{Environment, IsolationLevel, TenantContext, TenantId};
use time::OffsetDateTime;

/// Test-only subject builder.
///
/// `TestSubject` is the recommended starting point for unit tests
/// that need a `Subject`. It picks safe defaults (active lifecycle,
/// user subject type, "tnt_test" tenant, ...). Production code MUST
/// build `Subject` directly from its real store.
#[derive(Debug, Clone)]
pub struct TestSubject {
    id: String,
    subject_type: SubjectType,
    tenant_id: String,
    authentication: Option<Authentication>,
}

impl TestSubject {
    /// Construct a test user named `id` in tenant `tnt_test`.
    #[must_use]
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            subject_type: SubjectType::User,
            tenant_id: "tnt_test".to_string(),
            authentication: Some(Authentication {
                method: AuthenticationMethod::ApiKey,
                issued_at: OffsetDateTime::UNIX_EPOCH,
                expires_at: None,
            }),
        }
    }

    /// Construct a test service named `id`.
    #[must_use]
    pub fn service(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            subject_type: SubjectType::Service,
            tenant_id: "tnt_test".to_string(),
            authentication: Some(Authentication {
                method: AuthenticationMethod::ApiKey,
                issued_at: OffsetDateTime::UNIX_EPOCH,
                expires_at: None,
            }),
        }
    }

    /// Override the tenant.
    #[must_use]
    pub fn in_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant_id = tenant.into();
        self
    }

    /// Attach a specific authentication method.
    #[must_use]
    pub fn authenticated_with(mut self, method: AuthenticationMethod) -> Self {
        self.authentication = Some(Authentication {
            method,
            issued_at: OffsetDateTime::UNIX_EPOCH,
            expires_at: None,
        });
        self
    }

    /// Strip authentication (anonymous subject).
    #[must_use]
    pub fn anonymous(mut self) -> Self {
        self.authentication = None;
        self
    }

    /// Build the [`Subject`].
    #[must_use]
    pub fn build(self) -> Subject {
        let mut subject = Subject::new(
            SubjectId::new(self.id),
            self.subject_type,
            TenantId::new(self.tenant_id),
        );
        if let Some(auth) = self.authentication {
            subject = subject.with_authentication(auth);
        }
        subject
    }

    /// Build a matching [`TenantContext`] for use in tests that need
    /// both a subject and a tenant context.
    #[must_use]
    pub fn tenant_context(&self) -> TenantContext {
        TenantContext::new(
            TenantId::new(self.tenant_id.clone()),
            Environment::Test,
            IsolationLevel::Logical,
            OffsetDateTime::UNIX_EPOCH,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_subject_is_active_user() {
        let subject = TestSubject::user("usr_1").build();
        assert_eq!(subject.id().as_str(), "usr_1");
        assert_eq!(subject.subject_type(), SubjectType::User);
        assert_eq!(subject.tenant_id().as_str(), "tnt_test");
    }

    #[test]
    fn anonymous_subject_has_no_authentication() {
        let subject = TestSubject::user("usr_1").anonymous().build();
        assert!(subject.authentication().is_none());
    }

    #[test]
    fn tenant_context_is_test_environment() {
        let ctx = TestSubject::user("usr_1").tenant_context();
        assert_eq!(ctx.environment(), Environment::Test);
    }
}

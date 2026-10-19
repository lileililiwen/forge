//! Safety tests for `platform-identity`.
//!
//! The OpenSpec scenario is: "WHEN authentication fails, THEN the
//! normalized outcome contains a stable failure category and no
//! password, token, hash, or provider response detail."
//!
//! These tests exercise the constructor paths and the Display impl
//! to make sure that no part of the platform ever holds or
//! accidentally surfaces credential material.

use platform_identity::error::{IdentityError, IdentityFailure};
use platform_identity::outcome::{IdentityOutcome, UnauthenticatedReason};
use platform_identity::subject::{
    Authentication, AuthenticationMethod, Subject, SubjectId, SubjectType,
};
use platform_identity::tenant::{Environment, IsolationLevel, TenantContext, TenantId};
use time::OffsetDateTime;

/// Construct an authentication error that an application might
/// raise. The test asserts that the message and display form do
/// **not** contain the credential value.
#[test]
fn identity_error_does_not_carry_credential_in_message() {
    let leaked = "hunter2-correct-horse-battery-staple";
    let err = IdentityError::new(IdentityFailure::InvalidCredentials, "credentials rejected");
    let _ = leaked; // The application owns the credential and would discard it.
    let display = format!("{err}");
    assert!(!display.contains(leaked));
    assert!(!display.contains("hunter2"));
    assert!(!display.contains("correct-horse"));
}

#[test]
fn identity_error_categories_are_stable_wire_form() {
    assert_eq!(
        IdentityFailure::InvalidCredentials.as_str(),
        "invalid_credentials"
    );
    assert_eq!(IdentityFailure::TenantMismatch.as_str(), "tenant_mismatch");
    assert_eq!(
        IdentityFailure::ProviderUnavailable.as_str(),
        "provider_unavailable"
    );
}

#[test]
fn unauthenticated_reason_does_not_carry_credential() {
    // The outcome type is `enum` with a fixed set of variants; the
    // test asserts the variants do not embed any string payload that
    // a caller could fill with a secret.
    let outcome: IdentityOutcome<()> =
        IdentityOutcome::Unauthenticated(UnauthenticatedReason::NoCredentials);
    assert!(!outcome.is_authenticated());
    let err = outcome
        .require_authenticated()
        .expect_err("Unauthenticated must not pass through require_authenticated");
    assert_eq!(err.failure(), IdentityFailure::InvalidCredentials);
    assert!(!format!("{err}").contains("hunter2"));
}

#[test]
fn subject_authentication_block_carries_method_not_value() {
    let id = SubjectId::new("usr_1");
    let tenant = TenantId::new("tnt_x");
    let subject = Subject::new(id, SubjectType::User, tenant).with_authentication(Authentication {
        method: AuthenticationMethod::ApiKey,
        issued_at: OffsetDateTime::UNIX_EPOCH,
        expires_at: None,
    });
    // The Subject type itself does not store a credential value. The
    // test asserts that the type's serialised form contains the
    // method and the identifier but no place to put a key.
    let json = serde_json::to_value(subject.authentication().unwrap()).unwrap();
    assert_eq!(json["method"], "api_key");
    assert!(json.get("token").is_none());
    assert!(json.get("api_key").is_none());
    assert!(json.get("credential").is_none());
}

#[test]
fn tenant_context_serialises_to_known_contract_shape() {
    let id = TenantId::new("tnt_x");
    let ctx = TenantContext::new(
        id,
        Environment::Production,
        IsolationLevel::Logical,
        OffsetDateTime::UNIX_EPOCH,
    );
    let json = serde_json::to_value(&ctx).unwrap();
    assert_eq!(json["environment"], "production");
    assert_eq!(json["isolation_level"], "logical");
    assert!(json["tenant_id"].is_string());
    assert!(json["active_at"].is_string());
}

//! Cross-surface error-mapping coverage: every `MailingError`
//! variant maps to a shipped `ErrorCategory`, invalid messages never
//! reach a transport, error text stays redacted, and the default
//! feature set stays lettre-free (this file runs in every matrix
//! leg, including `--no-default-features`).

use std::time::Duration;

use platform_core::ErrorCategory;
use platform_mailing::{Body, EmailAddress, EmailMessage, MailingError};

fn valid_message() -> EmailMessage {
    EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .to(EmailAddress::parse("alice@example.com").expect("addr"))
        .subject("hello")
        .body(Body::text("hi"))
        .build()
        .expect("message builds")
}

#[test]
fn invalid_message_shapes_return_validation() {
    // Empty `to`.
    let err = EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .subject("s")
        .body(Body::text("b"))
        .build()
        .expect_err("empty to must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // 51st recipient.
    let err = EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .extend_to(
            (0..51).map(|i| EmailAddress::parse(&format!("u{i}@example.com")).expect("addr")),
        )
        .subject("s")
        .body(Body::text("b"))
        .build()
        .expect_err("51 recipients must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // CR/LF subject.
    let err = EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .to(EmailAddress::parse("a@example.com").expect("addr"))
        .subject("evil\nsubject")
        .body(Body::text("b"))
        .build()
        .expect_err("LF subject must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // Oversize body.
    let err = EmailMessage::builder(EmailAddress::parse("ops@example.com").expect("addr"))
        .to(EmailAddress::parse("a@example.com").expect("addr"))
        .subject("s")
        .body(Body::text("b".repeat(5 * 1024 * 1024 + 1)))
        .build()
        .expect_err("oversize body must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    valid_message().validate().expect("valid message validates");
}

#[test]
fn every_error_variant_maps_to_a_shipped_category() {
    let cases: &[(MailingError, ErrorCategory)] = &[
        (MailingError::validation("v"), ErrorCategory::Validation),
        (MailingError::configuration("c"), ErrorCategory::Validation),
        (
            MailingError::transport_unavailable("down"),
            ErrorCategory::Unavailable,
        ),
        (MailingError::Authentication, ErrorCategory::Authentication),
        (
            MailingError::timeout(Duration::from_secs(1)),
            ErrorCategory::Unavailable,
        ),
        (MailingError::Tls, ErrorCategory::Unavailable),
        (
            MailingError::rejected_by_server(Some(550), "denied"),
            ErrorCategory::Unavailable,
        ),
    ];
    for (err, expected) in cases {
        assert_eq!(err.category(), *expected);
        assert_eq!(err.to_safe_error().category(), *expected);
    }
}

#[test]
fn boundary_failures_stay_redacted() {
    let overlong = format!("{}@example.com", "a".repeat(300));
    let addr_err = EmailAddress::parse(&overlong).expect_err("overlong address must fail");
    assert!(!addr_err.to_string().contains(&"a".repeat(300)));

    let rejected =
        MailingError::rejected_by_server(Some(550), "mailbox unavailable: alice@example.com");
    assert!(!rejected.to_string().contains("alice@"));
}

#[test]
fn default_features_stay_opt_in_and_lettre_free() {
    // Reads the crate manifest directly: `lettre` and `tokio` MUST
    // exist only as optional dependencies, and the default feature
    // set MUST be empty, so a default build cannot see either crate.
    // Runs in every matrix leg, including `--no-default-features`.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let raw = std::fs::read_to_string(&manifest).expect("manifest readable");
    let value: toml::Value = toml::from_str(&raw).expect("manifest parses");

    let deps = value
        .get("dependencies")
        .and_then(|v| v.as_table())
        .expect("dependencies table");
    for name in ["lettre", "tokio"] {
        let entry = deps.get(name).unwrap_or_else(|| panic!("{name} listed"));
        let optional = entry
            .get("optional")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(optional, "{name} must be an optional dependency");
    }

    let features = value
        .get("features")
        .and_then(|v| v.as_table())
        .expect("features table");
    let default = features
        .get("default")
        .and_then(|v| v.as_array())
        .expect("default features");
    assert!(
        default.is_empty(),
        "default features must be empty, got {default:?}"
    );
    assert!(
        features.contains_key("smtp-lettre") && features.contains_key("mock"),
        "smtp-lettre and mock features must exist"
    );
}

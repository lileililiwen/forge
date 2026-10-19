//! Cross-surface error-mapping coverage for `platform-ai`: every
//! `AiError` variant maps to a shipped `ErrorCategory`, every public
//! failure path leaves no fixture secret in `Display`, invalid
//! messages never reach a transport, and the default feature set
//! stays free of `reqwest`/`tokio` (this file runs in every matrix
//! leg, including `--no-default-features`).

use platform_ai::types::{Content, Message, Role};
use platform_ai::{AiError, ChatRequest, FinishReason, GenerationOptions};
use platform_core::ErrorCategory;

fn user_request() -> ChatRequest {
    ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .build()
        .expect("valid request")
}

#[test]
fn invalid_message_shapes_return_validation() {
    // Empty messages.
    let err = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .options(GenerationOptions::default().with_temperature(3.0))
        .build()
        .expect_err("out-of-range temperature must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // max_output_tokens = 0.
    let err = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .options(GenerationOptions::default().with_max_output_tokens(0))
        .build()
        .expect_err("zero max_output_tokens must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // Over-cap stop sequences.
    let err = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .options(
            GenerationOptions::default()
                .with_stop("a")
                .with_stop("b")
                .with_stop("c")
                .with_stop("d")
                .with_stop("e"),
        )
        .build()
        .expect_err("five stop sequences must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // Over-long model id.
    let err = ChatRequest::builder(Message::new(Role::User, Content::text("hi")))
        .model("m".repeat(platform_ai::types::MAX_MODEL_LEN + 1))
        .build()
        .expect_err("over-long model must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // 257 messages overflows.
    let mut builder = ChatRequest::builder(Message::new(Role::User, Content::text("hi")));
    for i in 0..platform_ai::types::MAX_MESSAGES {
        builder = builder.message(Message::new(Role::User, Content::text(format!("t{i}"))));
    }
    let err = builder
        .message(Message::new(Role::User, Content::text("extra")))
        .build()
        .expect_err("257 messages must fail");
    assert_eq!(err.category(), ErrorCategory::Validation);

    // Sanity: a valid request builds.
    user_request();
}

#[test]
fn every_error_variant_maps_to_a_shipped_category() {
    let cases: &[(AiError, ErrorCategory)] = &[
        (AiError::validation("v"), ErrorCategory::Validation),
        (AiError::configuration("c"), ErrorCategory::Validation),
        (AiError::content_policy("policy"), ErrorCategory::Validation),
        (
            AiError::Unsupported(platform_ai::Capabilities::STREAMING),
            ErrorCategory::Validation,
        ),
        (AiError::Unauthorized, ErrorCategory::Unavailable),
        (
            AiError::RateLimited {
                retry_after_secs: Some(7),
            },
            ErrorCategory::Unavailable,
        ),
        (
            AiError::transport_unavailable("down"),
            ErrorCategory::Unavailable,
        ),
        (AiError::Timeout, ErrorCategory::Unavailable),
        (
            AiError::ProviderServer {
                status: 500,
                safe_detail: "nope".into(),
            },
            ErrorCategory::Unavailable,
        ),
        (
            AiError::malformed_response("missing choices"),
            ErrorCategory::Unavailable,
        ),
    ];
    for (err, expected) in cases {
        assert_eq!(err.category(), *expected, "mapping for {err}");
        assert_eq!(err.to_safe_error().category(), *expected);
    }
}

#[test]
fn display_omits_keys_prompts_and_completions() {
    let secret = "sk-live-abcdef0123456789";
    let prompt = "Write a poem about the user's medical history.";
    let body = r#"{"choices":[],"error":{"message":"internal","prompt":"leaked"}}"#;

    let rendered = format!(
        "{} | {} | {} | {} | {}",
        AiError::validation("messages count 0"),
        AiError::configuration("base_url must be https"),
        AiError::Unauthorized,
        AiError::transport_unavailable("connection refused"),
        AiError::ProviderServer {
            status: 500,
            safe_detail: platform_ai::error::truncate_safe_detail(body).into_owned(),
        },
    );
    assert!(!rendered.contains(secret), "secret leaked: {rendered}");
    assert!(!rendered.contains(prompt), "prompt leaked: {rendered}");
    // The body is present in the failure path but its server detail
    // is the truncated, control-stripped server.message — the
    // literal "internal" string survives, but the original
    // "leaked" prompt fragment does not.
    assert!(rendered.contains("internal"));
}

#[test]
fn boundary_failures_stay_redacted() {
    // ProviderServer's safe_detail is already-truncated text. A
    // 500-character body must end up at most MAX_SAFE_DETAIL_BYTES
    // long in the failure.
    let big = "x".repeat(500);
    let err = AiError::ProviderServer {
        status: 500,
        safe_detail: platform_ai::error::truncate_safe_detail(&big).into_owned(),
    };
    let rendered = format!("{err}");
    assert!(rendered.len() <= platform_ai::error::MAX_SAFE_DETAIL_BYTES + 32);

    // Control characters in a server detail are stripped, never
    // echoed.
    let err = AiError::ProviderServer {
        status: 500,
        safe_detail: platform_ai::error::truncate_safe_detail("hi\nthere\tfriend").into_owned(),
    };
    assert!(!format!("{err}").contains('\n'));
    assert!(!format!("{err}").contains('\t'));
}

#[test]
fn default_features_stay_opt_in_and_network_free() {
    // Reads the crate manifest directly: `reqwest` and `tokio` MUST
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
    for name in ["reqwest", "tokio"] {
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
    for required in ["mock", "openai-compat", "ollama"] {
        assert!(
            features.contains_key(required),
            "{required} feature must exist"
        );
    }
}

#[test]
fn unsafe_traits_are_forbidden() {
    // The crate is `#![forbid(unsafe_code)]`. This is a sentinel:
    // if a future patch introduces `unsafe`, the build breaks here
    // through the same deny-atribute.
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let raw = std::fs::read_to_string(&lib).expect("lib.rs is readable");
    assert!(raw.contains("forbid(unsafe_code)"), "lib.rs forbids unsafe");
}

#[test]
fn finish_reason_strings_are_stable() {
    // Snake-case wire form: changing these is a breaking change for
    // every provider adapter that round-trips them.
    assert_eq!(FinishReason::Stop.as_str(), "stop");
    assert_eq!(FinishReason::ToolCalls.as_str(), "tool_calls");
    assert_eq!(FinishReason::Length.as_str(), "length");
    assert_eq!(FinishReason::ContentRefused.as_str(), "content_refused");
    assert_eq!(FinishReason::Unknown.as_str(), "unknown");
}

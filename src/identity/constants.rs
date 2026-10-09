//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the identity API, the persisted
/// session records, and the CLI/MCP envelopes. Bumped when
/// a new field is added or a typed error code changes.
pub const IDENTITY_CONTRACT_VERSION: &str = "0.1.0";

/// Default per-project client id placeholder used by the
/// fixtures when a project does not declare one in its
/// manifest. Real manifests always carry an explicit
/// client id; the placeholder exists so the contract tests
/// can stand in for a real provider configuration.
pub const IDENTITY_FAKE_PROVIDER: &str = "okta-fixture";

/// Maximum number of scopes a project's identity block may
/// request. Larger lists are refused with
/// `identity-invalid` so the manifest stays bounded.
pub const MAX_SCOPES: usize = 16;

/// Maximum number of admin-claim values the manifest may
/// enumerate. Larger lists are refused with
/// `identity-invalid`.
pub const MAX_ADMIN_VALUES: usize = 16;

/// Minimum state/nonce lifetime in seconds. A shorter TTL
/// is refused because the round trip is unlikely to
/// complete in time.
pub const MIN_STATE_TTL_SECONDS: i64 = 30;

/// Maximum state/nonce lifetime in seconds. A longer TTL
/// enlarges the replay window and is refused.
pub const MAX_STATE_TTL_SECONDS: i64 = 600;

/// Minimum admin session lifetime in seconds.
pub const MIN_SESSION_TTL_SECONDS: i64 = 60;

/// Maximum admin session lifetime in seconds. The brief
/// (`Session and project isolation`) calls for short-lived
/// project sessions; a longer TTL is refused.
pub const MAX_SESSION_TTL_SECONDS: i64 = 86_400;

/// Directory (relative to the project root) holding the
/// per-project session records.
pub const IDENTITY_DIR: &str = ".forge/identity";

/// Directory (relative to the project root) holding
/// pending auth challenges awaiting the OIDC callback.
/// Each challenge is named by its `state` value so the
/// callback handler can find it deterministically.
pub const IDENTITY_PENDING_DIR: &str = ".forge/identity";

/// Permission granted to a session after a successful
/// admin-claim check. The string set is bounded on
/// purpose so the planner and the doctor can map a
/// permission to a known capability without parsing
/// free-form text.
pub const ADMIN_PERMISSION: &str = "admin:access";

/// OIDC provider names accepted in the manifest. The
/// contract is provider-agnostic: every supported
/// provider is a real OIDC provider, so the only
/// difference between providers is the issuer, the
/// JWKS URI and the admin claim convention. The list
/// is bounded so the validator can refuse unknown
/// providers without inventing a provider registry
/// during the contract cycle.
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "okta",
    "auth0",
    "keycloak",
    "azure-ad",
    "google",
    "github",
    "okta-fixture",
];

/// OIDC code challenge methods accepted by the
/// `build-challenge` contract. Only `S256` is supported
/// in v0.1; `plain` is refused because it offers no
/// replay protection.
pub const SUPPORTED_CODE_CHALLENGE_METHODS: &[&str] = &["S256"];

/// Length of the random `state`, `nonce` and
/// `code_verifier` values (in raw bytes; the values are
/// hex-encoded so the wire length is `2 * VALUE_LEN`).
pub const VALUE_LEN: usize = 32;

/// Length of the session id (in raw bytes; hex-encoded
/// for storage).
pub const SESSION_ID_LEN: usize = 16;

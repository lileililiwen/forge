//! The `forge-identity-provider/1.0` contract between Forge Core and an identity
//! provider pack.
//!
//! A pack is an independent process. Forge Core writes one JSON request object per
//! line on the pack's standard input and reads one JSON response object per line from
//! its standard output. The contract carries no secret value: every credential is a
//! reference, and every URL is checked before it is trusted.

pub mod digest;
pub mod redaction;
pub mod request;
pub mod response;
pub mod validation;

pub use digest::{plan_digest, DigestError};
pub use redaction::{redact_text, redact_url, SECRET_MARKERS};
pub use request::{ProviderOperation, ProviderRequest, CONTRACT_ID, CONTRACT_MAJOR};
pub use response::{Population, ProposedChange, ProviderResponse, ProviderStatus, RecoveryAction};
pub use validation::{validate_request, validate_response, ValidationError};

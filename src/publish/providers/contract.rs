//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::time::Duration;

pub const PUBLISH_PROVIDER_CONTRACT: &str = "forge-publish-provider/0.1.0";

pub const PUBLISH_PROVIDER_TIMEOUT: Duration = Duration::from_secs(1800);

/// Bounded character length for the optional `queue_id` field carried
/// in provider requests, responses, and progress events. The bound is
/// shared with the queue state module so a single change tightens the
/// whole transport.
pub const QUEUE_ID_MAX: usize = 128;

/// Bounded character length for the optional progress `detail` field.
/// Longer values are truncated (with a trailing `…`) so a verbose
/// provider cannot blow the journal row width.
pub const PROGRESS_DETAIL_MAX: usize = 512;

/// A full Git revision is exactly 40 lowercase or uppercase hex
/// characters. Providers MUST populate `request.revision` (and the
/// matching response field) with the committed SHA the publish
/// actually transferred; an empty, short, long or non-hex value is a
/// contract violation.
pub const REVISION_LEN: usize = 40;

/// The number of hex characters that travel in the Docker / Compose
/// identity. The full 40-character SHA stays in Forge state.
pub const REVISION_SHA12_LEN: usize = 12;

/// Progress event phase vocabulary. `build` and `run` mark the two
/// publish phases; `complete` marks the final healthy phase. Other
/// phase names from earlier providers (e.g. `preflight`,
/// `build-and-run`, `verify`) are accepted as legacy aliases so
/// sibling providers that have not yet migrated still classify.
pub const PHASE_BUILD: &str = "build";

pub const PHASE_RUN: &str = "run";

pub const PHASE_COMPLETE: &str = "complete";

/// Phase status vocabulary carried both on progress events and in the
/// additive terminal response fields. Status names are shared
/// verbatim with the journal and `forge deploy status` projection so
/// the operator can map `unknown` / `not_started` to missing evidence.
pub const PHASE_STATUS_SUCCEEDED: &str = "succeeded";

pub const PHASE_STATUS_FAILED: &str = "failed";

pub const PHASE_STATUS_NOT_STARTED: &str = "not_started";

pub const PHASE_STATUS_UNKNOWN: &str = "unknown";

/// Maximum length for the bounded `container_identity` field. The
/// Compose project / container name is bounded so a verbose provider
/// cannot blow the journal row width.
pub const CONTAINER_IDENTITY_MAX: usize = 256;

/// Versioned contract for the metadata propose request kind.
pub const METADATA_PROPOSE_CONTRACT: &str = "forge-metadata-propose/0.1.0";

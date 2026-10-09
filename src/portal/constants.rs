//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Contract data version for the portal surface. The CLI
/// and the future ASP.NET Core / Next.js renderer speak
/// the same version on the wire.
pub const PORTAL_CONTRACT_VERSION: &str = "0.1.0";

/// Synthetic project id recorded against the `portal`
/// journal when the view spans the entire registered
/// registry (e.g. `forge portal dashboard --all`). The id
/// keeps the operations table project-agnostic without
/// inventing a user-visible project.
pub const PORTAL_SYNTHETIC_PROJECT: &str = "__portal__";

/// Section id parsing. The brief lists twelve named
/// sections in §36; the contract refuses unknown ids so
/// the CLI cannot silently ask for a view the registry
/// cannot answer.
pub const SECTION_PROJECTS: &str = "projects";

pub const SECTION_FEATURES: &str = "features";

pub const SECTION_COMPONENTS: &str = "components";

pub const SECTION_POLICIES: &str = "policies";

pub const SECTION_SPECS: &str = "specs";

pub const SECTION_AGENTS: &str = "agents";

pub const SECTION_DEPLOYMENTS: &str = "deployments";

pub const SECTION_REPOSITORIES: &str = "repositories";

pub const SECTION_DOCUMENTATION: &str = "documentation";

pub const SECTION_ANALYTICS: &str = "analytics";

pub const SECTION_SERVERS: &str = "servers";

pub const SECTION_SETTINGS: &str = "settings";

/// Stable list of supported section ids, in the order the
/// brief enumerates them in §36. The order is preserved by
/// `parse_section` and by the human renderer.
pub const SUPPORTED_SECTIONS: &[&str] = &[
    SECTION_PROJECTS,
    SECTION_FEATURES,
    SECTION_COMPONENTS,
    SECTION_POLICIES,
    SECTION_SPECS,
    SECTION_AGENTS,
    SECTION_DEPLOYMENTS,
    SECTION_REPOSITORIES,
    SECTION_DOCUMENTATION,
    SECTION_ANALYTICS,
    SECTION_SERVERS,
    SECTION_SETTINGS,
];

/// Maximum number of section entries a single portal view
/// may carry. Larger lists are refused at validation time
/// so a malformed manifest or a runaway registry cannot
/// pin the renderer to an unbounded allocation.
pub const MAX_ENTRIES_PER_VIEW: usize = 256;

/// Maximum length of an entry `id` (a project id, a
/// feature id, …). Longer ids are refused at validation
/// time so a malicious input cannot inject oversized
/// text.
pub const MAX_ENTRY_ID_LEN: usize = 128;

/// Maximum number of fleet entries rendered into one portal section.
/// Larger portfolios roll up into the source meta entry so a runaway
/// registry cannot push the section past `MAX_ENTRIES_PER_VIEW`.
pub const MAX_PORTAL_FLEET_ENTRIES: usize = 128;

//! Portfolio subcommand shapes (`Portfolio*` families).
//!
//! Clap argument enums for the portfolio surface: project metadata,
//! tags, relations, reviews, evidence, goals, sharing, interest and
//! activation. Bodies moved verbatim from the split of `src/main.rs`.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioRelationCommands {
    /// Link two projects. The same link twice stays one row.
    Add {
        /// Source project id.
        from: String,
        /// Target project id.
        #[arg(long)]
        to: String,
        /// Relation type.
        #[arg(long)]
        r#type: String,
        /// Optional note.
        #[arg(long)]
        note: Option<String>,
    },
    /// Remove one declared relation.
    Remove {
        /// Source project id.
        from: String,
        /// Target project id.
        #[arg(long)]
        to: String,
        /// Relation type.
        #[arg(long)]
        r#type: String,
    },
    /// List relations touching one project, or every relation.
    List {
        /// Registered project id (omit for every relation).
        #[arg(default_value = "")]
        project: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioReviewCommands {
    /// Record a review decision and optionally the current lifecycle,
    /// next action and blocker for one project.
    Set {
        /// Registered project id.
        project: String,
        /// Review confidence.
        #[arg(long)]
        confidence: String,
        /// Review note.
        #[arg(long)]
        note: Option<String>,
        /// Current lifecycle classification.
        #[arg(long)]
        lifecycle: Option<String>,
        /// Next action the operator intends to take.
        #[arg(long)]
        next_action: Option<String>,
        /// Current blocker.
        #[arg(long)]
        blocker: Option<String>,
    },
    /// List the review history for one project.
    List {
        /// Registered project id.
        project: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioInterestCommands {
    /// Import a versioned batch of aggregate snapshots from a JSON
    /// file, or from stdin when the path is `-`. Every record is
    /// reported separately; a refused record never blocks the others.
    Import {
        /// Path to the import document, or `-` for stdin.
        file: String,
        /// Who is importing. Recorded verbatim as provenance.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// List one project's stored snapshots, or every project's.
    List {
        /// Registered project id (omit for every project).
        #[arg(default_value = "")]
        project: String,
        /// Maximum rows to read.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Days after which a window reads as `stale`.
        #[arg(
            long,
            default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS
        )]
        stale_after_days: i64,
    },
    /// Show one project's whole interest projection with its refusals.
    Show {
        /// Registered project id.
        project: String,
        /// Days after which a window reads as `stale`.
        #[arg(
            long,
            default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS
        )]
        stale_after_days: i64,
    },
    /// Compare projects on the allowlisted metrics. Forge never totals
    /// or ranks across windows and labels every row with its source and
    /// freshness.
    Compare {
        /// Registered project ids to compare.
        #[arg(required = true)]
        projects: Vec<String>,
        /// Metric to compare; repeat for several (default: all).
        #[arg(long = "metric")]
        metrics: Vec<String>,
        /// Narrow the comparison to one analytics source.
        #[arg(long)]
        source: Option<String>,
        /// Days after which a window reads as `stale`.
        #[arg(
            long,
            default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS
        )]
        stale_after_days: i64,
    },
    /// One metric's windowed history for one project.
    Trend {
        /// Registered project id.
        project: String,
        /// Metric to plot.
        #[arg(long)]
        metric: String,
        /// Maximum windows to return.
        #[arg(long, default_value_t = 12)]
        limit: usize,
        /// Days after which a window reads as `stale`.
        #[arg(
            long,
            default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS
        )]
        stale_after_days: i64,
    },
    /// The refusals this store recorded, newest first.
    Audit {
        /// Maximum rows to read.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioEvidenceCommands {
    /// Append one source-owned observation as a snapshot. The payload is
    /// redacted before it is stored and is never rewritten afterwards.
    Import {
        /// Registered project id.
        #[arg(long)]
        project: String,
        /// Source system that produced the observation.
        #[arg(long)]
        source: String,
        /// Revision the source observed.
        #[arg(long)]
        revision: String,
        /// Status: observed, stale, unavailable, invalid or not-run.
        #[arg(long)]
        status: String,
        /// RFC 3339 observation time (default: now).
        #[arg(long)]
        observed_at: Option<String>,
        /// RFC 3339 freshness bound; past this the read model shows stale.
        #[arg(long)]
        stale_after: Option<String>,
        /// Evidence document as JSON, or `@path` to read it from a file.
        #[arg(long, default_value = "{}")]
        evidence: String,
    },
    /// List every snapshot for one project, newest first.
    List {
        /// Registered project id.
        project: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioShareCommands {
    /// Define or replace one project's public share record. Nothing is
    /// public until the manifest is approved and published.
    Set {
        /// Registered project id.
        project: String,
        /// Public title.
        #[arg(long)]
        title: String,
        /// Public one-paragraph summary.
        #[arg(long)]
        summary: String,
        /// Public category label.
        #[arg(long)]
        category: String,
        /// Public HTTPS source URL.
        #[arg(long)]
        source_url: String,
        /// Optional public HTTPS demo URL.
        #[arg(long)]
        demo_url: Option<String>,
        /// Visibility: public or unlisted.
        #[arg(long, default_value = "public")]
        visibility: String,
        /// Mark the record as featured.
        #[arg(long)]
        featured: bool,
        /// Showcase status: planned, demo, beta, stable, archived or unknown.
        #[arg(long, default_value = "unknown")]
        status: String,
        /// Optional status-evidence JSON object, or `@path` to read it from a file.
        #[arg(long)]
        evidence: Option<String>,
        /// Allowlisted public surface as `label=https://…`; repeat for more.
        #[arg(long = "surface", value_name = "LABEL=URL")]
        surfaces: Vec<String>,
    },
    /// Withdraw a project's share record; it leaves the public catalog.
    Remove {
        /// Registered project id.
        project: String,
    },
    /// Show one project's share record and its allowlisted surfaces.
    Show {
        /// Registered project id.
        project: String,
    },
    /// List every share record, newest state first.
    List {
        /// Registered project id (omit for every record).
        #[arg(default_value = "")]
        project: String,
    },
    /// Preview the candidate manifest: exact canonical bytes, hash and findings.
    Preview {
        /// Also print the rendered public document.
        #[arg(long)]
        document: bool,
    },
    /// Approve one exact manifest hash.
    Approve {
        /// The `manifest_sha256` a preview reported.
        #[arg(long)]
        hash: String,
        /// Who is approving; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// Publish the approved manifest through the default-safe local export
    /// or an optional credential-injected adapter.
    Publish {
        /// Publication target: an artifact path for the local publisher, or a
        /// deployment target name for an adapter.
        #[arg(long)]
        target: String,
        /// Operation key; a retry under the same key never publishes twice.
        #[arg(long, default_value = "share-publish-1")]
        operation_key: String,
        /// Who is publishing; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
        /// Optional publication adapter executable receiving the manifest on stdin.
        #[arg(long, value_name = "PATH")]
        adapter: Option<PathBuf>,
    },
    /// Resolve a partial publication reported as `unknown`.
    Reconcile {
        /// Publication attempt id from the audit trail.
        #[arg(long)]
        publication: i64,
        /// The observed outcome: published or failed.
        #[arg(long)]
        result: String,
        /// Who reconciled it; recorded in the publication audit trail.
        #[arg(long, default_value = "local-admin")]
        actor: String,
    },
    /// Show the approval and publication audit trail.
    Audit {
        /// Maximum rows per section (1-500).
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioTagCommands {
    /// Attach a tag to a project, creating the tag on first use.
    Add {
        /// Registered project id.
        project: String,
        /// Tag name (lowercase, digits and single dashes).
        #[arg(long)]
        name: String,
        /// Optional `#rrggbb` colour or lowercase word.
        #[arg(long)]
        color: Option<String>,
    },
    /// Detach a tag from a project. The tag itself is kept.
    Remove {
        /// Registered project id.
        project: String,
        /// Tag name to detach.
        #[arg(long)]
        name: String,
    },
    /// List every tag, or the tags on one project.
    List {
        /// Registered project id (omit for every tag).
        #[arg(default_value = "")]
        project: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioActivationCommands {
    /// Read-only verdict on whether a project's aggregate interest
    /// evidence justifies the product-owned activation follow-up.
    /// Prints the report and exits non-zero when any evaluated
    /// project is `not-ready`. Adds no billing of any kind.
    Readiness {
        /// Registered project id (omit for every registered project).
        #[arg(default_value = "")]
        project: String,
        /// Aggregate metric the verdict rests on.
        #[arg(long = "metric")]
        metric: String,
        /// Minimum value the operator declares; absent is the
        /// `threshold-not-declared` verdict, not an error.
        #[arg(long = "min-value")]
        min_value: Option<u64>,
        /// Narrow the verdict to one analytics source.
        #[arg(long)]
        source: Option<String>,
        /// Narrow the verdict to one window `<START>..<END>`.
        #[arg(long)]
        window: Option<String>,
        /// Days after which a window reads as `stale`.
        #[arg(
            long,
            default_value_t = forge::portfolio::interest::DEFAULT_STALE_AFTER_DAYS
        )]
        stale_after_days: i64,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioCommands {
    /// Manage user-owned tags.
    Tag {
        #[command(subcommand)]
        command: PortfolioTagCommands,
    },
    /// Manage user-owned relationships between projects.
    Relation {
        #[command(subcommand)]
        command: PortfolioRelationCommands,
    },
    /// Record a review decision and the current classification.
    Review {
        #[command(subcommand)]
        command: PortfolioReviewCommands,
    },
    /// Manage user-owned portfolio goals.
    Goal {
        #[command(subcommand)]
        command: PortfolioGoalCommands,
    },
    /// Import source-owned observations as append-only snapshots.
    Evidence {
        #[command(subcommand)]
        command: PortfolioEvidenceCommands,
    },
    /// Show one project's whole portfolio projection.
    Show {
        /// Registered project id.
        project: String,
    },
    /// Manage the explicit allowlist of projects that may be published.
    Share {
        #[command(subcommand)]
        command: PortfolioShareCommands,
    },
    /// Import and compare aggregate, privacy-safe interest evidence.
    Interest {
        #[command(subcommand)]
        command: PortfolioInterestCommands,
    },
    /// Read-only readiness verdict on whether aggregate evidence
    /// justifies the product-owned activation follow-up. No billing.
    Activation {
        #[command(subcommand)]
        command: PortfolioActivationCommands,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PortfolioGoalCommands {
    /// Create a goal. Re-running with the same title updates it.
    Add {
        /// Goal title (unique).
        title: String,
        /// Goal status.
        #[arg(long, default_value = "planned")]
        status: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Attach a project to a goal, creating the goal when new.
    Link {
        /// Goal title.
        goal: String,
        /// Registered project id.
        #[arg(long)]
        project: String,
    },
    /// List every goal with its projects.
    List,
}

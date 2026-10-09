//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::registry::Registry;

use super::commands_portfolio::{
    PortfolioActivationCommands, PortfolioInterestCommands, PortfolioShareCommands,
};
use super::functions_9::{read_evidence_payload, share_invalid};
use super::projects::as_output;
use super::readiness::render_comparison;
use crate::{Format, Output};

/// The one CLI entry point for the public share allowlist. Every
/// subcommand dispatches into the same
/// [`forge::portfolio::publication`] orchestration the JSON API uses,
/// so the terminal adds no publication rule of its own.
pub(super) fn cmd_portfolio_share(
    registry: &Registry,
    command: &PortfolioShareCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::share::{
        self as share, ShowcaseStatus, Visibility, SHARE_CONTRACT_VERSION,
    };
    let now = chrono::Utc::now();
    let contract = SHARE_CONTRACT_VERSION;
    match command {
        PortfolioShareCommands::Set {
            project,
            title,
            summary,
            category,
            source_url,
            demo_url,
            visibility,
            featured,
            status,
            evidence,
            surfaces,
        } => {
            let visibility = Visibility::parse(visibility).map_err(share_invalid)?;
            let showcase_status = ShowcaseStatus::parse(status).map_err(share_invalid)?;
            let mut parsed = Vec::with_capacity(surfaces.len());
            for surface in surfaces {
                parsed.push(share::ShareSurface::split(surface).map_err(share_invalid)?);
            }
            let write = forge::portfolio::share::ShareWrite {
                title: title.clone(),
                summary: summary.clone(),
                category: category.clone(),
                source_url: source_url.clone(),
                demo_url: demo_url.clone(),
                visibility,
                featured: *featured,
                showcase_status,
                status_evidence: match evidence {
                    Some(raw) => Some(read_evidence_payload(raw)?),
                    None => None,
                },
                surfaces: parsed,
            };
            let record = registry.share_upsert_record(project, &write)?;
            let human = format!(
                "share record for {} is {} at revision {} with {} public surface(s)\n",
                record.project_id,
                record.state.label(),
                record.revision,
                record.surfaces.len()
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "share": record,
                }),
            ))
        }
        PortfolioShareCommands::Remove { project } => {
            let removed = registry.share_remove_record(project)?;
            let human = if removed {
                format!("withdrew the share record for {project}\n")
            } else {
                format!("{project} had no share record to withdraw\n")
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "shared": false,
                    "removed": removed,
                }),
            ))
        }
        PortfolioShareCommands::Show { project } => {
            let record = registry.share_record(project)?;
            let findings = registry.share_findings(20)?;
            let findings: Vec<_> = findings
                .into_iter()
                .filter(|finding| finding.project_id == *project)
                .collect();
            let human = match &record {
                None => {
                    format!("{project} has no share record; it is absent from the public catalog\n")
                }
                Some(record) => {
                    let mut out = String::new();
                    out.push_str(&format!("project: {}\n", record.project_id));
                    out.push_str(&format!("state: {}\n", record.state.label()));
                    out.push_str(&format!("revision: {}\n", record.revision));
                    out.push_str(&format!("title: {}\n", record.title));
                    out.push_str(&format!("category: {}\n", record.category));
                    out.push_str(&format!("source url: {}\n", record.source_url));
                    out.push_str(&format!(
                        "demo url: {}\n",
                        record.demo_url.clone().unwrap_or_else(|| "—".to_string())
                    ));
                    out.push_str(&format!("visibility: {}\n", record.visibility));
                    out.push_str(&format!("showcase status: {}\n", record.showcase_status));
                    out.push_str(&format!("featured: {}\n", record.featured));
                    out.push_str(&format!(
                        "status evidence: {}\n",
                        record
                            .status_evidence
                            .clone()
                            .unwrap_or_else(|| "—".to_string())
                    ));
                    if record.surfaces.is_empty() {
                        out.push_str("surfaces: none\n");
                    } else {
                        out.push_str("surfaces:\n");
                        for surface in &record.surfaces {
                            out.push_str(&format!("  {} {}\n", surface.label, surface.url));
                        }
                    }
                    out
                }
            };
            // The persisted refusals are the reason a record is not
            // public; an operator reading `show` needs them, not just
            // the machine projection.
            let human = if findings.is_empty() {
                human
            } else {
                let mut out = human;
                out.push_str("findings:\n");
                for finding in &findings {
                    out.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "project_id": project,
                    "shared": record.is_some(),
                    "share": record,
                    "findings": findings,
                }),
            ))
        }
        PortfolioShareCommands::List { project } => {
            let records = if project.is_empty() {
                registry.share_records()?
            } else {
                registry.share_record(project)?.into_iter().collect()
            };
            let human = if records.is_empty() {
                "No share records.\n".to_string()
            } else {
                let mut out = String::new();
                for record in &records {
                    out.push_str(&format!(
                        "{} {} {} {} {}\n",
                        record.project_id,
                        record.state.label(),
                        record.visibility,
                        record.showcase_status,
                        record.title
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "records": records,
                }),
            ))
        }
        PortfolioShareCommands::Preview { document } => {
            let draft = forge::portfolio::publication::preview_manifest(registry)?;
            let mut human = format!(
                "manifest revision {}\nproject count: {}\nmanifest sha256: {}\napprovable: {}\n",
                draft.body.manifest_revision,
                draft.project_count(),
                draft.manifest_sha256(),
                draft.approvable()
            );
            if draft.findings.is_empty() {
                human.push_str("findings: none\n");
            } else {
                human.push_str("findings:\n");
                for finding in &draft.findings {
                    human.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
            }
            if *document {
                human.push_str(&draft.document(&now.to_rfc3339()));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "preview": {
                        "manifest_revision": draft.body.manifest_revision,
                        "manifest_sha256": draft.manifest_sha256(),
                        "project_count": draft.project_count(),
                        "approvable": draft.approvable(),
                        "canonical_body": draft.body.canonical_json(),
                        "document": draft.document(&now.to_rfc3339()),
                        "findings": draft.findings,
                    },
                }),
            ))
        }
        PortfolioShareCommands::Approve { hash, actor } => {
            let approval = registry.share_approve(hash, actor)?;
            let human = format!(
                "approved manifest revision {} ({}) with {} project(s) as {}\n",
                approval.revision, approval.manifest_sha256, approval.project_count, approval.actor
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "approval": approval,
                }),
            ))
        }
        PortfolioShareCommands::Publish {
            target,
            operation_key,
            actor,
            adapter,
        } => {
            let plan = forge::portfolio::publication::PublishPlan {
                operation_key: operation_key.clone(),
                target: target.clone(),
                actor: actor.clone(),
                adapter: adapter.clone(),
            };
            let report =
                forge::portfolio::publication::publish_approved_manifest(registry, &plan, now)?;
            let human = format!(
                "publication {} of manifest revision {} ({} project(s)) via {} is {}{}\n",
                report.publication_id,
                report.manifest_revision,
                report.project_count,
                report.publisher,
                report.status_label(),
                if report.already_present {
                    " (already present; no second publication)"
                } else {
                    ""
                }
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "publication": report,
                }),
            ))
        }
        PortfolioShareCommands::Reconcile {
            publication,
            result,
            actor,
        } => {
            let status = share::PublicationStatus::parse(result).map_err(share_invalid)?;
            let attempt = registry.share_reconcile_publication(*publication, status, actor)?;
            let human = format!(
                "publication {} is now {} after reconciliation by {}\n",
                attempt.publication_id, attempt.status, attempt.actor
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "publication": attempt,
                }),
            ))
        }
        PortfolioShareCommands::Audit { limit } => {
            if *limit == 0 || *limit > 500 {
                return Err(share_invalid("limit must be between 1 and 500".to_string()));
            }
            let approvals = registry.share_approvals(*limit)?;
            let publications = registry.share_publications(*limit)?;
            let unreconciled = registry.share_unreconciled_publication()?;
            let human = {
                let mut out = String::new();
                out.push_str("approvals:\n");
                if approvals.is_empty() {
                    out.push_str("  none\n");
                }
                for approval in &approvals {
                    out.push_str(&format!(
                        "  revision {} {} {} {} by {}\n",
                        approval.revision,
                        approval.state,
                        approval.manifest_sha256,
                        format!("{} project(s)", approval.project_count),
                        approval.actor
                    ));
                }
                out.push_str("publications:\n");
                if publications.is_empty() {
                    out.push_str("  none\n");
                }
                for attempt in &publications {
                    out.push_str(&format!(
                        "  #{} {} revision {} {} target {}{}\n",
                        attempt.publication_id,
                        attempt.status,
                        attempt.manifest_revision,
                        attempt.manifest_sha256,
                        attempt.target,
                        attempt
                            .error_code
                            .clone()
                            .map(|code| format!(" [{code}]"))
                            .unwrap_or_default()
                    ));
                }
                out.push_str(&format!(
                    "unreconciled: {}\n",
                    match &unreconciled {
                        Some(attempt) => format!("#{}", attempt.publication_id),
                        None => "none".to_string(),
                    }
                ));
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "approvals": approvals,
                    "publications": publications,
                    "unreconciled": unreconciled,
                }),
            ))
        }
    }
}

fn interest_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInterestInvalid { reason }
}

/// The one CLI entry point for aggregate interest evidence. Every
/// subcommand dispatches into the same
/// [`forge::portfolio::interest_report`] orchestration the JSON API
/// uses, so the terminal adds no import or comparison rule of its own.
pub(super) fn cmd_portfolio_interest(
    registry: &Registry,
    command: &PortfolioInterestCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::interest::{
        self as interest, InterestMetric, RawSnapshot, INTEREST_CONTRACT_VERSION,
    };
    use forge::portfolio::interest_report as report;
    let now = chrono::Utc::now();
    let contract = INTEREST_CONTRACT_VERSION;
    match command {
        PortfolioInterestCommands::Import { file, actor } => {
            let raw = read_import_document(file)?;
            let decoded = interest::parse_import_document(&raw).map_err(interest_invalid)?;
            let records: Vec<RawSnapshot> = decoded
                .into_iter()
                .map(|(index, record)| RawSnapshot { index, record })
                .collect();
            let imported = report::import_snapshots(
                registry,
                &interest::InterestImport { records },
                actor,
                now,
            )?;
            // A batch where every record was refused is a failed
            // import, and saying so through the exit code is what lets
            // a script notice without parsing the report.
            let human = format!(
                "imported {} snapshot(s): {} accepted, {} already present, {} superseded, {} refused\n",
                imported.received,
                imported.accepted.len(),
                imported.already_present.len(),
                imported.supersessions.len(),
                imported.refused()
            );
            let json = serde_json::json!({
                "contract": contract,
                "generated_at": now.to_rfc3339(),
                "import": imported,
            });
            let rendered = as_output(format, human, json);
            if imported.is_complete_failure() {
                // A batch where every record was refused is a failed
                // import, and the reason is the diagnosis: naming each
                // refused record and its code is what lets an importer
                // fix the batch without re-running the CLI to find out
                // which of fifty records was wrong.
                return Err(interest_invalid(format!(
                    "every snapshot in the batch was refused: {}",
                    summarize_refusals(&imported.rejected)
                )));
            }
            Ok(rendered)
        }
        PortfolioInterestCommands::List {
            project,
            limit,
            stale_after_days,
        } => {
            if *limit == 0 || *limit > 500 {
                return Err(interest_invalid(
                    "limit must be between 1 and 500".to_string(),
                ));
            }
            let snapshots: Vec<forge::portfolio::interest::InterestSnapshot> = if project.is_empty()
            {
                let mut all = Vec::new();
                for record in registry.list()? {
                    all.extend(registry.interest_snapshots(&record.id, *limit)?);
                }
                all
            } else {
                registry.interest_snapshots(project, *limit)?
            };
            let human = if snapshots.is_empty() {
                "No interest snapshots.\n".to_string()
            } else {
                let mut out = String::new();
                for snapshot in &snapshots {
                    out.push_str(&format!(
                        "{} {} {} {}..{} {} {} {}\n",
                        snapshot.project_id,
                        snapshot.source,
                        snapshot.source_revision,
                        snapshot.window_start,
                        snapshot.window_end,
                        snapshot.state.label(),
                        snapshot.privacy_mode,
                        snapshot.freshness(now, *stale_after_days).label(),
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "stale_after_days": stale_after_days,
                    "snapshots": snapshots,
                }),
            ))
        }
        PortfolioInterestCommands::Show {
            project,
            stale_after_days,
        } => {
            let projection = report::project_interest(registry, project, *stale_after_days, now)?;
            let human = format!(
                "project: {}\nstale after: {} day(s)\nstale windows: {}\nsuperseded revisions: {}\n",
                projection.project_id,
                projection.stale_after_days,
                projection.stale_snapshots,
                projection.superseded_snapshots
            );
            let mut human = human;
            if projection.snapshots.is_empty() {
                human.push_str("snapshots: none\n");
            } else {
                human.push_str("snapshots:\n");
                for snapshot in &projection.snapshots {
                    human.push_str(&format!(
                        "  {} {}..{} {} {} {}\n",
                        snapshot.source,
                        snapshot.window_start,
                        snapshot.window_end,
                        snapshot.source_revision,
                        snapshot.state.label(),
                        snapshot.freshness(now, projection.stale_after_days).label(),
                    ));
                    for value in &snapshot.metrics {
                        human.push_str(&format!("    {} = {}\n", value.metric, value.value));
                    }
                }
            }
            if !projection.findings.is_empty() {
                human.push_str("refusals:\n");
                for finding in &projection.findings {
                    human.push_str(&format!(
                        "  {} [{}] {}\n",
                        finding.field, finding.code, finding.detail
                    ));
                }
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "interest": projection,
                }),
            ))
        }
        PortfolioInterestCommands::Compare {
            projects,
            metrics,
            source,
            stale_after_days,
        } => {
            let selected = parse_interest_metrics(metrics)?;
            let comparison = report::compare_projects(
                registry,
                projects,
                &selected,
                source.as_deref(),
                *stale_after_days,
                now,
            )?;
            let human = render_comparison(&comparison);
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "comparison": comparison,
                }),
            ))
        }
        PortfolioInterestCommands::Trend {
            project,
            metric,
            limit,
            stale_after_days,
        } => {
            let metric = InterestMetric::parse(metric.trim()).map_err(interest_invalid)?;
            let trend =
                report::interest_trend(registry, project, metric, *limit, *stale_after_days, now)?;
            let human = format!(
                "{} {}\n{} window(s) plotted, {} window(s) reported no such metric\nnote: {}\n",
                trend.project_id,
                trend.metric,
                trend.points.len(),
                trend.unreported_windows,
                trend.note
            );
            let mut human = human;
            for point in &trend.points {
                human.push_str(&format!(
                    "  {}..{} {} {} {}\n",
                    point.window_start,
                    point.window_end,
                    point.value,
                    point.privacy_mode,
                    point.freshness
                ));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "trend": trend,
                }),
            ))
        }
        PortfolioInterestCommands::Audit { limit } => {
            if *limit == 0 || *limit > 500 {
                return Err(interest_invalid(
                    "limit must be between 1 and 500".to_string(),
                ));
            }
            let findings = registry.interest_findings(*limit)?;
            let human = if findings.is_empty() {
                "No refused interest snapshots.\n".to_string()
            } else {
                let mut out = String::new();
                for finding in &findings {
                    out.push_str(&format!(
                        "  {} {} [{}] {}\n",
                        finding.project_id, finding.field, finding.code, finding.detail
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": contract,
                    "generated_at": now.to_rfc3339(),
                    "refusals": findings,
                }),
            ))
        }
    }
}

/// The one CLI entry point for activation readiness. Mirrors
/// `cmd_fleet_online` exactly: build the report, print it, then return
/// the typed gate error when any evaluated project is `not-ready`.
/// Input errors are typed `portfolio-interest-invalid` refusals with
/// empty stdout; only a verdict report may print on stdout with a
/// non-zero exit.
pub(super) fn cmd_portfolio_activation(
    registry: &Registry,
    command: &PortfolioActivationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::portfolio::interest::{self as interest, InterestMetric};
    use forge::portfolio::interest_report as report;
    let now = chrono::Utc::now();
    let contract = interest::ACTIVATION_CONTRACT_VERSION;
    match command {
        PortfolioActivationCommands::Readiness {
            project,
            metric,
            min_value,
            source,
            window,
            stale_after_days,
        } => {
            let metric = InterestMetric::parse(metric.trim()).map_err(interest_invalid)?;
            let threshold = match min_value {
                Some(value) => {
                    Some(interest::validate_threshold(*value).map_err(interest_invalid)?)
                }
                None => None,
            };
            let source = match source {
                Some(raw) if raw.trim().is_empty() => {
                    return Err(interest_invalid("source must not be blank".to_string()));
                }
                Some(raw) => Some(raw.trim().to_string()),
                None => None,
            };
            let parsed_window = match window {
                Some(raw) => Some(interest::parse_window(raw.trim()).map_err(interest_invalid)?),
                None => None,
            };
            let window_pair = parsed_window
                .as_ref()
                .map(|(start, end)| (start.as_str(), end.as_str()));
            let projects: Vec<String> = if project.trim().is_empty() {
                let records = registry.list()?;
                if records.is_empty() {
                    return Err(interest_invalid(
                        "readiness needs at least one project to evaluate".to_string(),
                    ));
                }
                records.into_iter().map(|record| record.id).collect()
            } else {
                vec![project.trim().to_string()]
            };
            let activation = report::activation_readiness(
                registry,
                &projects,
                metric,
                threshold,
                source.as_deref(),
                window_pair,
                *stale_after_days,
                now,
            )?;
            let human = render_activation(&activation);
            let json = serde_json::json!({
                "contract": contract,
                "generated_at": now.to_rfc3339(),
                "activation": {
                    "metric": activation.metric,
                    "threshold": activation.threshold,
                    "stale_after_days": activation.stale_after_days,
                    "requested_window": activation.requested_window,
                    "requested_source": activation.requested_source,
                    "ready": activation.is_ready(),
                    "ready_count": activation.ready_count,
                    "not_ready_count": activation.not_ready_count,
                    "verdicts": activation.verdicts,
                },
            });
            let output = as_output(format, human, json);
            if !activation.is_ready() {
                match &output {
                    Output::Human(text) => println!("{text}"),
                    Output::Json(value) => {
                        println!("{}", serde_json::to_string_pretty(value).unwrap());
                    }
                    Output::Raw(text) => print!("{text}"),
                }
                return Err(ForgeError::PortfolioActivationNotReady {
                    reason: format!(
                        "{} of {} project(s) are not ready ({})",
                        activation.not_ready_count,
                        activation.verdicts.len(),
                        activation.verdict_reason_labels().join(", "),
                    ),
                });
            }
            Ok(output)
        }
    }
}

/// Parse the `--window <START>..<END>` flag shared by the activation
/// readiness command. Both sides normalize through the interest
/// timestamp rule so an offset form matches stored UTC.
#[allow(dead_code)]
fn parse_activation_window(raw: &str) -> Result<(String, String), ForgeError> {
    forge::portfolio::interest::parse_window(raw.trim()).map_err(interest_invalid)
}

/// Render a readiness report: one header line, one block per verdict
/// in verdict order, then the summary.
fn render_activation(activation: &forge::portfolio::interest::ActivationReport) -> String {
    let mut out = String::new();
    let threshold = activation
        .threshold
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    out.push_str(&format!(
        "activation readiness ({}): metric={} threshold={} stale_after_days={}\n",
        forge::portfolio::interest::ACTIVATION_CONTRACT_VERSION,
        activation.metric,
        threshold,
        activation.stale_after_days,
    ));
    for verdict in &activation.verdicts {
        if let Some(evidence) = &verdict.evidence {
            out.push_str(&format!(
                "  {} {}      window={}..{} source={} revision={} privacy={} coverage={} freshness={} value={}\n",
                verdict.project_id,
                verdict.readiness.label(),
                evidence.window_start,
                evidence.window_end,
                evidence.source,
                evidence.source_revision,
                evidence.privacy_mode,
                evidence.coverage,
                evidence.freshness,
                evidence.value,
            ));
        } else {
            out.push_str(&format!(
                "  {} {}\n",
                verdict.project_id,
                verdict.readiness.label(),
            ));
        }
        for reason in &verdict.reasons {
            out.push_str(&format!(
                "      {}: {}\n",
                reason.reason.label(),
                reason.detail
            ));
        }
        for note in &verdict.notes {
            out.push_str(&format!("    {note}\n"));
        }
    }
    out.push_str(&format!(
        "summary: ready={} not-ready={}\n",
        activation.ready_count, activation.not_ready_count
    ));
    out
}

/// Summarize a fully refused batch into one refusal message.
///
/// The message names each refused record's index, its stable code and
/// its reason — so a one-record batch is diagnosable without a second
/// command, and a five-hundred record batch cannot produce a
/// five-hundred line error. Each detail has already been scrubbed by
/// the domain, so no offending value can reach the message.
fn summarize_refusals(rejected: &[forge::portfolio::interest::InterestRejection]) -> String {
    const MAX_LISTED: usize = 5;
    const MAX_DETAIL_CHARS: usize = 200;
    let listed: Vec<String> = rejected
        .iter()
        .take(MAX_LISTED)
        .map(|rejection| {
            let detail: String = rejection.detail.chars().take(MAX_DETAIL_CHARS).collect();
            let ellipsis = if rejection.detail.chars().count() > MAX_DETAIL_CHARS {
                "…"
            } else {
                ""
            };
            format!(
                "record {} [{}]: {detail}{ellipsis}",
                rejection.index, rejection.code
            )
        })
        .collect();
    let mut summary = listed.join("; ");
    if rejected.len() > MAX_LISTED {
        summary.push_str(&format!(
            "; and {} more, see `forge portfolio interest audit`",
            rejected.len() - MAX_LISTED
        ));
    }
    summary
}

/// Read an import document from a path or from stdin, refusing
/// anything past the size bound before it is parsed.
fn read_import_document(file: &str) -> Result<String, ForgeError> {
    const MAX: usize = forge::portfolio::interest::MAX_IMPORT_BYTES;
    let trimmed = file.trim();
    if trimmed == "-" {
        // Read one byte past the bound so an oversized document is
        // detected rather than silently truncated into a parse error
        // that would read like malformed JSON.
        let mut reader = std::io::Read::take(std::io::stdin().lock(), MAX as u64 + 1);
        let mut raw = String::new();
        std::io::Read::read_to_string(&mut reader, &mut raw)
            .map_err(|err| interest_invalid(format!("import document could not be read: {err}")))?;
        if raw.len() > MAX {
            return Err(interest_invalid(format!(
                "import document is larger than {MAX} bytes"
            )));
        }
        return Ok(raw);
    }
    let metadata = std::fs::metadata(trimmed).map_err(|err| {
        interest_invalid(format!("import document `{trimmed}` is unreadable: {err}"))
    })?;
    if metadata.len() > MAX as u64 {
        return Err(interest_invalid(format!(
            "import document is larger than {MAX} bytes"
        )));
    }
    std::fs::read_to_string(trimmed).map_err(|err| {
        interest_invalid(format!("import document `{trimmed}` is unreadable: {err}"))
    })
}

/// Resolve the requested metrics, defaulting to the whole allowlist.
///
/// An unknown metric is a typed refusal rather than a silent skip: an
/// importer that asked for a metric Forge does not carry should learn
/// that, not see a comparison quietly missing a column.
fn parse_interest_metrics(
    raw: &[String],
) -> Result<Vec<forge::portfolio::interest::InterestMetric>, ForgeError> {
    use forge::portfolio::interest::InterestMetric;
    if raw.is_empty() {
        return Ok(InterestMetric::ALL.to_vec());
    }
    let mut selected = Vec::with_capacity(raw.len());
    for value in raw {
        let metric = InterestMetric::parse(value.trim()).map_err(interest_invalid)?;
        if !selected.contains(&metric) {
            selected.push(metric);
        }
    }
    Ok(selected)
}

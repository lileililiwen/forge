//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::portal::{
    build_dashboard_with_fleet, build_section_view_with_fleet, parse_section,
    render_dashboard_human, render_section_human as render_portal_section_human, FleetProjection,
    PortalDashboard, PORTAL_CONTRACT_VERSION, PORTAL_SYNTHETIC_PROJECT,
};
use forge::registry::Registry;
use std::path::Path;

use super::analytics::cmd_analytics_inspect;
use super::api::cmd_api_serve;
use super::commands::PortalCommands;
use super::commands_ops::{AnalyticsCommands, ReadinessCommands};
use super::commands_portfolio::{
    PortfolioCommands, PortfolioEvidenceCommands, PortfolioGoalCommands, PortfolioRelationCommands,
    PortfolioReviewCommands, PortfolioTagCommands,
};
use super::commands_services::{ApiCommands, WebCommands};
use super::fleet_exec::portal_fleet_projection;
use super::functions_10::{cmd_portfolio_activation, cmd_portfolio_interest, cmd_portfolio_share};
use super::functions_13::cmd_analytics_metrics;
use super::projects::{as_output, open_registry};
use super::readiness::{cmd_readiness_artifact, cmd_readiness_check, cmd_readiness_matrix};
use crate::{Format, Output};

/// Read exactly one password line from stdin for `--password-stdin`. Unlike
/// [`read_secret`] this never requires a terminal and never prompts, so a
/// script can pipe the value in without exposing it in argv.
pub(super) fn read_password_stdin() -> Result<String, ForgeError> {
    use std::io::BufRead;
    let mut value = String::new();
    let read = std::io::stdin().lock().read_line(&mut value);
    read.map_err(|_| ForgeError::IdentityInvalid {
        reason: "cannot read password from stdin".to_string(),
    })?;
    if value.is_empty() {
        return Err(ForgeError::IdentityInvalid {
            reason: "no password was provided on stdin".to_string(),
        });
    }
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

pub(super) fn delete_session_file_at(
    dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<(), ForgeError> {
    let path = forge::identity::session_path_for(dir, project_id, session_id)?;
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(&path).map_err(|err| ForgeError::IdentityInvalid {
        reason: format!("cannot delete session file {}: {err}", path.display()),
    })?;
    Ok(())
}

pub(super) fn parse_rfc3339(
    raw: &str,
    field: &str,
) -> Result<chrono::DateTime<chrono::Utc>, ForgeError> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|err| ForgeError::IdentityInvalid {
            reason: format!("identity {field} `{raw}` is not RFC 3339: {err}"),
        })
}

pub(crate) fn cmd_analytics(
    db_path: &Path,
    command: &AnalyticsCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        AnalyticsCommands::Inspect { target, dry_run } => {
            cmd_analytics_inspect(db_path, target, *dry_run, format)
        }
        AnalyticsCommands::Metrics {
            target,
            window_days,
            all,
        } => cmd_analytics_metrics(db_path, target, *window_days, *all, format),
    }
}

pub(crate) fn cmd_api(
    db_path: &Path,
    command: &ApiCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ApiCommands::Serve {
            bind,
            port,
            max_body_bytes,
        } => cmd_api_serve(db_path, *bind, *port, *max_body_bytes, format),
    }
}

pub(crate) fn cmd_web(command: &WebCommands, _format: Format) -> Result<Output, ForgeError> {
    match command {
        WebCommands::Serve { bind, port, root } => {
            let root = root
                .canonicalize()
                .map_err(|_| ForgeError::PathUnavailable {
                    path: root.display().to_string(),
                })?;
            println!(
                "forge web serving frontend at http://{}:{}/ (static files from {}) — Ctrl-C to stop",
                bind,
                port,
                root.display()
            );
            let accepted = forge::web::serve(*bind, *port, &root)
                .map_err(|reason| ForgeError::ApiInvalid { reason })?;
            Ok(Output::Human(format!(
                "forge web stopped after {accepted} request(s)"
            )))
        }
    }
}

pub(crate) fn cmd_portal(
    db_path: &Path,
    command: &PortalCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortalCommands::Dashboard { target, all } => {
            cmd_portal_dashboard(db_path, target, *all, format)
        }
        PortalCommands::View { section, target } => {
            cmd_portal_view(db_path, section, target, format)
        }
    }
}

fn cmd_portal_dashboard(
    db_path: &Path,
    target: &str,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let resolved_target = if all { "" } else { target };
    let query: Option<&str> = if resolved_target.trim().is_empty() {
        None
    } else {
        Some(resolved_target.trim())
    };
    let fleet_source = portal_fleet_projection(&registry);
    let projection = match fleet_source.as_ref() {
        None => None,
        Some(Ok(report)) => Some(FleetProjection::Configured(report)),
        Some(Err(err)) => Some(FleetProjection::Failed {
            code: err.code(),
            reason: err.to_string(),
        }),
    };
    let dashboard = build_dashboard_with_fleet(&registry, query, projection)?;
    let detail = portal_journal_detail(&dashboard);
    journal_portal_operation(&registry, dashboard.project_id.as_deref(), "done", &detail);
    let json = serde_json::json!({
        "contract": PORTAL_CONTRACT_VERSION,
        "dashboard": dashboard,
    });
    let human = render_dashboard_human(&dashboard);
    Ok(as_output(format, human, json))
}

fn cmd_portal_view(
    db_path: &Path,
    section: &str,
    target: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let parsed = parse_section(section)?;
    let registry = open_registry(db_path)?;
    let query: Option<&str> = if target.trim().is_empty() {
        None
    } else {
        Some(target.trim())
    };
    let fleet_source = portal_fleet_projection(&registry);
    let projection = match fleet_source.as_ref() {
        None => None,
        Some(Ok(report)) => Some(FleetProjection::Configured(report)),
        Some(Err(err)) => Some(FleetProjection::Failed {
            code: err.code(),
            reason: err.to_string(),
        }),
    };
    let view = build_section_view_with_fleet(&registry, query, parsed, projection)?;
    let detail = format!(
        "view: section={id} status={status} entries={n}",
        id = view.section_id,
        status = view.status.id(),
        n = view.entries.len()
    );
    journal_portal_operation(&registry, view.project_id.as_deref(), "done", &detail);
    let json = serde_json::json!({
        "contract": PORTAL_CONTRACT_VERSION,
        "view": view,
    });
    let human = render_portal_section_human(&view);
    Ok(as_output(format, human, json))
}

fn portal_journal_detail(dashboard: &PortalDashboard) -> String {
    format!(
        "dashboard: scope={scope} sections={n} rollup={rollup}",
        scope = dashboard.scope.id(),
        n = dashboard.section_count,
        rollup = dashboard.rollup().id()
    )
}

fn journal_portal_operation(
    registry: &Registry,
    project_id: Option<&str>,
    state: &str,
    detail: &str,
) {
    let journal_project = project_id.unwrap_or(PORTAL_SYNTHETIC_PROJECT);
    let _ = registry.record_operation("portal", journal_project, state, detail);
}

pub(crate) fn cmd_readiness(
    command: &ReadinessCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ReadinessCommands::Matrix { profiles } => cmd_readiness_matrix(profiles, format),
        ReadinessCommands::Artifact => cmd_readiness_artifact(format),
        ReadinessCommands::Check { profiles } => cmd_readiness_check(profiles, format),
    }
}

fn portfolio_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInvalid { reason }
}

fn parse_lifecycle(raw: &str) -> Result<forge::portfolio::Lifecycle, ForgeError> {
    forge::portfolio::Lifecycle::parse(raw).map_err(portfolio_invalid)
}

pub(super) fn parse_confidence(raw: &str) -> Result<forge::portfolio::Confidence, ForgeError> {
    forge::portfolio::Confidence::parse(raw).map_err(portfolio_invalid)
}

fn parse_relation_type(raw: &str) -> Result<forge::portfolio::RelationType, ForgeError> {
    forge::portfolio::RelationType::parse(raw).map_err(portfolio_invalid)
}

fn parse_evidence_status(raw: &str) -> Result<forge::portfolio::EvidenceStatus, ForgeError> {
    forge::portfolio::EvidenceStatus::parse(raw).map_err(portfolio_invalid)
}

/// Read an evidence payload supplied as inline JSON or as `@path`.
/// A file read is bounded by the same [`forge::portfolio::MAX_EVIDENCE_BYTES`]
/// cap as an inline payload, so a large file cannot pin the CLI.
pub(super) fn read_evidence_payload(raw: &str) -> Result<String, ForgeError> {
    let trimmed = raw.trim();
    let Some(path) = trimmed.strip_prefix('@') else {
        return Ok(trimmed.to_string());
    };
    let metadata = std::fs::metadata(path).map_err(|err| {
        portfolio_invalid(format!("evidence file `{path}` could not be read: {err}"))
    })?;
    if metadata.len() > forge::portfolio::MAX_EVIDENCE_BYTES as u64 {
        return Err(portfolio_invalid(format!(
            "evidence file `{path}` is larger than {} bytes",
            forge::portfolio::MAX_EVIDENCE_BYTES
        )));
    }
    std::fs::read_to_string(path).map_err(|err| {
        portfolio_invalid(format!("evidence file `{path}` could not be read: {err}"))
    })
}

pub(crate) fn cmd_portfolio(
    db_path: &Path,
    command: &PortfolioCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    match command {
        PortfolioCommands::Show { project } => cmd_portfolio_show(&registry, project, format),
        PortfolioCommands::Tag { command } => cmd_portfolio_tag(&registry, command, format),
        PortfolioCommands::Relation { command } => {
            cmd_portfolio_relation(&registry, command, format)
        }
        PortfolioCommands::Review { command } => cmd_portfolio_review(&registry, command, format),
        PortfolioCommands::Goal { command } => cmd_portfolio_goal(&registry, command, format),
        PortfolioCommands::Evidence { command } => {
            cmd_portfolio_evidence(&registry, command, format)
        }
        PortfolioCommands::Share { command } => cmd_portfolio_share(&registry, command, format),
        PortfolioCommands::Interest { command } => {
            cmd_portfolio_interest(&registry, command, format)
        }
        PortfolioCommands::Activation { command } => {
            cmd_portfolio_activation(&registry, command, format)
        }
    }
}

fn cmd_portfolio_show(
    registry: &Registry,
    project: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let now = chrono::Utc::now();
    let view = registry.portfolio_project_view(project, now)?;
    let human = {
        let mut out = String::new();
        out.push_str(&format!("project: {}\n", view.profile.project_id));
        out.push_str(&format!(
            "lifecycle: {}\n",
            view.profile
                .lifecycle
                .map(|v| v.label().to_string())
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "confidence: {}\n",
            view.profile
                .confidence
                .map(|v| v.label().to_string())
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "next action: {}\n",
            view.profile
                .next_action
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "blocker: {}\n",
            view.profile
                .blocker
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "reviewed at: {}\n",
            view.profile
                .reviewed_at
                .clone()
                .unwrap_or_else(|| "—".to_string())
        ));
        out.push_str(&format!(
            "tags: {}\n",
            if view.tags.is_empty() {
                "—".to_string()
            } else {
                view.tags
                    .iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        ));
        out.push_str(&format!(
            "goals: {}\n",
            if view.goals.is_empty() {
                "—".to_string()
            } else {
                view.goals
                    .iter()
                    .map(|g| format!("{} [{}]", g.title, g.status))
                    .collect::<Vec<String>>()
                    .join(", ")
            }
        ));
        out.push_str("relations:\n");
        if view.relations.is_empty() {
            out.push_str("  —\n");
        }
        for relation in &view.relations {
            out.push_str(&format!(
                "  {} {} {}",
                relation.direction,
                relation.relation_type.label(),
                relation.other_project
            ));
            if let Some(note) = &relation.note {
                out.push_str(&format!(" — {note}"));
            }
            out.push('\n');
        }
        out.push_str("evidence:\n");
        if view.evidence.is_empty() {
            out.push_str("  — no source observation imported\n");
        }
        for snapshot in &view.evidence {
            out.push_str(&format!(
                "  {} {} (source revision {}, observed {}",
                snapshot.source_system,
                snapshot.effective_status.label(),
                snapshot.source_revision,
                snapshot.observed_at
            ));
            if let Some(bound) = &snapshot.stale_after {
                out.push_str(&format!(", stale after {bound}"));
            }
            out.push_str(")\n");
        }
        out.push_str("reviews:\n");
        if view.reviews.is_empty() {
            out.push_str("  —\n");
        }
        for review in &view.reviews {
            out.push_str(&format!(
                "  {} {}",
                review.reviewed_at,
                review.confidence.label()
            ));
            if let Some(note) = &review.note {
                out.push_str(&format!(" — {note}"));
            }
            out.push('\n');
        }
        out
    };
    Ok(as_output(
        format,
        human,
        serde_json::to_value(&view).unwrap(),
    ))
}

fn cmd_portfolio_tag(
    registry: &Registry,
    command: &PortfolioTagCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioTagCommands::Add {
            project,
            name,
            color,
        } => {
            let tag = registry.portfolio_add_tag(project, name, color.as_deref())?;
            let human = format!(
                "tagged {} with {} (tag id {})\n",
                project, tag.name, tag.tag_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tag": tag,
                }),
            ))
        }
        PortfolioTagCommands::Remove { project, name } => {
            let removed = registry.portfolio_remove_tag(project, name)?;
            let human = if removed {
                format!("removed tag {name} from {project}\n")
            } else {
                format!("tag {name} was not attached to {project}; nothing changed\n")
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tag": name,
                    "removed": removed,
                }),
            ))
        }
        PortfolioTagCommands::List { project } => {
            let all = registry.portfolio_all_tags()?;
            if project.trim().is_empty() {
                let human = if all.is_empty() {
                    "No portfolio tags.".to_string()
                } else {
                    let mut out = String::new();
                    for (tag, projects) in &all {
                        out.push_str(&format!(
                            "{} {}{}\n",
                            tag.name,
                            tag.color.clone().unwrap_or_else(|| "—".to_string()),
                            if projects.is_empty() {
                                String::new()
                            } else {
                                format!(" [{}]", projects.join(", "))
                            }
                        ));
                    }
                    out
                };
                let json: Vec<serde_json::Value> = all
                    .iter()
                    .map(|(tag, projects)| serde_json::json!({ "tag": tag, "projects": projects }))
                    .collect();
                return Ok(as_output(
                    format,
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "tags": json,
                    }),
                ));
            }
            let tags = registry.portfolio_tags_for(project)?;
            let human = if tags.is_empty() {
                format!("No portfolio tags on {project}.\n")
            } else {
                let mut out = String::new();
                for tag in &tags {
                    out.push_str(&format!(
                        "{} {}\n",
                        tag.name,
                        tag.color.clone().unwrap_or_else(|| "—".to_string())
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "tags": tags,
                }),
            ))
        }
    }
}

fn cmd_portfolio_relation(
    registry: &Registry,
    command: &PortfolioRelationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioRelationCommands::Add {
            from,
            to,
            r#type,
            note,
        } => {
            let kind = parse_relation_type(r#type)?;
            let relation = registry.portfolio_add_relation(from, to, kind, note.as_deref())?;
            let human = format!(
                "{} {} {} (relation {})\n",
                relation.from_project,
                relation.relation_type.label(),
                relation.to_project,
                relation.relation_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "relation": relation,
                }),
            ))
        }
        PortfolioRelationCommands::Remove { from, to, r#type } => {
            let kind = parse_relation_type(r#type)?;
            let removed = registry.portfolio_remove_relation(from, to, kind)?;
            let human = if removed {
                format!("removed {from} {} {to}\n", kind.label())
            } else {
                format!(
                    "{from} {} {to} was not recorded; nothing changed\n",
                    kind.label()
                )
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "from": from,
                    "to": to,
                    "relation_type": kind.label(),
                    "removed": removed,
                }),
            ))
        }
        PortfolioRelationCommands::List { project } => {
            let (human, json) = if project.trim().is_empty() {
                let all = registry.portfolio_all_relations()?;
                let human = if all.is_empty() {
                    "No portfolio relations.".to_string()
                } else {
                    let mut out = String::new();
                    for relation in &all {
                        out.push_str(&format!(
                            "{} {} {} (relation {})\n",
                            relation.from_project,
                            relation.relation_type.label(),
                            relation.to_project,
                            relation.relation_id
                        ));
                    }
                    out
                };
                (
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "relations": all,
                    }),
                )
            } else {
                let relations = registry.portfolio_relations_for(project)?;
                let human = if relations.is_empty() {
                    format!("No portfolio relations on {project}.\n")
                } else {
                    let mut out = String::new();
                    for relation in &relations {
                        out.push_str(&format!(
                            "{} {} {}\n",
                            relation.direction,
                            relation.relation_type.label(),
                            relation.other_project
                        ));
                    }
                    out
                };
                (
                    human,
                    serde_json::json!({
                        "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                        "project_id": project,
                        "relations": relations,
                    }),
                )
            };
            Ok(as_output(format, human, json))
        }
    }
}

fn cmd_portfolio_review(
    registry: &Registry,
    command: &PortfolioReviewCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioReviewCommands::Set {
            project,
            confidence,
            note,
            lifecycle,
            next_action,
            blocker,
        } => {
            let confidence_value = parse_confidence(confidence)?;
            let lifecycle_value = lifecycle.as_deref().map(parse_lifecycle).transpose()?;
            let write = forge::registry::PortfolioWrite {
                lifecycle: lifecycle_value,
                confidence: Some(confidence_value),
                next_action: next_action.clone(),
                blocker: blocker.clone(),
            };
            let profile = registry.portfolio_write(project, &write)?;
            let review =
                registry.portfolio_record_review(project, confidence_value, note.as_deref())?;
            let human = format!(
                "reviewed {project}: confidence {}, lifecycle {}, next action {}, blocker {}\n",
                confidence_value.label(),
                lifecycle_value.map(|v| v.label()).unwrap_or("unchanged"),
                profile
                    .next_action
                    .clone()
                    .unwrap_or_else(|| "—".to_string()),
                profile.blocker.clone().unwrap_or_else(|| "—".to_string()),
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "profile": profile,
                    "review": review,
                }),
            ))
        }
        PortfolioReviewCommands::List { project } => {
            let reviews = registry.portfolio_reviews_for(project, 50)?;
            let human = if reviews.is_empty() {
                format!("No portfolio reviews for {project}.\n")
            } else {
                let mut out = String::new();
                for review in &reviews {
                    out.push_str(&format!(
                        "{} {}",
                        review.reviewed_at,
                        review.confidence.label()
                    ));
                    if let Some(note) = &review.note {
                        out.push_str(&format!(" — {note}"));
                    }
                    out.push('\n');
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "reviews": reviews,
                }),
            ))
        }
    }
}

fn cmd_portfolio_goal(
    registry: &Registry,
    command: &PortfolioGoalCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioGoalCommands::Add {
            title,
            status,
            description,
        } => {
            let goal = registry.portfolio_add_goal(title, status, description.as_deref())?;
            let human = format!(
                "goal {} [{}] (goal {})\n",
                goal.title, goal.status, goal.goal_id
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goal": goal,
                }),
            ))
        }
        PortfolioGoalCommands::Link { goal, project } => {
            let linked = registry.portfolio_link_goal(goal, project)?;
            let human = format!(
                "linked {} to {} ({})\n",
                linked.projects.join(", "),
                linked.title,
                linked.status
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goal": linked,
                }),
            ))
        }
        PortfolioGoalCommands::List => {
            let goals = registry.portfolio_goals()?;
            let human = if goals.is_empty() {
                "No portfolio goals.".to_string()
            } else {
                let mut out = String::new();
                for goal in &goals {
                    out.push_str(&format!(
                        "{} [{}] {}\n",
                        goal.title,
                        goal.status,
                        if goal.projects.is_empty() {
                            "—".to_string()
                        } else {
                            goal.projects.join(", ")
                        }
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "goals": goals,
                }),
            ))
        }
    }
}

fn cmd_portfolio_evidence(
    registry: &Registry,
    command: &PortfolioEvidenceCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        PortfolioEvidenceCommands::Import {
            project,
            source,
            revision,
            status,
            observed_at,
            stale_after,
            evidence,
        } => {
            let status_value = parse_evidence_status(status)?;
            let payload = read_evidence_payload(evidence)?;
            let write = forge::registry::SnapshotWrite {
                source_system: source.clone(),
                source_revision: revision.clone(),
                observed_at: observed_at
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                status: status_value,
                stale_after: stale_after.clone(),
                evidence_json: payload,
            };
            let snapshot = registry.portfolio_import_snapshot(project, &write)?;
            let human = format!(
                "imported {} snapshot {} for {} (source revision {})\n",
                snapshot.status.label(),
                snapshot.snapshot_id,
                project,
                snapshot.source_revision
            );
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "snapshot": snapshot,
                }),
            ))
        }
        PortfolioEvidenceCommands::List { project } => {
            let now = chrono::Utc::now();
            let mut snapshots = registry.portfolio_snapshots_for(project, 100)?;
            for snapshot in &mut snapshots {
                snapshot.effective_status =
                    forge::portfolio::snapshot_effective_status(snapshot, now);
            }
            let human = if snapshots.is_empty() {
                format!("No portfolio evidence snapshots for {project}.\n")
            } else {
                let mut out = String::new();
                for snapshot in &snapshots {
                    out.push_str(&format!(
                        "{} {} {} {} (source revision {})\n",
                        snapshot.observed_at,
                        snapshot.source_system,
                        snapshot.status.label(),
                        snapshot.effective_status.label(),
                        snapshot.source_revision
                    ));
                }
                out
            };
            Ok(as_output(
                format,
                human,
                serde_json::json!({
                    "contract": forge::portfolio::PORTFOLIO_CONTRACT_VERSION,
                    "project_id": project,
                    "generated_at": now.to_rfc3339(),
                    "snapshots": snapshots,
                }),
            ))
        }
    }
}

pub(super) fn share_invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioShareInvalid { reason }
}

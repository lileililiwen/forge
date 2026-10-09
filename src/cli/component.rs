//! Component commands (`component`).
//!
//! Typed CLI handlers for semantic component discovery and qualification.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::component::{
    component_catalog, inspect_component, record_qualification,
    render_outcome_human as render_component_outcome_human, render_qualify_human, resolve_outcome,
    ComponentQualifyEvidence, ComponentQualifyRequest, ComponentQuality,
};
use forge::core::ForgeError;
use std::path::{Path, PathBuf};

use super::commands_ops::ComponentCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(crate) fn cmd_component(
    db_path: &Path,
    command: &ComponentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ComponentCommands::List => {
            let catalog = component_catalog();
            let entries: Vec<serde_json::Value> = catalog
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "version": c.version,
                        "quality": c.quality.label(),
                        "profiles": c.profiles,
                        "purpose": c.purpose,
                    })
                })
                .collect();
            let mut human = format!(
                "{:<28} {:<10} {:<8} {}",
                "Component", "Version", "Quality", "Profiles"
            );
            for c in &catalog {
                human.push_str(&format!(
                    "\n{:<28} {:<10} {:<8} {}",
                    c.id,
                    c.version,
                    c.quality.label(),
                    c.profiles.join(",")
                ));
            }
            Ok(as_output(
                format,
                human,
                serde_json::json!({"components": entries}),
            ))
        }
        ComponentCommands::Inspect { id } => {
            let descriptor = inspect_component(id)?;
            let json = serde_json::json!({
                "id": descriptor.id,
                "version": descriptor.version,
                "purpose": descriptor.purpose,
                "quality": descriptor.quality.label(),
                "profiles": descriptor.profiles,
                "depends_on": descriptor.depends_on,
                "install_strategy": descriptor.install_strategy,
                "validation": descriptor.validation,
                "documentation": descriptor.documentation,
                "tests": descriptor.tests,
                "contract": {
                    "inputs": descriptor.contract.inputs,
                    "outputs": descriptor.contract.outputs,
                },
                "evidence": descriptor.evidence,
            });
            let mut human = vec![
                format!("component: {}@{}", descriptor.id, descriptor.version),
                format!("quality: {}", descriptor.quality.label()),
                format!("purpose: {}", descriptor.purpose),
                format!("profiles: {}", descriptor.profiles.join(", ")),
                format!("install: {}", descriptor.install_strategy),
                format!("tests: {}", descriptor.tests),
                format!("documentation: {}", descriptor.documentation),
                "contract inputs:".to_string(),
            ];
            for port in &descriptor.contract.inputs {
                human.push(format!("  - {}: {}", port.name, port.description));
            }
            human.push("contract outputs:".to_string());
            for port in &descriptor.contract.outputs {
                human.push(format!("  - {}: {}", port.name, port.description));
            }
            human.push("evidence:".to_string());
            human.push(format!(
                "  usage={} coverage={:.2} security_review={} last_verified={}",
                descriptor.evidence.usage_count,
                descriptor.evidence.test_coverage,
                descriptor.evidence.security_review,
                descriptor.evidence.last_verified.to_rfc3339()
            ));
            Ok(as_output(format, human.join("\n"), json))
        }
        ComponentCommands::Resolve {
            profile,
            components,
        } => {
            let request = forge::component::ComponentRequest {
                profile: profile.clone(),
                component_ids: components.clone(),
            };
            let outcome = resolve_outcome(&request)?;
            let journal_state = if outcome.plan.steps.is_empty() {
                "rejected"
            } else {
                "done"
            };
            let summary = format!(
                "{} resolved={} rejected={}",
                request.profile,
                outcome.plan.steps.len(),
                outcome.plan.rejections.len()
            );
            let note = format!("{}; {}", summary, outcome.note);
            // Journal: component operations are catalog-global; the
            // synthetic `__component__` project id keeps the
            // registry contract satisfied without inventing a
            // user-visible project.
            if let Ok(registry) = open_registry(db_path) {
                let _ =
                    registry.record_operation("component", "__component__", journal_state, &note);
            }
            let json = serde_json::json!({
                "profile": outcome.profile,
                "note": note,
                "plan": outcome.plan,
                "evidence_summary": outcome.evidence_summary,
            });
            let human = render_component_outcome_human(&outcome) + "\n" + &note;
            Ok(as_output(format, human, json))
        }
        ComponentCommands::Qualify {
            id,
            to,
            reason,
            coverage,
            last_verified,
            known_issues,
            security_review,
            path,
        } => {
            let target = match to.as_str() {
                "experimental" => ComponentQuality::Experimental,
                "verified" => ComponentQuality::Verified,
                "certified" => ComponentQuality::Certified,
                "deprecated" => ComponentQuality::Deprecated,
                other => {
                    return Err(ForgeError::ComponentInvalid {
                        reason: format!(
                            "unknown quality '{other}'; expected one of experimental, \
                             verified, certified, deprecated"
                        ),
                    });
                }
            };
            let last_verified_ts = match last_verified.as_deref() {
                Some(value) => parse_rfc3339_for_qualify(value).ok_or_else(|| {
                    ForgeError::ComponentInvalid {
                        reason: format!(
                            "last_verified '{value}' is not a valid RFC 3339 timestamp"
                        ),
                    }
                })?,
                None => chrono::Utc::now(),
            };
            let evidence = ComponentQualifyEvidence {
                test_coverage: coverage.unwrap_or(0.95),
                last_verified: last_verified_ts,
                known_issues: known_issues.clone(),
                security_review: security_review.unwrap_or(target == ComponentQuality::Certified),
            };
            let request = ComponentQualifyRequest {
                component_id: id.clone(),
                target_quality: target,
                reason: reason.clone(),
            };
            // Qualify writes the receipt to `.forge/components/<id>/qualify.json`
            // inside the named project directory (default: current
            // working directory).
            let work_dir = match path.as_deref() {
                Some(p) => p.to_path_buf(),
                None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            };
            let outcome = record_qualification(&work_dir, &request, &evidence)?;
            let journal_state = if outcome.promoted { "done" } else { "blocked" };
            let detail = format!(
                "{}: {} -> {} ({}); {}",
                outcome.component_id,
                outcome.prior_quality.label(),
                outcome.target_quality.label(),
                if outcome.promoted {
                    "promoted"
                } else {
                    "refused"
                },
                outcome.note
            );
            if let Ok(registry) = open_registry(db_path) {
                let _ =
                    registry.record_operation("component", "__component__", journal_state, &detail);
            }
            let json = serde_json::json!({
                "component_id": outcome.component_id,
                "prior_quality": outcome.prior_quality.label(),
                "target_quality": outcome.target_quality.label(),
                "promoted": outcome.promoted,
                "files_written": outcome.files_written,
                "note": outcome.note,
            });
            Ok(as_output(format, render_qualify_human(&outcome), json))
        }
    }
}

fn parse_rfc3339_for_qualify(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

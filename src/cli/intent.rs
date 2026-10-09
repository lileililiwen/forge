//! Intent commands (`intent`).
//!
//! Typed CLI handlers for structured intent validation and assembly plans.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::planner::{
    apply_plan as apply_planner_plan, intent_hash, plans_dir, render_apply_human,
    render_intent_validation_human, render_plan_human, resolve_plan as resolve_planner_plan,
    validate_intent, write_plan_receipt, Intent, IntentConstraint, IntentResolveOutcome,
    IntentValidationOutcome, PLANNER_CONTRACT_VERSION,
};
use std::path::{Path, PathBuf};

use super::commands::IntentCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

use forge::planner::IntentAction;

fn parse_intent_constraints(raw: &[String]) -> Result<Vec<IntentConstraint>, ForgeError> {
    let mut out = Vec::new();
    for entry in raw {
        let (key, value) = entry
            .split_once('=')
            .ok_or_else(|| ForgeError::IntentInvalid {
                reason: format!(
                    "constraint '{entry}' is not in 'key=value' form; the planner refuses a \
                 malformed constraint"
                ),
            })?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(ForgeError::IntentInvalid {
                reason: format!(
                    "constraint '{entry}' has an empty key or value; the planner refuses a \
                     malformed constraint"
                ),
            });
        }
        out.push(IntentConstraint {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}

fn build_intent(
    action: &str,
    profile: &str,
    required: &[String],
    forbidden: &[String],
    raw_constraints: &[String],
) -> Result<Intent, ForgeError> {
    Ok(Intent {
        action: parse_intent_action(action)?,
        profile: profile.to_string(),
        required_capabilities: required.to_vec(),
        forbidden_capabilities: forbidden.to_vec(),
        constraints: parse_intent_constraints(raw_constraints)?,
    })
}

pub(crate) fn cmd_intent(
    db_path: &Path,
    command: &IntentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        IntentCommands::Validate {
            action,
            profile,
            required,
            forbidden,
            constraints,
        } => {
            let intent = build_intent(action, profile, required, forbidden, constraints)?;
            match validate_intent(&intent) {
                Ok(validated) => {
                    let outcome = IntentValidationOutcome {
                        validated: Some(validated.clone()),
                        note: validated.note.clone(),
                    };
                    let journal_state = "done";
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "planner",
                            "__planner__",
                            journal_state,
                            &format!("validate: {}", validated.note),
                        );
                    }
                    let json = serde_json::json!({
                        "contract": PLANNER_CONTRACT_VERSION,
                        "validated": validated,
                        "intent_hash": intent_hash(&validated.intent),
                    });
                    Ok(as_output(
                        format,
                        render_intent_validation_human(&outcome),
                        json,
                    ))
                }
                Err(err) => {
                    let outcome = IntentValidationOutcome {
                        validated: None,
                        note: err.to_string(),
                    };
                    if let Ok(registry) = open_registry(db_path) {
                        let _ = registry.record_operation(
                            "planner",
                            "__planner__",
                            "rejected",
                            &outcome.note,
                        );
                    }
                    Err(err)
                }
            }
        }
        IntentCommands::Resolve {
            action,
            profile,
            required,
            forbidden,
            constraints,
            path,
        } => {
            let intent = build_intent(action, profile, required, forbidden, constraints)?;
            let validated = validate_intent(&intent)?;
            let work_dir = path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let plan = resolve_planner_plan(&validated, None)?;
            let receipt_path = write_plan_receipt(&work_dir, &plan)?;
            let outcome = IntentResolveOutcome {
                note: plan.note.clone(),
                plan: plan.clone(),
                receipt_path: receipt_path.display().to_string(),
            };
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "planner",
                    &plan.plan_id,
                    "done",
                    &format!("resolve: {}", plan.note),
                );
            }
            let json = serde_json::json!({
                "contract": PLANNER_CONTRACT_VERSION,
                "plan": plan,
                "receipt_path": outcome.receipt_path,
            });
            Ok(as_output(format, render_plan_human(&outcome.plan), json))
        }
        IntentCommands::Apply {
            plan_id,
            confirm,
            path,
        } => {
            let work_dir = path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let outcome = apply_planner_plan(&work_dir, plan_id, *confirm, Some(db_path))?;
            if let Ok(registry) = open_registry(db_path) {
                let _ = registry.record_operation(
                    "planner",
                    &outcome.plan_id,
                    if outcome.stale { "rejected" } else { "done" },
                    &outcome.note,
                );
            }
            let json = serde_json::json!({
                "contract": PLANNER_CONTRACT_VERSION,
                "outcome": outcome.clone(),
            });
            Ok(as_output(format, render_apply_human(&outcome), json))
        }
        IntentCommands::List { path } => {
            let mut entries: Vec<serde_json::Value> = Vec::new();
            let dir = plans_dir(path);
            if dir.exists() {
                if let Ok(read) = std::fs::read_dir(&dir) {
                    for entry in read.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let plan_id = entry.file_name().to_string_lossy().to_string();
                            entries.push(serde_json::json!({
                                "plan_id": plan_id,
                                "receipt": entry.path().join("plan.json").display().to_string(),
                            }));
                        }
                    }
                }
            }
            let mut human = format!("planner plans: {}", entries.len());
            for e in &entries {
                if let Some(id) = e.get("plan_id").and_then(|v| v.as_str()) {
                    human.push_str(&format!("\n  - {id}"));
                }
            }
            let json = serde_json::json!({"plans": entries});
            Ok(as_output(format, human, json))
        }
    }
}

pub(super) fn parse_intent_action(value: &str) -> Result<IntentAction, ForgeError> {
    match value {
        "create_project" => Ok(IntentAction::CreateProject),
        "extend_project" => Ok(IntentAction::ExtendProject),
        other => Err(ForgeError::IntentInvalid {
            reason: format!(
                "unknown intent action '{other}'; accepted actions: create_project, \
                 extend_project"
            ),
        }),
    }
}

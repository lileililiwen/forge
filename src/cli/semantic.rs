//! Review commands (`standard`/`remediate`/`describe`/`classify`).
//!
//! Typed CLI handlers for standard-pack review, remediation plans,
//! semantic proposal inspection and the classify preview/apply flow.
//! Bodies moved verbatim from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::remediation::{self, RemediationPlan};
use forge::semantic::{self, ProposalEvidence, ProposalKind, SuggestRequest};
use std::path::Path;

use super::commands::{ClassifyCommands, RemediateCommands, RemediationArgs, SemanticSuggestArgs};
use super::commands_ops::{DescribeCommands, StandardCommands};
use super::feature::{
    diff_change_word, file_state_word, pack_evidence_word, pack_state_word, resolve_spec_target,
    semantic_decide_output, snapshot_state_word,
};
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_standard(
    command: &StandardCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::standard;
    match command {
        StandardCommands::List => {
            let packs = standard::all_packs();
            let mut human = format!(
                "{:<18} {:<8} {:<11} {:<10} {}",
                "Pack", "Version", "State", "Evidence", "Compatible profiles"
            );
            for pack in &packs {
                human.push_str(&format!(
                    "\n{:<18} {:<8} {:<11} {:<10} {}",
                    pack.id,
                    pack.version,
                    pack_state_word(pack.support_state),
                    pack_evidence_word(pack.evidence),
                    pack.compatible_profiles.join(", ")
                ));
            }
            let json = serde_json::json!({"packs": packs});
            Ok(as_output(format, human, json))
        }
        StandardCommands::Inspect { pack } => {
            let descriptor = standard::inspect_pack(pack)?;
            let mut human = format!(
                "pack {}@{}\nsupport state: {}\nevidence: {}\nasset digest: {}\ncompatible profiles: {}",
                descriptor.id,
                descriptor.version,
                pack_state_word(descriptor.support_state),
                pack_evidence_word(descriptor.evidence),
                descriptor.asset_digest,
                descriptor.compatible_profiles.join(", ")
            );
            if let Some(source) = descriptor.external_source.as_deref() {
                human.push_str(&format!(
                    "\nexternal template source: {source} (bundled local fallback; never fetched)"
                ));
            }
            human.push_str(&format!(
                "\nselectable for new generation: {}",
                descriptor.is_selectable()
            ));
            let json = serde_json::to_value(&descriptor).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Check { path } => {
            let report = standard::check_snapshot(path)?;
            let mut human = format!(
                "standard snapshot: {} ({})",
                snapshot_state_word(report.state),
                report.path
            );
            if let Some(pack) = report.pack.as_deref() {
                human.push_str(&format!(
                    "\npack {}@{} for profile {}",
                    pack,
                    report.version.as_deref().unwrap_or("?"),
                    report.profile.as_deref().unwrap_or("?")
                ));
            }
            for file in &report.files {
                human.push_str(&format!(
                    "\n  {} {}",
                    file_state_word(file.state),
                    file.path
                ));
            }
            for issue in &report.issues {
                human.push_str(&format!("\nissue: {issue}"));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Diff { path, against } => {
            let report = standard::diff_snapshot(path, against)?;
            let mut human = format!(
                "standard diff {} -> {}\nproject {} (profile {})",
                report.from, report.against, report.project, report.profile
            );
            for entry in &report.entries {
                human.push_str(&format!(
                    "\n  {} {}",
                    diff_change_word(entry.change),
                    entry.path
                ));
            }
            if !report.conflicts.is_empty() {
                human.push_str(&format!(
                    "\nconflicts: {} (upgrade refuses without --force)",
                    report.conflicts.join(", ")
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        StandardCommands::Upgrade {
            path,
            to,
            confirm,
            force,
        } => {
            let report = standard::upgrade_snapshot(
                path,
                to,
                *confirm,
                *force,
                &chrono::Utc::now().to_rfc3339(),
            )?;
            let mut human = format!(
                "upgraded {} ({}) to {}; {} file(s) written",
                report.project,
                report.profile,
                report.to,
                report.written.len()
            );
            if !report.forced.is_empty() {
                human.push_str(&format!(
                    "\nforced replacements: {}",
                    report.forced.join(", ")
                ));
            }
            if !report.orphaned.is_empty() {
                human.push_str(&format!(
                    "\npreserved (not in target): {}",
                    report.orphaned.join(", ")
                ));
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
    }
}

pub(crate) fn cmd_remediate(
    registry_path: &Path,
    command: &RemediateCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let build = |args: &RemediationArgs| -> Result<RemediationPlan, ForgeError> {
        if let Some(path) = args.plan.as_deref() {
            remediation::load_plan(path)
        } else {
            remediation::build_plan(
                &args.target,
                args.finding
                    .as_deref()
                    .ok_or_else(|| ForgeError::RemediationInvalid {
                        reason: "--finding is required when --plan is not supplied".to_string(),
                    })?,
                args.pack.as_deref(),
            )
        }
    };
    match command {
        RemediateCommands::Scan(args) if args.finding.is_none() && args.plan.is_none() => {
            let report = remediation::scan(&args.target)?;
            let names: Vec<_> = report
                .findings
                .iter()
                .map(|finding| finding.finding_id.as_str())
                .collect();
            let human = if names.is_empty() {
                format!(
                    "no automatic remediation findings for {}",
                    report.target.project_id
                )
            } else {
                format!(
                    "automatic findings for {}: {}",
                    report.target.project_id,
                    names.join(", ")
                )
            };
            let json =
                serde_json::to_value(&report).map_err(|err| ForgeError::RemediationInvalid {
                    reason: format!("cannot serialize scan report: {err}"),
                })?;
            Ok(as_output(format, human, json))
        }
        RemediateCommands::Scan(args) | RemediateCommands::Plan(args) => {
            let plan = build(args)?;
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
            });
            Ok(as_output(
                format,
                format!(
                    "remediation plan {}\n{} action(s)",
                    json["plan"]["plan_id"],
                    json["plan"]["actions"].as_array().map_or(0, Vec::len)
                ),
                json,
            ))
        }
        RemediateCommands::Diff(args) => {
            let plan = build(args)?;
            let entries = remediation::diff(&plan);
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
                "diff": entries,
            });
            Ok(as_output(
                format,
                format!("remediation diff: {} action(s)", entries.len()),
                json,
            ))
        }
        RemediateCommands::Apply(args) => {
            let plan = build(args)?;
            let outcome = remediation::apply(&plan, args.confirm, registry_path)?;
            let json = serde_json::json!({
                "contract": remediation::REMEDIATION_CONTRACT_VERSION,
                "plan": plan,
                "outcome": outcome,
            });
            Ok(as_output(
                format,
                format!(
                    "remediation {}: {}",
                    json["outcome"]["status"], json["outcome"]["detail"]
                ),
                json,
            ))
        }
    }
}

pub(crate) fn cmd_describe(
    command: &DescribeCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DescribeCommands::Suggest(args) => {
            let request = build_suggest_request(ProposalKind::Description, args)?;
            let outcome = semantic::suggest(&request)?;
            semantic_suggest_output(&outcome, format)
        }
        DescribeCommands::List { target } => {
            let project_path = resolve_spec_target(target)?;
            let entries = semantic::list(&project_path)?;
            semantic_list_output(&entries, format)
        }
        DescribeCommands::Show { proposal, target } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            match semantic::read(&project_path, &id)? {
                Some(proposal) => semantic_show_output(&proposal, format),
                None => Err(ForgeError::SemanticInvalid {
                    reason: format!("proposal `{proposal}` was not found under `.forge/semantic/`"),
                }),
            }
        }
        DescribeCommands::Approve {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::approve(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
        DescribeCommands::Reject {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::reject(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
    }
}

pub(crate) fn cmd_classify(
    db_path: &Path,
    command: &ClassifyCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ClassifyCommands::Suggest(args) => {
            let kind = ProposalKind::Domain;
            let request = build_suggest_request(kind, args)?;
            let outcome = semantic::suggest(&request)?;
            semantic_suggest_output(&outcome, format)
        }
        ClassifyCommands::Apply { target, confirm } => {
            let project_path = resolve_spec_target(target)?;
            let outcome = forge::semantic::apply(&project_path, *confirm)?;
            let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&project_path, None)
                .map_err(|err| ForgeError::SemanticInvalid {
                    reason: format!("cannot resolve project id: {err}"),
                })?;
            let registry = forge::registry::Registry::open(db_path)?;
            registry.record_operation(
                "classify.apply",
                &manifest.project.id,
                "succeeded",
                &format!(
                    "plugin={} fields={} pr={}",
                    outcome.plugin_id,
                    outcome.fields.join(","),
                    outcome.pr_reference.as_deref().unwrap_or("unknown")
                ),
            )?;
            classify_apply_output(&outcome, format)
        }
        ClassifyCommands::Derive { target } => {
            let project_path = resolve_spec_target(target)?;
            let outcomes = forge::semantic::derive(&project_path)?;
            semantic_derive_output(&outcomes, format)
        }
        ClassifyCommands::List { target } => {
            let project_path = resolve_spec_target(target)?;
            let entries = semantic::list(&project_path)?;
            semantic_list_output(&entries, format)
        }
        ClassifyCommands::Show { proposal, target } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            match semantic::read(&project_path, &id)? {
                Some(proposal) => semantic_show_output(&proposal, format),
                None => Err(ForgeError::SemanticInvalid {
                    reason: format!("proposal `{proposal}` was not found under `.forge/semantic/`"),
                }),
            }
        }
        ClassifyCommands::Approve {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::approve(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
        ClassifyCommands::Reject {
            proposal,
            target,
            confirm,
        } => {
            let project_path = resolve_spec_target(target)?;
            let id = parse_proposal_id(target, &project_path, proposal)?;
            let outcome = semantic::reject(&project_path, &id, chrono::Utc::now(), *confirm)?;
            semantic_decide_output(&outcome, format)
        }
    }
}

fn build_suggest_request(
    kind: ProposalKind,
    args: &SemanticSuggestArgs,
) -> Result<SuggestRequest, ForgeError> {
    let project_path = resolve_spec_target(&args.target)?;
    if args.evidence_paths.is_empty() {
        return Err(ForgeError::SemanticInvalid {
            reason: "suggest requires at least one --evidence-path".to_string(),
        });
    }
    if args.evidence_revisions.len() != 1 {
        return Err(ForgeError::SemanticInvalid {
            reason: "--evidence-revision must be passed exactly once; the same revision applies to every --evidence-path".to_string(),
        });
    }
    let revision = &args.evidence_revisions[0];
    let mut evidence: Vec<ProposalEvidence> = Vec::with_capacity(args.evidence_paths.len());
    for (index, path) in args.evidence_paths.iter().enumerate() {
        let excerpt = args
            .evidence_excerpts
            .get(index)
            .cloned()
            .unwrap_or_default();
        evidence.push(ProposalEvidence::new(
            path.clone(),
            revision.clone(),
            excerpt,
        )?);
    }
    Ok(SuggestRequest {
        project_path,
        kind,
        current_value: args.current_value.clone(),
        suggested_value: args.suggested_value.clone(),
        confidence: forge::semantic::parse_confidence(&args.confidence)?,
        provider: forge::semantic::parse_provider(&args.provider)?,
        evidence,
        note: args.note.clone(),
        now: chrono::Utc::now(),
    })
}

fn parse_proposal_id(
    _target: &str,
    project_path: &std::path::Path,
    raw: &str,
) -> Result<forge::semantic::ProposalId, ForgeError> {
    let (kind_label, hash) = raw.rsplit_once('-').ok_or_else(|| ForgeError::SemanticInvalid {
        reason: format!(
            "proposal id `{raw}` is malformed: expected `<kind>-<hash>` (e.g. `description-deadbeef`)"
        ),
    })?;
    let kind = forge::semantic::parse_kind(kind_label)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(project_path, None)
        .map_err(|err| ForgeError::SemanticInvalid {
            reason: format!("cannot resolve project id: {err}"),
        })?;
    Ok(forge::semantic::ProposalId {
        project_id: manifest.project.id,
        kind,
        hash: hash.to_string(),
    })
}

fn semantic_suggest_output(
    outcome: &forge::semantic::SuggestOutcome,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "status": outcome.status_label(),
        "note": outcome.note,
        "files_written": outcome.files_written,
        "proposal": outcome.proposal,
    });
    let human = match outcome.proposal.as_ref() {
        Some(proposal) => format!(
            "semantic {}: {}\nfiles: {}\n{}",
            outcome.status_label(),
            proposal.id.dir_name(),
            if outcome.files_written.is_empty() {
                "(none)".to_string()
            } else {
                outcome.files_written.join(", ")
            },
            outcome.note
        ),
        None => format!(
            "semantic {}: {}\n{}",
            outcome.status_label(),
            outcome.note,
            "(no proposal stored)"
        ),
    };
    Ok(as_output(format, human, json))
}

fn classify_apply_output(
    outcome: &forge::semantic::ApplyOutcome,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "contract": forge::publish::providers::METADATA_PROPOSE_CONTRACT,
        "plugin": outcome.plugin_id,
        "fields": outcome.fields,
        "pr": outcome.pr_reference,
        "changes": outcome.changes,
        "note": outcome.note,
    });
    let human = format!(
        "classify apply: plugin={} fields={} pr={}\n{}",
        outcome.plugin_id,
        outcome.fields.join(", "),
        outcome.pr_reference.as_deref().unwrap_or("unknown"),
        outcome.note
    );
    Ok(as_output(format, human, json))
}

fn semantic_derive_output(
    outcomes: &[forge::semantic::SuggestOutcome],
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "contract": forge::semantic::SEMANTIC_CONTRACT_VERSION,
        "proposals": outcomes.iter().map(|outcome| serde_json::json!({
            "status": outcome.status_label(),
            "note": outcome.note,
            "files_written": outcome.files_written,
            "proposal": outcome.proposal,
        })).collect::<Vec<_>>(),
    });
    let mut human = String::new();
    for outcome in outcomes {
        match outcome.proposal.as_ref() {
            Some(proposal) => human.push_str(&format!(
                "semantic {}: {} ({})\n  files: {}\n  {}\n",
                outcome.status_label(),
                proposal.id.dir_name(),
                proposal.confidence.label(),
                if outcome.files_written.is_empty() {
                    "(none)".to_string()
                } else {
                    outcome.files_written.join(", ")
                },
                outcome.note
            )),
            None => human.push_str(&format!(
                "semantic {}: {}\n  {}\n",
                outcome.status_label(),
                outcome.note,
                "(no proposal stored)"
            )),
        }
    }
    Ok(as_output(format, human, json))
}

fn semantic_list_output(
    entries: &[forge::semantic::ProposalListEntry],
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"proposals": entries});
    let mut human = format!(
        "{:<48} {:<16} {:<11} {:<8} {:<8} {}",
        "Proposal", "Kind", "State", "Conf", "Provider", "Evidence"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<16} {:<11} {:<8} {:<8} {}",
            entry.dir_name,
            entry.kind.label(),
            entry.state.label(),
            entry.confidence.label(),
            entry.provider.label(),
            entry.evidence_count
        ));
    }
    Ok(as_output(format, human, json))
}

fn semantic_show_output(
    proposal: &forge::semantic::Proposal,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"proposal": proposal});
    let mut human = format!(
        "proposal: {}\nstate: {}\nkind: {}\nconfidence: {}\nprovider: {}\nsuggested_value: {}\n",
        proposal.id.dir_name(),
        proposal.state,
        proposal.kind,
        proposal.confidence,
        proposal.provider,
        proposal.suggested_value
    );
    for ev in &proposal.evidence {
        human.push_str(&format!(
            "evidence: path=`{}` revision=`{}`\n",
            ev.path, ev.revision
        ));
    }
    if let Some(conflict) = &proposal.conflict {
        human.push_str(&format!(
            "conflict: first=`{}` second=`{}`\n",
            conflict.first_value, conflict.second_value
        ));
    }
    Ok(as_output(format, human, json))
}

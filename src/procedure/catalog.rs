//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;

use super::constants::{
    BYPASS_MARKERS, MAX_PROCEDURE_STEPS, MAX_STEP_ARGS, PROCEDURE_CONTRACT_VERSION,
};
use super::model::{
    CoreOperation, ProcedureListEntry, ProcedureSpec, ProcedureStep, ProcedureValidationOutcome,
};

/// Return the eight named procedures in stable id order.
/// The order is part of the contract: an existing caller
/// that enumerates the catalog and indexes by ordinal must
/// see the same ordering on every Forge build that ships the
/// same [`PROCEDURE_CONTRACT_VERSION`].
pub fn procedure_catalog() -> Vec<ProcedureListEntry> {
    let mut entries: Vec<ProcedureListEntry> = all_procedures()
        .into_iter()
        .map(|spec| ProcedureListEntry {
            id: spec.id,
            version: spec.version,
            title: spec.title,
            description: spec.description,
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// Look up a single procedure by id. Returns
/// `procedure-invalid` when the id is empty, unknown or
/// carries a non-kebab-case shape.
pub fn inspect_procedure(id: &str) -> Result<ProcedureSpec, ForgeError> {
    if id.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: "procedure id must not be empty".to_string(),
        });
    }
    if !is_kebab_case(id) {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure id '{id}' is not kebab-case; the catalog uses lowercase letters, digits and single dashes"
            ),
        });
    }
    all_procedures()
        .into_iter()
        .find(|spec| spec.id == id)
        .ok_or_else(|| {
            let known: Vec<String> = all_procedures().into_iter().map(|s| s.id).collect();
            ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{id}' is not in the catalog; known procedures: {}",
                    known.join(", ")
                ),
            }
        })
}

/// Validate a procedure spec end to end. Returns the spec
/// unchanged on success; returns the typed Core rejection
/// (`procedure-invalid` / `procedure-unsupported-operation` /
/// `procedure-bypass-refused`) on any of the following:
///
/// - The spec is missing `id`, `version`, `title`,
///   `description` or `verification`.
/// - The id is empty, not kebab-case, or not in the catalog.
/// - The step count is zero or above
///   [`MAX_PROCEDURE_STEPS`].
/// - The steps are not in monotonic 1-based ordinal order.
/// - Any step's `args` carry a bypass marker (R2 failure
///   scenario).
/// - The procedure does not end with a single
///   [`CoreOperation::ReportFindings`] step (R2 boundary
///   scenario).
/// - A step's argument list is above [`MAX_STEP_ARGS`] or
///   contains a non-finite flag (a flag without `=` and
///   without a value).
pub fn validate_procedure(spec: &ProcedureSpec) -> ProcedureValidationOutcome {
    if spec.id.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: "procedure spec id must not be empty".to_string(),
        });
    }
    if spec.title.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has an empty title; the validator refuses an undocumented SOP",
                spec.id
            ),
        });
    }
    if spec.description.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has an empty description; the validator refuses an undocumented SOP",
                spec.id
            ),
        });
    }
    if spec.verification.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has an empty verification; the validator refuses an unauditable SOP",
                spec.id
            ),
        });
    }
    if spec.steps.is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has no steps; the validator refuses an empty workflow",
                spec.id
            ),
        });
    }
    if spec.steps.len() > MAX_PROCEDURE_STEPS {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has {} steps; the validator refuses more than {} steps",
                spec.id,
                spec.steps.len(),
                MAX_PROCEDURE_STEPS
            ),
        });
    }
    let mut last_ordinal: u32 = 0;
    let mut report_count: usize = 0;
    for (idx, step) in spec.steps.iter().enumerate() {
        if step.ordinal == 0 {
            return Err(ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{}' step {} has ordinal 0; ordinals must be 1-based and monotonic",
                    spec.id, idx
                ),
            });
        }
        if step.ordinal <= last_ordinal {
            return Err(ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{}' step {} has ordinal {} which is not strictly greater than {}; ordinals must be 1-based and monotonic",
                    spec.id, idx, step.ordinal, last_ordinal
                ),
            });
        }
        last_ordinal = step.ordinal;
        if step.description.trim().is_empty() {
            return Err(ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{}' step {} (op {}) has an empty description; the validator refuses an undocumented step",
                    spec.id, step.ordinal, step.op.label()
                ),
            });
        }
        if step.args.len() > MAX_STEP_ARGS {
            return Err(ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{}' step {} (op {}) carries {} args; the validator refuses more than {} args per step",
                    spec.id,
                    step.ordinal,
                    step.op.label(),
                    step.args.len(),
                    MAX_STEP_ARGS
                ),
            });
        }
        for raw in &step.args {
            if raw.is_empty() {
                return Err(ForgeError::ProcedureInvalid {
                    reason: format!(
                        "procedure '{}' step {} (op {}) has an empty argument; the validator refuses empty args",
                        spec.id,
                        step.ordinal,
                        step.op.label()
                    ),
                });
            }
            if let Some(marker) = contains_bypass_marker(raw) {
                return Err(ForgeError::ProcedureBypassRefused {
                    reason: format!(
                        "procedure '{}' step {} (op {}) carries the bypass marker '{marker}'; Core still validates every operation and the procedure layer refuses to forward the request",
                        spec.id, step.ordinal, step.op.label()
                    ),
                });
            }
        }
        if step.op.is_report() {
            report_count += 1;
        }
    }
    if report_count == 0 {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has no report_findings step; the validator refuses a workflow that does not surface its gaps (R2 boundary)",
                spec.id
            ),
        });
    }
    if report_count > 1 {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' has {} report_findings steps; the validator refuses a workflow with multiple end-of-flow reports",
                spec.id, report_count
            ),
        });
    }
    if !spec.steps.last().map(|s| s.op.is_report()).unwrap_or(false) {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{}' does not end with a report_findings step; the validator refuses a workflow that does not surface its gaps at the end (R2 boundary)",
                spec.id
            ),
        });
    }
    Ok(spec.clone())
}

/// Validate a single step. Useful for transports that load a
/// partial SOP or a test fixture that targets one operation;
/// the full-procedure validator composes this function.
pub fn validate_step(spec_id: &str, step: &ProcedureStep) -> Result<(), ForgeError> {
    if step.description.trim().is_empty() {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{spec_id}' step {} (op {}) has an empty description",
                step.ordinal,
                step.op.label()
            ),
        });
    }
    if step.args.len() > MAX_STEP_ARGS {
        return Err(ForgeError::ProcedureInvalid {
            reason: format!(
                "procedure '{spec_id}' step {} (op {}) carries {} args; the validator refuses more than {} args per step",
                step.ordinal,
                step.op.label(),
                step.args.len(),
                MAX_STEP_ARGS
            ),
        });
    }
    for raw in &step.args {
        if raw.is_empty() {
            return Err(ForgeError::ProcedureInvalid {
                reason: format!(
                    "procedure '{spec_id}' step {} (op {}) has an empty argument",
                    step.ordinal,
                    step.op.label()
                ),
            });
        }
        if let Some(marker) = contains_bypass_marker(raw) {
            return Err(ForgeError::ProcedureBypassRefused {
                reason: format!(
                    "procedure '{spec_id}' step {} (op {}) carries the bypass marker '{marker}'",
                    step.ordinal,
                    step.op.label()
                ),
            });
        }
    }
    Ok(())
}

/// Render the catalog as a stable human-readable table.
pub fn render_list_human(entries: &[ProcedureListEntry]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{:<28} {:<10} {}\n",
        "Procedure", "Version", "Description"
    ));
    for entry in entries {
        out.push_str(&format!(
            "{:<28} {:<10} {}\n",
            truncate(&entry.id, 28),
            truncate(&entry.version, 10),
            entry.description
        ));
    }
    out
}

/// Render a single procedure spec as a stable human-readable
/// block. The output is suitable for `forge procedure
/// inspect <id>` and for the MCP `inspect_procedure` tool.
pub fn render_inspect_human(spec: &ProcedureSpec) -> String {
    let mut out = String::new();
    out.push_str(&format!("procedure {} ({})\n", spec.id, spec.version));
    out.push_str(&format!("title: {}\n", spec.title));
    out.push_str(&format!("description: {}\n", spec.description));
    if !spec.prerequisites.is_empty() {
        out.push_str("prerequisites:\n");
        for prereq in &spec.prerequisites {
            out.push_str(&format!("  - {prereq}\n"));
        }
    }
    out.push_str("steps:\n");
    for step in &spec.steps {
        let args = if step.args.is_empty() {
            String::new()
        } else {
            format!(" {}", step.args.join(" "))
        };
        out.push_str(&format!(
            "  {:>2}. {} {} -- {}\n",
            step.ordinal,
            step.op.label(),
            args,
            step.description
        ));
    }
    out.push_str(&format!("verification: {}\n", spec.verification));
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut prev_dash = false;
    for c in chars {
        if c == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return false;
        }
    }
    !prev_dash
}

pub(super) fn contains_bypass_marker(raw: &str) -> Option<&'static str> {
    let lower = raw.to_ascii_lowercase();
    for marker in BYPASS_MARKERS {
        // Match the marker as a standalone token (either
        // exactly the marker or a `--key=value` form that
        // begins with the marker). Substring matches inside
        // an arbitrary value are still rejected because
        // Core must not be told to override itself.
        if lower == *marker || lower.starts_with(&format!("{marker}=")) {
            return Some(*marker);
        }
    }
    None
}

/// Return the eight named procedures in stable id order.
/// The list is intentionally hand-authored. The validator
/// refuses any deviation from the four invariants above.
pub fn all_procedures() -> Vec<ProcedureSpec> {
    vec![
    ProcedureSpec {
        id: "create-project".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Create a new project from a profile".to_string(),
        description: "Inspect available profiles, resolve required capabilities, prefer certified assets, generate the project deterministically, then run doctor, tests and configured quality checks and surface any unresolved gaps.".to_string(),
        prerequisites: vec![
            "A registered Forge installation with a working Core registry.".to_string(),
            "A destination directory that is empty or absent.".to_string(),
            "The target profile id and a list of required capabilities.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::ProfileInspect,
                args: vec!["<profile>".to_string()],
                description: "Inspect the chosen profile descriptor to confirm capabilities, packages, build/test commands and maturity target.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::ProfilePreflight,
                args: vec!["<profile>".to_string()],
                description: "Preflight the profile's required toolchain so a missing toolchain surfaces before any file is written.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::ComponentResolve,
                args: vec!["--profile".to_string(), "<profile>".to_string(), "--component".to_string(), "<capability>".to_string()],
                description: "Resolve compatible semantic components for the requested capabilities, preferring the certified candidate when multiple candidates satisfy the same id.".to_string(),
            },
            ProcedureStep {
                ordinal: 4,
                op: CoreOperation::UiPatternResolve,
                args: vec!["--profile".to_string(), "<profile>".to_string(), "--pattern".to_string(), "<pattern>".to_string()],
                description: "Resolve compatible UI patterns for the requested intents, refusing the resolve on profile-incompatibility rather than substituting copied web markup.".to_string(),
            },
            ProcedureStep {
                ordinal: 5,
                op: CoreOperation::FeatureAdd,
                args: vec!["<feature>".to_string(), "<project>".to_string()],
                description: "Add the resolved features in dependency order through the existing feature lifecycle, preserving manifest sections and the ownership receipt contract.".to_string(),
            },
            ProcedureStep {
                ordinal: 6,
                op: CoreOperation::DoctorRun,
                args: vec!["<project>".to_string(), "--target".to_string(), "L2".to_string()],
                description: "Run the doctor at the project's target maturity so the L0..L4 finding inventory reflects the freshly generated assets.".to_string(),
            },
            ProcedureStep {
                ordinal: 7,
                op: CoreOperation::TestRun,
                args: vec!["<project>".to_string()],
                description: "Run the profile's native test command to confirm the generated project compiles and the test suite passes.".to_string(),
            },
            ProcedureStep {
                ordinal: 8,
                op: CoreOperation::PolicyRun,
                args: vec!["<project>".to_string()],
                description: "Run the configured DriftWatch adapter so the policy findings are part of the workflow's evidence.".to_string(),
            },
            ProcedureStep {
                ordinal: 9,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface any unresolved gaps, deprecated components or remaining warnings so the operator can decide before declaring the project ready.".to_string(),
            },
        ],
        verification: "Project compiles via the profile's native toolchain; `forge doctor` reports no FAIL/UNAVAILABLE finding; `forge test` exits 0; DriftWatch findings are recorded; unresolved gaps are listed in the workflow report.".to_string(),
    },
    ProcedureSpec {
        id: "upgrade-project".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Upgrade a registered project".to_string(),
        description: "Resolve a pinned upgrade plan, apply it, then run doctor, tests and configured quality checks. When the upgrade surfaces a semantic conflict, the workflow requests a bounded spec through the existing remediation surface rather than rewriting mature assets.".to_string(),
        prerequisites: vec![
            "A registered project whose manifest has a profile and a feature map.".to_string(),
            "Optional --feature argument to upgrade a single feature; absent means upgrade all features in dependency order.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::FeatureResolve,
                args: vec!["<profile>".to_string(), "--feature".to_string(), "<feature>".to_string()],
                description: "Resolve the requested features into a deterministic install plan so the operator can audit versions and conflicts before any mutation.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::UpgradeApply,
                args: vec!["<project>".to_string(), "--feature".to_string(), "<feature>".to_string()],
                description: "Apply the upgrade; a semantic conflict surfaces as a typed outcome and the workflow proceeds to the spec handoff rather than overwriting the conflicting file.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::SpecGenerate,
                args: vec!["<project>".to_string(), "--finding".to_string(), "semantic-<feature>".to_string()],
                description: "When the upgrade step surfaces a semantic conflict, generate a bounded spec through the existing remediation surface so the agent or human author can resolve the gap (R2 success).".to_string(),
            },
            ProcedureStep {
                ordinal: 4,
                op: CoreOperation::DoctorRun,
                args: vec!["<project>".to_string(), "--target".to_string(), "L2".to_string()],
                description: "Run the doctor at the project's target maturity so the upgrade's impact on the finding inventory is visible.".to_string(),
            },
            ProcedureStep {
                ordinal: 5,
                op: CoreOperation::TestRun,
                args: vec!["<project>".to_string()],
                description: "Run the profile's native test command so the upgrade's impact on the test suite is visible.".to_string(),
            },
            ProcedureStep {
                ordinal: 6,
                op: CoreOperation::PolicyRun,
                args: vec!["<project>".to_string()],
                description: "Run the configured DriftWatch adapter so the policy findings are part of the upgrade's evidence.".to_string(),
            },
            ProcedureStep {
                ordinal: 7,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface any remaining warnings or unresolved specs so the operator can decide before declaring the upgrade complete.".to_string(),
            },
        ],
        verification: "`forge doctor` reports no FAIL/UNAVAILABLE finding introduced by the upgrade; the feature ownership receipts are preserved; the manifest sections other than features are unchanged; any semantic conflict has a `.forge/specs/<id>/proposal.md` to read.".to_string(),
    },
    ProcedureSpec {
        id: "prepare-release".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Prepare a release".to_string(),
        description: "Capture semver, changelog, source revision and the doctor/test/DriftWatch evidence into a reviewable plan. The plan refuses to apply when any applicable check is failing or stale; the workflow ends with a gap report so partial success is observable.".to_string(),
        prerequisites: vec![
            "A registered project with a CHANGELOG.md and a semver in the manifest's release block.".to_string(),
            "A working tree with a clean HEAD (or a clear recovery plan for dirty state).".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::ReleasePrepare,
                args: vec!["<project>".to_string(), "--version".to_string(), "<semver>".to_string()],
                description: "Capture the semver, changelog, source revision and the captured doctor/test/DriftWatch evidence into a reviewable plan; a failed check surfaces as a typed outcome and the plan reports `ready: false`.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::ReleaseApply,
                args: vec!["<project>".to_string(), "--version".to_string(), "<semver>".to_string(), "--confirm".to_string()],
                description: "Apply the release only when the plan is ready; the apply path requires explicit confirmation and walks each stage with stable per-stage outcomes.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface any per-stage failure (`failed`, `conflict`, `unavailable`) so the operator can decide whether to retry, skip the offending stage or abort the release.".to_string(),
            },
        ],
        verification: "The release state under `.forge/release/<id>/state.json` carries `delivered` for every selected stage; a re-run reports `skipped` for already-delivered stages; a tag conflict names `existing:` and `requested:` evidence and the recovery note.".to_string(),
    },
    ProcedureSpec {
        id: "fix-quality-findings".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Fix quality findings".to_string(),
        description: "Classify each doctor or DriftWatch finding into a deterministic, semantic or manual route. Deterministic findings run the action; semantic findings emit a bounded spec; manual findings are recorded without an AI claim.".to_string(),
        prerequisites: vec![
            "A registered project whose `forge doctor` finding inventory is the workflow's input.".to_string(),
            "The configured DriftWatch adapter is on PATH.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::DoctorRun,
                args: vec!["<project>".to_string()],
                description: "Capture the project's finding inventory so the route decisions are grounded in the current evidence.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::PolicyRun,
                args: vec!["<project>".to_string()],
                description: "Capture the DriftWatch findings so policy findings are routed alongside doctor findings.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::SpecApply,
                args: vec!["<project>".to_string(), "--finding".to_string(), "<finding>".to_string()],
                description: "Route each finding through `forge spec apply` so a deterministic action is recorded, a semantic gap produces a bounded spec and a manual finding is noted without an AI fix.".to_string(),
            },
            ProcedureStep {
                ordinal: 4,
                op: CoreOperation::TestRun,
                args: vec!["<project>".to_string()],
                description: "Run the profile's native test command so the deterministic fix is verified before the workflow ends.".to_string(),
            },
            ProcedureStep {
                ordinal: 5,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface every finding whose route was `manual` or whose spec handoff is still open so the operator can decide before declaring the workflow complete.".to_string(),
            },
        ],
        verification: "Every doctor / DriftWatch finding has a recorded route; deterministic actions are visible in the registry's `operations` table; semantic gaps have a `.forge/specs/<id>/proposal.md`; manual findings are listed in the report.".to_string(),
    },
    ProcedureSpec {
        id: "onboard-existing-project".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Onboard an existing project".to_string(),
        description: "Detect the language, framework, package manager and deployment surface of an existing project, propose a manifest and accept it explicitly. The workflow does not modify the project until `--accept` is passed.".to_string(),
        prerequisites: vec![
            "An existing project directory under a version-control system.".to_string(),
            "An explicit profile hint (or `forge import` resolves one from the detected evidence).".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::ProfileInspect,
                args: vec!["<profile>".to_string()],
                description: "Inspect the chosen profile descriptor so the import's compatibility check has a known target.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::ImportRun,
                args: vec!["<project>".to_string(), "--profile".to_string(), "<profile>".to_string()],
                description: "Run `forge import` in read-only mode to surface the detected evidence and the proposed manifest; the workflow refuses to write until the operator passes `--accept`.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::ImportRun,
                args: vec!["<project>".to_string(), "--profile".to_string(), "<profile>".to_string(), "--accept".to_string()],
                description: "On explicit operator approval, adopt the import and register the project in the Core registry; the legacy/typed manifest ambiguity is refused before any write.".to_string(),
            },
            ProcedureStep {
                ordinal: 4,
                op: CoreOperation::DoctorRun,
                args: vec!["<project>".to_string(), "--target".to_string(), "L1".to_string()],
                description: "Run the doctor at L1 (the typical onboarding target) so the imported project has a known starting maturity.".to_string(),
            },
            ProcedureStep {
                ordinal: 5,
                op: CoreOperation::TestRun,
                args: vec!["<project>".to_string()],
                description: "Run the profile's native test command so the import's compatibility with the chosen profile is verified end to end.".to_string(),
            },
            ProcedureStep {
                ordinal: 6,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface any FAIL/UNAVAILABLE finding and any profile-incompatibility signal so the operator can decide before declaring the onboarding complete.".to_string(),
            },
        ],
        verification: "`forge list` shows the imported project; `forge inspect` returns the registered record; `forge doctor` reports the imported project's maturity; the manifest's `distribution`/`release`/`deployment` blocks (if present) are preserved verbatim.".to_string(),
    },
    ProcedureSpec {
        id: "deploy-project".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Deploy a project".to_string(),
        description: "Plan a deploy against the named target, apply it with explicit confirmation, observe the runtime health and surface the result. The workflow refuses to apply without `--confirm`; a missing target adapter or a failed observation is reported, not masked.".to_string(),
        prerequisites: vec![
            "A registered project with a `deployment` block in the manifest.".to_string(),
            "The configured `FORGE_DEPLOYER_BIN` (or its per-target override) is on PATH.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::DeployPlan,
                args: vec!["<project>".to_string(), "--target-name".to_string(), "<target>".to_string()],
                description: "Capture the target, artifact, revision and configured health check into a reviewable plan; the plan is `ready: true` only when the chosen target has a usable artifact and the working tree carries a git HEAD.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::DeployApply,
                args: vec!["<project>".to_string(), "--target-name".to_string(), "<target>".to_string(), "--confirm".to_string()],
                description: "Apply the deploy only on explicit confirmation; the adapter is invoked with argument arrays and a bounded timeout so an unresponsive target cannot hang the registry.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::DeployObserve,
                args: vec!["<project>".to_string(), "--target-name".to_string(), "<target>".to_string()],
                description: "Re-run the configured health check on the deployed artifact; the last successful observation is preserved on a transient unreachable target.".to_string(),
            },
            ProcedureStep {
                ordinal: 4,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface any per-stage failure or `unknown` observation so the operator can decide before declaring the deploy successful.".to_string(),
            },
        ],
        verification: "`.forge/deploy/<id>/state.json` carries the per-stage outcomes plus the running observation; a credential-shaped substring in evidence is redacted by the policy adapter; the R2 boundary (disconnected is unknown, not offline proof) holds.".to_string(),
    },
    ProcedureSpec {
        id: "mirror-repository".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Mirror a repository to the configured remotes".to_string(),
        description: "Plan and apply a primary + mirror push against the project's distribution config. The workflow refuses to apply without `--confirm`; a divergent mirror history surfaces as a typed outcome and the recovery guidance names the recovery command.".to_string(),
        prerequisites: vec![
            "A registered project with a `distribution` block in the manifest.".to_string(),
            "A clean working tree or a clear recovery plan for dirty state.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::DoctorRun,
                args: vec!["<project>".to_string()],
                description: "Capture the doctor verdict so the mirror workflow inherits the project's known health; a FAIL finding surfaces in the gap report rather than blocking the push.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::MirrorApply,
                args: vec!["<project>".to_string(), "--ref".to_string(), "<ref>".to_string(), "--confirm".to_string()],
                description: "Plan and apply the mirror against the configured primary and mirror remotes; a missing remote, a divergent history or a credential failure surfaces as a typed outcome and the prior `MirrorState` stays intact.".to_string(),
            },
            ProcedureStep {
                ordinal: 3,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface every per-remote outcome (`delivered`, `skipped`, `disabled`, `diverged`, `unavailable`, `failed`) so the operator can decide whether to retry the failing remote.".to_string(),
            },
        ],
        verification: "`.forge/distribution/<id>/state.json` carries the per-remote outcomes plus the local HEAD SHA; a `--retry-failed` re-push only fires when the local HEAD SHA differs from the previously delivered SHA; a credential-shaped substring in evidence is redacted.".to_string(),
    },
    ProcedureSpec {
        id: "translate-docs".to_string(),
        version: PROCEDURE_CONTRACT_VERSION.to_string(),
        title: "Translate the canonical source document".to_string(),
        description: "Translate the canonical source into every enabled locale. The workflow refuses to translate a disabled locale and refuses a derivative path that resolves outside the project; the provider is invoked with argument arrays and a bounded timeout so an unresponsive tool cannot hang the registry.".to_string(),
        prerequisites: vec![
            "A registered project with a `docs` block in the manifest.".to_string(),
            "The configured `FORGE_DOCS_TRANSLATOR_BIN` (or its per-locale override) is on PATH.".to_string(),
        ],
        steps: vec![
            ProcedureStep {
                ordinal: 1,
                op: CoreOperation::DocsTranslate,
                args: vec!["<project>".to_string(), "--all".to_string()],
                description: "Translate every enabled locale through the existing `forge docs translate --all` workflow; unchanged segments are reused by content hash and a source edit during generation is reported as `failed` without overwriting the prior derivative.".to_string(),
            },
            ProcedureStep {
                ordinal: 2,
                op: CoreOperation::ReportFindings,
                args: vec![],
                description: "Surface every per-locale outcome plus the `needs-review` markers (dropped link, non-translatable term violation) so the operator can decide before declaring the translation complete.".to_string(),
            },
        ],
        verification: "`forge doctor` reports `[PASS] docs-<locale>` and `[PASS] docs-freshness` for every enabled locale; the prior derivative and state stay intact on a provider failure; a credential-shaped substring in evidence is redacted.".to_string(),
    },
    ]
}

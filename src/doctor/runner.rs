//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{Manifest, Maturity};
use crate::core::ForgeError;
use crate::policy::PolicyOutcome;
use std::path::Path;

use super::assessment::{assess_maturity, policy_findings};
use super::freshness::{docs_freshness_findings, manifest_mtime, observation_is_stale};
use super::model::DOCTOR_POLICY_VERSION;
use super::model::{
    DoctorReport, Finding, FindingStatus, GitState, MaturityControl, RegistryObservation,
    Remediation,
};
use super::probes::{
    declaration_vocabulary_finding, dependency_text, detect_ci, detect_deployment, detect_docs,
    detect_driftwatch, file_exists, gate_evidence_finding, has_build_definition, maturity_name,
    probe_git_remote, release_evidence_finding,
};

/// Run the read-only doctor inspection over a project directory.
///
/// Manifest IO failures (missing/invalid/ambiguous/legacy/unsupported) are
/// hard errors; everything else — including an unknown profile — is
/// reported as findings so the caller sees evidence instead of a refusal.
/// `policy_outcome` is the live DriftWatch result for this project; when
/// supplied, doctor normalizes its findings into the typed inventory and
/// flags the absence of evidence as `unavailable`.
pub fn run_doctor(
    dir: &Path,
    target_override: Option<Maturity>,
    observation: Option<&RegistryObservation>,
    policy_outcome: Option<&PolicyOutcome>,
) -> Result<DoctorReport, ForgeError> {
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: dir.display().to_string(),
        });
    }
    let (manifest, _) = Manifest::load_from_dir(dir, None)?;

    let profile_known = crate::profile::inspect_profile(&manifest.project.profile).is_ok();
    let descriptor = crate::profile::inspect_profile(&manifest.project.profile).ok();

    let requested: Vec<String> = manifest.features.keys().cloned().collect();
    let features_compatible = if profile_known {
        crate::profile::resolve_profile(&manifest.project.profile, &requested).is_ok()
    } else {
        false
    };

    let deps = dependency_text(dir);
    let current = manifest.project.maturity;
    let manifest_target = manifest.project.target_maturity;
    let target = target_override.or(manifest_target).unwrap_or(Maturity::L1);

    let mut findings: Vec<Finding> = Vec::new();

    findings.push(Finding::new(
        "manifest-valid",
        FindingStatus::Pass,
        vec!["forge.yaml parses against schema 1".to_string()],
        true,
        Remediation::Manual,
        "manifest is present and schema-valid",
    ));

    if profile_known {
        findings.push(Finding::new(
            "profile-known",
            FindingStatus::Pass,
            vec![format!(
                "profile '{}' matches a versioned MVP descriptor",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "profile is known",
        ));
    } else {
        findings.push(Finding::new(
            "profile-known",
            FindingStatus::Fail,
            vec![format!(
                "profile '{}' matches no MVP profile descriptor",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "unknown profile: descriptor-gated checks are unavailable",
        ));
    }

    if !profile_known {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Unavailable,
            vec!["profile descriptor is unknown; compatibility cannot be evaluated".to_string()],
            true,
            Remediation::Manual,
            "required inspector (profile descriptor) cannot run",
        ));
    } else if features_compatible {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Pass,
            vec![if requested.is_empty() {
                "no capabilities requested".to_string()
            } else {
                format!("capabilities compatible: {}", requested.join(", "))
            }],
            true,
            Remediation::Manual,
            "requested capabilities are compatible with the profile",
        ));
    } else {
        findings.push(Finding::new(
            "features-compatible",
            FindingStatus::Fail,
            vec![format!(
                "capabilities incompatible with profile '{}'",
                manifest.project.profile
            )],
            true,
            Remediation::Manual,
            "requested capabilities are incompatible with the profile",
        ));
    }

    // Dependency drift: manifest runtime language versus on-disk evidence.
    if !profile_known {
        findings.push(Finding::new(
            "dependency-drift",
            FindingStatus::Unavailable,
            vec!["profile descriptor is unknown; drift cannot be evaluated".to_string()],
            true,
            Remediation::Manual,
            "required inspector (profile descriptor) cannot run",
        ));
    } else {
        let profile = manifest.project.profile.as_str();
        let expected_lang = descriptor
            .as_ref()
            .map(|d| d.language.as_str())
            .unwrap_or("");
        let manifest_lang = manifest
            .runtime
            .as_ref()
            .and_then(|r| r.language.clone())
            .unwrap_or_default();
        let mut drift: Vec<String> = Vec::new();
        if !manifest_lang.is_empty() && manifest_lang != expected_lang {
            drift.push(format!(
                "manifest runtime language '{manifest_lang}' disagrees with profile language '{expected_lang}'"
            ));
        }
        let (build_present, build_evidence) = has_build_definition(dir, profile);
        if !build_present {
            drift.push(build_evidence.join("; "));
        }
        if drift.is_empty() {
            findings.push(Finding::new(
                "dependency-drift",
                FindingStatus::Pass,
                vec!["manifest runtime agrees with on-disk dependency evidence".to_string()],
                true,
                Remediation::Automatic,
                "no dependency drift detected",
            ));
        } else {
            findings.push(Finding::new(
                "dependency-drift",
                FindingStatus::Warn,
                drift,
                true,
                Remediation::Automatic,
                "dependency evidence disagrees with the manifest",
            ));
        }
    }

    // Build / deployment configuration.
    if profile_known {
        let (present, evidence) = has_build_definition(dir, &manifest.project.profile);
        findings.push(Finding::new(
            "build-config",
            if present {
                FindingStatus::Pass
            } else {
                FindingStatus::Fail
            },
            evidence,
            true,
            Remediation::Automatic,
            if present {
                "build definition is present"
            } else {
                "build definition required by L1 is missing"
            },
        ));
    } else {
        findings.push(Finding::new(
            "build-config",
            FindingStatus::Unavailable,
            vec![
                "profile descriptor is unknown; build expectations cannot be evaluated".to_string(),
            ],
            true,
            Remediation::Automatic,
            "required inspector (profile descriptor) cannot run",
        ));
    }

    let (deploy_present, deploy_evidence) = detect_deployment(dir);
    findings.push(Finding::new(
        "deployment-config",
        if deploy_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        deploy_evidence,
        true,
        Remediation::Manual,
        if deploy_present {
            "deployment configuration is present"
        } else {
            "no deployment configuration; required for L2 deployment"
        },
    ));

    // Repository: detected / missing / unknown (unknown is never PASS).
    match probe_git_remote(dir) {
        GitState::Detected(url) => findings.push(Finding::new(
            "repository",
            FindingStatus::Pass,
            vec![format!("git remote origin: {url}")],
            true,
            Remediation::Manual,
            "git remote is configured",
        )),
        GitState::Missing => findings.push(Finding::new(
            "repository",
            FindingStatus::Warn,
            vec!["git repository has no origin remote".to_string()],
            true,
            Remediation::Manual,
            "repository exists but no origin remote is configured",
        )),
        GitState::Unknown => findings.push(Finding::new(
            "repository",
            FindingStatus::Unavailable,
            vec!["not a git repository; remote cannot be determined".to_string()],
            true,
            Remediation::Manual,
            "required inspector (git repository) cannot run",
        )),
    }

    let (ci_present, ci_evidence) = detect_ci(dir);
    findings.push(Finding::new(
        "ci-config",
        if ci_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        ci_evidence,
        true,
        Remediation::Automatic,
        if ci_present {
            "ci configuration is present"
        } else {
            "no ci configuration; required for L2 ci"
        },
    ));

    let (docs_present, docs_evidence) = detect_docs(dir, &manifest);
    findings.push(Finding::new(
        "docs-present",
        if docs_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        docs_evidence,
        true,
        Remediation::Ai,
        if docs_present {
            "documentation evidence is present"
        } else {
            "no readme or manifest docs section"
        },
    ));

    // Derivative documentation freshness. Read-only: the
    // recorded source hash per enabled locale is compared
    // against the current source so stale, missing or
    // needs-review derivatives surface with the
    // `forge docs translate` recovery. Disabled locales are
    // skipped entirely (automatic-workflow boundary).
    findings.extend(docs_freshness_findings(dir, &manifest));

    // Workspace Governance declaration: presence is informational
    // evidence only. It never gates health, maturity or the checker
    // plane (adoption is the sibling's own workflow decision).
    if file_exists(dir, crate::generate::workspace::METADATA_PATH) {
        findings.push(Finding::new(
            "workspace-metadata",
            FindingStatus::Pass,
            vec![crate::generate::workspace::METADATA_PATH.to_string()],
            false,
            Remediation::Manual,
            "workspace governance declaration is present (informational; not Forge maturity evidence)",
        ));
    }

    // Declaration vocabulary: declared values validated against the
    // consumed governance vocabulary. Projects without a declaration get
    // no finding at all, so the checker plane stays byte-identical for
    // them; canonical declarations pass as not-applicable.
    if let Some(finding) = declaration_vocabulary_finding(dir) {
        findings.push(finding);
    }

    let (dw_present, dw_evidence) = detect_driftwatch(dir);
    findings.push(Finding::new(
        "driftwatch-config",
        if dw_present {
            FindingStatus::Pass
        } else {
            FindingStatus::Warn
        },
        dw_evidence,
        true,
        Remediation::Automatic,
        if dw_present {
            "driftwatch configuration is present"
        } else {
            "no driftwatch configuration; required for L2 driftwatch (execution belongs to v0.3)"
        },
    ));

    // DriftWatch policy findings. The execution path is delegated to
    // the configured adapter (see `policy::run_driftwatch`); doctor maps
    // each finding to the typed inventory and reports a `pass`/`warn`/
    // `fail`/`unavailable` finding while retaining the original rule ID,
    // category, severity, tool version and redacted evidence.
    findings.extend(policy_findings(dir, policy_outcome));

    // Gate runtime evidence (`gate-runtime-evidence`). Read-only: the
    // persisted revision-bound record renders its verdict through the
    // shared vocabulary; a moved revision reads as stale and a declared
    // but never-run gate reads as unverified — never as a pass. Projects
    // with neither a declaration nor recorded evidence keep the finding
    // not-applicable so no surface renders silence as health.
    findings.push(gate_evidence_finding(dir));

    // Release-evidence consumption: reads the consumed export record and
    // reports the per-field state distribution. Fails the specific
    // inconsistency (verified declaration with no release_evidence block)
    // that the structural audit cannot see; warns on stale or partial
    // records; not-applicable when neither declaration nor export exists.
    if let Some(finding) = release_evidence_finding(dir) {
        findings.push(finding);
    }

    // Registry observation: registered + fresh / stale / not registered.
    let mtime = manifest_mtime(dir);
    let mut stale = false;
    match observation {
        Some(obs) if obs.registered => {
            let is_stale = obs
                .observed_at
                .as_deref()
                .map(|at| observation_is_stale(at, mtime))
                .unwrap_or(false);
            stale = is_stale;
            if is_stale {
                findings.push(Finding::new(
                    "registry-observation",
                    FindingStatus::Warn,
                    vec![format!(
                        "registry observation {} predates the current forge.yaml; shown as stale",
                        obs.observed_at.as_deref().unwrap_or("unknown")
                    )],
                    true,
                    Remediation::Automatic,
                    "stale observation: re-run forge register to refresh",
                ));
            } else {
                findings.push(Finding::new(
                    "registry-observation",
                    FindingStatus::Pass,
                    vec!["registry observation is current".to_string()],
                    true,
                    Remediation::Automatic,
                    "project is registered and the observation is fresh",
                ));
            }
        }
        _ => {
            findings.push(Finding::new(
                "registry-observation",
                FindingStatus::Warn,
                vec!["project is not registered; maturity evidence is local-only".to_string()],
                true,
                Remediation::Automatic,
                "project is not registered",
            ));
        }
    }

    let controls = assess_maturity(
        dir,
        &manifest,
        descriptor.as_ref(),
        &deps,
        target,
        observation,
    );

    let unmet = controls.iter().filter(|c| c.applicable && !c.met).count();
    if unmet == 0 {
        findings.push(Finding::new(
            "maturity-requirements",
            FindingStatus::Pass,
            vec![format!(
                "all applicable controls for target {} are met",
                maturity_name(target)
            )],
            true,
            Remediation::Manual,
            "maturity requirements satisfied for the target level",
        ));
    } else {
        findings.push(Finding::new(
            "maturity-requirements",
            FindingStatus::Fail,
            vec![format!(
                "{unmet} applicable control(s) for target {} lack evidence",
                maturity_name(target)
            )],
            true,
            Remediation::Manual,
            "maturity requirements are not satisfied for the target level",
        ));
    }

    let healthy = findings
        .iter()
        .all(|f| matches!(f.status, FindingStatus::Pass | FindingStatus::Warn))
        && unmet == 0
        && !stale;

    Ok(DoctorReport {
        path: dir.display().to_string(),
        profile: Some(manifest.project.profile.clone()),
        policy_version: DOCTOR_POLICY_VERSION.to_string(),
        current_maturity: current.map(|m| m.to_string()),
        target_maturity: Some(maturity_name(target).to_string()),
        findings,
        controls,
        healthy,
        stale,
    })
}

pub(super) fn control(
    id: &str,
    level: Maturity,
    description: &str,
    applicable: bool,
    met: bool,
    evidence: Vec<String>,
) -> MaturityControl {
    MaturityControl {
        id: id.to_string(),
        level: maturity_name(level).to_string(),
        description: description.to_string(),
        applicable,
        met: met && applicable,
        evidence,
    }
}

//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::profile::{inspect_profile, mvp_profiles};
use crate::registry::Registry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::contract::{RECEIPT_DIR, TESTED_VERSION};
use super::model::{FeatureDescriptor, FeaturePlan, LifecycleOutcome, PlanStep, Rollback};

#[allow(clippy::too_many_arguments)]
pub(super) fn descriptor(
    id: &str,
    depends: &[&str],
    conflicts: &[&str],
    install_strategy: &str,
    validation: &[&str],
    documentation: &str,
) -> FeatureDescriptor {
    let compatible_profiles: Vec<String> = mvp_profiles()
        .iter()
        .filter(|p| p.capabilities.iter().any(|c| c == id))
        .map(|p| p.id.clone())
        .collect();
    FeatureDescriptor {
        id: id.to_string(),
        version: TESTED_VERSION.to_string(),
        compatible_profiles,
        depends: depends.iter().map(|s| s.to_string()).collect(),
        conflicts: conflicts.iter().map(|s| s.to_string()).collect(),
        install_strategy: install_strategy.to_string(),
        upgrade_strategy: "manifest-repin".to_string(),
        validation: validation.iter().map(|s| s.to_string()).collect(),
        documentation: documentation.to_string(),
        tests: "forge feature resolve/add/remove/upgrade contract".to_string(),
    }
}

/// All catalog features in stable ID order. Compatibility is derived from
/// the MVP profile descriptors, so only tested mappings are installable.
pub fn feature_catalog() -> Vec<FeatureDescriptor> {
    vec![
        descriptor(
            "admin",
            &["auth"],
            &[],
            "package-plus-generator",
            &["AUTH-001"],
            "Administration panel; requires auth (requirement.md section 10).",
        ),
        descriptor(
            "analytics",
            &[],
            &[],
            "generator",
            &[],
            "Product analytics capability (requirement.md section 10).",
        ),
        descriptor(
            "audit",
            &["auth"],
            &[],
            "package-plus-generator",
            &["AUTH-001"],
            "Audit logging; requires auth (requirement.md section 10).",
        ),
        descriptor(
            "auth",
            &[],
            &[],
            "package-plus-generator",
            &["AUTH-001"],
            "Authentication capability (requirement.md section 10).",
        ),
        descriptor(
            "background-jobs",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Background job processing (requirement.md section 10).",
        ),
        descriptor(
            "billing",
            &["auth"],
            &[],
            "package-plus-generator",
            &[],
            "Billing capability; requires auth (requirement.md section 10).",
        ),
        descriptor(
            "content",
            &[],
            &[],
            "generator",
            &["A11Y-001"],
            "Multilingual content capability (requirement.md section 10).",
        ),
        descriptor(
            "email",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Email delivery capability (requirement.md section 10).",
        ),
        descriptor(
            "health-check",
            &[],
            &[],
            "generator",
            &["DEPLOY-001"],
            "Health check endpoint (requirement.md section 10).",
        ),
        descriptor(
            "i18n",
            &[],
            &[],
            "generator",
            &["A11Y-001"],
            "Internationalization capability (requirement.md section 10).",
        ),
        descriptor(
            "notifications",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Notifications capability (requirement.md section 10).",
        ),
        descriptor(
            "postgres",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "PostgreSQL persistence (requirement.md section 10).",
        ),
        descriptor(
            "privacy",
            &[],
            &[],
            "generator",
            &["PRIVACY-003"],
            "Privacy compliance capability (requirement.md section 10).",
        ),
        descriptor(
            "rate-limit",
            &[],
            &[],
            "generator",
            &[],
            "Rate limiting capability (requirement.md section 10).",
        ),
        descriptor(
            "redis",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Redis caching capability (requirement.md section 10).",
        ),
        descriptor(
            "search",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Search capability (requirement.md section 10).",
        ),
        descriptor(
            "storage",
            &[],
            &[],
            "package-plus-generator",
            &[],
            "Object storage capability (requirement.md section 10).",
        ),
        descriptor(
            "telemetry",
            &[],
            &[],
            "generator",
            &["DEPLOY-001"],
            "Telemetry/observability capability (requirement.md section 10).",
        ),
    ]
}

/// Look up one descriptor by ID.
pub fn inspect_feature(id: &str) -> Result<FeatureDescriptor, ForgeError> {
    feature_catalog()
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| ForgeError::UnknownFeature { id: id.to_string() })
}

pub(super) fn lookup<'a>(
    catalog: &'a [FeatureDescriptor],
    id: &str,
) -> Result<&'a FeatureDescriptor, ForgeError> {
    catalog
        .iter()
        .find(|f| f.id == id)
        .ok_or_else(|| ForgeError::UnknownFeature { id: id.to_string() })
}

/// Resolve `requested` for `profile` into a deterministic plan over
/// `catalog`: exact versions in dependency order. Fails before any edit on
/// unknown features, unsupported mappings, conflicts or cycles, naming the
/// blocking graph edges.
pub fn resolve_plan_with_catalog(
    profile: &str,
    requested: &[String],
    catalog: &[FeatureDescriptor],
) -> Result<FeaturePlan, ForgeError> {
    inspect_profile(profile)?;
    let mut ordered: Vec<String> = Vec::new();
    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();
    let mut sorted_requested = requested.to_vec();
    sorted_requested.sort();
    sorted_requested.dedup();
    for id in &sorted_requested {
        visit(profile, id, catalog, &mut ordered, &mut placed, &mut stack)?;
    }
    // Conflict edges are checked over the full closure so that dependency-
    // pulled features conflict exactly like directly requested ones.
    for id in &ordered {
        let descriptor = lookup(catalog, id)?;
        let mut conflicts: Vec<String> = descriptor.conflicts.clone();
        conflicts.sort();
        for other in conflicts {
            if placed.contains(&other) {
                return Err(ForgeError::IncompatibleFeature {
                    reason: format!(
                        "feature '{id}' conflicts with '{other}' for profile '{profile}'; \
                         resolution fails before edits and neither file was changed"
                    ),
                });
            }
        }
    }
    let mut validators: Vec<String> = Vec::new();
    let mut steps: Vec<PlanStep> = Vec::new();
    for id in &ordered {
        let descriptor = lookup(catalog, id)?;
        for policy in &descriptor.validation {
            if !validators.contains(policy) {
                validators.push(policy.clone());
            }
        }
        steps.push(PlanStep {
            feature: id.clone(),
            version: descriptor.version.clone(),
            action: "install".to_string(),
        });
    }
    Ok(FeaturePlan {
        profile: profile.to_string(),
        requested: sorted_requested,
        steps,
        validators,
    })
}

fn visit(
    profile: &str,
    id: &str,
    catalog: &[FeatureDescriptor],
    ordered: &mut Vec<String>,
    placed: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
) -> Result<(), ForgeError> {
    if placed.contains(id) {
        return Ok(());
    }
    if let Some(pos) = stack.iter().position(|s| s == id) {
        let mut cycle = stack[pos..].to_vec();
        cycle.push(id.to_string());
        return Err(ForgeError::IncompatibleFeature {
            reason: format!(
                "feature dependency cycle: {}; resolution fails before edits \
                 and neither file was changed",
                cycle.join(" -> ")
            ),
        });
    }
    let descriptor = lookup(catalog, id)?;
    if !descriptor.compatible_profiles.iter().any(|p| p == profile) {
        return Err(ForgeError::IncompatibleFeature {
            reason: format!(
                "feature '{id}' has no implementation for profile '{profile}' \
                 (tested profiles: {}); discovery reports unsupported rather \
                 than inventing an implementation and no files were changed",
                if descriptor.compatible_profiles.is_empty() {
                    "none".to_string()
                } else {
                    descriptor.compatible_profiles.join(", ")
                }
            ),
        });
    }
    stack.push(id.to_string());
    let mut deps = descriptor.depends.clone();
    deps.sort();
    for dep in &deps {
        if lookup(catalog, dep).is_err() {
            return Err(ForgeError::IncompatibleFeature {
                reason: format!(
                    "feature '{id}' depends on unknown feature '{dep}'; \
                     resolution fails before edits and neither file was changed"
                ),
            });
        }
        visit(profile, dep, catalog, ordered, placed, stack)?;
    }
    stack.pop();
    placed.insert(id.to_string());
    ordered.push(id.to_string());
    Ok(())
}

/// Resolve against the compiled-in catalog.
pub fn resolve_plan(profile: &str, requested: &[String]) -> Result<FeaturePlan, ForgeError> {
    resolve_plan_with_catalog(profile, requested, &feature_catalog())
}

/// Dependency closure in install order (dependency-first, deterministic).
/// Used by new-project feature selection so `forge new --feature admin`
/// records its `auth` dependency instead of shipping a broken manifest.
pub fn resolve_feature_closure(
    profile: &str,
    requested: &[String],
) -> Result<Vec<String>, ForgeError> {
    Ok(resolve_plan(profile, requested)?
        .steps
        .into_iter()
        .map(|s| s.feature)
        .collect())
}

/// Expected bytes of the ownership receipt for an installed feature.
/// A pure function of identity: any drift means user-owned edits.
pub fn expected_receipt(descriptor: &FeatureDescriptor, version: &str) -> String {
    let validators = if descriptor.validation.is_empty() {
        "none".to_string()
    } else {
        descriptor.validation.join(",")
    };
    format!(
        "# Forge feature ownership record (Forge-managed; manual edits block feature remove/upgrade).\n\
         id: {}\n\
         version: {}\n\
         strategy: {}\n\
         validators: {}\n",
        descriptor.id, version, descriptor.install_strategy, validators
    )
}

pub(super) fn receipt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(RECEIPT_DIR).join(format!("{id}.receipt"))
}

/// Resolve a lifecycle target (registered id or filesystem path) to a
/// project directory without changing anything.
fn resolve_target_dir(registry: &Registry, target: &str) -> Result<PathBuf, ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        return candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            });
    }
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    Ok(dir)
}

/// Guard that every installed feature touched by the operation still has a
/// pristine receipt. A drifted receipt means user-owned edits are present
/// and the operation blocks with the files preserved.
fn check_receipts_clean(dir: &Path, manifest: &Manifest, ids: &[String]) -> Result<(), ForgeError> {
    let catalog = feature_catalog();
    for id in ids {
        let installed = match manifest.features.get(id) {
            Some(v) => v.clone(),
            None => continue,
        };
        let descriptor = lookup(&catalog, id)?;
        let path = receipt_path(dir, id);
        let actual = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue, // No receipt (e.g. generated-in feature): manifest-only change is safe.
        };
        if actual != expected_receipt(descriptor, &installed) {
            return Err(ForgeError::FeatureOwnershipConflict {
                reason: format!(
                    "feature '{id}' has user-modified owned file '{}'; \
                     resolve the edits manually (or restore the receipt) and re-run; \
                     no files were changed",
                    path.display()
                ),
            });
        }
    }
    Ok(())
}

/// Edit the `features:` map of manifest bytes, preserving every other
/// section. `None` values remove entries; an emptied map drops the key.
pub(super) fn edit_manifest_features(
    original: &[u8],
    manifest_path: &Path,
    updates: &BTreeMap<String, Option<String>>,
) -> Result<Vec<u8>, ForgeError> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_slice(original).map_err(|err| ForgeError::ManifestInvalid {
            path: manifest_path.display().to_string(),
            reason: err.to_string(),
        })?;
    let mapping = value
        .as_mapping_mut()
        .ok_or_else(|| ForgeError::ManifestInvalid {
            path: manifest_path.display().to_string(),
            reason: "manifest root must be a mapping".to_string(),
        })?;
    let key = serde_yaml::Value::String("features".to_string());
    if !updates.values().any(|v| v.is_some()) && !mapping.contains_key(&key) {
        return Ok(original.to_vec()); // Pure removal with no features section: nothing to do.
    }
    let features = mapping
        .entry(key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let features_map = features
        .as_mapping_mut()
        .ok_or_else(|| ForgeError::ManifestInvalid {
            path: manifest_path.display().to_string(),
            reason: "manifest 'features' must be a mapping".to_string(),
        })?;
    for (id, version) in updates {
        match version {
            Some(v) => {
                features_map.insert(
                    serde_yaml::Value::String(id.clone()),
                    serde_yaml::Value::String(v.clone()),
                );
            }
            None => {
                features_map.remove(serde_yaml::Value::String(id.clone()));
            }
        }
    }
    if features_map.is_empty() {
        mapping.remove(serde_yaml::Value::String("features".to_string()));
    }
    serde_yaml::to_string(&value)
        .map_err(|err| ForgeError::ManifestInvalid {
            path: manifest_path.display().to_string(),
            reason: format!("cannot serialize manifest: {err}"),
        })
        .map(|s| s.into_bytes())
}

/// Write bytes atomically (temp file plus rename) inside `dir`.
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, ForgeError> {
    let target = dir.join(name);
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    fs::write(&tmp, bytes).map_err(|err| ForgeError::IncompatibleFeature {
        reason: format!(
            "cannot write '{}': {err}; prior state left unchanged",
            target.display()
        ),
    })?;
    fs::rename(&tmp, &target).map_err(|err| ForgeError::IncompatibleFeature {
        reason: format!(
            "cannot replace '{}': {err}; prior state left unchanged",
            target.display()
        ),
    })?;
    Ok(target)
}

/// Apply manifest updates plus receipt writes, then post-change validate
/// (re-parse plus full graph re-resolution). A validation failure restores
/// the prior manifest bytes and receipts and reports the failure.
fn apply_and_validate(
    dir: &Path,
    manifest_path: &Path,
    manifest_before: &[u8],
    updates: &BTreeMap<String, Option<String>>,
    profile: &str,
) -> Result<Vec<String>, ForgeError> {
    let next = edit_manifest_features(manifest_before, manifest_path, updates)?;
    let manifest_name = manifest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("forge.yaml");
    debug_assert_eq!(manifest_name, "forge.yaml");
    write_atomic(dir, "forge.yaml", &next)?;

    let catalog = feature_catalog();
    let mut receipts_written: Vec<PathBuf> = Vec::new();
    let mut receipts_deleted: Vec<(PathBuf, String)> = Vec::new();
    // Receipts follow the post-change manifest: installed features get a
    // fresh deterministic receipt, removed features lose theirs.
    for (id, version) in updates {
        let path = receipt_path(dir, id);
        match version {
            Some(v) => {
                let descriptor = lookup(&catalog, id)?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|err| ForgeError::IncompatibleFeature {
                        reason: format!(
                            "cannot create receipt directory: {err}; prior state left unchanged"
                        ),
                    })?;
                }
                let bytes = expected_receipt(descriptor, v);
                if let Err(err) = fs::write(&path, &bytes) {
                    let rollback = Rollback {
                        manifest_path: manifest_path.to_path_buf(),
                        manifest_before: manifest_before.to_vec(),
                        receipts_written,
                        receipts_deleted,
                    };
                    rollback.restore();
                    return Err(ForgeError::IncompatibleFeature {
                        reason: format!(
                            "cannot write receipt '{}': {err}; prior state restored",
                            path.display()
                        ),
                    });
                }
                receipts_written.push(path);
            }
            None => {
                if let Ok(contents) = fs::read_to_string(&path) {
                    let _ = fs::remove_file(&path);
                    receipts_deleted.push((path, contents));
                }
            }
        }
    }

    let rollback = Rollback {
        manifest_path: manifest_path.to_path_buf(),
        manifest_before: manifest_before.to_vec(),
        receipts_written,
        receipts_deleted,
    };
    let validation: Result<(), ForgeError> = (|| {
        let bytes = fs::read(manifest_path).map_err(|_| ForgeError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        })?;
        let manifest = Manifest::parse(manifest_path, &bytes)?;
        let installed: Vec<String> = manifest.features.keys().cloned().collect();
        resolve_plan(&manifest.project.profile, &installed)?;
        Ok(())
    })();
    if let Err(err) = validation {
        rollback.restore();
        return Err(err);
    }
    drop(rollback);

    let mut files = vec!["forge.yaml".to_string()];
    for id in updates.keys() {
        files.push(format!("{RECEIPT_DIR}/{id}.receipt"));
    }
    files.sort();
    files.dedup();
    let _ = profile;
    Ok(files)
}

fn finish(
    registry: &mut Registry,
    dir: &Path,
    operation: &str,
    plan: FeaturePlan,
    changed: bool,
    note: String,
    files_changed: Vec<String>,
) -> Result<LifecycleOutcome, ForgeError> {
    let manifest_name = "forge.yaml";
    let manifest_path = dir.join(manifest_name);
    let manifest_bytes = fs::read(&manifest_path).map_err(|_| ForgeError::ManifestNotFound {
        path: manifest_path.display().to_string(),
    })?;
    let manifest = Manifest::parse(&manifest_path, &manifest_bytes)?;
    if changed {
        // Refresh registry observations so source, manifest and registry
        // agree. A refresh failure after a validated write is reported as
        // partial execution with an explicit next action.
        registry
            .register(dir, None)
            .map_err(|err| ForgeError::Registry {
                reason: format!(
                    "manifest updated but registry refresh failed: {err}; re-run `forge register`"
                ),
            })?;
    }
    Ok(LifecycleOutcome {
        project_id: manifest.project.id.clone(),
        profile: manifest.project.profile.clone(),
        operation: operation.to_string(),
        plan,
        changed,
        note,
        files_changed,
        features: manifest.features.clone(),
    })
}

/// Add `feature` (plus missing dependencies) to the project at `target`.
/// Exact-version reinstalls are a no-op with no duplicate registration.
pub fn add_feature(
    registry: &mut Registry,
    target: &str,
    feature: &str,
    version: Option<&str>,
) -> Result<LifecycleOutcome, ForgeError> {
    let catalog = feature_catalog();
    let descriptor = lookup(&catalog, feature)?.clone();
    let requested_version = version.unwrap_or(&descriptor.version).to_string();
    if requested_version != descriptor.version {
        return Err(ForgeError::IncompatibleFeature {
            reason: format!(
                "unknown version '{requested_version}' for feature '{feature}': \
                 only '{}' is available; manifest left unchanged",
                descriptor.version
            ),
        });
    }
    let dir = resolve_target_dir(registry, target)?;
    let (manifest, manifest_path) = Manifest::load_from_dir(&dir, None)?;
    let plan =
        resolve_plan_with_catalog(&manifest.project.profile, &[feature.to_string()], &catalog)?;
    // Receipt preflight before any mutation: installed features with
    // drifted receipts block to preserve user-owned edits.
    let touched: Vec<String> = plan.steps.iter().map(|s| s.feature.clone()).collect();
    check_receipts_clean(&dir, &manifest, &touched)?;

    let mut updates: BTreeMap<String, Option<String>> = BTreeMap::new();
    for step in &plan.steps {
        let installed = manifest.features.get(&step.feature);
        if installed.is_some_and(|v| v == &step.version) {
            continue;
        }
        updates.insert(step.feature.clone(), Some(step.version.clone()));
    }
    if updates.is_empty() {
        return finish(
            registry,
            &dir,
            "add",
            plan,
            false,
            format!(
                "feature '{feature}' version '{requested_version}' already installed; \
                 no changes made and nothing was re-registered"
            ),
            Vec::new(),
        );
    }
    let manifest_before = fs::read(&manifest_path).map_err(|_| ForgeError::ManifestNotFound {
        path: manifest_path.display().to_string(),
    })?;
    let files_changed = apply_and_validate(
        &dir,
        &manifest_path,
        &manifest_before,
        &updates,
        &manifest.project.profile,
    )?;
    finish(
        registry,
        &dir,
        "add",
        plan,
        true,
        format!("added feature '{feature}' version '{requested_version}'"),
        files_changed,
    )
}

/// Remove `feature` from the project at `target`. Blocks when a remaining
/// installed feature depends on it or when its receipt has user-owned
/// edits; blocked operations preserve every file.
pub fn remove_feature(
    registry: &mut Registry,
    target: &str,
    feature: &str,
) -> Result<LifecycleOutcome, ForgeError> {
    let catalog = feature_catalog();
    lookup(&catalog, feature)?;
    let dir = resolve_target_dir(registry, target)?;
    let (manifest, manifest_path) = Manifest::load_from_dir(&dir, None)?;
    if !manifest.features.contains_key(feature) {
        let installed: Vec<String> = manifest.features.keys().cloned().collect();
        let plan = resolve_plan_with_catalog(&manifest.project.profile, &installed, &catalog)
            .unwrap_or(FeaturePlan {
                profile: manifest.project.profile.clone(),
                requested: installed,
                steps: Vec::new(),
                validators: Vec::new(),
            });
        return finish(
            registry,
            &dir,
            "remove",
            plan,
            false,
            format!("feature '{feature}' is not installed; no changes made"),
            Vec::new(),
        );
    }
    // Reverse dependencies block removal: every remaining feature must
    // still resolve without the target.
    let remaining: Vec<String> = manifest
        .features
        .keys()
        .filter(|k| k.as_str() != feature)
        .cloned()
        .collect();
    let repair = resolve_plan_with_catalog(&manifest.project.profile, &remaining, &catalog)?;
    if repair.steps.iter().any(|s| s.feature == feature) {
        let dependents: Vec<String> = remaining
            .iter()
            .filter(|other| {
                resolve_plan_with_catalog(&manifest.project.profile, &[other.to_string()], &catalog)
                    .map(|p| p.steps.iter().any(|s| s.feature == feature))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        return Err(ForgeError::IncompatibleFeature {
            reason: format!(
                "cannot remove feature '{feature}': installed {} depend on it; \
                 remove them first; no files were changed",
                if dependents.is_empty() {
                    "features".to_string()
                } else {
                    dependents.join(", ")
                }
            ),
        });
    }
    check_receipts_clean(&dir, &manifest, &[feature.to_string()])?;
    let installed: Vec<String> = manifest.features.keys().cloned().collect();
    let plan = resolve_plan_with_catalog(&manifest.project.profile, &installed, &catalog)?;
    let mut updates: BTreeMap<String, Option<String>> = BTreeMap::new();
    updates.insert(feature.to_string(), None);
    let manifest_before = fs::read(&manifest_path).map_err(|_| ForgeError::ManifestNotFound {
        path: manifest_path.display().to_string(),
    })?;
    let files_changed = apply_and_validate(
        &dir,
        &manifest_path,
        &manifest_before,
        &updates,
        &manifest.project.profile,
    )?;
    finish(
        registry,
        &dir,
        "remove",
        plan,
        true,
        format!("removed feature '{feature}'"),
        files_changed,
    )
}

/// Upgrade `feature` to the tested catalog version. Already-current
/// installations are a no-op; uninstalled features must use `add`.
pub fn upgrade_feature(
    registry: &mut Registry,
    target: &str,
    feature: &str,
    version: Option<&str>,
) -> Result<LifecycleOutcome, ForgeError> {
    let catalog = feature_catalog();
    let descriptor = lookup(&catalog, feature)?.clone();
    let requested_version = version.unwrap_or(&descriptor.version).to_string();
    if requested_version != descriptor.version {
        return Err(ForgeError::IncompatibleFeature {
            reason: format!(
                "unknown version '{requested_version}' for feature '{feature}': \
                 only '{}' is available; manifest left unchanged",
                descriptor.version
            ),
        });
    }
    let dir = resolve_target_dir(registry, target)?;
    let (manifest, manifest_path) = Manifest::load_from_dir(&dir, None)?;
    let installed = manifest.features.get(feature).cloned();
    let installed = match installed {
        Some(v) => v,
        None => {
            return Err(ForgeError::IncompatibleFeature {
                reason: format!(
                    "cannot upgrade feature '{feature}': it is not installed; \
                     use `forge feature add` instead; no files were changed"
                ),
            });
        }
    };
    let plan =
        resolve_plan_with_catalog(&manifest.project.profile, &[feature.to_string()], &catalog)?;
    let touched: Vec<String> = plan.steps.iter().map(|s| s.feature.clone()).collect();
    check_receipts_clean(&dir, &manifest, &touched)?;
    let mut updates: BTreeMap<String, Option<String>> = BTreeMap::new();
    for step in &plan.steps {
        let current = manifest.features.get(&step.feature);
        if current.is_some_and(|v| v == &step.version) {
            continue;
        }
        // Only the requested feature may gain a version here; missing
        // dependencies are installed at their tested versions.
        if step.feature == feature || !manifest.features.contains_key(&step.feature) {
            updates.insert(step.feature.clone(), Some(step.version.clone()));
        }
    }
    if updates.is_empty() {
        return finish(
            registry,
            &dir,
            "upgrade",
            plan,
            false,
            format!("feature '{feature}' already at version '{installed}'; no changes made"),
            Vec::new(),
        );
    }
    let manifest_before = fs::read(&manifest_path).map_err(|_| ForgeError::ManifestNotFound {
        path: manifest_path.display().to_string(),
    })?;
    let files_changed = apply_and_validate(
        &dir,
        &manifest_path,
        &manifest_before,
        &updates,
        &manifest.project.profile,
    )?;
    finish(
        registry,
        &dir,
        "upgrade",
        plan,
        true,
        format!("upgraded feature '{feature}' '{installed}' -> '{requested_version}'"),
        files_changed,
    )
}

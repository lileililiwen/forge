use crate::model::MatrixJob;
use cargo_metadata::MetadataCommand;
use std::collections::HashSet;
use std::path::Path;

pub fn resolve_matrix(
    project_dir: &Path,
    toolchains: &[String],
    user_features: Option<Vec<String>>,
    exclude_features: &[String],
    include_default: bool,
    max_combinations: usize,
) -> anyhow::Result<(String, Vec<MatrixJob>)> {
    // Run cargo_metadata to extract workspace package info
    let mut cmd = MetadataCommand::new();
    cmd.current_dir(project_dir);

    let metadata = cmd.exec().map_err(|e| {
        anyhow::anyhow!(
            "Failed to read cargo metadata: {}. Are you in a Rust project directory?",
            e
        )
    })?;

    let package = metadata
        .root_package()
        .ok_or_else(|| anyhow::anyhow!("No root package found in Cargo workspace"))?;

    let project_name = package.name.clone();

    // Resolve which features we are going to test
    let available_features: Vec<String> = package
        .features
        .keys()
        .filter(|&f| f != "default")
        .cloned()
        .collect();

    let mut features_to_combine = if let Some(mut uf) = user_features {
        // If user explicitly provided features, validate and use them
        uf.retain(|f| available_features.contains(f) || package.features.contains_key(f));
        uf
    } else {
        // Otherwise, use all available features minus excluded ones
        let exclude_set: HashSet<&str> = exclude_features.iter().map(|s| s.as_str()).collect();
        available_features
            .into_iter()
            .filter(|f| !exclude_set.contains(f.as_str()))
            .collect()
    };

    features_to_combine.sort();

    // Generate feature combinations (subsets)
    let mut feature_configs = Vec::new();

    // 1. Default features configuration (if include_default is enabled)
    if include_default {
        feature_configs.push((false, Vec::new())); // no_default = false, features = empty
    }

    // 2. Base no-default features configuration
    feature_configs.push((true, Vec::new())); // no_default = true, features = empty

    if !features_to_combine.is_empty() {
        if features_to_combine.len() <= 4 {
            // Full powerset generation for 4 or fewer features (max 16 combos)
            let subsets = generate_powerset(&features_to_combine);
            for subset in subsets {
                if !subset.is_empty() {
                    // Test each subset without default features
                    feature_configs.push((true, subset.clone()));
                }
            }
        } else {
            // Avoid combinatorial explosion: test individual features and all features combined
            // Individual features (no-default)
            for feat in &features_to_combine {
                feature_configs.push((true, vec![feat.clone()]));
            }
            // All features combined (no-default)
            feature_configs.push((true, features_to_combine.clone()));
            // All features combined + default features
            feature_configs.push((false, features_to_combine.clone()));
        }
    }

    // Limit configurations to max_combinations
    if feature_configs.len() > max_combinations {
        feature_configs.truncate(max_combinations);
    }

    // Generate Cartesian product of toolchains x feature configurations
    let mut jobs = Vec::new();
    let mut job_id = 0;

    let toolchain_list = if toolchains.is_empty() {
        vec![None]
    } else {
        toolchains.iter().map(|t| Some(t.clone())).collect()
    };

    for tc in toolchain_list {
        for (no_default, feats) in &feature_configs {
            jobs.push(MatrixJob::new(
                job_id,
                tc.clone(),
                feats.clone(),
                *no_default,
            ));
            job_id += 1;
        }
    }

    Ok((project_name, jobs))
}

fn generate_powerset(features: &[String]) -> Vec<Vec<String>> {
    let mut result = vec![vec![]];
    for feat in features {
        let mut new_subsets = Vec::new();
        for subset in &result {
            let mut new_subset = subset.clone();
            new_subset.push(feat.clone());
            new_subsets.push(new_subset);
        }
        result.extend(new_subsets);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powerset() {
        let features = vec!["a".to_string(), "b".to_string()];
        let subsets = generate_powerset(&features);
        assert_eq!(subsets.len(), 4);
        assert!(subsets.contains(&vec![]));
        assert!(subsets.contains(&vec!["a".to_string()]));
        assert!(subsets.contains(&vec!["b".to_string()]));
        assert!(subsets.contains(&vec!["a".to_string(), "b".to_string()]));
    }
}

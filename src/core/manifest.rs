//! Schema-versioned `forge.yaml` contract.
//!
//! Loading is strictly read-only: validation never mutates either manifest
//! file. `forge.yaml` is canonical; `platform.yaml` is accepted only as an
//! explicit legacy import source.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use super::{validate_project_id, ForgeError};

/// The only schema version this release understands.
pub const SUPPORTED_SCHEMA: i64 = 1;
/// Canonical manifest filename.
pub const CANONICAL_MANIFEST: &str = "forge.yaml";
/// Legacy manifest filename (explicit import only).
pub const LEGACY_MANIFEST: &str = "platform.yaml";

/// Feature version accepts the YAML author's spelling: `auth: 2.1`
/// parses as a float, `auth: "2.1"` as a string. Both normalize to text.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FeatureVersion {
    Text(String),
    Int(i64),
    Float(f64),
}

impl fmt::Display for FeatureVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeatureVersion::Text(s) => write!(f, "{s}"),
            FeatureVersion::Int(i) => write!(f, "{i}"),
            FeatureVersion::Float(v) => write!(f, "{v}"),
        }
    }
}

/// Project maturity levels L0 (prototype) through L4 (production).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maturity {
    L0,
    L1,
    L2,
    L3,
    L4,
}

impl<'de> Deserialize<'de> for Maturity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "L0" => Ok(Maturity::L0),
            "L1" => Ok(Maturity::L1),
            "L2" => Ok(Maturity::L2),
            "L3" => Ok(Maturity::L3),
            "L4" => Ok(Maturity::L4),
            other => Err(serde::de::Error::custom(format!(
                "invalid maturity '{other}': expected one of L0, L1, L2, L3, L4"
            ))),
        }
    }
}

impl fmt::Display for Maturity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Maturity::L0 => "L0",
            Maturity::L1 => "L1",
            Maturity::L2 => "L2",
            Maturity::L3 => "L3",
            Maturity::L4 => "L4",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeMeta {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QualityMeta {
    #[serde(default)]
    pub driftwatch: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AiMeta {
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub default_agent: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeploymentMeta {
    #[serde(rename = "type", default)]
    pub deploy_type: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
}

/// Mirror entries accept the short (`- gitee`) and detailed
/// (`- provider: gitee, enabled: true`) spellings.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MirrorEntry {
    Name(String),
    Detailed {
        provider: String,
        #[serde(default)]
        enabled: Option<bool>,
    },
}

impl MirrorEntry {
    pub fn provider(&self) -> &str {
        match self {
            MirrorEntry::Name(name) => name,
            MirrorEntry::Detailed { provider, .. } => provider,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DistributionMeta {
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub mirrors: Vec<MirrorEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TranslationMeta {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocsMeta {
    #[serde(default)]
    pub source_language: Option<String>,
    #[serde(default)]
    pub translations: BTreeMap<String, TranslationMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectMeta {
    pub id: String,
    pub name: String,
    pub profile: String,
    #[serde(default)]
    pub maturity: Option<Maturity>,
    #[serde(default)]
    pub target_maturity: Option<Maturity>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawManifest {
    schema: serde_yaml::Value,
    project: ProjectMeta,
    #[serde(default)]
    runtime: Option<RuntimeMeta>,
    #[serde(default)]
    features: BTreeMap<String, FeatureVersion>,
    #[serde(default)]
    quality: Option<QualityMeta>,
    #[serde(default)]
    ai: Option<AiMeta>,
    #[serde(default)]
    deployment: Option<DeploymentMeta>,
    #[serde(default)]
    distribution: Option<DistributionMeta>,
    #[serde(default)]
    docs: Option<DocsMeta>,
}

/// Validated, normalized project manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub schema: i64,
    pub project: ProjectMeta,
    pub runtime: Option<RuntimeMeta>,
    pub features: BTreeMap<String, String>,
    pub quality: Option<QualityMeta>,
    pub ai: Option<AiMeta>,
    pub deployment: Option<DeploymentMeta>,
    pub distribution: Option<DistributionMeta>,
    pub docs: Option<DocsMeta>,
}

impl Manifest {
    /// Parse and validate manifest bytes. Never touches the filesystem
    /// beyond the read the caller already performed.
    pub fn parse(path: &Path, bytes: &[u8]) -> Result<Self, ForgeError> {
        let raw: RawManifest =
            serde_yaml::from_slice(bytes).map_err(|err| ForgeError::ManifestInvalid {
                path: path.display().to_string(),
                reason: err.to_string(),
            })?;

        let schema = match &raw.schema {
            serde_yaml::Value::Number(n) => n
                .as_i64()
                .filter(|v| *v == SUPPORTED_SCHEMA)
                .ok_or_else(|| ForgeError::UnsupportedSchema {
                    path: path.display().to_string(),
                    found: schema_value_to_string(&raw.schema),
                })?,
            other => {
                return Err(ForgeError::UnsupportedSchema {
                    path: path.display().to_string(),
                    found: schema_value_to_string(other),
                })
            }
        };

        validate_project_id(&raw.project.id).map_err(|reason| ForgeError::ManifestInvalid {
            path: path.display().to_string(),
            reason,
        })?;
        if raw.project.name.trim().is_empty() {
            return Err(ForgeError::ManifestInvalid {
                path: path.display().to_string(),
                reason: "project.name must not be empty".to_string(),
            });
        }
        if raw.project.profile.trim().is_empty() {
            return Err(ForgeError::ManifestInvalid {
                path: path.display().to_string(),
                reason: "project.profile must not be empty".to_string(),
            });
        }

        Ok(Manifest {
            schema,
            project: raw.project,
            runtime: raw.runtime,
            features: raw
                .features
                .into_iter()
                .map(|(k, v)| (k, v.to_string()))
                .collect(),
            quality: raw.quality,
            ai: raw.ai,
            deployment: raw.deployment,
            distribution: raw.distribution,
            docs: raw.docs,
        })
    }

    /// Resolve the manifest file for a project directory and load it.
    ///
    /// `explicit` names a manifest file relative to `dir` (or absolute)
    /// and is the only way to load the legacy `platform.yaml`.
    pub fn load_from_dir(
        dir: &Path,
        explicit: Option<&Path>,
    ) -> Result<(Self, PathBuf), ForgeError> {
        let path = resolve_manifest_path(dir, explicit)?;
        let bytes = fs::read(&path).map_err(|_| ForgeError::ManifestNotFound {
            path: path.display().to_string(),
        })?;
        Ok((Self::parse(&path, &bytes)?, path))
    }
}

fn schema_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => format!("'{s}'"),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => "complex".to_string(),
    }
}

/// Pick the manifest file for `dir` without modifying anything.
pub fn resolve_manifest_path(dir: &Path, explicit: Option<&Path>) -> Result<PathBuf, ForgeError> {
    if let Some(name) = explicit {
        let path = if name.is_absolute() {
            name.to_path_buf()
        } else {
            dir.join(name)
        };
        if path.is_file() {
            return Ok(path);
        }
        return Err(ForgeError::ManifestNotFound {
            path: path.display().to_string(),
        });
    }

    let canonical = dir.join(CANONICAL_MANIFEST);
    let legacy = dir.join(LEGACY_MANIFEST);
    match (canonical.is_file(), legacy.is_file()) {
        (true, true) => Err(ForgeError::AmbiguousManifest {
            dir: dir.display().to_string(),
            first: CANONICAL_MANIFEST.to_string(),
            second: LEGACY_MANIFEST.to_string(),
        }),
        (true, false) => Ok(canonical),
        (false, true) => Err(ForgeError::LegacyManifestRequiresExplicit {
            found: LEGACY_MANIFEST.to_string(),
        }),
        (false, false) => Err(ForgeError::ManifestNotFound {
            path: canonical.display().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Manifest, ForgeError> {
        Manifest::parse(Path::new("forge.yaml"), text.as_bytes())
    }

    #[test]
    fn parses_full_manifest_with_numeric_feature_versions() {
        let m = parse(
            r#"
schema: 1
project:
  id: mortality-reflection
  name: Mortality Reflection
  profile: rust-web
  maturity: L2
  target_maturity: L3
runtime:
  language: rust
  version: stable
features:
  auth: 2.1
  admin: "1.4"
quality:
  driftwatch: true
ai:
  spec: openspec
  default_agent: opencode
deployment:
  type: docker
  target: home-server-01
distribution:
  primary: github
  mirrors:
    - gitee
    - provider: codeberg
      enabled: true
docs:
  source_language: en
  translations:
    zh-CN:
      enabled: false
"#,
        )
        .expect("valid manifest");
        assert_eq!(m.schema, 1);
        assert_eq!(m.features.get("auth").map(String::as_str), Some("2.1"));
        assert_eq!(m.features.get("admin").map(String::as_str), Some("1.4"));
        assert_eq!(m.distribution.as_ref().unwrap().mirrors.len(), 2);
    }

    #[test]
    fn minimal_manifest_needs_no_integration_sections() {
        let m = parse("schema: 1\nproject:\n  id: tiny\n  name: Tiny\n  profile: rust-web\n")
            .expect("minimal manifest");
        assert!(m.features.is_empty());
        assert!(m.runtime.is_none());
        assert!(m.docs.is_none());
    }

    #[test]
    fn rejects_unsupported_schema() {
        let err = parse("schema: 2\nproject:\n  id: x\n  name: X\n  profile: rust-web\n")
            .expect_err("schema 2 must fail");
        assert_eq!(err.code(), "unsupported-schema");
    }

    #[test]
    fn rejects_bad_maturity_and_id() {
        let err = parse("schema: 1\nproject:\n  id: Bad_ID\n  name: X\n  profile: rust-web\n")
            .expect_err("bad id must fail");
        assert_eq!(err.code(), "manifest-invalid");
        let err = parse(
            "schema: 1\nproject:\n  id: ok-id\n  name: X\n  profile: rust-web\n  maturity: L9\n",
        )
        .expect_err("bad maturity must fail");
        assert_eq!(err.code(), "manifest-invalid");
    }
}

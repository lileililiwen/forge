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
    /// Legacy single-target type (`docker` / `ssh` / `local`).
    /// `targets[*].kind` is the modern contract; this field
    /// remains so a manifest that only declares one target
    /// still validates.
    #[serde(rename = "type", default)]
    pub deploy_type: Option<String>,
    /// Legacy single-target reference. When `targets` is
    /// present the legacy field is treated as a free-form
    /// note and the named targets are authoritative.
    #[serde(default)]
    pub target: Option<String>,
    /// Artifact path or reference consumed by the adapter
    /// (e.g. `docker-compose.yml`). Required for
    /// `docker-compose` and `local` adapters; ignored by
    /// planned adapters.
    #[serde(default)]
    pub artifact: Option<String>,
    /// Default target name. Required when `targets` lists
    /// more than one entry; optional otherwise (a single
    /// target is its own default).
    #[serde(default)]
    pub default: Option<String>,
    /// Named deployment targets. Each entry owns an adapter
    /// kind and the adapter-specific fields.
    #[serde(default)]
    pub targets: Vec<DeploymentTargetEntry>,
    /// Per-target health check configuration applied after a
    /// successful adapter invocation. `kind` is one of
    /// `docker` (checks a Compose service is running),
    /// `http` (probes a URL), or `process` (checks a
    /// process exists). The previous contract reserved
    /// health observation to the doctor; deployment makes
    /// the same checks available after apply.
    #[serde(default)]
    pub health: Option<DeploymentHealthMeta>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeploymentTargetEntry {
    /// Target id (kebab-case). When omitted the manifest
    /// validator derives one from the entry's position in
    /// the targets list.
    #[serde(default)]
    pub name: Option<String>,
    /// Adapter kind. Supported in v0.1.0: `local`,
    /// `docker-compose`. `ssh` is planned.
    #[serde(default)]
    pub kind: Option<String>,
    /// Adapter-specific free-form fields, recorded verbatim
    /// so the deploy adapter can read them. The Core
    /// contract does not interpret the contents; only the
    /// typed `name`/`kind` matter.
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    /// Optional human note surfaced in `forge deploy list`.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeploymentHealthMeta {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub process: Option<String>,
    #[serde(default)]
    pub interval_seconds: Option<u32>,
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

/// One configured external content or repository analytics
/// provider. The provider name is the stable id the catalog
/// recognises (`unified-content`, `github-analytics`, …);
/// `project_ref` is the external project id at the provider
/// (the content plane's slug or the GitHub `owner/repo`).
/// The provider plane retains its own ownership; Forge records
/// the reference and reports health, never mirrors the
/// upstream's CMS.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsProviderEntry {
    pub provider: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    /// External project id at the provider. Required when
    /// `enabled: true`; the manifest carries a reference, not
    /// an embedded secret.
    #[serde(default)]
    pub project_ref: Option<String>,
    /// Optional adapter binary used to probe the provider.
    /// Falls back to `FORGE_ANALYTICS_BIN` (or the default
    /// `forge-analytics-adapter`) when omitted.
    #[serde(default)]
    pub adapter_command: Option<String>,
}

/// Existing content and analytics planes block. The block is
/// the manifest's source of truth for which external systems
/// own this project's content and repository analytics; the
/// health and metrics surface reads the block and reports
/// observations without ever mutating upstream state.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnalyticsMeta {
    /// Master switch. When `false` the entire surface reports
    /// `disabled` and never contacts a provider (R1 boundary
    /// scenario: a disabled integration must not contact its
    /// provider).
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Default observation window in days for growth-style
    /// metrics. Bounded between 1 and 90 so the manifest
    /// cannot ask for an unbounded window.
    #[serde(default)]
    pub default_window_days: Option<u32>,
    /// External content providers (CMS, documentation,
    /// design repos). Each entry is one configured provider.
    #[serde(default)]
    pub content: Vec<AnalyticsProviderEntry>,
    /// External repository analytics providers (GitHub stars,
    /// watchers, fork count). Each entry is one configured
    /// provider.
    #[serde(default)]
    pub repository: Vec<AnalyticsProviderEntry>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DocsMeta {
    #[serde(default)]
    pub source_language: Option<String>,
    /// Canonical source document, relative to the project root.
    /// Defaults to `README.md` when omitted so manifests that only
    /// declare `source_language` keep working.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub translations: BTreeMap<String, TranslationMeta>,
    /// Explicit non-translatable terms. Every term that appears in
    /// the source must still appear verbatim in the derivative;
    /// a missing term marks the derivative `needs-review` instead
    /// of claiming translation quality from provider success.
    #[serde(default)]
    pub non_translatable: Vec<String>,
}

/// One required check before a release side effect. The kind id
/// matches the typed outcomes the doctor/test/DriftWatch modules
/// already produce: `doctor`, `test`, `driftwatch`. Future check
/// kinds are added here and matched in the release contract.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ReleaseCheck {
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReleasePackageEntry {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReleaseContainerEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub dockerfile: Option<String>,
    #[serde(default)]
    pub registry: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReleaseNotesMeta {
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReleaseDocsMeta {
    #[serde(default)]
    pub translate: Vec<String>,
}

/// OIDC admin federation block. Each project owns its own
/// per-project client id and redirect URI; the issuer and
/// admin claim live in the manifest so a project's OIDC
/// configuration is the source of truth (and the doctor
/// surface can re-validate it without contacting any
/// external provider).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IdentityMeta {
    /// OIDC provider name (kebab-case, e.g. `okta`, `auth0`,
    /// `keycloak`). The catalog only stores provider metadata
    /// — actual provider discovery and JWKS retrieval are
    /// out of scope for the v0.1 contract.
    #[serde(default)]
    pub provider: Option<String>,
    /// OIDC issuer URL (the `iss` claim the provider issues).
    /// Must be HTTPS, no whitespace, no shell metacharacters.
    #[serde(default)]
    pub issuer: Option<String>,
    /// Per-project OIDC client id registered at the provider.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Optional OIDC audience (`aud` claim). Defaults to the
    /// client id when omitted so a single-project client does
    /// not need a separate audience value.
    #[serde(default)]
    pub audience: Option<String>,
    /// Per-project admin redirect URI. The OIDC provider
    /// returns to this address after authentication.
    #[serde(default)]
    pub redirect_uri: Option<String>,
    /// Scopes the project requests (`openid` is always
    /// included; this list may add `profile`, `email`,
    /// `groups`, etc.).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// OIDC JWKS URI used to verify id_token signatures
    /// (deferred — captured here so a future integration can
    /// resolve the keys without changing the manifest).
    #[serde(default)]
    pub jwks_uri: Option<String>,
    /// State/nonce lifetime in seconds. Bounded between
    /// [`MIN_STATE_TTL_SECONDS`] and [`MAX_STATE_TTL_SECONDS`].
    #[serde(default)]
    pub state_ttl_seconds: Option<i64>,
    /// Admin session lifetime in seconds. Bounded between
    /// [`MIN_SESSION_TTL_SECONDS`] and [`MAX_SESSION_TTL_SECONDS`].
    #[serde(default)]
    pub session_ttl_seconds: Option<i64>,
    /// Claim name whose value gates admin access (e.g.
    /// `groups`, `roles`, `https://forge/permission`). The
    /// provider login alone is never enough; the claim value
    /// must match one of `admin_values`.
    #[serde(default)]
    pub admin_claim: Option<String>,
    /// Claim values that grant admin access. Empty list
    /// means no value grants admin; a request whose claim
    /// value is not in the list is refused with
    /// `identity-permission-denied`.
    #[serde(default)]
    pub admin_values: Vec<String>,
    /// Secret reference (e.g. `env://OIDC_CLIENT_SECRET`)
    /// for the per-project client. The manifest never embeds
    /// a secret; Core never logs the resolved value.
    #[serde(default)]
    pub client_secret_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReleaseMeta {
    /// Versioning scheme; only `semver` is supported in v0.5.
    #[serde(default)]
    pub versioning: Option<String>,
    /// Required checks before any release side effect.
    #[serde(default)]
    pub checks: Vec<ReleaseCheck>,
    /// Changelog filename relative to the project root. When
    /// present, the changelog's contents are bound to the release
    /// record and surfaced in the report.
    #[serde(default)]
    pub changelog: Option<String>,
    /// Package destinations. Each entry becomes one `package`
    /// stage. Real provider integration is out of scope; the
    /// adapter pattern is reused (see `docs` and `policy`).
    #[serde(default)]
    pub packages: Vec<ReleasePackageEntry>,
    /// Container destinations. Each entry becomes one
    /// `container` stage.
    #[serde(default)]
    pub containers: Vec<ReleaseContainerEntry>,
    /// Release notes generation entry; exactly one `notes`
    /// stage is allowed per release.
    #[serde(default)]
    pub notes: Option<ReleaseNotesMeta>,
    /// Documentation translations to run as part of the
    /// release. Each locale becomes one `docs` stage
    /// through the docs-translate contract.
    #[serde(default)]
    pub docs: Option<ReleaseDocsMeta>,
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

/// Optional portal block declaring the control-plane
/// dashboard title, default scope and master switch. The
/// portal is a read-only view; `enabled: false` is a
/// documented opt-out for projects that do not want a
/// dashboard rendered.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PortalMeta {
    #[serde(default = "default_portal_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub default_scope: Option<String>,
}

fn default_portal_enabled() -> bool {
    true
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
    #[serde(default)]
    release: Option<ReleaseMeta>,
    #[serde(default)]
    identity: Option<IdentityMeta>,
    #[serde(default)]
    analytics: Option<AnalyticsMeta>,
    #[serde(default)]
    portal: Option<PortalMeta>,
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
    pub release: Option<ReleaseMeta>,
    pub identity: Option<IdentityMeta>,
    pub analytics: Option<AnalyticsMeta>,
    pub portal: Option<PortalMeta>,
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
            release: raw.release,
            identity: raw.identity,
            analytics: raw.analytics,
            portal: raw.portal,
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

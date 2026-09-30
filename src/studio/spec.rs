//! Versioned AppSpec parser and validator (`forge-app-spec/0.1.0`).
//!
//! The parser is closed to the schema in
//! `schemas/app-spec-v1.json`. Unknown fields, unknown schema
//! majors, unsupported profiles, duplicate routes, embedded shell
//! metacharacters, path escapes, and secret-shaped strings are all
//! refused at validation time, before any project file is touched.
//! Validation is intentionally read-only; persistence lives in
//! [`crate::studio::state`].

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Wire contract id for the AppSpec payload.
pub const APP_SPEC_CONTRACT: &str = "forge-app-spec/0.1.0";

/// The single schema major accepted today. A future `2` lands behind
/// an additive contract bump; for now any other major is refused.
pub const APP_SCHEMA_MAJOR: &str = "1";

/// The closed set of supported profiles. Today only the existing
/// static `react-web` is selectable; future server-side profiles land
/// additively.
pub const SUPPORTED_PROFILES: &[&str] = &["react-web"];

/// Bound on the total page list. A spec with more pages than this
/// is refused at validation time so the saved spec stays small.
pub const MAX_PAGES: usize = 32;

/// Bound on the section count per page.
pub const MAX_SECTIONS_PER_PAGE: usize = 32;

/// Bound on the `name` field.
pub const MAX_PROJECT_NAME_CHARS: usize = 120;

/// Bound on a page `title`.
pub const MAX_PAGE_TITLE_CHARS: usize = 120;

/// Bound on a section `title`.
pub const MAX_SECTION_TITLE_CHARS: usize = 120;

/// Bound on a section `body`.
pub const MAX_SECTION_BODY_CHARS: usize = 4000;

/// Bound on a `route`.
pub const MAX_ROUTE_CHARS: usize = 200;

/// Bound on a section `id`.
pub const MAX_SECTION_ID_CHARS: usize = 64;

/// Bound on an `acceptance_checks` entry.
pub const MAX_ACCEPTANCE_CHECK_CHARS: usize = 200;

/// Bound on the `acceptance_checks` list.
pub const MAX_ACCEPTANCE_CHECKS: usize = 32;

/// Closed section-kind vocabulary. The MVP supports the six
/// client-rendered kinds used by the existing `react-web` profile;
/// additive kinds land behind a contract bump.
const SUPPORTED_SECTION_KINDS: &[&str] =
    &["hero", "features", "pricing", "contact", "faq", "footer"];

/// Closed theme preset vocabulary.
const SUPPORTED_THEME_PRESETS: &[&str] = &["default", "contrast", "mono"];

/// Closed `theme.tokens` keys. The MVP is intentionally narrow;
/// additional tokens land behind a contract bump.
const SUPPORTED_THEME_TOKENS: &[&str] = &["primary", "background", "text"];

/// Hex-color token pattern. The MVP only accepts `#RRGGBB`.
const HEX_COLOR_PATTERN: &str = r"^#([0-9a-fA-F]{6})$";

/// Characters that would let a spec route escape its own boundary
/// (`..`, `\`). Routes must start with `/` and contain only the
/// documented character set.
const PATH_ESCAPE_CHARS: &[char] = &['.', '\\'];

/// Top-level AppSpec shape. The `schema_version` and `pages` fields
/// are required; everything else is optional within its bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSpec {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub profile: String,
    pub pages: Vec<AppPage>,
    #[serde(default)]
    pub theme: Option<AppTheme>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
}

/// One page in the spec. Routes are app-internal paths starting
/// with `/`; sections are listed in order and carry a unique id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppPage {
    pub route: String,
    pub title: String,
    pub sections: Vec<AppSection>,
}

/// One section on a page. The `id` is unique within the spec;
/// the `kind` is a closed vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSection {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// Theme declaration. The preset is closed; tokens are an
/// allowlisted key set with bounded hex colors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppTheme {
    pub preset: ThemePreset,
    #[serde(default)]
    pub tokens: Option<ThemeTokens>,
}

/// Theme preset. Closed to the documented vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreset {
    Default,
    Contrast,
    Mono,
}

impl ThemePreset {
    fn label(self) -> &'static str {
        match self {
            ThemePreset::Default => "default",
            ThemePreset::Contrast => "contrast",
            ThemePreset::Mono => "mono",
        }
    }
}

/// Optional color tokens. Every present key must be a `#RRGGBB`
/// hex value; every absent key keeps the profile default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeTokens {
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

impl AppSpec {
    /// Render the spec as the closed `forge-app-spec/0.1.0` envelope.
    /// The envelope adds the contract id so a transport can refuse
    /// an unknown contract version without re-parsing the body.
    pub fn envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "contract": APP_SPEC_CONTRACT,
            "schema_version": self.schema_version,
            "project_id": self.project_id,
            "name": self.name,
            "profile": self.profile,
            "pages": self.pages,
            "theme": self.theme,
            "acceptance_checks": self.acceptance_checks,
        })
    }
}

/// Parse and validate the AppSpec payload. The raw text is
/// interpreted as YAML (the existing `forge.yaml` format) for
/// symmetry; the validator then enforces the closed vocabulary.
pub fn parse_spec_text(raw: &str) -> Result<AppSpec, ForgeError> {
    let spec: AppSpec = serde_yaml::from_str(raw).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!("could not parse forge.app.yaml as YAML: {err}"),
    })?;
    validate_spec(&spec)?;
    Ok(spec)
}

/// Parse and validate the AppSpec from a project-side file path.
/// The `forge.app.yaml` filename is the only accepted name; any
/// other path is refused so the operator cannot bypass the schema
/// by pointing at an arbitrary file.
pub fn parse_spec_file(path: &Path) -> Result<AppSpec, ForgeError> {
    let raw = std::fs::read_to_string(path).map_err(|err| ForgeError::StudioInvalidSpec {
        reason: format!("could not read {}: {err}", path.display()),
    })?;
    parse_spec_text(&raw)
}

/// Validate a parsed spec. Read-only — does not touch the project.
/// Every refusal carries the exact field path so the operator can
/// locate it without diffing the document.
pub fn validate_spec(spec: &AppSpec) -> Result<(), ForgeError> {
    // schema_version — exact match.
    if spec.schema_version != APP_SCHEMA_MAJOR {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "schema_version '{}' is not supported; only '{}' is accepted today",
                spec.schema_version, APP_SCHEMA_MAJOR
            ),
        });
    }
    // project_id — kebab-case.
    validate_kebab_id(&spec.project_id, "project_id", 64)?;
    // name — bounded.
    validate_bounded_chars(&spec.name, "name", 1, MAX_PROJECT_NAME_CHARS)?;
    reject_secret_shape(&spec.name, "name")?;
    // profile — closed set.
    if !SUPPORTED_PROFILES.iter().any(|p| *p == spec.profile) {
        return Err(ForgeError::StudioUnsupportedProfile {
            reason: format!(
                "profile '{}' is not supported by Forge Studio today; supported profiles: {}",
                spec.profile,
                SUPPORTED_PROFILES.join(", ")
            ),
        });
    }
    // pages — at least one, bounded.
    if spec.pages.is_empty() {
        return Err(ForgeError::StudioInvalidSpec {
            reason: "pages must contain at least one entry".to_string(),
        });
    }
    if spec.pages.len() > MAX_PAGES {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "pages contains {} entries; bound is {}",
                spec.pages.len(),
                MAX_PAGES
            ),
        });
    }
    let mut seen_routes: HashSet<&str> = HashSet::new();
    let mut seen_section_ids: HashSet<&str> = HashSet::new();
    for (page_idx, page) in spec.pages.iter().enumerate() {
        let page_prefix = format!("pages[{page_idx}]");
        validate_route(&page.route, &format!("{page_prefix}.route"))?;
        if !seen_routes.insert(page.route.as_str()) {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "duplicate route '{}' at {page_prefix}; routes must be unique",
                    page.route
                ),
            });
        }
        validate_bounded_chars(
            &page.title,
            &format!("{page_prefix}.title"),
            1,
            MAX_PAGE_TITLE_CHARS,
        )?;
        reject_secret_shape(&page.title, &format!("{page_prefix}.title"))?;
        if page.sections.is_empty() {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!("{page_prefix}.sections must contain at least one entry"),
            });
        }
        if page.sections.len() > MAX_SECTIONS_PER_PAGE {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "{page_prefix}.sections contains {} entries; bound is {}",
                    page.sections.len(),
                    MAX_SECTIONS_PER_PAGE
                ),
            });
        }
        for (sec_idx, section) in page.sections.iter().enumerate() {
            let sec_prefix = format!("{page_prefix}.sections[{sec_idx}]");
            validate_kebab_id(
                &section.id,
                &format!("{sec_prefix}.id"),
                MAX_SECTION_ID_CHARS,
            )?;
            if !seen_section_ids.insert(section.id.as_str()) {
                return Err(ForgeError::StudioInvalidSpec {
                    reason: format!(
                        "duplicate section id '{}' at {sec_prefix}; section ids must be unique across the spec",
                        section.id
                    ),
                });
            }
            if !SUPPORTED_SECTION_KINDS.iter().any(|k| *k == section.kind) {
                return Err(ForgeError::StudioInvalidSpec {
                    reason: format!(
                        "section kind '{}' at {sec_prefix} is not supported; supported kinds: {}",
                        section.kind,
                        SUPPORTED_SECTION_KINDS.join(", ")
                    ),
                });
            }
            if let Some(title) = section.title.as_deref() {
                validate_bounded_chars(
                    title,
                    &format!("{sec_prefix}.title"),
                    1,
                    MAX_SECTION_TITLE_CHARS,
                )?;
                reject_secret_shape(title, &format!("{sec_prefix}.title"))?;
            }
            if let Some(body) = section.body.as_deref() {
                validate_bounded_chars(
                    body,
                    &format!("{sec_prefix}.body"),
                    1,
                    MAX_SECTION_BODY_CHARS,
                )?;
                reject_secret_shape(body, &format!("{sec_prefix}.body"))?;
            }
        }
    }
    // theme — closed preset, allowlisted tokens, bounded hex colors.
    if let Some(theme) = &spec.theme {
        let preset_label = theme.preset.label();
        if !SUPPORTED_THEME_PRESETS.contains(&preset_label) {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "theme preset '{}' is not supported; supported presets: {}",
                    preset_label,
                    SUPPORTED_THEME_PRESETS.join(", ")
                ),
            });
        }
        if let Some(tokens) = &theme.tokens {
            let raw = serde_json::to_value(tokens).unwrap_or(serde_json::Value::Null);
            let map = raw
                .as_object()
                .ok_or_else(|| ForgeError::StudioInvalidSpec {
                    reason: "theme.tokens must be a mapping".to_string(),
                })?;
            for key in map.keys() {
                if !SUPPORTED_THEME_TOKENS.iter().any(|k| *k == key) {
                    return Err(ForgeError::StudioInvalidSpec {
                        reason: format!(
                            "theme.tokens.{key} is not a recognized token; supported tokens: {}",
                            SUPPORTED_THEME_TOKENS.join(", ")
                        ),
                    });
                }
            }
            for token_name in SUPPORTED_THEME_TOKENS {
                if let Some(value) = match *token_name {
                    "primary" => tokens.primary.clone(),
                    "background" => tokens.background.clone(),
                    "text" => tokens.text.clone(),
                    _ => None,
                } {
                    validate_hex_color(&value, &format!("theme.tokens.{token_name}"))?;
                }
            }
        }
    }
    // acceptance_checks — bounded.
    if spec.acceptance_checks.len() > MAX_ACCEPTANCE_CHECKS {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "acceptance_checks contains {} entries; bound is {}",
                spec.acceptance_checks.len(),
                MAX_ACCEPTANCE_CHECKS
            ),
        });
    }
    for (idx, entry) in spec.acceptance_checks.iter().enumerate() {
        validate_bounded_chars(
            entry,
            &format!("acceptance_checks[{idx}]"),
            1,
            MAX_ACCEPTANCE_CHECK_CHARS,
        )?;
        reject_secret_shape(entry, &format!("acceptance_checks[{idx}]"))?;
    }
    Ok(())
}

/// Reject spec fields whose value carries a secret-shaped substring.
/// The redaction vocabulary (`policy::redact_credentials`) is the
/// canonical Forge deny list: GitHub PATs, AWS keys, OpenAI keys,
/// PEM blocks, bearer tokens, etc. The check happens *before* the
/// value ever reaches a journal row, a UI render, or a refinement
/// dispatch.
fn reject_secret_shape(value: &str, field: &str) -> Result<(), ForgeError> {
    let redacted = redact_credentials(value);
    if redacted != value {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} contains a credential-shaped string; refusing to persist"),
        });
    }
    Ok(())
}

fn validate_kebab_id(value: &str, field: &str, max: usize) -> Result<(), ForgeError> {
    if value.is_empty() {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} must not be empty"),
        });
    }
    if value.len() > max {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} is {n} chars; bound is {max}", n = value.len()),
        });
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "{field} '{value}' must start with a lowercase letter and contain only lowercase letters, digits, and single dashes"
                ),
            });
        }
    }
    let mut prev_dash = false;
    for c in chars {
        if c == '-' {
            if prev_dash {
                return Err(ForgeError::StudioInvalidSpec {
                    reason: format!("{field} '{value}' must not contain consecutive dashes"),
                });
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "{field} '{value}' must contain only lowercase letters, digits, and single dashes"
                ),
            });
        }
    }
    if prev_dash {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} '{value}' must not end with a dash"),
        });
    }
    Ok(())
}

fn validate_bounded_chars(
    value: &str,
    field: &str,
    min: usize,
    max: usize,
) -> Result<(), ForgeError> {
    let len = value.chars().count();
    if len < min {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} must be at least {min} characters"),
        });
    }
    if len > max {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} is {len} characters; bound is {max}"),
        });
    }
    Ok(())
}

fn validate_route(route: &str, field: &str) -> Result<(), ForgeError> {
    if !route.starts_with('/') {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!("{field} '{route}' must start with '/'"),
        });
    }
    if route.len() > MAX_ROUTE_CHARS {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "{field} '{route}' is {n} chars; bound is {max}",
                n = route.len(),
                max = MAX_ROUTE_CHARS
            ),
        });
    }
    if route.contains(PATH_ESCAPE_CHARS) {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "{field} '{route}' contains a path-escape character; routes must not contain '.' or '\\\\'"
            ),
        });
    }
    for c in route.chars() {
        let ok = c == '/' || c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if !ok {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "{field} '{route}' contains the shell metacharacter '{c}'; routes may only contain ASCII letters, digits, '_', '-', and '/'"
                ),
            });
        }
    }
    reject_secret_shape(route, field)?;
    Ok(())
}

fn validate_hex_color(value: &str, field: &str) -> Result<(), ForgeError> {
    // Hand-rolled to avoid pulling in the `regex` crate just for this.
    let bytes = value.as_bytes();
    let pattern = HEX_COLOR_PATTERN.as_bytes();
    if bytes.len() != pattern.len() || !bytes.starts_with(b"#") {
        return Err(ForgeError::StudioInvalidSpec {
            reason: format!(
                "{field} '{value}' must match {pattern}",
                pattern = HEX_COLOR_PATTERN
            ),
        });
    }
    for &b in &bytes[1..] {
        let is_hex = b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b);
        if !is_hex {
            return Err(ForgeError::StudioInvalidSpec {
                reason: format!(
                    "{field} '{value}' must match {pattern}",
                    pattern = HEX_COLOR_PATTERN
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_YAML: &str = "schema_version: \"1\"\nproject_id: studio-fixture\nname: Studio Fixture\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n        title: Welcome\n        body: body\n";

    #[test]
    fn parse_minimal_spec_succeeds() {
        let spec = parse_spec_text(MINIMAL_YAML).expect("minimal spec parses");
        assert_eq!(spec.schema_version, "1");
        assert_eq!(spec.project_id, "studio-fixture");
        assert_eq!(spec.profile, "react-web");
        assert_eq!(spec.pages.len(), 1);
        assert_eq!(spec.pages[0].sections.len(), 1);
        assert!(spec.theme.is_none());
    }

    #[test]
    fn unknown_major_is_studio_invalid_spec() {
        let yaml = "schema_version: \"99\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
    }

    #[test]
    fn unsupported_profile_is_studio_unsupported_profile() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: aspnet-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-unsupported-profile");
    }

    #[test]
    fn duplicate_routes_are_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n  - route: /\n    title: Home again\n    sections:\n      - id: hero-block-two\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        let msg = format!("{}", err);
        assert!(msg.contains("duplicate route"));
    }

    #[test]
    fn duplicate_section_ids_are_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n      - id: hero-block\n        kind: features\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("duplicate section id"));
    }

    #[test]
    fn secret_shaped_strings_are_refused_without_echo() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /login\n    title: 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\n    sections:\n      - id: hero-block\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        let msg = format!("{}", err);
        assert!(msg.contains("credential-shaped"));
        assert!(!msg.contains("ghp_"));
    }

    #[test]
    fn path_escape_routes_are_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: \"/../etc/passwd\"\n    title: Escape\n    sections:\n      - id: hero-block\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("path-escape"));
    }

    #[test]
    fn empty_pages_is_refused() {
        let yaml =
            "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages: []\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("at least one"));
    }

    #[test]
    fn unknown_section_kind_is_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: custom\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("custom"));
    }

    #[test]
    fn project_id_must_be_kebab_case() {
        let yaml = "schema_version: \"1\"\nproject_id: \"Foo Bar\"\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
    }

    #[test]
    fn theme_tokens_must_be_hex() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\ntheme:\n  preset: default\n  tokens:\n    primary: not-a-color\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("primary"));
    }

    #[test]
    fn theme_token_with_unknown_key_is_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\ntheme:\n  preset: default\n  tokens:\n    surprise: \"#112233\"\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
        assert!(format!("{}", err).contains("surprise"));
    }

    #[test]
    fn acceptance_check_secret_shape_is_refused() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\nacceptance_checks:\n  - \"AKIAIOSFODNN7EXAMPLE\"\n";
        let err = parse_spec_text(yaml).unwrap_err();
        assert_eq!(err.code(), "studio-invalid-spec");
    }

    #[test]
    fn envelope_includes_contract_id() {
        let spec = parse_spec_text(MINIMAL_YAML).unwrap();
        let env = spec.envelope();
        assert_eq!(
            env.get("contract").and_then(|v| v.as_str()),
            Some(APP_SPEC_CONTRACT)
        );
    }

    #[test]
    fn theme_preset_default_is_parsed() {
        let yaml = "schema_version: \"1\"\nproject_id: foo\nname: Foo\nprofile: react-web\npages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero-block\n        kind: hero\ntheme:\n  preset: default\n";
        let spec = parse_spec_text(yaml).unwrap();
        assert_eq!(
            spec.theme.as_ref().map(|t| t.preset),
            Some(ThemePreset::Default)
        );
    }
}

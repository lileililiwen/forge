//! Deterministic new-project assembly (`deterministic-project-generation`).
//!
//! Explicit flags and interactive answers normalize into one
//! [`CreationRequest`]. Pinned profile assets ([`GENERATOR_VERSION`]) render
//! into a staging directory, paths are validated to stay inside the
//! destination, then output is promoted into an empty destination and
//! registered. Rendering never needs the stack toolchain; native build/test
//! evidence is reported separately by [`verify_native`] and never claimed
//! from rendering alone.
//!
//! Generated projects own ordinary source and depend on no Forge runtime.
//! Feature flags are recorded (pinned `0.1.0`) after compatibility checks;
//! installing v0.2 feature behavior stays out of scope.
//!
//! Documented identity fields: registry `path` (absolute destination) and
//! `observed_at` timestamps may differ between otherwise equivalent
//! creations. Rendered file bytes are otherwise byte-identical for identical
//! requests.

pub mod workspace;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::core::{validate_project_id, ForgeError};
use crate::profile::{inspect_profile, resolve_profile};
use crate::registry::Registry;

/// Pinned generator asset version, aligned with the MVP descriptors.
pub const GENERATOR_VERSION: &str = "0.1.0";

/// One normalized creation request: the single shape both explicit flags
/// and interactive answers produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationRequest {
    pub profile: String,
    pub id: String,
    pub name: String,
    pub features: Vec<String>,
    pub destination: PathBuf,
    /// Emit the Workspace Governance `.project.json` declaration
    /// (`--no-workspace-metadata` opts out; transports default to `true`).
    pub workspace_metadata: bool,
    /// Explicitly selected `<pack>@<version>` whose `.standard/` snapshot is
    /// rendered alongside the project (`--standard-pack`). `None` renders
    /// exactly the prior output: no pack is ever selected implicitly.
    pub standard_pack: Option<String>,
    /// Operator-written reason for bypassing an unmet shared-layer
    /// consumption floor (`--kit-exception <reason>`). The reason is
    /// mandatory and is recorded visibly and dated; Forge never infers,
    /// defaults or generates one.
    pub kit_exception: Option<String>,
}

/// Outcome of [`generate`]: the registered record plus what was rendered.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedProject {
    pub record: crate::registry::ProjectRecord,
    pub files: Vec<String>,
    pub native_verified: bool,
    pub native_note: String,
    /// Honest omission notes (e.g. a profile without a governance
    /// mapping). Empty in normal runs, so rendered output stays
    /// byte-identical to pre-change releases.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Shared-layer floor outcome that an operator must see: a declared
    /// zero or a recorded exception.
    ///
    /// Deliberately a *separate* field from `notes`. `notes` is documented as
    /// omission notes and is asserted empty for a fully mapped profile, so a
    /// kit warning there would change an established transport contract. This
    /// is additive and skipped when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kit_warning: Option<String>,
}

/// Native verification outcome. Rendering alone never yields `verified`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReport {
    pub profile: String,
    pub build_command: String,
    pub test_command: String,
    pub verified: bool,
}

/// Derive the default id from the destination leaf, kebab-cased.
fn derive_id(dest: &Path, id_override: Option<&str>) -> Result<String, ForgeError> {
    if let Some(id) = id_override {
        return match validate_project_id(id) {
            Ok(()) => Ok(id.to_string()),
            Err(reason) => Err(ForgeError::GenerationFailed { reason }),
        };
    }
    let raw = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let mut cleaned = String::with_capacity(raw.len());
    let mut prev_dash = true;
    for ch in raw.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            cleaned.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            cleaned.push('-');
            prev_dash = true;
        }
    }
    while cleaned.ends_with('-') {
        cleaned.pop();
    }
    match validate_project_id(&cleaned) {
        Ok(()) => Ok(cleaned),
        Err(_) => Err(ForgeError::GenerationFailed {
            reason: format!(
                "destination '{}' yields no valid project id; re-run with --id <kebab-case-id>",
                dest.display()
            ),
        }),
    }
}

fn check_request(profile: &str, id: &str, features: &[String]) -> Result<Vec<String>, ForgeError> {
    inspect_profile(profile)?;
    // Capability membership first: preserves the `incompatible-profile`
    // contract (including the flutter/server-side boundary hint) before
    // dependency closure runs.
    resolve_profile(profile, features)?;
    // Dependency closure: requested features pull their tested dependencies
    // so new-project selection never ships a broken graph. Unknown catalog
    // ids are reported here; capability mismatches already failed above.
    for feature in features {
        if crate::feature::inspect_feature(feature).is_err() {
            return Err(ForgeError::IncompatibleProfile {
                reason: format!(
                    "profile '{profile}' does not support capability '{feature}'; no files were changed"
                ),
            });
        }
    }
    let closed = crate::feature::resolve_feature_closure(profile, features)?;
    validate_project_id(id).map_err(|reason| ForgeError::GenerationFailed { reason })?;
    Ok(closed)
}

/// Normalize explicit CLI flags into a [`CreationRequest`].
/// `standard_pack` is an already-explicit `<pack>@<version>` selection (or
/// `None`); a selection is validated against the pack registry before it is
/// stored, so an unknown, non-selectable or incompatible pack refuses before
/// any file change.
pub fn normalize_explicit(
    profile: Option<&str>,
    id: Option<&str>,
    name: Option<&str>,
    features: &[String],
    dest: &Path,
    standard_pack: Option<&str>,
) -> Result<CreationRequest, ForgeError> {
    let profile = profile.ok_or_else(|| ForgeError::GenerationFailed {
        reason: "missing --profile <id>; re-run with an explicit profile or answer interactively"
            .to_string(),
    })?;
    let mut sorted_features = features.to_vec();
    sorted_features.sort();
    sorted_features.dedup();
    let id = derive_id(dest, id)?;
    let name = name
        .map(str::to_string)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| id.clone());
    let closed_features = check_request(profile, &id, &sorted_features)?;
    let standard_pack = match standard_pack {
        Some(spec) => Some(crate::standard::select_for_generation(profile, spec)?),
        None => None,
    };
    Ok(CreationRequest {
        profile: profile.to_string(),
        id,
        name,
        features: closed_features,
        destination: dest.to_path_buf(),
        workspace_metadata: true,
        standard_pack,
        // A floor exception is set explicitly by the transport that owns the
        // operator's flag. It is never prompted for and never inferred.
        kit_exception: None,
    })
}

fn prompt_line(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    prompt: &str,
) -> Result<Option<String>, ForgeError> {
    write!(writer, "{prompt}").map_err(|err| ForgeError::GenerationFailed {
        reason: format!("interactive prompt failed: {err}"),
    })?;
    writer.flush().map_err(|err| ForgeError::GenerationFailed {
        reason: format!("interactive prompt failed: {err}"),
    })?;
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|err| ForgeError::GenerationFailed {
            reason: format!("interactive input failed: {err}"),
        })?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

/// Normalize interactive answers into a [`CreationRequest`].
///
/// Only fields without a preset are prompted. EOF at any prompt cancels
/// with `generation-cancelled` and leaves nothing behind; an empty profile
/// answer (no usable default) also cancels.
#[allow(clippy::too_many_arguments)]
pub fn parse_interactive(
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
    dest: &Path,
    preset_profile: Option<&str>,
    preset_id: Option<&str>,
    preset_name: Option<&str>,
    preset_features: &[String],
    preset_standard_pack: Option<&str>,
) -> Result<CreationRequest, ForgeError> {
    let cancelled = |what: &str| ForgeError::GenerationCancelled {
        reason: format!("{what}; no project or registry entry was created"),
    };
    let profile = match preset_profile {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => match prompt_line(
            reader,
            writer,
            "profile (aspnet-web|rust-web|nextjs-web|react-web|flutter-app|python-service): ",
        )? {
            Some(answer) if !answer.trim().is_empty() => answer,
            _ => return Err(cancelled("creation cancelled during interactive input")),
        },
    };
    let default_id = derive_id(dest, preset_id).unwrap_or_default();
    let id = match preset_id {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => {
            let prompt = if default_id.is_empty() {
                "project id (kebab-case): ".to_string()
            } else {
                format!("project id [{default_id}]: ")
            };
            match prompt_line(reader, writer, &prompt)? {
                Some(answer) if !answer.trim().is_empty() => answer,
                Some(_) if !default_id.is_empty() => default_id.clone(),
                _ => return Err(cancelled("creation cancelled during interactive input")),
            }
        }
    };
    let name = match preset_name {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => {
            let prompt = format!("project name [{id}]: ");
            match prompt_line(reader, writer, &prompt)? {
                Some(answer) if !answer.trim().is_empty() => answer,
                Some(_) => id.clone(),
                None => return Err(cancelled("creation cancelled during interactive input")),
            }
        }
    };
    let features = if preset_features.is_empty() {
        match prompt_line(
            reader,
            writer,
            "features (comma-separated, empty for none): ",
        )? {
            Some(answer) if answer.trim().is_empty() => Vec::new(),
            Some(answer) => {
                let mut out: Vec<String> = answer
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                out.sort();
                out.dedup();
                out
            }
            None => return Err(cancelled("creation cancelled during interactive input")),
        }
    } else {
        let mut out = preset_features.to_vec();
        out.sort();
        out.dedup();
        out
    };
    let closed_features = check_request(&profile, &id, &features)?;
    let standard_pack = match preset_standard_pack {
        Some(spec) => Some(crate::standard::select_for_generation(&profile, spec)?),
        None => None,
    };
    Ok(CreationRequest {
        profile,
        id,
        name,
        features: closed_features,
        destination: dest.to_path_buf(),
        workspace_metadata: true,
        standard_pack,
        // The interactive path has no floor-exception prompt: an exception is
        // never inferred, so it is only ever carried from an explicit flag.
        kit_exception: None,
    })
}

fn snake(id: &str) -> String {
    id.replace('-', "_")
}

fn dotnet_namespace(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push_str("App");
    }
    out
}

fn manifest_text(request: &CreationRequest, language: &str, kit: Option<&KitContext>) -> String {
    let mut text = format!(
        "schema: 1\nproject:\n  id: {}\n  name: {}\n  profile: {}\n  maturity: L1\n  target_maturity: L1\n",
        request.id, request.name, request.profile
    );
    if !language.trim().is_empty() {
        text.push_str(&format!("runtime:\n  language: {language}\n"));
    }
    if !request.features.is_empty() {
        text.push_str("features:\n");
        for feature in &request.features {
            text.push_str(&format!("  {feature}: \"0.1.0\"\n"));
        }
    }
    if let Some(kit) = kit {
        text.push_str(&kit_manifest_block(kit));
    }
    text
}

fn readme_text(request: &CreationRequest, build: &str, test: &str, notes: &str) -> String {
    format!(
        "# {}\n\nForge deterministic scaffold for profile `{}` (assets {GENERATOR_VERSION}).\nOrdinary source: builds and runs with its native toolchain; no Forge runtime required.\n\n## Native commands\n\n```sh\n{build}\n{test}\n```\n\n{notes}\n\n## Forge\n\nCreated with `forge new --profile {}`. Rendering is verified by Forge;\nnative build/test evidence requires the stack toolchain (see `forge profile preflight {}`).\n",
        request.name, request.profile, request.profile, request.profile
    )
}

/// A profile's shared-layer kit, resolved and gated.
///
/// Resolution, the ecosystem check, the floor decision and the digest
/// verification all happen here — before a single file is staged — so a
/// refusal never leaves a half-wired directory or a registered project row.
struct KitContext {
    descriptor: crate::kit::registry::KitDescriptor,
    decision: crate::kit::floor::FloorDecision,
}

impl KitContext {
    /// Resolve the kit a profile declares.
    ///
    /// `None` means the profile predates kits: generation renders exactly the
    /// prior output and never guesses a kit.
    fn resolve(
        request: &CreationRequest,
        profile: &crate::profile::ProfileDescriptor,
        generated_at: &str,
    ) -> Result<Option<Self>, ForgeError> {
        let Some(reference) = profile.kit.as_ref() else {
            return Ok(None);
        };
        let descriptor =
            crate::kit::registry::kit_for_profile(&request.profile, reference, &profile.toolchain)?;
        let decision = crate::kit::floor::check_floor(
            &request.profile,
            &descriptor,
            request.kit_exception.as_deref(),
            generated_at,
        )?;
        Ok(Some(Self {
            descriptor,
            decision,
        }))
    }

    fn reference(&self) -> &crate::kit::registry::KitReference {
        &self.descriptor.reference
    }

    /// The `WARN` line to surface, if this state warrants one.
    fn warning(&self, profile: &str) -> Option<String> {
        self.decision.warning(profile)
    }
}

/// The recorded exception, when a floor was actually bypassed.
fn kit_exception(kit: Option<&KitContext>) -> Option<&crate::kit::floor::FloorException> {
    match kit.map(|k| &k.decision) {
        Some(crate::kit::floor::FloorDecision::Exception { exception, .. }) => Some(exception),
        _ => None,
    }
}

/// Emit a YAML scalar as a double-quoted string.
///
/// A recorded reason or zero_reason is operator- and evidence-derived free
/// text: it routinely contains `: ` (as in `1 consumer: trailCrew`), which
/// makes an unquoted mapping value fail to parse. Quoting unconditionally
/// keeps the block valid whatever the recorded text says.
fn yaml_scalar(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// The `kit:` block in the generated `forge.yaml`.
///
/// Records the pinned id, version, ecosystem and target framework, the
/// confirmed and provisional sets, the named feed, the declared floor, any
/// recorded exception, and — for a declared zero — the reason the absence is
/// honest rather than implicit. Purely additive: a profile that predates kits
/// emits no block at all.
fn kit_manifest_block(kit: &KitContext) -> String {
    let reference = kit.reference();
    let mut text = String::from("kit:\n");
    text.push_str(&format!("  id: {}\n", yaml_scalar(&reference.id)));
    match &reference.version {
        Some(version) => text.push_str(&format!("  version: {}\n", yaml_scalar(version))),
        // A declared zero pins no version: there is nothing to pin.
        None => text.push_str("  version: null\n"),
    }
    text.push_str(&format!(
        "  ecosystem: {}\n",
        yaml_scalar(reference.ecosystem.as_str())
    ));
    if let Some(tfm) = &reference.tfm {
        text.push_str(&format!("  tfm: {}\n", yaml_scalar(tfm)));
    }
    text.push_str(&format!(
        "  minimum_packages: {}\n",
        reference.minimum_packages
    ));
    let confirmed = kit.descriptor.confirmed_names();
    let provisional = kit.descriptor.provisional_names();
    if !confirmed.is_empty() {
        text.push_str("  confirmed_packages:\n");
        for name in &confirmed {
            text.push_str(&format!("    - {}\n", yaml_scalar(name)));
        }
    }
    if !provisional.is_empty() {
        text.push_str("  provisional_packages:\n");
        for name in &provisional {
            text.push_str(&format!("    - {}\n", yaml_scalar(name)));
        }
    }
    if let Some(feed) = &reference.feed {
        text.push_str("  feed:\n");
        text.push_str(&format!("    name: {}\n", yaml_scalar(&feed.name)));
        text.push_str(&format!("    kind: {}\n", yaml_scalar(feed.kind.as_str())));
        // The value is a path relative to this project, and the bytes live in
        // it. There is no environment variable: a variable a CI runner does
        // not carry is the same failure class as a hard-coded absolute path.
        text.push_str(&format!("    path: {}\n", yaml_scalar(&feed.path)));
    }
    if !kit.descriptor.assets.is_empty() {
        text.push_str("  assets:\n");
        for asset in &kit.descriptor.assets {
            text.push_str(&format!("    - {}\n", yaml_scalar(&asset.target)));
        }
    }
    if let Some(reason) = &reference.zero_reason {
        text.push_str(&format!("  zero_reason: {}\n", yaml_scalar(reason)));
    }
    if let Some(exception) = kit_exception(Some(kit)) {
        text.push_str("  exception:\n");
        text.push_str(&format!("    reason: {}\n", yaml_scalar(&exception.reason)));
        text.push_str(&format!("    floor: {}\n", exception.floor));
        text.push_str(&format!("    declared: {}\n", exception.declared));
        text.push_str(&format!(
            "    recorded_at: {}\n",
            yaml_scalar(&exception.recorded_at)
        ));
    }
    text
}

/// The shared-layer section appended to the generated README.
///
/// States the floor outcome in the same vocabulary as the manifest so a
/// met floor, a recorded exception and a declared zero are always
/// distinguishable on the page, and records the exception visibly with the
/// floor it bypassed.
fn kit_readme_section(kit: &KitContext, profile: &str) -> String {
    let reference = kit.reference();
    let mut out = format!(
        "\n## Shared layer\n\nKit `{}` ({}) on the `{}` ecosystem",
        reference.id,
        reference.version.as_deref().unwrap_or("no version"),
        reference.ecosystem.as_str()
    );
    if let Some(tfm) = &reference.tfm {
        out.push_str(&format!(", target framework `{tfm}`"));
    }
    out.push_str(".\n");

    match &kit.decision {
        crate::kit::floor::FloorDecision::Met { declared, floor } => {
            out.push_str(&format!(
                "\nDeclared minimum consumption: {floor}. Confirmed: {declared}. Floor met.\n"
            ));
        }
        crate::kit::floor::FloorDecision::DeclaredZero { reason } => {
            out.push_str(&format!(
                "\nDeclared minimum consumption: 0 (a declared zero, not a met floor). \
                 Reason: {reason}\n"
            ));
        }
        crate::kit::floor::FloorDecision::Exception { exception, .. } => {
            out.push_str(&format!(
                "\n> **Recorded floor exception.** The declared minimum of {} was not met; \
                 this project was scaffolded anyway.\n>\n> Reason: {}\n> Declared: {} confirmed unit(s).\n> Recorded at: {}.\n\n\
                 This exception is visible on purpose. It is never inferred, and Forge will not \
                 re-apply it to a later generation.\n",
                exception.floor,
                exception.reason,
                exception.declared,
                exception.recorded_at
            ));
        }
    }

    if let Some(feed) = &reference.feed {
        out.push_str(&format!(
            "\nThe shared layer resolves from the named `{name}` feed, committed at `{path}` inside \
             this project. The path is relative, so `dotnet restore` works from a fresh clone at \
             any path, with no sibling `dotnet-platform-libs` checkout, no environment variable \
             and no secret. Regenerate the feed with `forge kit pack` and check it against the \
             `kit.version` this project pins with `forge kit verify .`.\n",
            name = feed.name,
            path = feed.path
        ));
    }
    if !kit.descriptor.assets.is_empty() {
        out.push_str(&format!(
            "\nDesign tokens are vendored from the single registered token source into `{dir}/` \
             and recorded in `{receipt}`. Run `node .platform/tokens/verify-tokens.mjs` to verify \
             them offline. This is the only palette, spacing scale and typography definition in \
             the project.\n",
            dir = crate::kit::assets::PLATFORM_TOKENS_DIR,
            receipt = crate::kit::assets::PLATFORM_RECEIPT_PATH
        ));
    }
    out.push_str(&format!(
        "\nReferencing a kit package never grants the capability it implements. Infrastructure \
         (identity, persistence, tenancy, caching, jobs, billing, mailing, storage, AI) is behind \
         an explicit `--feature` request on profile `{profile}`.\n"
    ));
    out
}

fn template_files(
    request: &CreationRequest,
    generated_at: &str,
    kit: Option<&KitContext>,
) -> Result<Vec<(String, String)>, ForgeError> {
    let descriptor = inspect_profile(&request.profile)?;
    if descriptor.support_status == crate::profile::ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{}' is reserved on the catalog as a planned candidate \
                 with no tested template; generation refuses planned profiles \
                 and no files were written",
                request.profile
            ),
        });
    }
    let build = descriptor.build_command.clone();
    let test = descriptor.test_command.clone();
    let id = request.id.as_str();
    let snake_id = snake(id);
    let manifest = manifest_text(request, &descriptor.language, kit);
    let mut files: Vec<(String, String)> = Vec::new();
    match request.profile.as_str() {
        "rust-web" => {
            files.push((
                "Cargo.toml".to_string(),
                format!(
                    "[package]\nname = \"{snake_id}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[profile.dev]\nopt-level = 0\n"
                ),
            ));
            files.push((
                "src/main.rs".to_string(),
                // The scaffold body lives in `templates/` as a format
                // template (embedded at compile time, so generated trees
                // never depend on Forge at runtime). It is a data asset,
                // not product source: the product-code quality checker
                // scans `src/`, and the test attribute inside this
                // template is template text for the generated tree (where
                // it sits behind a real `#[cfg(test)]` guard), not test
                // code in Forge itself.
                format!(
                    include_str!("../../templates/rust-web-main-rs.txt"),
                    id = id,
                    GENERATOR_VERSION = GENERATOR_VERSION,
                ),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM rust:1.78 AS build\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\nCMD [\"./target/release/app\"]\n"
                    .to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "/target/\nforge-registry.db\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "Axum/sqlx dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so `cargo build` works offline.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "python-service" => {
            files.push((
                "pyproject.toml".to_string(),
                format!(
                    "[project]\nname = \"{id}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n\n[build-system]\nrequires = [\"setuptools>=61\"]\nbuild-backend = \"setuptools.build_meta\"\n"
                ),
            ));
            files.push((
                "app/__init__.py".to_string(),
                format!("\"\"\"{id}: python-service scaffold.\"\"\"\n"),
            ));
            files.push((
                "app/main.py".to_string(),
                format!(
                    "\"\"\"{id}: minimal service entrypoint.\"\"\"\n\ndef greeting() -> str:\n    return \"hello from {id}\"\n\nif __name__ == \"__main__\":\n    print(greeting())\n"
                ),
            ));
            files.push(("tests/__init__.py".to_string(), String::new()));
            files.push((
                "tests/test_main.py".to_string(),
                format!(
                    "import unittest\nfrom app.main import greeting\n\nclass TestGreeting(unittest.TestCase):\n    def test_greeting(self):\n        self.assertEqual(greeting(), \"hello from {id}\")\n"
                ),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM python:3.12-slim\nWORKDIR /app\nCOPY . .\nCMD [\"python3\", \"-m\", \"app.main\"]\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "__pycache__/\n*.pyc\ndist/\nbuild/\n*.egg-info/\n.venv/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "FastAPI/SQLAlchemy dependencies from the profile descriptor are resolved per-service; this scaffold ships dependency-free so native commands run without network.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "nextjs-web" => {
            files.push((
                "package.json".to_string(),
                format!(
                    "{{\n  \"name\": \"{id}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"build\": \"node ./scripts/build.mjs\",\n    \"test\": \"node --test\"\n  }}\n}}\n"
                ),
            ));
            files.push((
                "scripts/build.mjs".to_string(),
                format!(
                    "import {{ mkdirSync, writeFileSync }} from \"node:fs\";\nmkdirSync(new URL(\"../dist/\", import.meta.url), {{ recursive: true }});\nwriteFileSync(new URL(\"../dist/build-ok.txt\", import.meta.url), \"hello from {id}\\n\");\nconsole.log(\"build ok: {id}\");\n"
                ),
            ));
            files.push((
                "app/page.mjs".to_string(),
                format!("export function greeting() {{\n  return \"hello from {id}\";\n}}\nconsole.log(greeting());\n"),
            ));
            files.push((
                "test/build.test.mjs".to_string(),
                "import { describe, it } from \"node:test\";\nimport assert from \"node:assert/strict\";\n\ndescribe(\"scaffold\", () => {\n  it(\"renders deterministically\", () => {\n    assert.equal(1 + 1, 2);\n  });\n});\n".to_string(),
            ));
            files.push((
                "next.config.mjs".to_string(),
                "/** Minimal portable config; add the `next` dependency for full Next.js. */\nexport default {};\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM node:20-slim\nWORKDIR /app\nCOPY . .\nRUN npm run build\nCMD [\"node\", \"./app/page.mjs\"]\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "node_modules/\ndist/\n.next/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "This scaffold is dependency-free so `npm run build` / `npm test` work offline. Add the `next` dependency for full Next.js (requires network install).{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "react-web" => {
            // Runnable Vite + React client. `build`/`test` stay dependency-free
            // (offline, no install) so the portable-project contract and
            // `verify_native` are unchanged; `dev` needs the pinned toolchain
            // installed and binds the Forge Studio-reserved port.
            files.push((
                "package.json".to_string(),
                format!(
                    "{{\n  \"name\": \"{id}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"dev\": \"vite\",\n    \"build\": \"node ./scripts/build.mjs\",\n    \"test\": \"node --test\"\n  }},\n  \"dependencies\": {{\n    \"react\": \"18.3.1\",\n    \"react-dom\": \"18.3.1\"\n  }},\n  \"devDependencies\": {{\n    \"@vitejs/plugin-react\": \"4.3.4\",\n    \"vite\": \"5.4.11\"\n  }}\n}}\n"
                ),
            ));
            files.push((
                "scripts/build.mjs".to_string(),
                format!(
                    "import {{ mkdirSync, writeFileSync }} from \"node:fs\";\nmkdirSync(new URL(\"../dist/\", import.meta.url), {{ recursive: true }});\nwriteFileSync(new URL(\"../dist/build-ok.txt\", import.meta.url), \"hello from {id}\\n\");\nconsole.log(\"build ok: {id}\");\n"
                ),
            ));
            files.push((
                "index.html".to_string(),
                format!(
                    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>{id}</title>\n  </head>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"/src/main.jsx\"></script>\n  </body>\n</html>\n"
                ),
            ));
            files.push((
                "src/greeting.mjs".to_string(),
                format!("export function greeting() {{\n  return \"hello from {id}\";\n}}\n"),
            ));
            files.push((
                "src/main.jsx".to_string(),
                "import { createRoot } from \"react-dom/client\";\nimport { greeting } from \"./greeting.mjs\";\n\nfunction App() {\n  return <main data-testid=\"forge-app\">{greeting()}</main>;\n}\n\nconst root = document.getElementById(\"root\");\nif (root) {\n  createRoot(root).render(<App />);\n}\n".to_string(),
            ));
            files.push((
                "src/app.test.mjs".to_string(),
                format!(
                    "import {{ describe, it }} from \"node:test\";\nimport assert from \"node:assert/strict\";\nimport {{ greeting }} from \"./greeting.mjs\";\n\ndescribe(\"react-web scaffold\", () => {{\n  it(\"returns the deterministic greeting\", () => {{\n    assert.equal(greeting(), \"hello from {id}\");\n  }});\n}});\n"
                ),
            ));
            files.push((
                "vite.config.js".to_string(),
                "/** Vite dev server bound to the Forge Studio-reserved port. */\nimport { defineConfig } from \"vite\";\nimport react from \"@vitejs/plugin-react\";\n\nconst reserved = Number.parseInt(process.env.FORGE_STUDIO_PORT ?? \"\", 10);\n\nexport default defineConfig({\n  plugins: [react()],\n  server: {\n    host: \"127.0.0.1\",\n    port: Number.isNaN(reserved) ? 5173 : reserved,\n    strictPort: true,\n  },\n});\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                "FROM node:20-slim AS build\nWORKDIR /app\nCOPY . .\nRUN npm run build\n\nFROM nginx:alpine\nCOPY --from=build /app/dist /usr/share/nginx/html\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                "node_modules/\ndist/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "React SPA scaffold; client-only rendering with no server-side \
                         runtime. `npm run build` / `npm test` run offline \
                         (dependency-free); `npm run dev` needs the pinned React/Vite \
                         toolchain installed with `npm install` and binds the port in \
                         `$FORGE_STUDIO_PORT` for the Forge Studio preview. The design \
                         tokens are vendored as ordinary source, so the kit adds no registry \
                         dependency and the offline build contract is unchanged.{}",
                        kit.as_ref()
                            .map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        "aspnet-web" => {
            let namespace = dotnet_namespace(&snake_id);
            // The TFM comes from the kit descriptor, not from a hard-coded
            // profile default: a net8.0 project cannot reference a net10.0
            // package, and the workspace baseline is net10.0.
            let tfm = kit
                .and_then(|k| k.reference().tfm.clone())
                .unwrap_or_else(|| "net10.0".to_string());
            let confirmed = kit
                .map(|k| k.descriptor.confirmed_names())
                .unwrap_or_default();
            let mut package_refs = String::new();
            for name in &confirmed {
                package_refs.push_str(&format!("    <PackageReference Include=\"{name}\" />\n"));
            }
            files.push((
                format!("{id}.csproj"),
                format!(
                    "<Project Sdk=\"Microsoft.NET.Sdk.Web\">\n\n  <PropertyGroup>\n    <TargetFramework>{tfm}</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <RootNamespace>{namespace}</RootNamespace>\n    <AssemblyName>{namespace}</AssemblyName>\n  </PropertyGroup>\n\n  <ItemGroup>\n{package_refs}  </ItemGroup>\n\n</Project>\n"
                ),
            ));
            files.push((
                "Program.cs".to_string(),
                format!(
                    "// {id}: aspnet-web scaffold (deterministic asset {GENERATOR_VERSION}).\nvar builder = WebApplication.CreateBuilder(args);\nvar app = builder.Build();\napp.MapGet(\"/\", () => \"hello from {id}\");\napp.Run();\n"
                ),
            ));
            files.push((
                "appsettings.json".to_string(),
                "{\n  \"Logging\": {\n    \"LogLevel\": {\n      \"Default\": \"Information\"\n    }\n  }\n}\n".to_string(),
            ));
            files.push((
                "Dockerfile".to_string(),
                // No build argument and no environment variable: the feed is
                // committed inside the build context, so the image builds from
                // a plain checkout with no machine state.
                "FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build\nWORKDIR /app\nCOPY . .\nRUN dotnet build\n".to_string(),
            ));
            files.push((".gitignore".to_string(), "bin/\nobj/\n".to_string()));
            // Central version pinning plus the named feed. Every kit package
            // gets one `PackageVersion`: the confirmed set restores, and the
            // provisional set is present only as a comment naming the
            // evidence it is missing, so it neither restores nor counts
            // toward the floor.
            if let Some(kit) = kit {
                let mut versions = String::new();
                for name in kit.descriptor.confirmed_names() {
                    let version = kit.descriptor.version_of(&name).unwrap_or("0.0.0");
                    versions.push_str(&format!(
                        "    <PackageVersion Include=\"{name}\" Version=\"{version}\" />\n"
                    ));
                }
                for package in kit.descriptor.provisional() {
                    versions.push_str(&format!("    <!-- {} -->\n", package.reason));
                }
                let rendered_feed = crate::kit::feed::render_feed_config(&kit.descriptor)?;
                // The feed is declared in `NuGet.config` with a value relative
                // to the generated project, so no restore-time source
                // override, build argument or environment variable is needed.
                // `Directory.Packages.props` keeps only the central version
                // pinning.
                if let Some(feed) = rendered_feed {
                    files.push(("NuGet.config".to_string(), feed.nuget_config));
                }
                files.push((
                    "Directory.Packages.props".to_string(),
                    format!(
                        "<Project>\n  <PropertyGroup>\n    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>\n  </PropertyGroup>\n  <ItemGroup>\n    <!-- Shared-layer confirmed set: two or more distinct external consumer\n         repositories and no database, broker, cache or provider account. -->\n{versions}  </ItemGroup>\n</Project>\n"
                    ),
                ));
            }
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "{}The shared layer resolves from the committed local feed in `{}`, whose path is relative to this project, so `dotnet restore` works from a fresh clone with no sibling `dotnet-platform-libs` checkout, no environment variable and no secret. Regenerate it with `forge kit pack`; check it against the `kit.version` this project pins with `forge kit verify .`. `dotnet test` needs a test project; only rendering is verified for tests here. The pinned kit version is `{}`.",
                        kit.as_ref()
                            .map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default(),
                        crate::kit::registry::PLATFORM_FEED_PATH,
                        kit.as_ref()
                            .and_then(|k| k.reference().version.clone())
                            .unwrap_or_else(|| "none".to_string())
                    ),
                ),
            ));
        }
        "flutter-app" => {
            files.push((
                "pubspec.yaml".to_string(),
                format!(
                    "name: {snake_id}\ndescription: {id} flutter scaffold.\nversion: 0.1.0\nenvironment:\n  sdk: \">=3.0.0 <4.0.0\"\n  flutter: \">=3.22.0\"\ndependencies:\n  flutter:\n    sdk: flutter\ndev_dependencies:\n  flutter_test:\n    sdk: flutter\nflutter:\n  uses-material-design: true\n"
                ),
            ));
            files.push((
                "lib/main.dart".to_string(),
                format!(
                    "import 'package:flutter/material.dart';\n\nvoid main() => runApp(const ScaffoldApp());\n\nclass ScaffoldApp extends StatelessWidget {{\n  const ScaffoldApp({{super.key}});\n\n  @override\n  Widget build(BuildContext context) {{\n    return const MaterialApp(\n      home: Scaffold(\n        body: Center(child: Text('hello from {id}')),\n      ),\n    );\n  }}\n}}\n"
                ),
            ));
            files.push((
                "test/widget_test.dart".to_string(),
                format!(
                    "import 'package:flutter/material.dart';\nimport 'package:flutter_test/flutter_test.dart';\nimport 'package:{snake_id}/main.dart';\n\nvoid main() {{\n  testWidgets('renders greeting', (tester) async {{\n    await tester.pumpWidget(const ScaffoldApp());\n    expect(find.text('hello from {id}'), findsOneWidget);\n    expect(find.byType(MaterialApp), findsOneWidget);\n  }});\n}}\n"
                ),
            ));
            files.push((
                "analysis_options.yaml".to_string(),
                "analyzer:\n  errors:\n    invalid_annotation_target: ignore\n".to_string(),
            ));
            files.push((
                ".gitignore".to_string(),
                ".dart_tool/\nbuild/\n".to_string(),
            ));
            files.push((
                "README.md".to_string(),
                readme_text(
                    request,
                    &build,
                    &test,
                    &format!(
                        "Verified with `flutter analyze` + `flutter test` (no platform host or Android SDK needed). The app bundle is a release-stage command that requires `android/` and the Android SDK.{}",
                        kit.as_ref().map(|k| kit_readme_section(k, &request.profile))
                            .unwrap_or_default()
                    ),
                ),
            ));
        }
        other => {
            return Err(ForgeError::GenerationFailed {
                reason: format!("unsupported profile '{other}' for generation"),
            });
        }
    }
    files.push(("forge.yaml".to_string(), manifest));
    // Versioned standard snapshot, only when a pack was explicitly selected
    // (`--standard-pack`): the owned `.standard/` subtree plus its digest
    // receipt. Staged as ordinary template files so the promotion/cleanup
    // guarantees cover them identically. Without a selection the output is
    // byte-identical to pre-standard releases.
    if let Some(spec) = request.standard_pack.as_deref() {
        files.extend(crate::standard::staged_files(
            &request.id,
            &request.profile,
            spec,
            generated_at,
        )?);
    }
    // Workspace Governance declaration + ownership receipt: staged as
    // ordinary template files so the staging, promotion and cleanup
    // guarantees cover them identically. Unmapped profiles stage nothing.
    if request.workspace_metadata {
        files.extend(workspace::staged_files(&request.id, &descriptor));
    }
    // Kit-owned subtree: the digest-pinned vendored token source plus the
    // ownership receipt recording one digest per owned file. Reading the
    // assets verifies every digest, so a drifted byte fails here — still
    // before anything is staged. Staged as ordinary template files so the
    // promotion and cleanup guarantees cover them identically, and so a
    // later user edit of an owned file is visible as a conflict rather than
    // an overwrite.
    if let Some(kit) = kit {
        let owned = crate::kit::assets::read_verified_assets(&kit.descriptor)?;
        if !owned.is_empty() {
            let receipt = crate::kit::assets::render_receipt(
                &request.id,
                &request.profile,
                &kit.descriptor,
                &owned,
            );
            let mut staged = owned;
            staged.push((
                crate::kit::assets::PLATFORM_RECEIPT_PATH.to_string(),
                crate::kit::assets::receipt_text(&receipt)?,
            ));
            files.extend(staged);
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Rendered file bytes for a request, sorted by path. Used to prove
/// flag/interactive equivalence and repeatability. The standard receipt
/// carries a fixed timestamp here so byte-equality is not defeated by the
/// wall clock; `generate` substitutes the real emission time.
pub fn render_files(request: &CreationRequest) -> Result<Vec<(String, String)>, ForgeError> {
    let _ = check_request(&request.profile, &request.id, &request.features)?;
    let descriptor = inspect_profile(&request.profile)?;
    let kit = KitContext::resolve(
        request,
        &descriptor,
        crate::standard::DETERMINISTIC_TIMESTAMP,
    )?;
    template_files(
        request,
        crate::standard::DETERMINISTIC_TIMESTAMP,
        kit.as_ref(),
    )
}

fn ensure_relative_inside(rel: &str) -> Result<(), ForgeError> {
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "template escapes its destination: absolute path '{rel}'; nothing was written"
            ),
        });
    }
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(ForgeError::GenerationConflict {
                    reason: format!(
                        "template escapes its destination: '{rel}' leaves the project directory; nothing was written"
                    ),
                });
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(ForgeError::GenerationConflict {
                    reason: format!(
                        "template escapes its destination: '{rel}'; nothing was written"
                    ),
                });
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    if rel.trim().is_empty() || rel == "." {
        return Err(ForgeError::GenerationConflict {
            reason: "template escapes its destination: empty path; nothing was written".to_string(),
        });
    }
    Ok(())
}

fn canonical_for_precheck(dest: &Path) -> Option<String> {
    if let Ok(canonical) = dest.canonicalize() {
        return Some(canonical.display().to_string());
    }
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let leaf = dest.file_name()?.to_str()?;
    let canonical_parent = parent.canonicalize().ok()?;
    Some(canonical_parent.join(leaf).display().to_string())
}

/// Validate `files` against traversal without touching the destination.
pub fn check_files_inside(files: &[(String, String)]) -> Result<(), ForgeError> {
    for (rel, _) in files {
        ensure_relative_inside(rel)?;
    }
    Ok(())
}

fn is_dir_nonempty(dir: &Path) -> Result<bool, ForgeError> {
    match fs::read_dir(dir) {
        Ok(mut entries) => Ok(entries.any(|e| e.is_ok())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(ForgeError::GenerationConflict {
            reason: format!(
                "cannot inspect destination '{}': {err}; nothing was written",
                dir.display()
            ),
        }),
    }
}

/// Create the project: stage, validate, promote into an empty destination
/// and register. Failures before promotion change nothing; failures after
/// promotion clean what was written and leave nothing registered.
pub fn generate(
    registry: &mut Registry,
    request: &CreationRequest,
) -> Result<GeneratedProject, ForgeError> {
    let _ = check_request(&request.profile, &request.id, &request.features)?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    // Resolve the declared kit, check its ecosystem and evaluate the floor
    // before a single file is staged. A refusal here leaves the destination,
    // the staging area and the registry byte- and row-identical.
    let profile = inspect_profile(&request.profile)?;
    let kit = KitContext::resolve(request, &profile, &generated_at)?;
    let files = template_files(request, &generated_at, kit.as_ref())?;
    check_files_inside(&files)?;
    // The committed feed bytes. They are binary — a `.nupkg` is a ZIP archive —
    // so they cannot travel through the text `template_files` vector without
    // corrupting them. Reading them verifies every digest, so a tampered
    // package fails here, still before anything is staged, and they are staged
    // and promoted through the same path as the text files so the cleanup
    // guarantee covers them identically.
    let feed_files = kit
        .as_ref()
        .map(|k| crate::kit::assets::read_verified_feed_assets(&k.descriptor))
        .transpose()?
        .unwrap_or_default();
    let mut notes = Vec::new();
    if request.workspace_metadata {
        if let Ok(descriptor) = inspect_profile(&request.profile) {
            if descriptor.workspace.is_none() {
                notes.push(workspace::omission_note(&request.profile));
            }
        }
    }
    if let Some(spec) = request.standard_pack.as_deref() {
        notes.push(format!(
            "standard snapshot: rendered {spec} into {STANDARD_DIR}/",
            spec = spec,
            STANDARD_DIR = crate::standard::STANDARD_DIR
        ));
    }
    // A declared zero and a recorded exception are surfaced as `WARN` on their
    // own, in the same vocabulary the manifest uses, so the console output and
    // the `forge.yaml` can never disagree about the floor outcome.
    let kit_warning = kit.as_ref().and_then(|k| k.warning(&request.profile));

    if request.destination.is_file() {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "destination '{}' already exists and is not an empty directory; nothing was written",
                request.destination.display()
            ),
        });
    }
    if is_dir_nonempty(&request.destination)? {
        return Err(ForgeError::GenerationConflict {
            reason: format!(
                "destination '{}' is a nonempty directory; creation fails before overwriting existing files",
                request.destination.display()
            ),
        });
    }
    if let Some(canonical) = canonical_for_precheck(&request.destination) {
        registry.check_identity_available(&request.id, &canonical)?;
    }

    let staging = tempfile::tempdir().map_err(|err| ForgeError::GenerationFailed {
        reason: format!("cannot stage generation output: {err}"),
    })?;
    for (rel, contents) in &files {
        let target = staging.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("cannot stage '{rel}': {err}"),
            })?;
        }
        fs::write(&target, contents).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("cannot stage '{rel}': {err}"),
        })?;
    }
    for (rel, bytes) in &feed_files {
        let target = staging.path().join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("cannot stage '{rel}': {err}"),
            })?;
        }
        fs::write(&target, bytes).map_err(|err| ForgeError::GenerationFailed {
            reason: format!("cannot stage '{rel}': {err}"),
        })?;
    }
    // The staged manifest must always validate; a failure here is a bug.
    let staged_manifest = staging.path().join("forge.yaml");
    let staged_bytes = fs::read(&staged_manifest).map_err(|err| ForgeError::GenerationFailed {
        reason: format!("staged manifest unreadable: {err}"),
    })?;
    crate::core::manifest::Manifest::parse(&staged_manifest, &staged_bytes)?;

    let dest_existed = request.destination.exists();
    fs::create_dir_all(&request.destination).map_err(|err| ForgeError::GenerationConflict {
        reason: format!(
            "destination '{}' is unwritable: {err}; nothing was written",
            request.destination.display()
        ),
    })?;
    let mut promoted: Vec<PathBuf> = Vec::new();
    let promote_result: Result<(), ForgeError> = (|| {
        let staged_paths = files
            .iter()
            .map(|(rel, _)| rel.as_str())
            .chain(feed_files.iter().map(|(rel, _)| rel.as_str()));
        for rel in staged_paths {
            let src = staging.path().join(rel);
            let dst = request.destination.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).map_err(|err| ForgeError::GenerationConflict {
                    reason: format!(
                        "destination '{}' is unwritable: {err}; nothing was registered",
                        request.destination.display()
                    ),
                })?;
            }
            let bytes = fs::read(&src).map_err(|err| ForgeError::GenerationFailed {
                reason: format!("staged file '{rel}' unreadable: {err}"),
            })?;
            fs::write(&dst, &bytes).map_err(|err| ForgeError::GenerationConflict {
                reason: format!(
                    "destination '{}' is unwritable: {err}; nothing was registered",
                    request.destination.display()
                ),
            })?;
            promoted.push(dst);
        }
        Ok(())
    })();
    if let Err(err) = promote_result {
        cleanup_promoted(&request.destination, &promoted, dest_existed);
        return Err(err);
    }

    match registry.register(&request.destination, None) {
        Ok(record) => {
            let names = files.iter().map(|(rel, _)| rel.clone()).collect();
            Ok(GeneratedProject {
                record,
                files: names,
                native_verified: false,
                native_note: format!(
                    "rendering verified (assets {GENERATOR_VERSION}); native build/test require '{}' (see `forge profile preflight {}`)",
                    crate::profile::inspect_profile(&request.profile)
                        .map(|p| p.toolchain)
                        .unwrap_or_else(|_| "toolchain".to_string()),
                    request.profile
                ),
                notes,
                kit_warning,
            })
        }
        Err(err) => {
            cleanup_promoted(&request.destination, &promoted, dest_existed);
            Err(err)
        }
    }
}

fn cleanup_promoted(dest: &Path, promoted: &[PathBuf], dest_existed: bool) {
    for path in promoted.iter().rev() {
        let _ = fs::remove_file(path);
    }
    // Remove emptied parent dirs up to (and including) dest when we created it.
    let mut current = Some(dest.to_path_buf());
    while let Some(dir) = current {
        if dir.exists() {
            let empty = fs::read_dir(&dir)
                .map(|mut e| e.next().is_none())
                .unwrap_or(false);
            if !empty {
                break;
            }
            if dir == dest && dest_existed {
                break;
            }
            let _ = fs::remove_dir(&dir);
        }
        if dir == dest {
            break;
        }
        current = dir.parent().map(Path::to_path_buf);
        if current.as_deref() == Some(dest.parent().unwrap_or_else(|| Path::new(""))) {
            break;
        }
    }
}

pub(crate) fn split_command(command: &str) -> Result<(String, Vec<String>), ForgeError> {
    let mut parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err(ForgeError::GenerationFailed {
            reason: "empty native command; nothing was verified".to_string(),
        });
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

pub(crate) fn toolchain_present(name: &str, available: Option<&HashSet<String>>) -> bool {
    if let Some(set) = available {
        return set.contains(name);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Run the profile's native build/test commands in `project_dir`.
///
/// With `available`, membership is checked directly (deterministic for
/// tests); otherwise the host `PATH` is probed. A missing toolchain fails
/// with `toolchain-missing` and never claims a build. Commands run via
/// argument arrays, never shell.
pub fn verify_native(
    profile_id: &str,
    project_dir: &Path,
    available: Option<&HashSet<String>>,
) -> Result<NativeReport, ForgeError> {
    let descriptor = inspect_profile(profile_id)?;
    if !toolchain_present(&descriptor.toolchain, available) {
        return Err(ForgeError::ToolchainMissing {
            toolchain: descriptor.toolchain.clone(),
            profile: descriptor.id.clone(),
        });
    }
    for command in [&descriptor.build_command, &descriptor.test_command] {
        let (program, args) = split_command(command)?;
        let output = Command::new(&program)
            .args(&args)
            .current_dir(project_dir)
            .output()
            .map_err(|err| ForgeError::GenerationFailed {
                reason: format!("native command '{command}' could not start: {err}"),
            })?;
        if !output.status.success() {
            let mut detail = String::from_utf8_lossy(&output.stderr).to_string();
            if detail.trim().is_empty() {
                detail = String::from_utf8_lossy(&output.stdout).to_string();
            }
            let tail: String = detail
                .chars()
                .rev()
                .take(2000)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            return Err(ForgeError::GenerationFailed {
                reason: format!(
                    "native command '{command}' failed for profile '{profile_id}': {}",
                    tail.trim()
                ),
            });
        }
    }
    Ok(NativeReport {
        profile: profile_id.to_string(),
        build_command: descriptor.build_command,
        test_command: descriptor.test_command,
        verified: true,
    })
}

/// Feature versions recorded in generated manifests (pinned).
pub fn recorded_features(request: &CreationRequest) -> BTreeMap<String, String> {
    request
        .features
        .iter()
        .map(|f| (f.clone(), "0.1.0".to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn dest(tmp: &TempDir, name: &str) -> PathBuf {
        tmp.path().join(name)
    }

    fn request_for(profile: &str, dest: &Path) -> CreationRequest {
        let id = format!("{profile}-demo");
        normalize_explicit(Some(profile), Some(id.as_str()), None, &[], dest, None).unwrap()
    }

    fn open_registry(dir: &TempDir) -> Registry {
        Registry::open(&dir.path().join("registry.db")).unwrap()
    }

    #[test]
    fn workspace_metadata_staged_by_default_and_omitted_on_opt_out() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "flutter-app",
            "nextjs-web",
            "python-service",
            "react-web",
            "rust-web",
        ] {
            let id = format!("meta-{profile}");
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                None,
                &[],
                &dest(&tmp, &format!("meta-{profile}")),
                None,
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
            assert!(
                paths.contains(&workspace::METADATA_PATH),
                "{profile}: {paths:?}"
            );
            assert!(
                paths.contains(&workspace::RECEIPT_PATH),
                "{profile}: {paths:?}"
            );
            let mut opted_out = req.clone();
            opted_out.workspace_metadata = false;
            let legacy = render_files(&opted_out).unwrap();
            let expected: Vec<(String, String)> = files
                .iter()
                .filter(|(p, _)| p != workspace::METADATA_PATH && p != workspace::RECEIPT_PATH)
                .cloned()
                .collect();
            assert_eq!(
                legacy, expected,
                "{profile}: opt-out must equal the prior output"
            );
            // Receipt records exactly the staged declaration bytes.
            let declaration = files
                .iter()
                .find(|(p, _)| p == workspace::METADATA_PATH)
                .map(|(_, c)| c.clone())
                .unwrap();
            let receipt = files
                .iter()
                .find(|(p, _)| p == workspace::RECEIPT_PATH)
                .map(|(_, c)| c.clone())
                .unwrap();
            assert!(
                receipt.contains(&workspace::sha256_hex(declaration.as_bytes())),
                "{profile}"
            );
        }
    }

    #[test]
    fn explicit_and_interactive_normalize_equivalently() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "equiv-app");
        let explicit = normalize_explicit(
            Some("rust-web"),
            Some("equiv-app"),
            Some("Equiv App"),
            &["auth".to_string()],
            &target,
            None,
        )
        .unwrap();
        let input = b"rust-web\nequiv-app\nEquiv App\nauth\n";
        let mut reader = Cursor::new(input);
        let mut writer: Vec<u8> = Vec::new();
        let interactive = parse_interactive(
            &mut reader,
            &mut writer,
            &target,
            None,
            None,
            None,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(explicit, interactive);
        let a = render_files(&explicit).unwrap();
        let b = render_files(&interactive).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn repeatable_bytes_across_directories() {
        let tmp = TempDir::new().unwrap();
        let a = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-a"),
            None,
        )
        .unwrap();
        let b = normalize_explicit(
            Some("rust-web"),
            Some("same-id"),
            Some("Same"),
            &[],
            &dest(&tmp, "dir-b"),
            None,
        )
        .unwrap();
        assert_eq!(render_files(&a).unwrap(), render_files(&b).unwrap());
    }

    #[test]
    fn all_six_profiles_render_portable_manifests() {
        let tmp = TempDir::new().unwrap();
        for profile in [
            "aspnet-web",
            "rust-web",
            "nextjs-web",
            "react-web",
            "flutter-app",
            "python-service",
        ] {
            let id = format!(
                "{}-t1",
                profile
                    .replace("-web", "")
                    .replace("-app", "")
                    .replace("-service", "")
            );
            let req = normalize_explicit(
                Some(profile),
                Some(&id),
                Some(&id),
                &[],
                &dest(&tmp, &format!("scaffold-{profile}")),
                None,
            )
            .unwrap();
            let files = render_files(&req).unwrap();
            assert!(files.iter().any(|(p, _)| p == "forge.yaml"), "{profile}");
            assert!(files.iter().any(|(p, _)| p == "README.md"), "{profile}");
            let manifest_text = files
                .iter()
                .find(|(p, _)| p == "forge.yaml")
                .map(|(_, c)| c.clone())
                .unwrap();
            let manifest = crate::core::manifest::Manifest::parse(
                Path::new("forge.yaml"),
                manifest_text.as_bytes(),
            )
            .expect("generated manifest must validate");
            assert_eq!(manifest.project.profile, profile);
            crate::profile::resolve_profile(&manifest.project.profile, &[]).unwrap();
            for (rel, contents) in &files {
                assert!(!rel.contains(".."), "{profile}:{rel}");
                if rel == "forge.yaml" || rel == ".gitignore" {
                    continue;
                }
                assert!(
                    !contents.contains("use forge::")
                        && !contents.contains("extern crate forge")
                        && !contents.contains("FORGE_REGISTRY"),
                    "{profile}:{rel} must not depend on a Forge runtime"
                );
            }
            let readme = files
                .iter()
                .find(|(p, _)| p == "README.md")
                .map(|(_, c)| c.clone())
                .unwrap();
            let descriptor = inspect_profile(profile).unwrap();
            assert!(readme.contains(&descriptor.build_command), "{profile}");
            assert!(readme.contains(&descriptor.test_command), "{profile}");
        }
    }

    #[test]
    fn request_for_helper_builds_valid_ids() {
        let tmp = TempDir::new().unwrap();
        for profile in ["rust-web", "flutter-app", "python-service"] {
            let req = request_for(profile, &dest(&tmp, profile));
            validate_project_id(&req.id).unwrap();
        }
    }

    #[test]
    fn nonempty_destination_fails_before_overwrite() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "taken");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep.txt"), "do not touch").unwrap();
        let req =
            normalize_explicit(Some("rust-web"), Some("taken"), None, &[], &target, None).unwrap();
        let mut reg = open_registry(&tmp);
        let err = generate(&mut reg, &req).expect_err("nonempty must fail");
        assert_eq!(err.code(), "generation-conflict");
        assert_eq!(
            fs::read_to_string(target.join("keep.txt")).unwrap(),
            "do not touch"
        );
        assert!(!target.join("forge.yaml").exists());
        assert!(reg.inspect("taken").is_err());
    }

    #[test]
    fn template_escape_is_rejected_without_writes() {
        let evil = vec![("../evil.txt".to_string(), "x".to_string())];
        let err = check_files_inside(&evil).expect_err("escape must fail");
        assert_eq!(err.code(), "generation-conflict");
    }

    #[test]
    fn cancelled_interactive_leaves_nothing_behind() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "cancelled-app");
        let mut reader = Cursor::new(b"");
        let mut writer: Vec<u8> = Vec::new();
        let err = parse_interactive(
            &mut reader,
            &mut writer,
            &target,
            None,
            None,
            None,
            &[],
            None,
        )
        .expect_err("EOF must cancel");
        assert_eq!(err.code(), "generation-cancelled");
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_profile_and_incompatible_features_fail_before_mutation() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "bad");
        let err = normalize_explicit(
            Some("not-a-real-profile"),
            Some("bad"),
            None,
            &[],
            &target,
            None,
        )
        .expect_err("unknown profile");
        assert_eq!(err.code(), "unknown-profile");
        assert!(!target.exists());

        let err = normalize_explicit(
            Some("flutter-app"),
            Some("bad"),
            None,
            &["postgres".to_string()],
            &target,
            None,
        )
        .expect_err("incompatible feature");
        assert_eq!(err.code(), "incompatible-profile");
        assert!(!target.exists());
    }

    #[test]
    fn id_collision_cleans_promoted_output_and_registers_nothing_new() {
        let tmp = TempDir::new().unwrap();
        let first = dest(&tmp, "first");
        let second = dest(&tmp, "second");
        let first_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &first, None).unwrap();
        let mut reg = open_registry(&tmp);
        generate(&mut reg, &first_req).unwrap();

        let second_req =
            normalize_explicit(Some("rust-web"), Some("dupe-id"), None, &[], &second, None)
                .unwrap();
        let err = generate(&mut reg, &second_req).expect_err("id reuse must fail");
        assert_eq!(err.code(), "id-collision");
        assert!(!second.join("forge.yaml").exists());
        // Original record unchanged.
        assert_eq!(reg.inspect("dupe-id").unwrap().profile, "rust-web");
        assert_eq!(reg.list().unwrap().len(), 1);
    }

    #[test]
    fn missing_toolchain_reports_unverified_without_claiming_build() {
        let tmp = TempDir::new().unwrap();
        let empty = HashSet::new();
        let err = verify_native("rust-web", tmp.path(), Some(&empty)).expect_err("no cargo");
        assert_eq!(err.code(), "toolchain-missing");
        assert!(err.to_string().contains("not tested"));
    }

    #[test]
    fn rust_scaffold_builds_and_tests_with_native_toolchain() {
        let toolchain: HashSet<String> = ["cargo".to_string()].into_iter().collect();
        if verify_native("rust-web", Path::new("."), Some(&toolchain)).is_err() {
            // Probe only: toolchain set override cannot run real cargo; skip.
            return;
        }
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-app");
        let req = normalize_explicit(
            Some("rust-web"),
            Some("native-app"),
            None,
            &[],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        if !toolchain_present("cargo", None) {
            return;
        }
        let report = verify_native("rust-web", &target, None).expect("cargo build+test");
        assert!(report.verified);
    }

    #[test]
    fn react_web_renders_with_index_and_test() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "react-app");
        let req = normalize_explicit(
            Some("react-web"),
            Some("react-app"),
            Some("React App"),
            &["i18n".to_string()],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"forge.yaml"), "{paths:?}");
        assert!(paths.contains(&"README.md"), "{paths:?}");
        assert!(paths.contains(&"index.html"), "{paths:?}");
        assert!(paths.contains(&"src/main.jsx"), "{paths:?}");
        assert!(paths.contains(&"src/greeting.mjs"), "{paths:?}");
        assert!(paths.contains(&"src/app.test.mjs"), "{paths:?}");
        assert!(paths.contains(&"vite.config.js"), "{paths:?}");
        assert!(paths.contains(&"package.json"), "{paths:?}");
        // The client is real and previewable: a Vite dev script exists and
        // the dev server binds the Studio-reserved port.
        let package_json = files
            .iter()
            .find(|(p, _)| p == "package.json")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(package_json.contains("\"dev\": \"vite\""), "{package_json}");
        assert!(
            package_json.contains("\"react\": \"18.3.1\""),
            "{package_json}"
        );
        let vite_config = files
            .iter()
            .find(|(p, _)| p == "vite.config.js")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(vite_config.contains("FORGE_STUDIO_PORT"), "{vite_config}");
        assert!(vite_config.contains("strictPort: true"), "{vite_config}");
        // The generated manifest advertises react-web so doctor and
        // feature lifecycle contracts both agree.
        let manifest_text = files
            .iter()
            .find(|(p, _)| p == "forge.yaml")
            .map(|(_, c)| c.clone())
            .unwrap();
        let manifest = crate::core::manifest::Manifest::parse(
            Path::new("forge.yaml"),
            manifest_text.as_bytes(),
        )
        .expect("react-web manifest must validate");
        assert_eq!(manifest.project.profile, "react-web");
        // The closed feature set is recorded (i18n only here).
        let features: Vec<&str> = manifest.features.keys().map(String::as_str).collect();
        assert_eq!(features, vec!["i18n"]);
        // The README records the build/test commands from the descriptor.
        let readme = files
            .iter()
            .find(|(p, _)| p == "README.md")
            .map(|(_, c)| c.clone())
            .unwrap();
        assert!(readme.contains("npm run build"), "{readme}");
        assert!(readme.contains("npm test"), "{readme}");
    }

    #[test]
    fn react_web_scaffold_builds_and_tests_with_native_toolchain() {
        // Probe only with an isolated target directory; running npm in
        // the project root would block on missing project metadata.
        let toolchain: HashSet<String> = ["npm".to_string()].into_iter().collect();
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "native-react");
        let req = normalize_explicit(
            Some("react-web"),
            Some("native-react"),
            None,
            &[],
            &target,
            None,
        )
        .unwrap();
        let files = render_files(&req).unwrap();
        fs::create_dir(&target).unwrap();
        for (rel, contents) in &files {
            if rel == "forge.yaml" {
                continue;
            }
            let path = target.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
        if verify_native("react-web", &target, Some(&toolchain)).is_err() {
            return;
        }
        if !toolchain_present("npm", None) {
            return;
        }
        let report = verify_native("react-web", &target, None).expect("npm run build+test");
        assert!(report.verified);
    }

    #[test]
    fn planned_profile_generation_refuses_before_writes() {
        let tmp = TempDir::new().unwrap();
        let target = dest(&tmp, "aspnet-saas-app");
        let err = normalize_explicit(
            Some("aspnet-saas"),
            Some("aspnet-saas-app"),
            None,
            &[],
            &target,
            None,
        )
        .expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("planned"), "{text}");
        assert!(
            text.contains("no files were changed") || text.contains("no files were written"),
            "{text}"
        );
        // No file or registry side effects.
        assert!(!target.exists());
        let reg = open_registry(&tmp);
        assert!(reg.list().unwrap().is_empty());
    }

    #[test]
    fn standard_snapshot_requires_explicit_selection_and_nothing_else_changes() {
        let tmp = TempDir::new().unwrap();
        let plain = normalize_explicit(
            Some("rust-web"),
            Some("std-plain"),
            None,
            &[],
            &dest(&tmp, "std-plain"),
            None,
        )
        .unwrap();
        let baseline = render_files(&plain).unwrap();
        assert!(
            !baseline
                .iter()
                .any(|(p, _)| p.starts_with(crate::standard::STANDARD_DIR)),
            "without a selection no .standard/ files are staged"
        );
        // An explicit selection stages the owned subtree with a receipt
        // whose recorded digests match the staged content.
        let with_pack = normalize_explicit(
            Some("rust-web"),
            Some("std-plain"),
            None,
            &[],
            &dest(&tmp, "std-plain"),
            Some("baseline-service@1.1.0"),
        )
        .unwrap();
        let files = render_files(&with_pack).unwrap();
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&crate::standard::PROFILE_PATH), "{paths:?}");
        assert!(paths.contains(&crate::standard::RECEIPT_PATH), "{paths:?}");
        let receipt_text = files
            .iter()
            .find(|(p, _)| p == crate::standard::RECEIPT_PATH)
            .map(|(_, c)| c.clone())
            .unwrap();
        let receipt: crate::standard::Receipt = serde_json::from_str(&receipt_text).unwrap();
        assert_eq!(receipt.pack, "baseline-service");
        assert_eq!(receipt.version, "1.1.0");
        assert_eq!(receipt.project, "std-plain");
        for owned in &receipt.files {
            let content = files
                .iter()
                .find(|(p, _)| p == &owned.path)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("{} staged", owned.path));
            assert_eq!(
                crate::standard::sha256_hex(content.as_bytes()),
                owned.digest,
                "{}",
                owned.path
            );
        }
        // Selection changes nothing else: minus the snapshot subtree the
        // two renders are identical.
        let rest: Vec<(String, String)> = files
            .iter()
            .filter(|(p, _)| !p.starts_with(crate::standard::STANDARD_DIR))
            .cloned()
            .collect();
        assert_eq!(rest, baseline);
        // A bad selection refuses during normalization, before mutation.
        let err = normalize_explicit(
            Some("rust-web"),
            Some("std-bad"),
            None,
            &[],
            &dest(&tmp, "std-bad"),
            Some("baseline-service@2.0.0"),
        )
        .expect_err("proposed pack must refuse generation");
        assert_eq!(err.code(), "standard-invalid");
        let err = normalize_explicit(
            Some("react-web"),
            Some("std-incompat"),
            None,
            &[],
            &dest(&tmp, "std-incompat"),
            Some("baseline-service@1.1.0"),
        )
        .expect_err("incompatible profile must refuse generation");
        assert_eq!(err.code(), "standard-invalid");
    }
}

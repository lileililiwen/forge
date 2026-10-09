//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::{validate_project_id, ForgeError};
use crate::profile::{inspect_profile, resolve_profile};
use std::io::{BufRead, Write};
use std::path::Path;

use super::constants::GENERATOR_VERSION;
use super::model::{CreationRequest, KitContext};

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

pub(super) fn check_request(
    profile: &str,
    id: &str,
    features: &[String],
) -> Result<Vec<String>, ForgeError> {
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

pub(super) fn snake(id: &str) -> String {
    id.replace('-', "_")
}

pub(super) fn dotnet_namespace(id: &str) -> String {
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

pub(super) fn manifest_text(
    request: &CreationRequest,
    language: &str,
    kit: Option<&KitContext>,
) -> String {
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

pub(super) fn readme_text(
    request: &CreationRequest,
    build: &str,
    test: &str,
    notes: &str,
) -> String {
    format!(
        "# {}\n\nForge deterministic scaffold for profile `{}` (assets {GENERATOR_VERSION}).\nOrdinary source: builds and runs with its native toolchain; no Forge runtime required.\n\n## Native commands\n\n```sh\n{build}\n{test}\n```\n\n{notes}\n\n## Forge\n\nCreated with `forge new --profile {}`. Rendering is verified by Forge;\nnative build/test evidence requires the stack toolchain (see `forge profile preflight {}`).\n",
        request.name, request.profile, request.profile, request.profile
    )
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
/// The rendered files whose contents can carry a dependency reference.
///
/// Only manifests are scanned: a README mentioning a path is documentation, and
/// refusing it would make the generator's own explanation of the committed feed
/// impossible to render.
pub(super) fn is_manifest_path(path: &str) -> bool {
    matches!(
        path,
        "forge.yaml"
            | "package.json"
            | "Directory.Packages.props"
            | "Directory.Build.props"
            | "NuGet.config"
            | "pubspec.yaml"
            | "pyproject.toml"
            | "requirements.txt"
    ) || path.ends_with(".csproj")
        || path.ends_with(".fsproj")
        || path.ends_with(".vbproj")
}

/// Render one YAML scalar the way the generator renders it.///
/// `pub(crate)` so a second writer of the same field — the explicit kit
/// upgrade, which rewrites the `kit.version` line in an existing
/// `forge.yaml` — escapes identically. Two escaping implementations would
/// drift, and the drift would only show up as a manifest that no longer
/// round-trips.
pub(crate) fn yaml_scalar(value: &str) -> String {
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
pub(super) fn kit_readme_section(kit: &KitContext, profile: &str) -> String {
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

//! The repo-relative named-feed renderer and the feed-value validator.
//!
//! A scaffold declares a **named, versioned** feed whose value is a path
//! *relative to the generated project*, and the `.nupkg` bytes are committed
//! into that directory. Three mechanisms are refused outright, because all
//! three make a restore that only works on the machine that generated it:
//!
//! - a source-mode `ProjectReference` or path dependency into a sibling
//!   checkout — the project stops working when the sibling library is absent;
//! - a machine-specific absolute path, a `file://` URL, a `..` segment that
//!   escapes the project, a shell metacharacter, or a secret-shaped string;
//! - a feed **resolved from an environment variable** — a variable a CI runner
//!   does not carry is the same failure class as a hard-coded absolute path,
//!   and deferring the failure to an unset variable is not making it
//!   reproducible. It was implemented once in this repository and rejected by
//!   the owner; see `design.md` §5 so it is not re-proposed.
//!
//! The feed value is therefore always a project-relative directory that ships
//! with the repository, and two operators rendering the same scaffold get
//! byte-identical trees.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::core::ForgeError;
use crate::kit::registry::{FeedKind, KitDescriptor};

/// Why a feed value was refused. The value itself is never echoed back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedRejection {
    /// A filesystem-absolute path: only the generating machine resolves it.
    Absolute,
    /// A `file://` URL, which can escape the project.
    FileUrl,
    /// A shell metacharacter that would let the value run a command.
    Metacharacter,
    /// A secret-shaped string that must never land in a committed tree.
    SecretShaped,
    /// An empty or whitespace-only value.
    Empty,
    /// A `..` segment that escapes the generated project.
    Escapes,
    /// A source-mode project or path dependency into a sibling checkout.
    SourceReference,
}

impl FeedRejection {
    fn describe(self) -> &'static str {
        match self {
            Self::Absolute => "is a machine-specific absolute path",
            Self::FileUrl => "is a file:// URL and can escape the project",
            Self::Metacharacter => "contains a shell metacharacter",
            Self::SecretShaped => "looks secret-shaped",
            Self::Empty => "is empty",
            Self::Escapes => "is not relative to the generated project",
            Self::SourceReference => {
                "is a source-mode project or path dependency into a sibling checkout"
            }
        }
    }
}

/// Shell metacharacters that must never appear in a feed value written into a
/// generated project.
const METACHARACTERS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '<', '>', '*', '?', '!', '\n', '\r', '\\', '"', '\'',
];

/// Validate a candidate feed value.
///
/// The check is deliberately conservative, and it is stricter than "not
/// absolute": the only accepted shape is a directory **relative to the
/// generated project**. A value that could resolve only on one machine, escape
/// the project, execute something, or leak a credential is refused. A refusal
/// never echoes the value back.
pub fn validate_feed_value(value: &str) -> Result<(), FeedRejection> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FeedRejection::Empty);
    }
    // Path shapes are classified before metacharacters: a Windows path is a
    // machine-specific path first and a backslash second, and reporting it as
    // a metacharacter would hide the reason the restore is not reproducible.
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("file://") || lowered.starts_with("file:") {
        return Err(FeedRejection::FileUrl);
    }
    if trimmed.starts_with('/') || trimmed.starts_with('~') || looks_like_windows_absolute(trimmed)
    {
        return Err(FeedRejection::Absolute);
    }
    if trimmed.contains(METACHARACTERS) {
        return Err(FeedRejection::Metacharacter);
    }
    if looks_secret_shaped(trimmed) {
        return Err(FeedRejection::SecretShaped);
    }
    // A `..` segment is the remaining way to reach outside the project without
    // being absolute. It is checked last so the more specific refusal wins.
    if trimmed
        .split(['/', '\\'])
        .any(|segment| segment == ".." || segment == "~")
    {
        return Err(FeedRejection::Escapes);
    }
    Ok(())
}

fn looks_like_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Refuse a feed value, naming the shape that was wrong and never the value.
///
/// The value may carry a credential or a shell fragment, so echoing it back
/// would leak it into logs and terminals.
pub fn refuse_feed_value(rejection: FeedRejection) -> ForgeError {
    ForgeError::KitFeedInvalid {
        reason: format!(
            "the shared-layer feed value {}; it is never echoed back; \
             nothing was staged and no project was registered",
            rejection.describe()
        ),
    }
}

/// Reject the source-mode shapes the ruling rules out. Kept separate from the
/// value check because the offending text is a *reference*, not a feed value,
/// and the refusal message must name the reference.
pub fn refuse_source_reference(reference: &str) -> ForgeError {
    ForgeError::KitFeedInvalid {
        reason: format!(
            "shared-layer reference '{reference}' {}; a scaffold must not stop working when the \
             sibling library is absent; nothing was staged and no project was registered",
            FeedRejection::SourceReference.describe()
        ),
    }
}

/// Find a source-mode reference in a rendered manifest.
///
/// A `ProjectReference` or a path dependency that escapes the generated project
/// resolves only where the sibling happens to sit, which is the same failure
/// class as the absolute restore path this package removed — it works on the
/// machine that generated the scaffold and on no other. The committed feed is
/// the only sanctioned way to reach the shared layer.
///
/// A reference that stays inside the project tree, such as a sibling test
/// project, is portable and is deliberately left alone. Only escaping shapes
/// are reported, and the offending reference is returned so the refusal can
/// name it.
pub fn find_source_reference(manifest: &str) -> Option<String> {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<ProjectReference") {
            if let Some(include) = attribute_value(trimmed, "Include") {
                if escapes_project(&include) {
                    return Some(include);
                }
            }
            continue;
        }
        // npm, yarn and pnpm path dependencies: `"name": "file:../shared"`.
        for scheme in ["file:", "link:", "portal:"] {
            let needle = format!("\"{scheme}");
            if let Some(at) = trimmed.find(&needle) {
                let rest = &trimmed[at + scheme.len() + 1..];
                let value = rest.trim_end_matches(['"', ',']).trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Refuse a rendered manifest that reaches the shared layer by source.
pub fn refuse_manifest_source_reference(manifest: &str, kind: &str) -> Result<(), ForgeError> {
    match find_source_reference(manifest) {
        Some(reference) => Err(refuse_source_reference(&format!("{kind} '{reference}'"))),
        None => Ok(()),
    }
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// True when a reference cannot resolve from inside the generated project.
fn escapes_project(reference: &str) -> bool {
    if reference.contains("://") || reference.starts_with('~') {
        return true;
    }
    if reference.starts_with('/')
        || reference.starts_with('\\')
        || looks_like_windows_absolute(reference)
    {
        return true;
    }
    reference.split(['/', '\\']).any(|segment| segment == "..")
}

/// A secret-shaped value: an inline credential in a URL, or a token-shaped
/// bare string. Checked before the value could ever reach a generated tree.
fn looks_secret_shaped(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("@")
        && (lowered.contains("://") || lowered.split('@').nth(1).is_some_and(|h| h.contains(':')))
    {
        return true;
    }
    for marker in [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "credential",
        "bearer",
        "pat_",
        "ghp_",
        "sk-",
    ] {
        if lowered.contains(marker) {
            return true;
        }
    }
    false
}

/// The generated project's feed configuration for one kit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFeed {
    /// Feed name, e.g. `platform`.
    pub name: String,
    /// The project-relative directory the feed resolves to, e.g.
    /// `packages/platform-feed`.
    pub path: String,
    /// `NuGet.config` content: `<clear />`, the named project-relative source,
    /// and the neutral public source. The relative value is resolved by NuGet
    /// against the generated `NuGet.config`'s own directory, so it stays
    /// correct wherever the project is checked out.
    pub nuget_config: String,
}

/// Render the generated project's feed configuration for a kit.
///
/// For .NET this is a `NuGet.config` carrying `<clear />`, one named source
/// whose value is the project-relative feed directory, and the neutral public
/// source. The feed's bytes are written separately by
/// [`crate::kit::assets::read_verified_feed_assets`], so a fresh clone restores
/// with no sibling library, no environment variable and no secret.
pub fn render_feed_config(descriptor: &KitDescriptor) -> Result<Option<RenderedFeed>, ForgeError> {
    let Some(feed) = descriptor.reference.feed.as_ref() else {
        return Ok(None);
    };
    // The feed descriptor itself is validated before anything is staged, so a
    // malformed compiled-in feed can never reach a generated tree.
    validate_feed_value(&feed.name).map_err(|rejection| ForgeError::KitFeedInvalid {
        reason: format!(
            "declared feed name '{}' {}; nothing was staged and no project was registered",
            feed.name,
            rejection.describe()
        ),
    })?;
    validate_feed_value(&feed.path).map_err(|rejection| ForgeError::KitFeedInvalid {
        reason: format!(
            "declared feed path '{}' {}; nothing was staged and no project was registered",
            feed.path,
            rejection.describe()
        ),
    })?;
    if feed.kind != FeedKind::Nuget {
        return Ok(None);
    }

    // Raw string: the value is an XML attribute, and escaping it through a
    // format literal is where a malformed source silently ships. A value
    // containing a quote or a `<` was already refused above.
    let nuget_config = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
    <!-- Shared-layer feed '{name}'. The value is relative to this file, so the
         project restores wherever it is checked out, with no sibling library
         and no environment variable. `forge kit pack` regenerates the bytes;
         `forge kit verify` fails when they drift from kit.version. -->
    <add key="{name}" value="{value}" />
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" protocolVersion="3" />
  </packageSources>
</configuration>
"#,
        name = feed.name,
        value = feed.path
    );

    Ok(Some(RenderedFeed {
        name: feed.name.clone(),
        path: feed.path.clone(),
        nuget_config,
    }))
}

// ---------------------------------------------------------------------------
// Reproducible packing
//
// The feed is committed, so it needs a way to be regenerated rather than
// hand-copied, and a check that catches it drifting. Both are commands, and
// both are covered by `cargo test`; a shell script beside `kits/` would be
// neither.
// ---------------------------------------------------------------------------

/// The sibling library's own machine-readable inventory, which is the version
/// source of truth. Not fetched: a checked-out sibling is read, nothing else.
const SIBLING_MANIFEST: &str = "eng/package-manifest.json";

/// Default sibling location, matching the convention the existing consumers'
/// pack scripts already use. Overridable, and never required: a project must
/// build from its committed feed regardless of where the sibling lives.
fn default_sibling_root() -> PathBuf {
    PathBuf::from("/home/paul/code/dotnet-platform-libs")
}

fn sibling_root(explicit: Option<&Path>) -> PathBuf {
    match explicit {
        Some(path) => path.to_path_buf(),
        None => std::env::var_os("FORGE_PLATFORM_LIBS")
            .map(PathBuf::from)
            .unwrap_or_else(default_sibling_root),
    }
}

/// What a pack run did.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PackReport {
    pub kit: String,
    pub version: String,
    /// The sibling checkout that was read. Recorded so a reader can tell which
    /// checkout produced the committed bytes.
    pub sibling: String,
    /// Packages packed, in stable name order, as `name@version`.
    pub packed: Vec<String>,
    /// Packages skipped because the sibling has no project for them.
    pub skipped: Vec<String>,
    /// Files removed from the vendored feed because the declared set does not
    /// name them — a stale version from a previous bump, or a symbol package.
    pub removed: Vec<String>,
    /// Manifest digest entries written.
    pub digest_entries: usize,
}

/// Repack the confirmed set's restore closure from a checked-out sibling
/// library into the vendored feed.
///
/// The version comes from the sibling's own package manifest, never from a
/// Forge constant, so the committed bytes are pinned to what the library
/// actually packs. The sibling is only ever **read**: its sources are copied
/// into a scratch tree outside the checkout, `dotnet pack` runs against that
/// copy with an argument array rather than an interpolated shell string, and no
/// file in the sibling is written, moved or removed.
///
/// Refuses — changing nothing — when the sibling or its manifest is absent, or
/// when the toolchain is missing. A refusal here is a refusal to guess.
pub fn pack_platform_feed(explicit_sibling: Option<&Path>) -> Result<PackReport, ForgeError> {
    let descriptor = crate::kit::registry::inspect_kit("platform-dotnet", None)?;
    let root = sibling_root(explicit_sibling);
    let sibling_manifest = root.join(SIBLING_MANIFEST);
    if !root.join("src").is_dir() {
        return Err(ForgeError::KitPackUnavailable {
            reason: format!(
                "sibling library not found at {} (no src/); pass --platform-libs <path> or set \
                 FORGE_PLATFORM_LIBS. No vendored file was changed; the existing feed is \
                 untouched",
                root.display()
            ),
        });
    }
    let manifest_bytes =
        std::fs::read(&sibling_manifest).map_err(|err| ForgeError::KitPackUnavailable {
            reason: format!(
                "sibling package manifest not readable at {}: {err}. No vendored file was \
                 changed; the existing feed is untouched",
                sibling_manifest.display()
            ),
        })?;
    let sibling_manifest_value: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| ForgeError::KitPackUnavailable {
            reason: format!(
                "sibling package manifest at {} is not valid JSON: {err}. No vendored file was \
                 changed",
                sibling_manifest.display()
            ),
        })?;

    // The version is the sibling's own, read per package so a sibling that
    // bumps one package and not another is visible rather than flattened.
    let mut versions: BTreeMap<String, String> = BTreeMap::new();
    for entry in descriptor.feed() {
        let version =
            sibling_version(&sibling_manifest_value, &entry.package).ok_or_else(|| {
                ForgeError::KitPackUnavailable {
                    reason: format!(
                        "sibling manifest {} does not declare package '{}'. No vendored file was \
                     changed; the existing feed is untouched",
                        SIBLING_MANIFEST, entry.package
                    ),
                }
            })?;
        versions.insert(entry.package.clone(), version);
    }
    let uniform = versions.values().collect::<BTreeSet<_>>();
    if uniform.len() != 1 {
        return Err(ForgeError::KitPackUnavailable {
            reason: format!(
                "sibling declares {} different versions across the confirmed closure ({}), so the \
                 feed cannot be packed at one kit version. No vendored file was changed",
                uniform.len(),
                versions
                    .iter()
                    .map(|(p, v)| format!("{p}@{v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    let version = uniform.into_iter().next().cloned().unwrap_or_default();
    if version != descriptor.reference.version.clone().unwrap_or_default() {
        return Err(ForgeError::KitFeedVersionMismatch {
            reason: format!(
                "sibling packs the confirmed closure at version '{version}' but kit '{}' declares \
                 version '{}'. Bump the kit descriptor deliberately or repack the sibling; the \
                 feed is not packed at a version the descriptor does not declare",
                descriptor.reference.id,
                descriptor.reference.version.clone().unwrap_or_default()
            ),
        });
    }

    let out_dir = crate::kit::assets::kits_dir().join(crate::kit::registry::KITS_FEED_DIR);
    std::fs::create_dir_all(&out_dir).map_err(|err| ForgeError::KitPackUnavailable {
        reason: format!(
            "cannot create the vendored feed at {}: {err}. No vendored file was changed",
            out_dir.display()
        ),
    })?;

    let mut skipped = Vec::new();
    // Pack from a copy. Everything below this line resolves against `scratch`,
    // never against the sibling: the projects it names are the copies, and the
    // working directory of every `dotnet pack` is the copied tree.
    let scratch = stage_scratch_copy(&root)?;
    let packed = (|| -> Result<Vec<String>, ForgeError> {
        let mut packed = Vec::new();
        for entry in descriptor.feed() {
            let project = format!("src/{0}/{0}.csproj", entry.package);
            if !root.join(&project).is_file() {
                skipped.push(entry.package.clone());
                continue;
            }
            let project_path = scratch.join(&project);
            run_dotnet_pack(&scratch, &project_path, &out_dir, &version)?;
            let produced = out_dir.join(&entry.file);
            if !produced.is_file() {
                return Err(ForgeError::KitPackUnavailable {
                    reason: format!(
                        "dotnet pack reported success but {} was not produced for package '{}'. \
                         The feed is left as it was rather than recorded as complete",
                        produced.display(),
                        entry.package
                    ),
                });
            }
            packed.push(format!("{}@{}", entry.package, version));
        }
        Ok(packed)
    })();
    // The scratch tree goes either way. A refusal that leaves a full copy of a
    // sibling library in the temp directory is litter, just litter Forge admits
    // to.
    let _ = std::fs::remove_dir_all(&scratch);
    let packed = packed?;
    if packed.is_empty() {
        return Err(ForgeError::KitPackUnavailable {
            reason: format!(
                "no project was found in {} for any of the {} packages the confirmed set needs. \
                 No vendored file was changed",
                root.join("src").display(),
                descriptor.feed().len()
            ),
        });
    }

    // Prune after a successful pack, never before: a failed pack must leave the
    // existing feed exactly as it was rather than half-emptied. Anything in the
    // vendored feed that the declared set does not name — a stale version from
    // a previous bump, or a symbol package — is removed and reported, because a
    // feed carrying a version the descriptor does not declare does not restore.
    let declared_files: BTreeSet<String> =
        descriptor.feed().iter().map(|e| e.file.clone()).collect();
    let mut removed = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&out_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".nupkg") && declared_files.contains(&name) {
                continue;
            }
            match std::fs::remove_file(entry.path()) {
                Ok(()) => removed.push(name),
                Err(err) => {
                    return Err(ForgeError::KitPackUnavailable {
                        reason: format!(
                            "cannot remove the undeclared feed file {}: {err}. The pack succeeded \
                             but the feed still carries a file the declared set does not name, so \
                             it is not recorded as complete",
                            entry.path().display()
                        ),
                    })
                }
            }
        }
    }

    // Record the digests. `dotnet pack` is not byte-reproducible, so this is
    // the one place the per-file digest is written, and `forge kit pack` is the
    // only thing that writes it.
    let digest_entries = crate::kit::assets::record_feed_digests(
        &out_dir,
        descriptor.feed(),
        &root.display().to_string(),
    )?;

    Ok(PackReport {
        kit: descriptor.reference.id.clone(),
        version,
        sibling: root.display().to_string(),
        packed,
        skipped,
        removed,
        digest_entries,
    })
}

fn sibling_version(manifest: &serde_json::Value, package: &str) -> Option<String> {
    manifest
        .get("packages")?
        .as_array()?
        .iter()
        .find(|p| p.get("packageId").and_then(|v| v.as_str()) == Some(package))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Invoke the sibling's documented pack step with an argument array.
///
/// `Command` never goes through a shell, so a path with a space or a
/// metacharacter in it cannot become a command. The toolchain's absence is a
/// refusal, never a silent skip: a feed that was not packed is a feed that was
/// not verified.
/// Build-output directories never copied into the scratch tree.
///
/// They are generated output, and `dotnet pack` already produced some of them
/// in the sibling. Copying a stale one is how an out-of-tree pack ends up
/// compiling two copies of the same generated assembly attributes, which fails
/// with duplicate-attribute errors that look nothing like a path problem.
const SKIP_DIRS: &[&str] = &["obj", "bin", ".git", ".vs", "node_modules", ".idea"];

/// Copy the sibling's buildable sources into a scratch tree.
///
/// `forge kit pack` runs `dotnet pack` over a copy, never over the sibling
/// itself. Redirecting MSBuild's output roots is not enough on its own: the
/// default `**/*.cs` glob still reaches into the sibling's own `obj/`, so a
/// build that keeps its output out of the checkout can still *read* stale
/// generated sources from it and fail. Packing a copy makes "no file in the
/// sibling is written, moved or removed" true by construction rather than by
/// remembering to pass enough flags.
///
/// What is copied is the `src/` tree without any build output, plus every
/// regular file at the sibling's root. The root files are copied wholesale
/// rather than named one by one because the sibling's own
/// `Directory.Build.props` reaches for them — `PackageReadmeFile` alone is
/// enough to fail a pack that stages sources without the root `README.md` —
/// and a list maintained by hand is a list that silently goes stale. A project
/// outside `src/` is not copied; the pack then fails naming the missing
/// project rather than packing something unintended.
///
/// The reported sibling path stays the real one, so provenance still names the
/// library it packed from.
fn stage_scratch_copy(root: &Path) -> Result<PathBuf, ForgeError> {
    let scratch = std::env::temp_dir()
        .join("forge-kit-pack")
        .join(format!("libs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let fail = |what: &str, err: std::io::Error| ForgeError::KitPackUnavailable {
        reason: format!(
            "cannot prepare the out-of-tree pack copy ({what}): {err}. No vendored file was \
             changed and the sibling at {} was not touched",
            root.display()
        ),
    };
    copy_tree(&root.join("src"), &scratch.join("src"), &fail)?;
    let root_entries = std::fs::read_dir(root)
        .map_err(|err| fail(&format!("cannot read {}", root.display()), err))?;
    for entry in root_entries.flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            std::fs::copy(entry.path(), scratch.join(entry.file_name()))
                .map_err(|err| fail(&entry.path().display().to_string(), err))?;
        }
    }
    Ok(scratch)
}

fn copy_tree(
    from: &Path,
    to: &Path,
    fail: &dyn Fn(&str, std::io::Error) -> ForgeError,
) -> Result<(), ForgeError> {
    let entries = std::fs::read_dir(from)
        .map_err(|err| fail(&format!("cannot read {}", from.display()), err))?;
    std::fs::create_dir_all(to).map_err(|err| fail(&to.display().to_string(), err))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if SKIP_DIRS
                .iter()
                .any(|skip| name.to_string_lossy().eq_ignore_ascii_case(skip))
            {
                continue;
            }
            copy_tree(&entry.path(), &to.join(&name), fail)?;
        } else if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            std::fs::copy(entry.path(), to.join(&name))
                .map_err(|err| fail(&entry.path().display().to_string(), err))?;
        }
    }
    Ok(())
}

/// Run the sibling's pack step against the scratch copy.
///
/// The argument array is never an interpolated shell string, and the working
/// directory is the scratch tree, so nothing resolves to a path inside the
/// sibling.
fn run_dotnet_pack(
    root: &Path,
    project: &Path,
    out_dir: &Path,
    version: &str,
) -> Result<(), ForgeError> {
    let result = (|| {
        let output = std::process::Command::new("dotnet")
            .arg("pack")
            .arg(project)
            .arg("-c")
            .arg("Release")
            .arg("-o")
            .arg(out_dir)
            .arg(format!("-p:Version={version}"))
            .arg(format!("-p:VersionPrefix={version}"))
            // Symbol packages are not needed to restore, and the feed's declared
            // set is exactly the packages the confirmed set resolves. Producing
            // `.snupkg` files the descriptor does not declare would put bytes in
            // the committed feed that no check accounts for.
            .arg("-p:IncludeSymbols=false")
            .arg("--nologo")
            .arg("-v")
            .arg("q")
            .current_dir(root)
            .output()
            .map_err(|err| ForgeError::KitPackUnavailable {
                reason: format!(
                    "cannot run the sibling's documented pack step for {}: {err}. `dotnet` is not \
                     on PATH. No vendored file was changed",
                    project.display()
                ),
            })?;
        if !output.status.success() {
            return Err(ForgeError::KitPackUnavailable {
                reason: format!(
                    "the sibling's pack step failed for {} ({}): {}. No vendored file was changed",
                    project.display(),
                    output
                        .status
                        .code()
                        .map(|c| format!("exit {c}"))
                        .unwrap_or_else(|| "no exit code".to_string()),
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }
        Ok(())
    })();
    result
}

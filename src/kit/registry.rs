//! The compiled-in shared-layer kit registry and its evidence fixture.
//!
//! A [`KitReference`] is what a profile *declares*: an id, a version, an
//! ecosystem, an optional named feed, an optional target framework and the
//! declared consumption floor. A [`KitDescriptor`] is what the registry
//! *knows* about that `(id, version)`: the confirmed package set, the
//! provisional set with each package's measured external consumer count, the
//! pinned package versions and the digest-pinned vendored assets.
//!
//! The confirmed/provisional split is **derived** from
//! [`PLATFORM_PACKAGE_EVIDENCE`] rather than hand-maintained twice, so a
//! package can never be confirmed in one list and provisional in the other.
//! `tests/kit_contract.rs` fails when the two disagree.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

/// Minimum number of distinct external consumer repositories before a package
/// is treated as confirmed. Mirrors the workspace's own two-consumer rule:
/// Forge counts consumers, not the package's existence.
pub const CONSUMER_BAR: u32 = 2;

/// The measured external-consumer evidence the floor is derived from.
///
/// Frozen fixture: `(package, distinct external consumer repositories)`.
/// Measured by sweeping every `<PackageVersion Include="Platform.*">` and
/// `<PackageReference Include="Platform.*">` outside `dotnet-platform-libs`
/// itself. Re-measuring is a deliberate, reviewable edit — a silent drift here
/// would quietly move the floor.
///
/// Infrastructure-bearing packages are excluded from the confirmed set even
/// when they clear the bar, because a scaffold that references a persistence
/// package without a database is the invented dependency this change exists to
/// prevent. That exclusion is expressed by `PLATFORM_REQUIRES_INFRA`.
pub const PLATFORM_PACKAGE_EVIDENCE: &[(&str, u32)] = &[
    ("Platform.AspNetCore", 6),
    ("Platform.Billing.Contracts", 0),
    ("Platform.Core", 7),
    ("Platform.Eventing", 0),
    ("Platform.FeatureManagement", 0),
    ("Platform.Http.Resilience", 0),
    ("Platform.Idempotency", 2),
    ("Platform.Identity.AspNetCore", 3),
    ("Platform.Observability", 2),
    ("Platform.Persistence.EfCore", 4),
    ("Platform.RateLimiting", 2),
    ("Platform.Tenant.Lifecycle.AspNetCore", 0),
    ("Platform.Testing", 4),
    ("Platform.Testing.AspNetCore", 1),
    ("Platform.UI.Razor", 0),
    ("Platform.Web.Composition", 1),
    ("Platform.Web.Cors", 1),
    ("Platform.Web.OpenApi", 1),
    ("Platform.Web.Resilience", 1),
    ("Platform.Web.Telemetry", 1),
    ("Platform.Web.Versioning", 1),
];

/// Packages that clear or approach the consumer bar but are withheld from the
/// confirmed set because they require infrastructure a scaffold does not have:
/// a database, a broker, a cache or a provider account. The recorded reason
/// is rendered into the generated comment block.
///
/// The reason states **only the infrastructure obstacle**. The consumer count is
/// authoritative in [`PLATFORM_PACKAGE_EVIDENCE`] and is emitted by
/// [`provisional_reason`]; a reason that also carried a count could only
/// duplicate it in the rendered comment or, worse, drift away from the evidence
/// it claims to quote.
pub const PLATFORM_REQUIRES_INFRA: &[(&str, &str)] = &[
    (
        "Platform.Identity.AspNetCore",
        "needs an identity store and provider configuration",
    ),
    (
        "Platform.Persistence.EfCore",
        "needs a database and a migration story",
    ),
    (
        "Platform.Tenant.Lifecycle.AspNetCore",
        "needs a tenant store; also pulls Platform.Tenant.Lifecycle.Contracts",
    ),
];

/// Package ecosystem of a profile's native toolchain. Drives the
/// `kit-ecosystem-mismatch` refusal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Dotnet,
    Npm,
    Cargo,
    Pub,
    /// No registered kit ecosystem for this toolchain.
    None,
}

impl Ecosystem {
    /// The ecosystem a profile toolchain belongs to. An unregistered
    /// toolchain maps to [`Ecosystem::None`] and can therefore only ever
    /// carry a declared zero.
    pub fn for_toolchain(toolchain: &str) -> Self {
        match toolchain {
            "dotnet" => Self::Dotnet,
            // `react-web` pins `npm` and `nextjs-web` pins `node`; both are
            // the Node ecosystem the vendored token kit serves.
            "npm" | "node" => Self::Npm,
            "cargo" => Self::Cargo,
            "flutter" => Self::Pub,
            _ => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dotnet => "dotnet",
            Self::Npm => "npm",
            Self::Cargo => "cargo",
            Self::Pub => "pub",
            Self::None => "none",
        }
    }
}

/// The native package manager a named feed is resolved by.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedKind {
    Nuget,
    Npm,
    Crates,
    Pub,
}

impl FeedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nuget => "nuget",
            Self::Npm => "npm",
            Self::Crates => "crates",
            Self::Pub => "pub",
        }
    }
}

/// A **named** feed whose value is a path **relative to the generated
/// project**. It never carries a machine path, and it never carries an
/// environment variable: both make a restore depend on state that exists in
/// exactly one place, which is the same failure class. The bytes live in the
/// generated tree, so a fresh clone restores wherever it is checked out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedRef {
    /// Stable feed name, e.g. `platform`.
    pub name: String,
    pub kind: FeedKind,
    /// Location inside the generated project, e.g. `packages/platform-feed`.
    /// Always relative: the project must restore at a path it was not
    /// generated at.
    pub path: String,
}

/// One digest-pinned vendored artifact. `path` is the location inside the
/// `kits/` tree; `target` is the path inside the generated project. `binary`
/// marks an artifact that is not UTF-8 text — a `.nupkg` is a ZIP archive — so
/// it is written from bytes rather than from a `String`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRef {
    pub path: String,
    pub sha256: String,
    pub target: String,
    #[serde(default)]
    pub binary: bool,
}

/// What a profile declares. This is the additive, serializable shape that
/// lives on `ProfileDescriptor.kit` and is rendered into the generated
/// `forge.yaml` `kit` block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitReference {
    pub id: String,
    /// The pinned kit version. A declared zero (`id: "none"`) carries no
    /// version and renders `version: null`.
    #[serde(default)]
    pub version: Option<String>,
    pub ecosystem: Ecosystem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<FeedRef>,
    /// Target framework the kit requires (`net10.0` for the .NET kit). The
    /// renderer drives the generated TFM from this rather than from a
    /// hard-coded profile default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tfm: Option<String>,
    /// The declared minimum-consumption floor. `0` is a **declared zero**, a
    /// first-class state distinct from a floor failure, and then
    /// `zero_reason` must be present.
    pub minimum_packages: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero_reason: Option<String>,
}

impl KitReference {
    /// The declared zero: no registered kit for this ecosystem. Never
    /// reported as having met a floor.
    pub fn is_declared_zero(&self) -> bool {
        self.id == NONE_KIT_ID
    }
}

/// Sentinel id for a profile that declares no kit. It still carries a
/// `minimum_packages: 0` and a `zero_reason`, so the absence is recorded
/// rather than implicit.
pub const NONE_KIT_ID: &str = "none";

/// How a package is classified for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageClass {
    /// Two or more distinct external consumers and no infrastructure weight:
    /// rendered as a restoring reference and counted toward the floor.
    Confirmed,
    /// Below the consumer bar, or withheld for infrastructure weight: rendered
    /// inside a comment block, never restoring, never counted.
    Provisional,
}

/// One package the kit knows about, with the evidence behind its class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitPackage {
    pub name: String,
    pub consumers: u32,
    pub class: PackageClass,
    /// Rendered into the comment block explaining why the package is not
    /// pre-wired: the missing second consumer, or the infrastructure weight.
    pub reason: String,
}

/// Why a package is in the feed. `Confirmed` packages are the pre-wired set
/// the floor counts; `Transitive` packages are present only because the
/// confirmed set needs them and are never a direct reference.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FeedPackageRole {
    Confirmed,
    Transitive,
}

/// One committed package in a kit's feed.
///
/// The descriptor declares **which** package at **which** version. It does
/// **not** carry a digest: `dotnet pack` is not byte-reproducible, so a digest
/// compiled into the binary would drift on every repack and turn a legitimate
/// repack into a `kit-digest-mismatch`. The per-file digest lives in exactly
/// one place, `kits/manifest.json`, which `forge kit pack` rewrites.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedPackage {
    pub package: String,
    pub version: String,
    /// File name inside the vendored feed, e.g. `Platform.Core.0.1.0.nupkg`.
    pub file: String,
    pub role: FeedPackageRole,
}

impl FeedPackage {
    /// Location inside the vendored `kits/` tree.
    pub fn kits_path(&self) -> String {
        format!("{KITS_FEED_DIR}/{}", self.file)
    }

    /// Location inside the generated project.
    pub fn target(&self) -> String {
        format!("{PLATFORM_FEED_PATH}/{}", self.file)
    }
}

/// The vendored feed directory inside the `kits/` tree.
pub const KITS_FEED_DIR: &str = "feed";

/// The full compiled-in descriptor for one `(id, version)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitDescriptor {
    pub reference: KitReference,
    /// Packages with no infrastructure weight, in stable name order.
    pub packages: Vec<KitPackage>,
    /// Pinned version per confirmed package.
    pub versions: BTreeMap<String, String>,
    /// Digest-pinned vendored **text** artifacts (the single token source).
    pub assets: Vec<AssetRef>,
    /// The committed feed: which packages, at which version, in which role.
    /// Empty for a kit that ships no feed.
    pub feed_packages: Vec<FeedPackage>,
}

impl KitDescriptor {
    /// The confirmed set: what a scaffold restores and what counts toward the
    /// floor. Infrastructure-bearing packages are never in it.
    pub fn confirmed(&self) -> Vec<&KitPackage> {
        self.packages
            .iter()
            .filter(|p| p.class == PackageClass::Confirmed)
            .collect()
    }

    /// The provisional set: rendered commented, restoring nothing.
    pub fn provisional(&self) -> Vec<&KitPackage> {
        self.packages
            .iter()
            .filter(|p| p.class == PackageClass::Provisional)
            .collect()
    }

    pub fn confirmed_names(&self) -> Vec<String> {
        self.confirmed()
            .into_iter()
            .map(|p| p.name.clone())
            .collect()
    }

    pub fn provisional_names(&self) -> Vec<String> {
        self.provisional()
            .into_iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// The pinned version for a confirmed package.
    pub fn version_of(&self, package: &str) -> Option<&str> {
        self.versions.get(package).map(String::as_str)
    }

    /// The vendored **feed** assets of this kit: the committed `.nupkg` bytes
    /// for the restore closure, in stable package-name order. Empty for a kit
    /// that ships no feed.
    pub fn feed_assets(&self) -> Vec<&AssetRef> {
        self.assets.iter().filter(|a| a.binary).collect()
    }

    /// The declared feed: every package the confirmed set resolves, at the kit
    /// version, in stable package-name order.
    pub fn feed(&self) -> &[FeedPackage] {
        &self.feed_packages
    }
}

/// Classify one package from the frozen evidence fixture.
///
/// A package clears the bar at [`CONSUMER_BAR`] distinct external consumers
/// **and** carries no infrastructure weight.
pub fn classify_package(name: &str) -> PackageClass {
    let infra = PLATFORM_REQUIRES_INFRA.iter().any(|(p, _)| *p == name);
    // An absent fixture entry is treated as zero consumers: an unclassified
    // package is never silently confirmed, and the completeness test names
    // it so the gap is fixed rather than absorbed.
    let consumers = consumers_of(name).unwrap_or(0);
    if consumers >= CONSUMER_BAR && !infra {
        PackageClass::Confirmed
    } else {
        PackageClass::Provisional
    }
}

/// Measured external consumer count from the frozen fixture. `None` when the
/// package is not in the fixture at all, which the completeness test treats as
/// an unclassified package.
pub fn consumers_of(name: &str) -> Option<u32> {
    PLATFORM_PACKAGE_EVIDENCE
        .iter()
        .find(|(p, _)| *p == name)
        .map(|(_, c)| *c)
}

fn infra_reason(name: &str) -> Option<&'static str> {
    PLATFORM_REQUIRES_INFRA
        .iter()
        .find(|(p, _)| *p == name)
        .map(|(_, r)| *r)
}

/// The human-readable reason a package is not pre-wired.
///
/// Always names the measured external consumer count — the evidence that
/// decides the class — and, when infrastructure weight is the reason, appends
/// it. A comment that does not say *why* is the silent zero this change
/// exists to remove.
///
/// The reason is assembled from parts rather than interpolated whole, because
/// the infra reason may itself name two facts (`needs a tenant store; also
/// pulls …`) and a naive concatenation leaves a doubled separator at the seam.
/// The reason is therefore trimmed of any trailing separator before the parts
/// are joined, which makes a stutter impossible rather than merely absent
/// today.
fn provisional_reason(name: &str) -> String {
    let consumers = consumers_of(name).unwrap_or(0);
    let count = if consumers == 0 {
        format!("{name}: 0 external consumers recorded")
    } else {
        format!(
            "{name}: {consumers} external consumer{} recorded",
            if consumers == 1 { "" } else { "s" }
        )
    };
    // What the package still needs, per branch. The infra branch always states
    // both halves — the bar and the store — because an infrastructure obstacle
    // is stated as the missing evidence in its own right: a package with a
    // tenant store still needs its second consumer. The sentence ends on this
    // clause so the missing evidence is the last thing an operator reads.
    let bar_and_store = format!("needs {CONSUMER_BAR} consumers and no store to be pre-wired");
    match infra_reason(name) {
        Some(reason) => {
            let reason = reason.trim_end_matches([';', ' ', '\t', ',']);
            format!("{count}, but {reason}; {bar_and_store}")
        }
        // A package that clears the bar with no infrastructure weight is not
        // provisional; this branch exists so an unclassified future entry
        // still yields an honest reason rather than an empty one.
        None if consumers >= CONSUMER_BAR => format!("{count}, but {bar_and_store}"),
        None => format!("{count}; needs {CONSUMER_BAR} to be pre-wired"),
    }
}

fn platform_packages() -> Vec<KitPackage> {
    let mut packages: Vec<KitPackage> = PLATFORM_PACKAGE_EVIDENCE
        .iter()
        .map(|(name, consumers)| KitPackage {
            name: (*name).to_string(),
            consumers: *consumers,
            class: classify_package(name),
            reason: provisional_reason(name),
        })
        .collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

fn platform_versions() -> BTreeMap<String, String> {
    platform_packages()
        .iter()
        .filter(|p| p.class == PackageClass::Confirmed)
        .map(|p| {
            (
                p.name.clone(),
                crate::kit::registry::PLATFORM_PACKAGE_VERSION.to_string(),
            )
        })
        .collect()
}

/// The version `dotnet-platform-libs` packs (`VersionPrefix` in its
/// `Directory.Build.props`). Read from the sibling checkout, never fetched.
pub const PLATFORM_PACKAGE_VERSION: &str = "0.1.0";

/// The version the vendored token assets were generated at.
pub const PLATFORM_UI_KIT_VERSION: &str = "0.1.0";

/// Target framework the .NET kit requires. A `net8.0` project cannot reference
/// a `net10.0` package, and the workspace baseline is `net10.0`; the owner
/// ruled that SDK 8 is removed, so the scaffold is raised rather than
/// multi-targeted down.
pub const PLATFORM_TFM: &str = "net10.0";

/// The feed directory **inside a generated project**, named by the generated
/// project's own `NuGet.config`.
///
/// It is deliberately relative: a project must restore at a path it was not
/// generated at. The bytes live in that directory, so a fresh clone restores
/// with no sibling library, no environment variable and no secret. This is the
/// same shape `therapist-commons` (`.packages/local-feed`) and `citylens`
/// (`artifacts/platform-feed`) already use.
pub const PLATFORM_FEED_PATH: &str = "packages/platform-feed";

/// The confirmed set's **restore closure**: the confirmed packages plus every
/// package they reach transitively through a project reference.
///
/// A restore resolves the closure, not the direct references. Measured from
/// `dotnet-platform-libs/eng/package-manifest.json`: `Platform.Testing` pulls
/// `Platform.Billing.Contracts` and `Platform.Eventing`, and
/// `Platform.Observability` pulls `Platform.Web.Telemetry`. A feed holding
/// only the six confirmed packages would fail restore, so the feed carries the
/// closure.
///
/// The floor still counts the **confirmed set only**. The three transitive
/// members are below the consumer bar and stay rendered commented, never as
/// direct references; they are in the feed because the confirmed set needs
/// them, not because they earned a pre-wired reference.
pub const PLATFORM_RESTORE_CLOSURE: &[(&str, FeedPackageRole)] = &[
    // The six confirmed packages.
    ("Platform.AspNetCore", FeedPackageRole::Confirmed),
    ("Platform.Core", FeedPackageRole::Confirmed),
    ("Platform.Idempotency", FeedPackageRole::Confirmed),
    ("Platform.Observability", FeedPackageRole::Confirmed),
    ("Platform.RateLimiting", FeedPackageRole::Confirmed),
    ("Platform.Testing", FeedPackageRole::Confirmed),
    // Transitive members, present in the feed because the confirmed set needs
    // them. Never a direct reference, never counted toward the floor.
    ("Platform.Billing.Contracts", FeedPackageRole::Transitive),
    ("Platform.Eventing", FeedPackageRole::Transitive),
    ("Platform.Web.Telemetry", FeedPackageRole::Transitive),
];

/// The declared feed of the `platform-dotnet` kit, derived from
/// [`PLATFORM_RESTORE_CLOSURE`] at the kit version.
///
/// The file name is the NuGet convention `{package}.{version}.nupkg`, which
/// is also what the drift check reads back: a feed packed at a different
/// version carries the different version in its file name, which is exactly the
/// drift a restore would silently accept and a reviewer would not see.
pub fn platform_feed_packages() -> Vec<FeedPackage> {
    PLATFORM_RESTORE_CLOSURE
        .iter()
        .map(|(package, role)| FeedPackage {
            package: (*package).to_string(),
            version: PLATFORM_PACKAGE_VERSION.to_string(),
            file: format!("{package}.{PLATFORM_PACKAGE_VERSION}.nupkg"),
            role: *role,
        })
        .collect()
}

/// Build a declared-zero reference. A declared zero is a **first-class state**,
/// distinct from a floor failure: it records that the ecosystem has no
/// registered kit and names the missing evidence.
pub fn declared_zero(ecosystem: Ecosystem, reason: &str) -> KitReference {
    KitReference {
        id: NONE_KIT_ID.to_string(),
        version: None,
        ecosystem,
        feed: None,
        tfm: None,
        minimum_packages: 0,
        zero_reason: Some(reason.to_string()),
    }
}

/// All compiled-in kit descriptors, in stable id order.
///
/// Only **registered** kits live here. A profile with no registered kit
/// declares a zero through [`declared_zero`], so the absence is recorded on
/// the profile rather than duplicated here — one sentinel id with three
/// ecosystem-specific meanings would be ambiguous to look up.
pub fn all_kits() -> Vec<KitDescriptor> {
    vec![
        KitDescriptor {
            reference: KitReference {
                id: "platform-dotnet".to_string(),
                version: Some(PLATFORM_PACKAGE_VERSION.to_string()),
                ecosystem: Ecosystem::Dotnet,
                feed: Some(FeedRef {
                    name: "platform".to_string(),
                    kind: FeedKind::Nuget,
                    path: PLATFORM_FEED_PATH.to_string(),
                }),
                tfm: Some(PLATFORM_TFM.to_string()),
                minimum_packages: crate::kit::floor::DEFAULT_MINIMUM_PACKAGES,
                zero_reason: None,
            },
            packages: platform_packages(),
            versions: platform_versions(),
            assets: Vec::new(),
            feed_packages: platform_feed_packages(),
        },
        KitDescriptor {
            reference: KitReference {
                id: "platform-ui-web".to_string(),
                version: Some(PLATFORM_UI_KIT_VERSION.to_string()),
                ecosystem: Ecosystem::Npm,
                feed: None,
                tfm: None,
                // The vendored token pair is the single confirmed unit; the
                // `@platform/react-ui` and `react-shell` npm packages have no
                // external consumer and stay commented.
                minimum_packages: 1,
                zero_reason: None,
            },
            packages: Vec::new(),
            versions: BTreeMap::new(),
            assets: crate::kit::assets::token_assets(),
            feed_packages: Vec::new(),
        },
    ]
}

/// Resolve a declared `(id, version)` against the compiled-in registry.
/// Offline, and the only place a kit is ever looked up.
pub fn inspect_kit(id: &str, version: Option<&str>) -> Result<KitDescriptor, ForgeError> {
    let known = all_kits();
    let matched = known
        .iter()
        .find(|kit| kit.reference.id == id)
        .ok_or_else(|| ForgeError::KitUnknown {
            reason: format!(
                "kit '{id}' is not in the compiled-in kit registry (known: {}); \
                 nothing was staged and no project was registered",
                known
                    .iter()
                    .map(|k| k.reference.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })?;
    if let Some(version) = version {
        let actual = matched.reference.version.as_deref();
        if actual != Some(version) {
            return Err(ForgeError::KitUnknown {
                reason: format!(
                    "kit '{id}@{version}' is not in the compiled-in kit registry \
                     (known version: {}); nothing was staged and no project was registered",
                    actual.unwrap_or("none")
                ),
            });
        }
    }
    Ok(matched.clone())
}

/// Resolve the kit a profile declares, checking that the declared ecosystem
/// matches the profile's own toolchain ecosystem.
///
/// A mismatch — a .NET package list on a Node profile, or any other cross — is
/// refused before any file is written. A declared zero carries no package list
/// and so cannot mismatch, but it is still checked for the recorded reason
/// that makes the absence honest rather than implicit.
pub fn kit_for_profile(
    profile_id: &str,
    reference: &KitReference,
    profile_toolchain: &str,
) -> Result<KitDescriptor, ForgeError> {
    if reference.is_declared_zero() {
        if reference.minimum_packages != 0 {
            return Err(ForgeError::KitFloorNotMet {
                reason: format!(
                    "profile '{profile_id}' declares kit '{}' as a zero but sets minimum_packages {}; \
                     a declared zero must be 0 with a zero_reason; nothing was staged and no project was registered",
                    NONE_KIT_ID,
                    reference.minimum_packages
                ),
            });
        }
        if reference
            .zero_reason
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .is_none()
        {
            return Err(ForgeError::KitUnknown {
                reason: format!(
                    "profile '{profile_id}' declares kit '{}' with no zero_reason; \
                     a declared zero must name the missing evidence; \
                     nothing was staged and no project was registered",
                    NONE_KIT_ID
                ),
            });
        }
        return Ok(KitDescriptor {
            reference: reference.clone(),
            packages: Vec::new(),
            versions: BTreeMap::new(),
            assets: Vec::new(),
            feed_packages: Vec::new(),
        });
    }

    let descriptor = inspect_kit(&reference.id, reference.version.as_deref())?;
    let profile_ecosystem = Ecosystem::for_toolchain(profile_toolchain);
    if descriptor.reference.ecosystem != profile_ecosystem {
        return Err(ForgeError::KitEcosystemMismatch {
            reason: format!(
                "profile '{profile_id}' declares kit '{}' for the {} ecosystem but its toolchain '{}' is {}; \
                 nothing was staged and no project was registered",
                reference.id,
                descriptor.reference.ecosystem.as_str(),
                profile_toolchain,
                profile_ecosystem.as_str()
            ),
        });
    }
    Ok(descriptor)
}

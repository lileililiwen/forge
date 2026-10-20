//! The build-target vocabulary and the per-toolchain resolution.
//!
//! Forge Core owns one canonical list of operating systems, one of
//! architectures, the documented aliases and the mapping from an
//! operating system and an architecture into the target form a
//! project's own toolchain reads. A toolchain with no target mechanism
//! refuses a requested target instead of silently building for the host
//! and reporting it as the requested target.
//!
//! The vocabulary is lowercase and exact: `linux`, `darwin` and
//! `windows` are the operating systems, `x86_64` and `aarch64` are the
//! architectures, and `macos`, `osx`, `win`, `amd64`, `x64` and `arm64`
//! are the accepted aliases.

pub mod target_tests;

use crate::core::ForgeError;

/// The canonical operating systems, in stable order.
pub const OPERATING_SYSTEMS: &[&str] = &["linux", "darwin", "windows"];

/// The canonical architectures, in stable order.
pub const ARCHITECTURES: &[&str] = &["x86_64", "aarch64"];

/// The target a build reports when it was not addressed to one.
pub const HOST_TARGET: &str = "host";

/// One canonical operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    /// The Linux operating system.
    Linux,
    /// The Darwin operating system (macOS).
    Darwin,
    /// The Windows operating system.
    Windows,
}

impl OperatingSystem {
    /// The canonical spelling of this operating system.
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Darwin => "darwin",
            OperatingSystem::Windows => "windows",
        }
    }
}

/// One canonical architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// The 64-bit x86 architecture.
    X86_64,
    /// The 64-bit ARM architecture.
    Aarch64,
}

impl Architecture {
    /// The canonical spelling of this architecture.
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        }
    }
}

/// Resolve one operating system value to its canonical form.
///
/// The accepted values are the canonical spellings and the documented
/// aliases. An unknown value is refused by name, and the refusal names
/// the accepted values.
pub fn parse_operating_system(value: &str) -> Result<OperatingSystem, ForgeError> {
    match value {
        "linux" => Ok(OperatingSystem::Linux),
        "darwin" | "macos" | "osx" => Ok(OperatingSystem::Darwin),
        "windows" | "win" => Ok(OperatingSystem::Windows),
        other => Err(ForgeError::BuildInvalid {
            reason: format!(
                "unknown operating system `{other}`; accepted values are {}",
                accepted_values(OPERATING_SYSTEMS)
            ),
        }),
    }
}

/// Resolve one architecture value to its canonical form.
///
/// The accepted values are the canonical spellings and the documented
/// aliases. An unknown value is refused by name, and the refusal names
/// the accepted values.
pub fn parse_architecture(value: &str) -> Result<Architecture, ForgeError> {
    match value {
        "x86_64" | "amd64" | "x64" => Ok(Architecture::X86_64),
        "aarch64" | "arm64" => Ok(Architecture::Aarch64),
        other => Err(ForgeError::BuildInvalid {
            reason: format!(
                "unknown architecture `{other}`; accepted values are {}",
                accepted_values(ARCHITECTURES)
            ),
        }),
    }
}

/// The accepted values of one vocabulary, rendered for a refusal.
fn accepted_values(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

/// The target form one toolchain reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetForm {
    /// A Rust target triple, bound through `CARGO_BUILD_TARGET`.
    RustTriple,
    /// A .NET runtime identifier, bound through `DOTNET_RUNTIME_ID`.
    RuntimeIdentifier,
    /// A platform and an architecture, bound through `npm_config_platform`
    /// and `npm_config_arch`.
    PlatformArch,
}

/// The target form a toolchain reads, or `None` when the toolchain has
/// no target-addressed build.
pub fn target_form_for(toolchain: &str) -> Option<TargetForm> {
    match toolchain {
        "cargo" => Some(TargetForm::RustTriple),
        "dotnet" => Some(TargetForm::RuntimeIdentifier),
        "node" | "npm" => Some(TargetForm::PlatformArch),
        _ => None,
    }
}

/// The target an owner asked a build to address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRequest {
    /// No target: the build runs for the host.
    Host,
    /// An operating system with an architecture.
    Platform {
        /// The requested operating system.
        os: OperatingSystem,
        /// The requested architecture.
        arch: Architecture,
    },
    /// A runtime identifier, used exactly as it was supplied.
    RuntimeIdentifier(String),
}

/// Build the requested target from the command-line flags.
///
/// An operating system needs an architecture and the reverse, a
/// runtime identifier cannot be combined with them, and no flag at all
/// is a host build. Each refusal names what is missing.
pub fn request_from_flags(
    os: Option<&str>,
    arch: Option<&str>,
    rid: Option<&str>,
) -> Result<TargetRequest, ForgeError> {
    let has_os = os.is_some();
    let has_arch = arch.is_some();
    let has_rid = rid.is_some();
    if has_rid && (has_os || has_arch) {
        return Err(ForgeError::BuildInvalid {
            reason: "a runtime identifier cannot be combined with an operating system and an \
                     architecture; choose one target"
                .to_string(),
        });
    }
    if has_os && !has_arch {
        return Err(ForgeError::BuildInvalid {
            reason: "an operating system needs an architecture; add --arch".to_string(),
        });
    }
    if has_arch && !has_os {
        return Err(ForgeError::BuildInvalid {
            reason: "an architecture needs an operating system; add --os".to_string(),
        });
    }
    if let Some(rid) = rid {
        let trimmed = rid.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::BuildInvalid {
                reason: "the runtime identifier is empty".to_string(),
            });
        }
        return Ok(TargetRequest::RuntimeIdentifier(trimmed.to_string()));
    }
    match (os, arch) {
        (Some(os), Some(arch)) => Ok(TargetRequest::Platform {
            os: parse_operating_system(os)?,
            arch: parse_architecture(arch)?,
        }),
        _ => Ok(TargetRequest::Host),
    }
}

/// A target a build is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    /// The label the outcome reports: the resolved target, or `host`.
    pub label: String,
    /// The environment variables that bind the target in the build.
    pub env: Vec<(String, String)>,
}

/// Resolve a requested target into the form the toolchain reads.
///
/// The host binds nothing. A platform resolves into the toolchain's
/// own target form. A runtime identifier is accepted only by a
/// toolchain that reads one. A toolchain with no mechanism refuses the
/// request and names the toolchain and the profile.
pub fn resolve_target(
    profile_id: &str,
    toolchain: &str,
    request: &TargetRequest,
) -> Result<ResolvedTarget, ForgeError> {
    match request {
        TargetRequest::Host => Ok(ResolvedTarget {
            label: HOST_TARGET.to_string(),
            env: Vec::new(),
        }),
        TargetRequest::Platform { os, arch } => {
            let form =
                target_form_for(toolchain).ok_or_else(|| no_mechanism(profile_id, toolchain))?;
            Ok(match form {
                TargetForm::RustTriple => {
                    let triple = rust_triple(*os, *arch);
                    ResolvedTarget {
                        label: triple.to_string(),
                        env: vec![("CARGO_BUILD_TARGET".to_string(), triple.to_string())],
                    }
                }
                TargetForm::RuntimeIdentifier => {
                    let rid = runtime_identifier(*os, *arch);
                    ResolvedTarget {
                        label: rid.to_string(),
                        env: vec![("DOTNET_RUNTIME_ID".to_string(), rid.to_string())],
                    }
                }
                TargetForm::PlatformArch => {
                    let platform = npm_platform(*os);
                    let arch_value = npm_arch(*arch);
                    ResolvedTarget {
                        label: format!("{platform}/{arch_value}"),
                        env: vec![
                            ("npm_config_platform".to_string(), platform.to_string()),
                            ("npm_config_arch".to_string(), arch_value.to_string()),
                        ],
                    }
                }
            })
        }
        TargetRequest::RuntimeIdentifier(rid) => {
            let form =
                target_form_for(toolchain).ok_or_else(|| no_mechanism(profile_id, toolchain))?;
            if form != TargetForm::RuntimeIdentifier {
                return Err(ForgeError::BuildInvalid {
                    reason: format!(
                        "toolchain `{toolchain}` does not read a runtime identifier; build for the \
                         host instead"
                    ),
                });
            }
            Ok(ResolvedTarget {
                label: rid.clone(),
                env: vec![("DOTNET_RUNTIME_ID".to_string(), rid.clone())],
            })
        }
    }
}

/// The refusal for a toolchain that has no target-addressed build.
fn no_mechanism(profile_id: &str, toolchain: &str) -> ForgeError {
    ForgeError::BuildInvalid {
        reason: format!(
            "profile `{profile_id}` uses toolchain `{toolchain}`, which has no target-addressed \
             build; build for the host instead"
        ),
    }
}

/// The Rust target triple for one operating system and architecture.
fn rust_triple(os: OperatingSystem, arch: Architecture) -> &'static str {
    match (os, arch) {
        (OperatingSystem::Linux, Architecture::X86_64) => "x86_64-unknown-linux-gnu",
        (OperatingSystem::Linux, Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
        (OperatingSystem::Darwin, Architecture::X86_64) => "x86_64-apple-darwin",
        (OperatingSystem::Darwin, Architecture::Aarch64) => "aarch64-apple-darwin",
        (OperatingSystem::Windows, Architecture::X86_64) => "x86_64-pc-windows-msvc",
        (OperatingSystem::Windows, Architecture::Aarch64) => "aarch64-pc-windows-msvc",
    }
}

/// The .NET runtime identifier for one operating system and architecture.
fn runtime_identifier(os: OperatingSystem, arch: Architecture) -> &'static str {
    match (os, arch) {
        (OperatingSystem::Linux, Architecture::X86_64) => "linux-x64",
        (OperatingSystem::Linux, Architecture::Aarch64) => "linux-arm64",
        (OperatingSystem::Darwin, Architecture::X86_64) => "osx-x64",
        (OperatingSystem::Darwin, Architecture::Aarch64) => "osx-arm64",
        (OperatingSystem::Windows, Architecture::X86_64) => "win-x64",
        (OperatingSystem::Windows, Architecture::Aarch64) => "win-arm64",
    }
}

/// The npm platform for one operating system.
fn npm_platform(os: OperatingSystem) -> &'static str {
    match os {
        OperatingSystem::Linux => "linux",
        OperatingSystem::Darwin => "darwin",
        OperatingSystem::Windows => "win32",
    }
}

/// The npm architecture for one architecture.
fn npm_arch(arch: Architecture) -> &'static str {
    match arch {
        Architecture::X86_64 => "x64",
        Architecture::Aarch64 => "arm64",
    }
}

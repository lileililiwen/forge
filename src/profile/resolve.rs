//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use std::collections::HashSet;
use std::path::Path;

use super::capabilities::SERVER_SIDE_CAPABILITIES;
use super::catalog::{mvp_profiles, planned_profiles};
use super::model::{
    PreflightReport, ProfileDescriptor, ProfileSupportStatus, ResolvedProfile, WorkspaceMapping,
};

/// List all supported descriptors in stable ID order. Resolution, preflight
/// and generation consume this list; planned candidates stay discoverable
/// through [`inspect_profile`] and [`planned_profiles`] but never enter the
/// selectable catalog until they are promoted to supported.
pub fn list_profiles() -> Vec<ProfileDescriptor> {
    mvp_profiles()
}

/// Inspect one descriptor by ID. Searches the supported catalog first and
/// then the planned catalog so the roadmap stays discoverable. The
/// descriptor's `support_status` field is the authoritative signal for
/// whether the id is selectable through resolution or generation.
pub fn inspect_profile(id: &str) -> Result<ProfileDescriptor, ForgeError> {
    if let Some(p) = mvp_profiles().into_iter().find(|p| p.id == id) {
        return Ok(p);
    }
    if let Some(p) = planned_profiles().into_iter().find(|p| p.id == id) {
        return Ok(p);
    }
    Err(ForgeError::UnknownProfile { id: id.to_string() })
}

/// True iff the id is in the supported catalog and therefore selectable
/// through resolution, preflight and generation.
pub fn is_supported(id: &str) -> bool {
    mvp_profiles().iter().any(|p| p.id == id)
}

/// Validate a descriptor parsed from external input. Names the missing
/// field so callers can report it without guessing.
pub fn validate_descriptor(profile: &ProfileDescriptor) -> Result<(), ForgeError> {
    let name = if profile.id.trim().is_empty() {
        "unknown"
    } else {
        profile.id.as_str()
    };
    let missing = if profile.id.trim().is_empty() {
        Some("id")
    } else if profile.version.trim().is_empty() {
        Some("version")
    } else if profile.adapter.trim().is_empty() {
        Some("adapter")
    } else if profile.language.trim().is_empty() {
        Some("language")
    } else if profile.toolchain.trim().is_empty() {
        Some("toolchain")
    } else if profile.build_command.trim().is_empty() {
        Some("build_command")
    } else if profile.test_command.trim().is_empty() {
        Some("test_command")
    } else if profile
        .workspace
        .as_ref()
        .is_some_and(|w| w.governance_profile.trim().is_empty())
    {
        Some("workspace.governance_profile")
    } else if profile
        .workspace
        .as_ref()
        .is_some_and(|w| w.kind.trim().is_empty())
    {
        Some("workspace.kind")
    } else {
        None
    };
    if let Some(field) = missing {
        return Err(ForgeError::InvalidProfile {
            reason: format!("profile descriptor '{name}' missing required field '{field}'"),
        });
    }
    Ok(())
}

/// Parse an external descriptor document and validate required fields.
///
/// The workspace mapping is additionally validated against the consumed
/// governance vocabulary: a `governance_profile` or `kind` outside the
/// canonical set refuses with `invalid-profile` naming the field and the
/// offending value, and no project tree is staged. When the vocabulary
/// itself is unavailable the parse proceeds exactly as before (the
/// declaration-vocabulary doctor finding reports the gap instead).
///
/// Built-in descriptors are grandfathered: `flutter-app` declares
/// `flutter-product`, which the consumed vocabulary does not contain.
/// Generation keeps emitting it (behaviour unchanged) while the doctor
/// finding surfaces the divergence and the companion request to
/// canonicalize it stays recorded. Refusing a supported profile at load
/// would break pinned generation behaviour; external input is where
/// Forge is asked to declare a new value, so refusal lives here.
pub fn descriptor_from_yaml(bytes: &[u8]) -> Result<ProfileDescriptor, ForgeError> {
    let profile: ProfileDescriptor =
        serde_yaml::from_slice(bytes).map_err(|err| ForgeError::InvalidProfile {
            reason: format!("malformed profile descriptor: {err}"),
        })?;
    validate_descriptor(&profile)?;
    if let Some(workspace) = profile.workspace.as_ref() {
        match crate::vocabulary::load(None) {
            Ok(vocab) => validate_workspace_mapping(workspace, &vocab)?,
            Err(crate::vocabulary::VocabularyError::Unavailable { .. }) => {}
            Err(crate::vocabulary::VocabularyError::Refused { path, reason }) => {
                return Err(ForgeError::InvalidProfile {
                    reason: format!(
                        "profile descriptor '{}' cannot be validated: {reason} ({path})",
                        profile.id
                    ),
                });
            }
        }
    }
    Ok(profile)
}

/// Refuse a workspace mapping whose declared values are outside the
/// consumed canonical sets. Names the field and the offending value;
/// never coerces a foreign value into a Forge value.
fn validate_workspace_mapping(
    workspace: &WorkspaceMapping,
    vocab: &crate::vocabulary::GovernanceVocabulary,
) -> Result<(), ForgeError> {
    if !vocab.is_canonical_profile(&workspace.governance_profile) {
        return Err(ForgeError::InvalidProfile {
            reason: format!(
                "workspace.governance_profile '{}' is not in the consumed governance vocabulary ({}); no project tree was staged",
                workspace.governance_profile,
                vocab.provenance()
            ),
        });
    }
    if !vocab.is_canonical_kind(&workspace.kind) {
        return Err(ForgeError::InvalidProfile {
            reason: format!(
                "workspace.kind '{}' is not in the consumed governance vocabulary ({}); no project tree was staged",
                workspace.kind,
                vocab.provenance()
            ),
        });
    }
    Ok(())
}

/// Resolve a profile plus requested capabilities. Fails before any file
/// change when the combination is unsupported and explains the boundary.
/// Planned profiles are reported as `unsupported-profile` so callers can
/// distinguish a missing template from a missing capability.
pub fn resolve_profile(id: &str, requested: &[String]) -> Result<ResolvedProfile, ForgeError> {
    let profile = inspect_profile(id)?;
    if profile.support_status == ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{id}' is reserved on the catalog as a planned candidate \
                 with no tested template; promote it to a versioned supported \
                 descriptor before selection; no files were changed"
            ),
        });
    }
    for capability in requested {
        if profile.capabilities.iter().any(|c| c == capability) {
            continue;
        }
        if SERVER_SIDE_CAPABILITIES.contains(&capability.as_str()) {
            return Err(ForgeError::IncompatibleProfile {
                reason: format!(
                    "profile '{id}' does not support server-side capability '{capability}'; \
                     use a backend profile (e.g. 'rust-web' or 'python-service') for '{capability}' \
                     and keep '{id}' as client; no files were changed"
                ),
            });
        }
        return Err(ForgeError::IncompatibleProfile {
            reason: format!(
                "profile '{id}' version '{}' does not support capability '{capability}' \
                 (adapter '{}'); no files were changed",
                profile.version, profile.adapter
            ),
        });
    }
    Ok(ResolvedProfile {
        id: profile.id,
        version: profile.version,
        adapter: profile.adapter,
        language: profile.language,
        toolchain: profile.toolchain,
    })
}

/// Preflight the required toolchain. With `available` set, membership is
/// checked directly (deterministic for tests); otherwise the host `PATH` is
/// probed. A missing toolchain is reported as unavailable — never as tested.
/// Planned profiles are refused with `unsupported-profile` so the
/// preflight never claims a planned candidate was tested.
pub fn preflight_profile(
    id: &str,
    available: Option<&HashSet<String>>,
) -> Result<PreflightReport, ForgeError> {
    let profile = inspect_profile(id)?;
    if profile.support_status == ProfileSupportStatus::Planned {
        return Err(ForgeError::UnsupportedProfile {
            reason: format!(
                "profile '{id}' is reserved on the catalog as a planned candidate \
                 with no tested template; preflight refuses planned profiles and \
                 no toolchain was probed"
            ),
        });
    }
    let present = match available {
        Some(set) => set.contains(profile.toolchain.as_str()),
        None => toolchain_on_path(profile.toolchain.as_str()),
    };
    if !present {
        return Err(ForgeError::ToolchainMissing {
            toolchain: profile.toolchain.clone(),
            profile: profile.id,
        });
    }
    Ok(PreflightReport {
        id: profile.id,
        version: profile.version,
        toolchain: profile.toolchain,
        available: true,
    })
}

fn toolchain_on_path(name: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return true;
        }
        if Path::new(&candidate).exists() {
            return true;
        }
    }
    false
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn lists_six_supported_ids_including_react_web() {
        let profiles = list_profiles();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "aspnet-web",
                "flutter-app",
                "nextjs-web",
                "python-service",
                "react-web",
                "rust-web",
            ]
        );
        for p in &profiles {
            assert!(!p.version.trim().is_empty(), "{} needs a version", p.id);
            assert_eq!(
                p.support_status,
                ProfileSupportStatus::Supported,
                "{}",
                p.id
            );
            validate_descriptor(p).expect("built-in descriptors must be valid");
        }
    }

    #[test]
    fn react_web_resolves_with_client_capabilities_only() {
        let resolved = resolve_profile("react-web", &strings(&["auth", "i18n"])).unwrap();
        assert_eq!(resolved.version, "0.1.0");
        assert_eq!(resolved.adapter, "adapter-react");
        assert_eq!(resolved.language, "typescript");
        assert_eq!(resolved.toolchain, "npm");

        // Boundary: react-web is client-only, the v0.1 server-side
        // capabilities must be refused with the backend hint.
        let err = resolve_profile("react-web", &strings(&["postgres"]))
            .expect_err("react-web + postgres must fail");
        assert_eq!(err.code(), "incompatible-profile");
        let text = err.to_string();
        assert!(text.contains("postgres"), "{text}");
        assert!(text.contains("backend"), "{text}");
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn react_web_descriptor_has_no_database_dependency() {
        let react = inspect_profile("react-web").unwrap();
        assert!(!react.requires_database);
        assert!(
            !react.packages.iter().any(|p| p.contains("postgres")),
            "client profile must not force a database package"
        );
        assert!(!react.capabilities.iter().any(|c| c == "postgres"));
        assert!(!react.capabilities.iter().any(|c| c == "redis"));
        assert!(!react.capabilities.iter().any(|c| c == "background-jobs"));
        assert!(!react.capabilities.iter().any(|c| c == "storage"));
    }

    #[test]
    fn supported_profiles_declare_workspace_mappings() {
        let expected = [
            ("aspnet-web", "dotnet-product"),
            ("flutter-app", "flutter-product"),
            ("nextjs-web", "typescript-product"),
            ("python-service", "python-product"),
            ("react-web", "typescript-product"),
            ("rust-web", "rust-product"),
        ];
        for (id, governance) in expected {
            let p = inspect_profile(id).unwrap();
            let w = p
                .workspace
                .as_ref()
                .unwrap_or_else(|| panic!("{id} declares a governance mapping"));
            assert_eq!(w.governance_profile, governance, "{id}");
            assert_eq!(w.kind, "product", "{id}");
            // No shared Gate Runtime is configured, so no profile declares one.
            assert_eq!(w.gate_runtime, None, "{id}");
        }
        // Planned candidates hold the no-mapping sentinel: nothing is ever
        // guessed for a profile without a tested scaffold.
        for p in planned_profiles() {
            assert!(p.workspace.is_none(), "{} must have no mapping", p.id);
        }
    }

    #[test]
    fn external_descriptor_with_unknown_governance_profile_refuses_by_name() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\nworkspace:\n  governance_profile: bogus-product\n  kind: product\n";
        let err = descriptor_from_yaml(yaml).expect_err("non-canonical profile must fail");
        assert_eq!(err.code(), "invalid-profile");
        let text = err.to_string();
        assert!(text.contains("workspace.governance_profile"), "{text}");
        assert!(text.contains("bogus-product"), "{text}");
        assert!(text.contains("no project tree was staged"), "{text}");
    }

    #[test]
    fn external_descriptor_with_unknown_kind_refuses_by_name() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\nworkspace:\n  governance_profile: rust-product\n  kind: control-plane\n";
        let err = descriptor_from_yaml(yaml).expect_err("non-canonical kind must fail");
        assert_eq!(err.code(), "invalid-profile");
        let text = err.to_string();
        assert!(text.contains("workspace.kind"), "{text}");
        assert!(text.contains("control-plane"), "{text}");
    }

    #[test]
    fn external_descriptor_with_canonical_mapping_parses() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\nworkspace:\n  governance_profile: rust-product\n  kind: product\n";
        let parsed = descriptor_from_yaml(yaml).unwrap();
        let mapping = parsed.workspace.expect("mapping survives");
        assert_eq!(mapping.governance_profile, "rust-product");
        assert_eq!(mapping.kind, "product");
    }

    #[test]
    fn builtin_flutter_mapping_divergence_is_recorded_not_refused() {
        // Design-vs-reality: `flutter-app` declares `flutter-product`,
        // which the consumed vocabulary (workspace-governance@pinned)
        // does not contain. Built-ins are grandfathered so pinned
        // generation behaviour stays byte-identical; the divergence is
        // visible through the declaration-vocabulary doctor finding and
        // the companion canonicalization request, never a load refusal.
        let vocab = crate::vocabulary::load(None).unwrap();
        assert!(
            !vocab.is_canonical_profile("flutter-product"),
            "companion request open: canonicalize flutter-product"
        );
        let flutter = inspect_profile("flutter-app").unwrap();
        assert_eq!(
            flutter.workspace.as_ref().unwrap().governance_profile,
            "flutter-product"
        );
    }

    #[test]
    fn workspace_mapping_parses_and_rejects_blank_governance_profile() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\nworkspace:\n  governance_profile: '  '\n  kind: product\n";
        let err = descriptor_from_yaml(yaml).expect_err("blank governance profile must fail");
        assert_eq!(err.code(), "invalid-profile");
        assert!(
            err.to_string().contains("workspace.governance_profile"),
            "{err}"
        );

        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: cargo test\n";
        let p = descriptor_from_yaml(yaml).unwrap();
        assert!(
            p.workspace.is_none(),
            "absent mapping parses to the sentinel"
        );
    }

    #[test]
    fn descriptors_carry_full_metadata() {
        for p in list_profiles() {
            assert!(!p.adapter.trim().is_empty(), "{}", p.id);
            assert!(!p.language.trim().is_empty(), "{}", p.id);
            assert!(!p.toolchain.trim().is_empty(), "{}", p.id);
            assert!(!p.capabilities.is_empty(), "{}", p.id);
            assert!(!p.packages.is_empty(), "{}", p.id);
            assert!(!p.layout.is_empty(), "{}", p.id);
            assert!(!p.conventions.is_empty(), "{}", p.id);
            assert!(!p.build_command.trim().is_empty(), "{}", p.id);
            assert!(!p.test_command.trim().is_empty(), "{}", p.id);
            assert!(!p.quality_policies.is_empty(), "{}", p.id);
        }
    }

    #[test]
    fn malformed_descriptor_names_missing_field() {
        let yaml = b"id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\ntoolchain: cargo\nbuild_command: cargo build\ntest_command: ''\n";
        let err = descriptor_from_yaml(yaml).expect_err("empty test_command must fail");
        assert_eq!(err.code(), "invalid-profile");
        assert!(
            err.to_string().contains("test_command"),
            "must name the field: {err}"
        );
    }

    #[test]
    fn valid_profile_without_database_forces_no_dependency() {
        let flutter = inspect_profile("flutter-app").unwrap();
        assert!(!flutter.requires_database);
        assert!(
            !flutter.packages.iter().any(|p| p.contains("postgres")),
            "client profile must not force a database package"
        );
        assert!(!flutter.capabilities.iter().any(|c| c == "postgres"));
        // Resolution without features succeeds and carries no db requirement.
        let resolved = resolve_profile("flutter-app", &[]).unwrap();
        assert_eq!(resolved.version, flutter.version);
    }

    #[test]
    fn resolve_returns_exact_version_and_adapter() {
        let resolved = resolve_profile("rust-web", &strings(&["auth"])).unwrap();
        assert_eq!(resolved.version, "0.1.0");
        assert_eq!(resolved.adapter, "adapter-rust");
        assert_eq!(resolved.language, "rust");
    }

    #[test]
    fn flutter_rejects_server_postgres_with_backend_hint() {
        let err = resolve_profile("flutter-app", &strings(&["postgres"]))
            .expect_err("flutter + postgres must fail");
        assert_eq!(err.code(), "incompatible-profile");
        let text = err.to_string();
        assert!(text.contains("postgres"), "{text}");
        assert!(text.contains("backend"), "{text}");
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn missing_toolchain_reports_without_claiming_tested() {
        let empty = HashSet::new();
        let err = preflight_profile("rust-web", Some(&empty)).expect_err("no toolchain");
        assert_eq!(err.code(), "toolchain-missing");
        let text = err.to_string();
        assert!(text.contains("cargo"), "{text}");
        assert!(text.contains("not tested"), "{text}");
        assert!(!text.contains("tested ok"), "{text}");
    }

    #[test]
    fn unknown_profile_is_reported() {
        let err = inspect_profile("not-a-real-profile").expect_err("must be unknown");
        assert_eq!(err.code(), "unknown-profile");
    }

    #[test]
    fn planned_profiles_are_inspectable_but_not_selectable() {
        // Discoverability: every reserved candidate has a descriptor with
        // status `planned` and a description that names the boundary.
        let planned = planned_profiles();
        let ids: Vec<&str> = planned.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "aspnet-saas",
                "flutter-client",
                "nextjs-content",
                "python-ai",
                "python-data",
                "rust-cli",
                "rust-worker",
            ]
        );
        for p in &planned {
            assert_eq!(p.support_status, ProfileSupportStatus::Planned, "{}", p.id);
            assert!(
                p.description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains("planned"),
                "{} must mark itself as planned",
                p.id
            );
            validate_descriptor(p).expect("planned descriptor must be well-formed");
        }
        // The supported catalog stays at 6 and never includes planned ids.
        for planned_id in &ids {
            assert!(
                !is_supported(planned_id),
                "{planned_id} must not be supported"
            );
            assert!(!list_profiles().iter().any(|p| p.id == *planned_id));
        }
    }

    #[test]
    fn planned_profile_resolve_reports_unsupported_profile() {
        let err = resolve_profile("aspnet-saas", &[]).expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("aspnet-saas"), "{text}");
        assert!(
            text.contains("planned"),
            "must name the planned status: {text}"
        );
        assert!(text.contains("no files were changed"), "{text}");
    }

    #[test]
    fn planned_profile_preflight_refuses_without_probing_toolchain() {
        let err = preflight_profile("flutter-client", None).expect_err("planned must refuse");
        assert_eq!(err.code(), "unsupported-profile");
        let text = err.to_string();
        assert!(text.contains("flutter-client"), "{text}");
        assert!(text.contains("planned"), "{text}");
    }

    #[test]
    fn flutter_client_planned_descriptor_describes_backend_boundary() {
        // Boundary scenario: a planned client profile that needs server
        // capabilities must describe a separate backend boundary rather
        // than embedding server infrastructure in the client.
        let flutter_client = inspect_profile("flutter-client").unwrap();
        assert_eq!(flutter_client.support_status, ProfileSupportStatus::Planned);
        let desc = flutter_client.description.unwrap_or_default();
        assert!(
            desc.contains("backend"),
            "flutter-client description must reference a backend boundary: {desc}"
        );
        assert!(
            desc.contains("server")
                || desc.contains("'rust-web'")
                || desc.contains("'python-service'"),
            "flutter-client description must name the supported backend alternatives: {desc}"
        );
    }
}

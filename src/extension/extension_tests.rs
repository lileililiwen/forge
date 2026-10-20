//! The shared contract test (`extension-point-contract`).
//!
//! The suite runs the shared verification over every registered
//! kind, so a new kind is held to the rules by the next test run
//! without anyone restating them.

#[cfg(test)]
mod tests {
    use crate::extension::contract::verify_extension;
    use crate::extension::kinds::{
        default_adapter_for_kind, deploy_target_kind_state, required_target_fields,
        supported_target_kinds, DEPLOY_TARGET_KINDS, TARGET_JENKINS, TARGET_SSH,
    };
    use crate::extension::state::{ExtensionDescriptor, ExtensionState};
    use crate::extension::version::ContractVersion;

    /// The executor contract the deploy surface requires.
    fn required() -> ContractVersion {
        ContractVersion::parse("forge-deploy-executor/0.1.0").expect("a well-formed requirement")
    }

    /// A declaration of the required family at the required major.
    fn descriptor() -> ExtensionDescriptor {
        ExtensionDescriptor {
            id: "forge-deployer".to_string(),
            contract: required(),
            enabled: true,
        }
    }

    #[test]
    fn every_registered_kind_is_held_to_the_shared_wording() {
        for entry in DEPLOY_TARGET_KINDS {
            let state = deploy_target_kind_state(entry.kind);
            if entry.wired {
                assert_eq!(
                    state,
                    ExtensionState::Ready,
                    "the wired kind `{}` must be ready",
                    entry.kind
                );
                continue;
            }
            let reason = state.reason().unwrap_or_default().to_string();
            assert_eq!(
                reason,
                format!(
                    "deployment target kind `{kind}` is known but not yet wired; v0.1.0 supports {supported}",
                    kind = entry.kind,
                    supported = supported_target_kinds()
                ),
                "an unwired kind must be refused in the shared wording"
            );
        }
    }

    #[test]
    fn an_unknown_kind_is_refused_with_the_shared_wording() {
        let state = deploy_target_kind_state("carrier-pigeon");
        assert_eq!(state.as_str(), "invalid");
        assert_eq!(
            state.reason().unwrap_or_default(),
            format!(
                "deployment target kind `carrier-pigeon` is not supported; expected one of {}",
                supported_target_kinds()
            )
        );
    }

    #[test]
    fn the_supported_list_names_every_wired_kind_once() {
        let supported = supported_target_kinds();
        assert_eq!(
            supported,
            "`local`, `docker-compose`, `jenkins`, `mac-runtime`, and `ssh`"
        );
        for entry in DEPLOY_TARGET_KINDS.iter().filter(|e| e.wired) {
            assert_eq!(
                supported.matches(&format!("`{}`", entry.kind)).count(),
                1,
                "the supported list names `{}` exactly once: {supported}",
                entry.kind
            );
        }
        assert!(
            supported.contains(TARGET_SSH),
            "a wired kind is advertised: {supported}"
        );
        assert!(
            supported.contains(TARGET_JENKINS),
            "a wired kind is advertised: {supported}"
        );
    }

    #[test]
    fn the_ssh_kind_declares_its_fields_and_its_own_adapter() {
        let entry = DEPLOY_TARGET_KINDS
            .iter()
            .find(|entry| entry.kind == TARGET_SSH)
            .expect("the ssh kind is registered");
        assert!(entry.wired, "the ssh kind is served by this build");
        assert_eq!(
            required_target_fields(TARGET_SSH),
            &["host", "path"],
            "an ssh target states the host and the remote path"
        );
        assert_eq!(
            entry.adapter_env,
            Some("FORGE_DEPLOYER_SSH_BIN"),
            "the ssh kind names its own adapter binary"
        );
        assert_eq!(
            default_adapter_for_kind(TARGET_SSH),
            Some("forge-deployer-ssh"),
            "the ssh kind has its own default adapter"
        );
    }

    #[test]
    fn every_wired_kind_declares_its_fields_and_its_adapter() {
        for entry in DEPLOY_TARGET_KINDS.iter().filter(|entry| entry.wired) {
            assert_eq!(
                required_target_fields(entry.kind),
                entry.required_fields,
                "the registry and the accessor must agree about `{}`",
                entry.kind
            );
            assert_eq!(
                default_adapter_for_kind(entry.kind),
                Some(entry.default_adapter),
                "the registry and the accessor must agree about `{}`",
                entry.kind
            );
            assert!(
                !entry.default_adapter.is_empty(),
                "the wired kind `{}` names a default adapter",
                entry.kind
            );
        }
    }

    #[test]
    fn an_unknown_kind_requires_nothing_and_names_no_adapter() {
        assert!(required_target_fields("carrier-pigeon").is_empty());
        assert_eq!(default_adapter_for_kind("carrier-pigeon"), None);
    }

    #[test]
    fn the_state_vocabulary_has_four_states() {
        let ready = verify_extension(&descriptor(), &required(), true);
        assert_eq!(ready.as_str(), "ready");
        assert!(ready.reason().is_none());
        assert!(ready.is_ready());

        let switched_off = ExtensionDescriptor {
            enabled: false,
            ..descriptor()
        };
        let disabled = verify_extension(&switched_off, &required(), true);
        assert_eq!(disabled.as_str(), "disabled");
        assert!(disabled
            .reason()
            .unwrap_or_default()
            .contains("forge-deployer"));

        let unavailable = verify_extension(&descriptor(), &required(), false);
        assert_eq!(unavailable.as_str(), "unavailable");
        assert!(unavailable
            .reason()
            .unwrap_or_default()
            .contains("forge-deployer"));

        let stranger = ExtensionDescriptor {
            contract: ContractVersion::parse("forge-deployer/0.1.0").expect("a well-formed claim"),
            ..descriptor()
        };
        let invalid = verify_extension(&stranger, &required(), true);
        assert_eq!(invalid.as_str(), "invalid");
        assert_eq!(
            invalid.reason().unwrap_or_default(),
            "forge-deployer declares the contract `forge-deployer/0.1.0`, which is incompatible with the required `forge-deploy-executor/0.1.0`: the families differ (`forge-deployer` and `forge-deploy-executor`)"
        );
    }

    #[test]
    fn a_newer_minor_of_the_same_family_is_compatible() {
        let declared =
            ContractVersion::parse("forge-deploy-executor/0.2.0").expect("a well-formed claim");
        assert!(declared.is_compatible_with(&required()));
        assert_eq!(declared.incompatibility_detail(&required()), None);
        let state = verify_extension(
            &ExtensionDescriptor {
                id: "forge-deployer".to_string(),
                contract: declared,
                enabled: true,
            },
            &required(),
            true,
        );
        assert_eq!(state, ExtensionState::Ready);
    }

    #[test]
    fn a_different_major_is_incompatible_and_names_both_majors() {
        let declared =
            ContractVersion::parse("forge-deploy-executor/1.0.0").expect("a well-formed claim");
        assert!(!declared.is_compatible_with(&required()));
        assert_eq!(
            declared.incompatibility_detail(&required()),
            Some("the major versions differ (1 and 0)".to_string())
        );
    }

    #[test]
    fn a_different_family_is_incompatible_and_names_both_families() {
        let declared = ContractVersion::parse("forge-deployer/0.1.0").expect("a well-formed claim");
        assert!(!declared.is_compatible_with(&required()));
        assert_eq!(
            declared.incompatibility_detail(&required()),
            Some("the families differ (`forge-deployer` and `forge-deploy-executor`)".to_string())
        );
    }

    #[test]
    fn an_unreadable_version_is_refused_and_names_the_value() {
        for raw in [
            "0.1.0",
            "",
            "forge-deploy-executor/0.1",
            "forge-deploy-executor/0.1.0.0",
        ] {
            let err = ContractVersion::parse(raw).expect_err("an unreadable version");
            assert_eq!(err.value, raw);
            assert!(
                err.reason.contains(raw),
                "the refusal names the offending value: {}",
                err.reason
            );
        }
        // The bare form names the expected shape.
        let err = ContractVersion::parse("0.1.0").expect_err("an unreadable version");
        assert_eq!(
            err.reason,
            "the version `0.1.0` does not name a contract family; expected `family/major.minor.patch`"
        );
    }

    #[test]
    fn a_version_renders_the_family_and_the_numbers() {
        let declared =
            ContractVersion::parse("forge-deploy-executor/0.1.9").expect("a well-formed claim");
        assert_eq!(declared.version(), "0.1.9");
        assert_eq!(declared.to_string(), "forge-deploy-executor/0.1.9");
        assert_eq!(declared.family, "forge-deploy-executor");
        assert_eq!(declared.major, 0);
        assert_eq!(declared.minor, 1);
        assert_eq!(declared.patch, 9);
    }
}

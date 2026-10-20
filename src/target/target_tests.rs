//! The build-target vocabulary and the per-toolchain resolution.

#[cfg(test)]
pub(super) mod tests {
    use crate::core::ForgeError;
    use crate::target::{
        parse_architecture, parse_operating_system, request_from_flags, resolve_target,
        target_form_for, Architecture, OperatingSystem, TargetForm, TargetRequest, ARCHITECTURES,
        OPERATING_SYSTEMS,
    };

    /// Every row of the resolution table, with the toolchain's own form.
    const TABLE: &[(&str, &str, &str, &str, &str, &str)] = &[
        (
            "linux",
            "x86_64",
            "x86_64-unknown-linux-gnu",
            "linux-x64",
            "linux",
            "x64",
        ),
        (
            "linux",
            "aarch64",
            "aarch64-unknown-linux-gnu",
            "linux-arm64",
            "linux",
            "arm64",
        ),
        (
            "darwin",
            "x86_64",
            "x86_64-apple-darwin",
            "osx-x64",
            "darwin",
            "x64",
        ),
        (
            "darwin",
            "aarch64",
            "aarch64-apple-darwin",
            "osx-arm64",
            "darwin",
            "arm64",
        ),
        (
            "windows",
            "x86_64",
            "x86_64-pc-windows-msvc",
            "win-x64",
            "win32",
            "x64",
        ),
        (
            "windows",
            "aarch64",
            "aarch64-pc-windows-msvc",
            "win-arm64",
            "win32",
            "arm64",
        ),
    ];

    #[test]
    fn the_canonical_values_are_the_documented_ones() {
        assert_eq!(OPERATING_SYSTEMS, &["linux", "darwin", "windows"]);
        assert_eq!(ARCHITECTURES, &["x86_64", "aarch64"]);
        for os in OPERATING_SYSTEMS {
            let parsed = parse_operating_system(os).expect("canonical os");
            assert_eq!(parsed.as_str(), *os);
        }
        for arch in ARCHITECTURES {
            let parsed = parse_architecture(arch).expect("canonical arch");
            assert_eq!(parsed.as_str(), *arch);
        }
    }

    #[test]
    fn every_documented_alias_resolves_to_the_canonical_value() {
        for (alias, canonical) in [("macos", "darwin"), ("osx", "darwin"), ("win", "windows")] {
            assert_eq!(
                parse_operating_system(alias)
                    .unwrap_or_else(|err| panic!("alias {alias}: {err}"))
                    .as_str(),
                canonical
            );
        }
        for (alias, canonical) in [("amd64", "x86_64"), ("x64", "x86_64"), ("arm64", "aarch64")] {
            assert_eq!(
                parse_architecture(alias)
                    .unwrap_or_else(|err| panic!("alias {alias}: {err}"))
                    .as_str(),
                canonical
            );
        }
    }

    #[test]
    fn an_unknown_operating_system_is_refused_by_name() {
        let err = parse_operating_system("plan9").expect_err("unknown os");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("plan9"), "reason: {reason}");
        for accepted in OPERATING_SYSTEMS {
            assert!(reason.contains(accepted), "reason: {reason}");
        }
    }

    #[test]
    fn an_unknown_architecture_is_refused_by_name() {
        let err = parse_architecture("riscv64").expect_err("unknown arch");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("riscv64"), "reason: {reason}");
        for accepted in ARCHITECTURES {
            assert!(reason.contains(accepted), "reason: {reason}");
        }
    }

    #[test]
    fn every_table_row_resolves_for_every_toolchain() {
        for (os, arch, triple, rid, platform, npm_arch) in TABLE {
            let request = TargetRequest::Platform {
                os: parse_operating_system(os).expect("os"),
                arch: parse_architecture(arch).expect("arch"),
            };
            let cargo = resolve_target("rust-web", "cargo", &request).expect("cargo target");
            assert_eq!(cargo.label, *triple);
            assert_eq!(
                cargo.env,
                vec![("CARGO_BUILD_TARGET".to_string(), (*triple).to_string())]
            );
            let dotnet = resolve_target("aspnet-web", "dotnet", &request).expect("dotnet target");
            assert_eq!(dotnet.label, *rid);
            assert_eq!(
                dotnet.env,
                vec![("DOTNET_RUNTIME_ID".to_string(), (*rid).to_string())]
            );
            let node = resolve_target("nextjs-web", "node", &request).expect("node target");
            assert_eq!(node.label, format!("{platform}/{npm_arch}"));
            assert_eq!(
                node.env,
                vec![
                    ("npm_config_platform".to_string(), (*platform).to_string()),
                    ("npm_config_arch".to_string(), (*npm_arch).to_string()),
                ]
            );
        }
    }

    #[test]
    fn the_target_form_is_declared_per_toolchain() {
        assert_eq!(target_form_for("cargo"), Some(TargetForm::RustTriple));
        assert_eq!(
            target_form_for("dotnet"),
            Some(TargetForm::RuntimeIdentifier)
        );
        assert_eq!(target_form_for("node"), Some(TargetForm::PlatformArch));
        assert_eq!(target_form_for("npm"), Some(TargetForm::PlatformArch));
        assert_eq!(target_form_for("python3"), None);
    }

    #[test]
    fn a_toolchain_without_a_mechanism_is_refused() {
        let request = TargetRequest::Platform {
            os: OperatingSystem::Linux,
            arch: Architecture::Aarch64,
        };
        let err = resolve_target("python-service", "python3", &request).expect_err("no mechanism");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("python3"), "reason: {reason}");
        assert!(reason.contains("python-service"), "reason: {reason}");
    }

    #[test]
    fn a_partial_target_is_refused_and_names_the_missing_half() {
        let os_only = request_from_flags(Some("linux"), None, None).expect_err("partial target");
        let reason = reason_of(&os_only);
        assert!(reason.contains("architecture"), "reason: {reason}");
        let arch_only =
            request_from_flags(None, Some("aarch64"), None).expect_err("partial target");
        let reason = reason_of(&arch_only);
        assert!(reason.contains("operating system"), "reason: {reason}");
    }

    #[test]
    fn two_targets_at_once_are_refused() {
        let err = request_from_flags(Some("linux"), Some("aarch64"), Some("linux-arm64"))
            .expect_err("two targets");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("runtime identifier"), "reason: {reason}");
    }

    #[test]
    fn a_runtime_identifier_is_refused_for_a_toolchain_that_reads_none() {
        let request = TargetRequest::RuntimeIdentifier("linux-arm64".to_string());
        let err = resolve_target("rust-web", "cargo", &request).expect_err("rid for cargo");
        assert_eq!(err.code(), "build-invalid");
        let reason = reason_of(&err);
        assert!(reason.contains("cargo"), "reason: {reason}");
    }

    #[test]
    fn a_runtime_identifier_binds_the_dotnet_toolchain() {
        let request = TargetRequest::RuntimeIdentifier("linux-arm64".to_string());
        let resolved = resolve_target("aspnet-web", "dotnet", &request).expect("rid target");
        assert_eq!(resolved.label, "linux-arm64");
        assert_eq!(
            resolved.env,
            vec![("DOTNET_RUNTIME_ID".to_string(), "linux-arm64".to_string())]
        );
    }

    #[test]
    fn a_host_build_resolves_to_the_host_and_binds_nothing() {
        let request = request_from_flags(None, None, None).expect("host request");
        assert_eq!(request, TargetRequest::Host);
        let resolved = resolve_target("rust-web", "cargo", &request).expect("host target");
        assert_eq!(resolved.label, "host");
        assert!(resolved.env.is_empty(), "env: {:?}", resolved.env);
    }

    #[test]
    fn the_flags_build_the_requested_platform() {
        let request = request_from_flags(Some("macos"), Some("arm64"), None).expect("platform");
        assert_eq!(
            request,
            TargetRequest::Platform {
                os: OperatingSystem::Darwin,
                arch: Architecture::Aarch64,
            }
        );
        let resolved = resolve_target("rust-web", "cargo", &request).expect("platform target");
        assert_eq!(resolved.label, "aarch64-apple-darwin");
    }

    /// The reason of a typed error, without the variant's own prefix.
    fn reason_of(err: &ForgeError) -> String {
        match err {
            ForgeError::BuildInvalid { reason } => reason.clone(),
            other => panic!("expected build-invalid, got {other:?}"),
        }
    }
}

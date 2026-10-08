//! Kit registry classification: every profile's kit reference, every package's
//! confirmed/provisional status, and the reason-string discipline.

use super::*;
use forge::kit;

#[test]
fn every_profile_kit_has_a_registry_row_and_every_package_is_classified() {
    for profile in forge::profile::mvp_profiles() {
        let reference = profile
            .kit
            .as_ref()
            .unwrap_or_else(|| panic!("profile '{}' declares no kit", profile.id));
        let descriptor = kit::registry::kit_for_profile(&profile.id, reference, &profile.toolchain)
            .unwrap_or_else(|err| panic!("profile '{}': {err}", profile.id));

        if reference.is_declared_zero() {
            // A declared zero must name the missing evidence, and must never
            // be reported as having met a floor.
            assert_eq!(reference.minimum_packages, 0, "{}", profile.id);
            assert!(
                reference
                    .zero_reason
                    .as_deref()
                    .is_some_and(|r| !r.trim().is_empty()),
                "profile '{}' declares a zero with no reason",
                profile.id
            );
            assert!(
                !kit::floor::check_floor(&profile.id, &descriptor, None, "t")
                    .expect("a declared zero renders")
                    .floor_met(),
                "profile '{}' reports a declared zero as a met floor",
                profile.id
            );
            continue;
        }

        // A registered kit must resolve against the compiled-in registry.
        let declared_floor = reference.minimum_packages;
        assert!(
            declared_floor > 0,
            "profile '{}' declares a registered kit with a zero floor",
            profile.id
        );
        // The floor is met by the descriptor the registry actually holds.
        let decision =
            kit::floor::check_floor(&profile.id, &descriptor, None, "t").unwrap_or_else(|err| {
                panic!(
                    "profile '{}' cannot meet its declared floor: {err}",
                    profile.id
                )
            });
        assert!(decision.floor_met(), "{}", profile.id);

        // Every package the kit declares is classified, and every confirmed
        // member is pinned at the kit version.
        //
        // The classification arm used to be `matches!(class, Confirmed |
        // Provisional)`, which is a tautology against a two-variant enum and
        // could never fail. What can actually go wrong is a package that is in
        // one place and not the other, so the check is bidirectional against
        // the frozen evidence fixture. The fixture measures .NET packages, so
        // it applies to the .NET kit; a kit with no packages at all (the Node
        // and Dart kits carry vendored assets, not a package set) is exempt
        // rather than trivially satisfied.
        let fixture: BTreeMap<&str, u32> = kit::PLATFORM_PACKAGE_EVIDENCE.iter().copied().collect();
        let mut declared: BTreeSet<&str> = BTreeSet::new();
        for package in &descriptor.packages {
            assert!(
                fixture.contains_key(package.name.as_str()),
                "kit '{}' declares package '{}', which the frozen evidence fixture does not \
                 carry; a package with no measured consumer count is unclassified by definition",
                profile.id,
                package.name
            );
            declared.insert(package.name.as_str());
            if package.class == kit::PackageClass::Confirmed {
                assert!(
                    descriptor.version_of(&package.name).is_some(),
                    "{} is confirmed but unpinned",
                    package.name
                );
            }
        }
        if descriptor.reference.id == "platform-dotnet" {
            let unreached: Vec<&str> = fixture
                .keys()
                .copied()
                .filter(|name| !declared.contains(name))
                .collect();
            assert!(
                unreached.is_empty(),
                "kit '{}' leaves {} evidence-fixture package(s) unaccounted for: {:?}. A package \
                 neither confirmed nor provisional is a surface nothing accounts for",
                profile.id,
                unreached.len(),
                unreached
            );
        }
        // Nothing is both confirmed and provisional.
        let confirmed = descriptor.confirmed_names();
        for name in descriptor.provisional_names() {
            assert!(!confirmed.contains(&name), "{name} is in both sets");
        }
    }
}

#[test]
fn a_provisional_reason_states_the_count_exactly_once_and_reads_cleanly() {
    // The consumer count is authoritative in the evidence fixture and is
    // emitted by the composer. A reason string that also carried a count
    // rendered "3 external consumers recorded, but 3 consumers, but needs…"
    // — the count twice, with a doubled "but". The reason states only the
    // infrastructure obstacle.
    let descriptor = kit::registry::inspect_kit("platform-dotnet", Some("0.1.0")).unwrap();
    let reason_of = |name: &str| {
        descriptor
            .packages
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the descriptor"))
            .reason
            .clone()
    };

    // Exactly the strings the generated comment block must read as.
    assert_eq!(
        reason_of("Platform.Identity.AspNetCore"),
        "Platform.Identity.AspNetCore: 3 external consumers recorded, but needs an identity store \
         and provider configuration; needs 2 consumers and no store to be pre-wired"
    );
    assert_eq!(
        reason_of("Platform.Persistence.EfCore"),
        "Platform.Persistence.EfCore: 4 external consumers recorded, but needs a database and a \
         migration story; needs 2 consumers and no store to be pre-wired"
    );
    // The tenant-lifecycle reason names two facts of its own. It must not gain
    // a second separator where the missing-evidence clause is appended.
    assert_eq!(
        reason_of("Platform.Tenant.Lifecycle.AspNetCore"),
        "Platform.Tenant.Lifecycle.AspNetCore: 0 external consumers recorded, but needs a tenant \
         store; also pulls Platform.Tenant.Lifecycle.Contracts; needs 2 consumers and no store to \
         be pre-wired"
    );
    // Below the bar with no infrastructure weight: a different shape, and the
    // singular/plural on the count.
    assert_eq!(
        reason_of("Platform.UI.Razor"),
        "Platform.UI.Razor: 0 external consumers recorded; needs 2 to be pre-wired"
    );
    assert_eq!(
        reason_of("Platform.Web.Telemetry"),
        "Platform.Web.Telemetry: 1 external consumer recorded; needs 2 to be pre-wired"
    );

    // The two defect classes, asserted over every package whose reason is
    // actually rendered, so a future reason string cannot reintroduce them.
    for package in descriptor.provisional() {
        let reason = &package.reason;
        assert!(
            !reason.contains(";;"),
            "{} has a doubled separator: {reason}",
            package.name
        );
        assert!(
            !reason.contains(",,"),
            "{} has a doubled comma: {reason}",
            package.name
        );
        // The old defect was two `but` clauses — one from the composer, one
        // embedded in the reason string. A below-bar package with no
        // infrastructure weight states no obstacle at all, so zero is correct
        // there; more than one is the defect.
        assert!(
            reason.matches("but ").count() <= 1,
            "{} must state its obstacle with at most one `but`: {reason}",
            package.name
        );
        // The count is stated exactly once, and it is the measured one.
        let measured = kit::consumers_of(&package.name).unwrap();
        let needle = if measured == 0 {
            "0 external consumers recorded".to_string()
        } else {
            format!(
                "{measured} external consumer{} recorded",
                if measured == 1 { "" } else { "s" }
            )
        };
        assert_eq!(
            reason.matches(&needle).count(),
            1,
            "{} must state its measured count ({needle}) exactly once: {reason}",
            package.name
        );
        // The old defect's exact shape: a bare "<count> consumers, but" that
        // restates the count the composer already emitted.
        assert!(
            !reason.contains(&format!("{measured} consumers, but")),
            "{} restates the count as a bare number: {reason}",
            package.name
        );
    }

    // An infrastructure reason states only the obstacle, never a count, so the
    // two sources of the number cannot disagree.
    for (package, reason) in kit::PLATFORM_REQUIRES_INFRA {
        let digits = reason.chars().filter(char::is_ascii_digit).count();
        assert_eq!(
            digits,
            0,
            "the infra reason for {package} must not carry a count, because the composer emits the \
             measured one: {reason}"
        );
    }
}

#[test]
fn the_evidence_fixture_is_never_silently_promoted() {
    // The classification is derived from the frozen fixture, so a package
    // cannot be confirmed in one list and provisional in another.
    for (name, consumers) in kit::PLATFORM_PACKAGE_EVIDENCE {
        let class = kit::classify_package(name);
        let infra = kit::PLATFORM_PACKAGE_EVIDENCE
            .iter()
            .find(|(p, _)| p == name)
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(infra, *consumers);
        if *consumers >= kit::CONSUMER_BAR {
            // Identity and persistence clear the bar but are withheld for
            // infrastructure weight, never silently confirmed.
            let infra_bearing = [
                "Platform.Identity.AspNetCore",
                "Platform.Persistence.EfCore",
                "Platform.Tenant.Lifecycle.AspNetCore",
            ];
            if infra_bearing.contains(name) {
                assert_eq!(class, kit::PackageClass::Provisional, "{name}");
            } else {
                assert_eq!(class, kit::PackageClass::Confirmed, "{name}");
            }
        } else {
            assert_eq!(class, kit::PackageClass::Provisional, "{name}");
        }
    }
}

#[test]
fn an_unclassified_package_is_reported_not_silently_confirmed() {
    // A package absent from the fixture is never confirmed.
    assert_eq!(kit::consumers_of("Platform.NotARealPackage"), None);
    assert_eq!(
        kit::classify_package("Platform.NotARealPackage"),
        kit::PackageClass::Provisional
    );
}

//! Pre-wired consumption for the .NET profile.

use super::*;

#[test]
fn the_confirmed_dotnet_set_is_pre_wired_and_pins_its_target_framework() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let csproj = fs::read_to_string(dest.join("net-app.csproj")).unwrap();
    // The owner ruling: the scaffold renders the TFM its kit requires.
    assert!(
        csproj.contains("<TargetFramework>net10.0</TargetFramework>"),
        "{csproj}"
    );
    assert!(!csproj.contains("net8.0"), "{csproj}");

    for name in [
        "Platform.Core",
        "Platform.AspNetCore",
        "Platform.Testing",
        "Platform.RateLimiting",
        "Platform.Idempotency",
        "Platform.Observability",
    ] {
        assert!(
            csproj.contains(&format!("<PackageReference Include=\"{name}\" />")),
            "{name} is not pre-wired: {csproj}"
        );
    }
    let dockerfile = fs::read_to_string(dest.join("Dockerfile")).unwrap();
    assert!(dockerfile.contains("sdk:10.0"), "{dockerfile}");
}

#[test]
fn below_bar_packages_render_only_inside_a_comment_block() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let csproj = fs::read_to_string(dest.join("net-app.csproj")).unwrap();
    let props = fs::read_to_string(dest.join("Directory.Packages.props")).unwrap();

    for name in [
        "Platform.Web.Composition",
        "Platform.Web.Telemetry",
        "Platform.Testing.AspNetCore",
        "Platform.UI.Razor",
        "Platform.Http.Resilience",
        "Platform.Web.OpenApi",
        "Platform.Web.Cors",
        "Platform.Web.Resilience",
        "Platform.Web.Versioning",
        "Platform.FeatureManagement",
        "Platform.Tenant.Lifecycle.AspNetCore",
    ] {
        // Never a restoring reference.
        assert!(
            !csproj.contains(name),
            "{name} must not restore from an aspnet-web scaffold: {csproj}"
        );
        // Present only as a comment, naming its external consumer count.
        let line = props
            .lines()
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("{name} is not named in the generated manifest:\n{props}"));
        assert!(line.trim_start().starts_with("<!--"), "{name}: {line}");
        assert!(
            line.contains("consumer"),
            "{name} does not name its consumer count: {line}"
        );
    }
}

#[test]
fn infrastructure_bearing_packages_are_not_pre_wired() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = scaffold(&db, &dest, "aspnet-web");
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let csproj = fs::read_to_string(dest.join("net-app.csproj")).unwrap();
    for name in [
        "Platform.Identity",
        "Platform.Persistence",
        "Platform.Caching",
        "Platform.Jobs",
        "Platform.Billing",
        "Platform.Mailing",
        "Platform.Storage",
        "Platform.Ai",
        "Platform.Tenant",
    ] {
        assert!(
            !csproj.contains(name),
            "infrastructure-bearing package {name} is pre-wired unconditionally: {csproj}"
        );
    }
    // Referencing a package never grants the capability it implements: the
    // scaffold ships no auth, database or tenancy behaviour at all.
    let program = fs::read_to_string(dest.join("Program.cs")).unwrap();
    for capability in [
        "AddAuthentication",
        "UseAuthentication",
        "AddAuthorization",
        "DbContext",
        "Npgsql",
        "RateLimiter",
        "UseRateLimiter",
    ] {
        assert!(
            !program.contains(capability),
            "the scaffold grants {capability} without an explicit feature request: {program}"
        );
    }
}

#[test]
fn infrastructure_packages_appear_only_behind_an_explicit_feature_request() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("net-app");

    let out = run(
        &db,
        &[
            "new",
            &dest.display().to_string(),
            "--profile",
            "aspnet-web",
            "--feature",
            "postgres",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let manifest = parse_yaml(&dest.join("forge.yaml"));
    // The feature is recorded as a request, still not as granted evidence.
    assert_eq!(manifest["features"]["postgres"], "0.1.0");
    let csproj = fs::read_to_string(dest.join("net-app.csproj")).unwrap();
    assert!(
        !csproj.contains("Platform.Persistence"),
        "a requested feature still does not pre-wire an infrastructure package: {csproj}"
    );
}

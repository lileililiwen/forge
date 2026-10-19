use serde_json::Value;
use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

#[test]
fn active_source_metadata_points_at_forge_owned_paths() {
    for (path, module) in [
        ("crates/driftwatchdog/Cargo.toml", "crates/driftwatchdog"),
        (
            "crates/cargo-audit-boost/Cargo.toml",
            "crates/cargo-audit-boost",
        ),
        (
            "crates/cargo-matrix-boost/Cargo.toml",
            "crates/cargo-matrix-boost",
        ),
        ("crates/cargo-sonar/Cargo.toml", "crates/cargo-sonar"),
    ] {
        let manifest = read(path);
        assert!(
            manifest.contains(&format!(
                "repository = \"https://github.com/lileililiwen/forge\""
            )),
            "{path} must identify the Forge source repository"
        );
        assert!(
            manifest.contains(&format!(
                "homepage = \"https://github.com/lileililiwen/forge/tree/main/{module}\""
            )),
            "{path} must point maintainers at {module}"
        );
    }

    for path in [
        "crates/driftwatchdog/.project.json",
        "modules/workspace-governance/.project.json",
    ] {
        let metadata = json(path);
        assert!(
            metadata["discoverability"]["homepage"]
                .as_str()
                .unwrap_or_default()
                .starts_with("https://github.com/lileililiwen/forge/tree/main/"),
            "{path} homepage must point to its Forge source"
        );
    }

    let projects = json("modules/workspace-governance/projects.json");
    let workspace_governance = projects["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|project| project["id"] == "workspace-governance")
        .expect("Workspace Governance module registration");
    assert_eq!(
        workspace_governance["path"],
        "forge/modules/workspace-governance"
    );
}

#[test]
fn active_module_taxonomy_and_clone_guidance_point_to_forge() {
    let taxonomy = read("modules/workspace-governance/tag-taxonomy.json");
    for module in [
        "modules/workspace-governance",
        "crates/driftwatchdog",
        "modules/dotnet-platform-libs",
        "modules/rust-platform-libs",
    ] {
        assert!(
            taxonomy.contains(&format!(
                "https://github.com/lileililiwen/forge/tree/main/{module}"
            )),
            "taxonomy homepage must point to {module}"
        );
    }

    for (path, package) in [
        ("crates/cargo-matrix-boost/README.md", "cargo-matrix-boost"),
        ("crates/cargo-sonar/README.md", "cargo-sonar"),
        ("crates/driftwatchdog/README.md", "driftwatchdog"),
    ] {
        let readme = read(path);
        assert!(readme.contains("https://github.com/lileililiwen/forge.git"));
        assert!(readme.contains(&format!("crates/{package}")));
        assert!(!readme.contains(&format!(
            "git clone https://github.com/lileililiwen/{package}"
        )));
    }
}

#[test]
fn runtime_template_provenance_names_forge_not_a_local_sibling_checkout() {
    let provenance = read("scripts/governance/runtime-templates/PROVENANCE.md");
    assert!(provenance.contains("modules/workspace-governance"));
    assert!(!provenance.contains("/home/paul/code/workspace-governance"));
    assert!(provenance.contains("2c766c64eb54560fa898115dec27c32abbf4968e"));
}

#[test]
fn driftwatch_npm_metadata_points_to_forge_and_keeps_legacy_release_assets() {
    let package = json("crates/driftwatchdog/npm/driftwatchdog/package.json");
    assert_eq!(
        package["repository"]["url"],
        "https://github.com/lileililiwen/forge.git"
    );
    assert_eq!(
        package["repository"]["directory"],
        "crates/driftwatchdog/npm/driftwatchdog"
    );
    assert!(read("crates/driftwatchdog/npm/driftwatchdog/README.md")
        .contains("https://github.com/lileililiwen/driftwatchdog/releases"));
    assert!(read("crates/driftwatchdog/npm/driftwatchdog/README.md")
        .contains("historical distribution files"));
    assert!(
        read("crates/driftwatchdog/README.md").contains("former standalone repository is archived")
    );
}

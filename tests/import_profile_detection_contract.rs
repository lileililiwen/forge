//! Import profile detection truth contract (`import-profile-detection-truth`).
//!
//! The import lane's profile knowledge is pinned against the catalogue's
//! supported list in both directions: a profile the catalogue supports and the
//! lane does not know fails this check, and a profile the lane claims to know
//! but cannot detect fails it too. Every supported profile is inspected
//! through the built binary, so the guard is end to end rather than a
//! restatement of the lane's own table.

use forge::import::{import_profiles, profile_language, profile_package_manager};
use forge::profile::{list_profiles, ProfileSupportStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `(profile, language, package manager)` for every profile the import
/// lane knows.
const MATRIX: [(&str, &str, &str); 6] = [
    ("aspnet-web", "csharp", "dotnet"),
    ("flutter-app", "dart", "pub"),
    ("nextjs-web", "typescript", "npm"),
    ("python-service", "python", "pip"),
    ("react-web", "typescript", "npm"),
    ("rust-web", "rust", "cargo"),
];

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

/// Write the markers a project of `profile` carries.
fn write_profile_fixture(dir: &Path, profile: &str) {
    match profile {
        "rust-web" => {
            write_file(
                dir,
                "Cargo.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            );
            write_file(dir, "src/main.rs", "fn main() {}\n");
        }
        // A real Next.js project names `next` and depends on React. Both
        // marker sets are one stack: Next.js wins and raises no ambiguity.
        "nextjs-web" => {
            write_file(
                dir,
                "package.json",
                "{\n  \"name\": \"demo\",\n  \"private\": true,\n  \"dependencies\": {\n    \"next\": \"15.0.0\",\n    \"react\": \"18.3.1\",\n    \"react-dom\": \"18.3.1\"\n  }\n}\n",
            );
        }
        "react-web" => {
            write_file(
                dir,
                "package.json",
                "{\n  \"name\": \"demo\",\n  \"private\": true,\n  \"dependencies\": {\n    \"react\": \"18.3.1\",\n    \"react-dom\": \"18.3.1\"\n  }\n}\n",
            );
        }
        "flutter-app" => {
            write_file(
                dir,
                "pubspec.yaml",
                "name: demo\nenvironment:\n  flutter: \"3.22.0\"\n",
            );
        }
        "python-service" => {
            write_file(
                dir,
                "pyproject.toml",
                "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
            );
        }
        "aspnet-web" => {
            write_file(
                dir,
                "demo.csproj",
                "<Project Sdk=\"Microsoft.NET.Sdk.Web\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
            );
        }
        other => panic!("no fixture shape for {other}"),
    }
}

/// The proposal the built binary emitted for one directory.
fn proposal_for(db: &Path, dir: &Path) -> serde_json::Value {
    let out = run_json(db, &["import", &dir.display().to_string()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "import must succeed: {}",
        lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("import must emit JSON: {err}"));
    value["proposal"].clone()
}

#[test]
fn the_lane_knows_exactly_the_supported_profiles() {
    let lane: Vec<&str> = import_profiles();
    let descriptors = list_profiles();
    let catalogue: Vec<&str> = descriptors
        .iter()
        .filter(|descriptor| descriptor.support_status == ProfileSupportStatus::Supported)
        .map(|descriptor| descriptor.id.as_str())
        .collect();

    for profile in &catalogue {
        assert!(
            lane.contains(profile),
            "the import lane does not know the supported profile '{profile}'"
        );
    }
    for profile in &lane {
        assert!(
            catalogue.contains(profile),
            "the import lane knows '{profile}', which the catalogue does not list as supported"
        );
    }
    assert_eq!(
        lane.len(),
        MATRIX.len(),
        "the lane's profile list must cover every supported profile"
    );

    for (profile, language, package_manager) in MATRIX {
        assert_eq!(
            profile_language(profile),
            language,
            "{profile} must declare its language"
        );
        assert_eq!(
            profile_package_manager(profile),
            package_manager,
            "{profile} must declare its package manager"
        );
    }
}

#[test]
fn every_supported_profile_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");

    for (profile, language, package_manager) in MATRIX {
        let dir = tmp.path().join(format!("fixture-{profile}"));
        fs::create_dir_all(&dir).unwrap();
        write_profile_fixture(&dir, profile);

        let proposal = proposal_for(&db, &dir);
        assert_eq!(
            proposal["suggested_profile"], profile,
            "{profile} must be the suggested profile"
        );
        assert_eq!(
            proposal["language"]["value"], language,
            "{profile} must report its language"
        );
        assert_eq!(
            proposal["package_manager"]["value"], package_manager,
            "{profile} must report its package manager"
        );
        assert_eq!(
            proposal["alternatives"],
            serde_json::json!([]),
            "{profile} must propose no other profile"
        );
    }
}

#[test]
fn a_generated_nextjs_project_is_detected_whatever_it_is_called() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dir = tmp.path().join("webapp");
    fs::create_dir_all(&dir).unwrap();

    // The generated Next.js scaffold is dependency-free: only its own
    // config file and its own source root name the stack.
    write_file(
        &dir,
        "package.json",
        "{\n  \"name\": \"webapp\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"scripts\": {\n    \"build\": \"node ./scripts/build.mjs\",\n    \"test\": \"node --test\"\n  }\n}\n",
    );
    write_file(&dir, "next.config.mjs", "export default {};\n");
    write_file(&dir, "app/page.mjs", "export function greeting() {}\n");

    let proposal = proposal_for(&db, &dir);
    assert_eq!(
        proposal["suggested_profile"], "nextjs-web",
        "a generated Next.js project must be detected whatever it is called"
    );
}

#[test]
fn a_plain_npm_project_is_not_guessed() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dir = tmp.path().join("plain-lib");
    fs::create_dir_all(&dir).unwrap();

    // Neither marker set: a plain npm library is neither a Next.js nor a
    // React project, and the lane does not guess.
    write_file(
        &dir,
        "package.json",
        "{\n  \"name\": \"plain-lib\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"main\": \"index.js\"\n}\n",
    );
    write_file(&dir, "index.js", "module.exports = {};\n");

    let proposal = proposal_for(&db, &dir);
    assert_eq!(
        proposal["suggested_profile"],
        serde_json::Value::Null,
        "a plain npm project must have no suggested profile"
    );
    for field in ["language", "framework", "package_manager"] {
        assert_eq!(
            proposal[field]["status"], "unknown",
            "{field} must stay unknown for a plain npm project"
        );
    }
}

#[test]
fn an_accepted_react_import_carries_the_profiles_language() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dir = tmp.path().join("react-app");
    fs::create_dir_all(dir.join("src")).unwrap();

    write_file(
        &dir,
        "package.json",
        "{\n  \"name\": \"react-app\",\n  \"private\": true,\n  \"dependencies\": {\n    \"react\": \"18.3.1\",\n    \"react-dom\": \"18.3.1\"\n  }\n}\n",
    );
    write_file(
        &dir,
        "index.html",
        "<!doctype html>\n<html lang=\"en\">\n  <head><meta charset=\"utf-8\" /><title>react-app</title></head>\n  <body><div id=\"root\"></div></body>\n</html>\n",
    );
    write_file(
        &dir,
        "src/main.jsx",
        "import { createRoot } from \"react-dom/client\";\n",
    );

    let out = run(&db, &["import", &dir.display().to_string(), "--accept"]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let manifest = fs::read_to_string(dir.join("forge.yaml")).unwrap();
    assert!(
        manifest.contains("language: typescript"),
        "the accepted manifest must carry the profile's language:\n{manifest}"
    );

    let out = run_json(&db, &["doctor", &dir.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("doctor must emit JSON: {err}"));
    let drift = value["doctor"]["findings"]
        .as_array()
        .expect("the doctor must report findings")
        .iter()
        .find(|finding| finding["id"] == "dependency-drift")
        .expect("the doctor must report dependency-drift");
    assert_eq!(
        drift["status"], "pass",
        "the accepted import must not report dependency drift: {drift}"
    );
}

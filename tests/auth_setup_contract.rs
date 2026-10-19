//! R1 — The application auth setup contract.
//!
//! The setup is explicit, previewed, digest-bound and project-scoped. These tests
//! drive it against a fake provider pack, so they prove the no-write guarantees
//! without a provider runtime.

use forge::auth::packs::discover;
use forge::auth::setup::{AuthSetupOutcome, AuthSetupRequest, AuthSetupRunner};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

/// Writes a project manifest and returns the project directory.
fn project(dir: &Path, id: &str) -> PathBuf {
    let project_dir = dir.join(id);
    fs::create_dir_all(&project_dir).expect("create the project directory");
    fs::write(
        project_dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  kind: web\nruntime:\n  profile: dotnet-aspnet\nfeatures:\n  web: 1.0.0\n"
        ),
    )
    .expect("write the manifest");
    project_dir
}

/// Writes a fake provider pack that answers the contract.
fn fake_pack(dir: &Path, name: &str, apply: &str) -> PathBuf {
    let packs = dir.join("packs");
    fs::create_dir_all(&packs).expect("create the packs directory");
    let executable = packs.join(name);
    let script = r#"#!/bin/sh
input=$(cat)
field() { printf '%s' "$input" | sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p"; }
operation=$(field operation)
operation_id=$(field operation_id)
project_id=$(field project_id)
project_revision=$(field project_revision)
realm="realm-$project_id"
case "$operation" in
  probe)
    printf '%s\n' '{"contract":"forge-identity-provider/1.0","provider_id":"fake","operation_id":"'"$operation_id"'","project_id":"'"$project_id"'","project_revision":"'"$project_revision"'","operation":"probe","status":"ready","summary":"The provider can serve this project."}'
    ;;
  plan)
    printf '%s\n' '{"contract":"forge-identity-provider/1.0","provider_id":"fake","operation_id":"'"$operation_id"'","project_id":"'"$project_id"'","project_revision":"'"$project_revision"'","operation":"plan","status":"ready","summary":"The plan is ready.","plan_digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","proposed_changes":[{"kind":"provider.realm","target":"realm-'"$realm"'","summary":"Create the isolated realm for this project."}]}'
    ;;
  apply)
    __APPLY__
    ;;
  *)
    printf '%s\n' '{"contract":"forge-identity-provider/1.0","provider_id":"fake","operation_id":"'"$operation_id"'","project_id":"'"$project_id"'","project_revision":"'"$project_revision"'","operation":"'"$operation"'","status":"failed","summary":"unsupported"}'
    ;;
esac
"#;
    let script = script.replace("__APPLY__", apply);
    fs::write(&executable, script).expect("write the pack");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make the pack executable");
    }
    fs::write(
        packs.join("fake.json"),
        format!(
            "{{\"id\":\"fake\",\"description\":\"A fake provider pack.\",\"modes\":[\"test\"],\"executable\":\"{}\",\"enabled\":true}}",
            executable.file_name().unwrap().to_string_lossy()
        ),
    )
    .expect("write the descriptor");
    packs
}

/// The apply answer of the fake pack.
fn apply_response(status: &str, extra: &str) -> String {
    let body = format!(
        "{{\"contract\":\"forge-identity-provider/1.0\",\"provider_id\":\"fake\",\"operation_id\":\"'\"$operation_id\"'\",\"project_id\":\"'\"$project_id\"'\",\"project_revision\":\"'\"$project_revision\"'\",\"operation\":\"apply\",\"status\":\"{status}\",\"summary\":\"The provider applied the plan.\",\"population\":{{\"population_id\":\"fake/'\"$realm\"'\",\"issuer\":\"https://127.0.0.1:18090/realms/'\"$realm\"'\",\"client_id\":\"'\"$realm\"'\",\"callback_path\":\"/signin-oidc\",\"scopes\":[\"openid\"]}}{extra}}}"
    );
    format!("printf '%s\\n' '{body}'")
}

fn request(project_dir: &Path, id: &str, confirm: bool) -> AuthSetupRequest {
    AuthSetupRequest {
        project_id: id.to_string(),
        project_dir: project_dir.to_path_buf(),
        provider_id: "fake".to_string(),
        provider_input: json!({ "state_dir": "/tmp/state", "port": 18090 }),
        confirm,
    }
}

fn manifest_of(project_dir: &Path) -> String {
    fs::read_to_string(project_dir.join("forge.yaml")).expect("read the manifest")
}

#[test]
fn an_uninstalled_provider_is_reported_and_writes_nothing() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);

    let catalog = discover(&temp.path().join("packs"));
    let runner = AuthSetupRunner::new(catalog);
    let outcome = runner
        .run(&request(&project_dir, "demo-app", true))
        .expect("run the setup");

    match outcome {
        AuthSetupOutcome::Blocked { reason, .. } => assert_eq!(reason, "provider_unknown"),
        other => panic!("expected a blocked outcome, got {other:?}"),
    }
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn a_preview_names_the_changes_and_writes_nothing() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    let outcome = runner
        .run(&request(&project_dir, "demo-app", false))
        .expect("run the setup");

    match outcome {
        AuthSetupOutcome::Preview { plan, digest } => {
            assert!(digest.starts_with("sha256:"), "{digest}");
            assert!(!plan.proposed_changes.is_empty());
            assert!(plan
                .proposed_changes
                .iter()
                .any(|change| change.kind == "provider.realm"));
        }
        other => panic!("expected a preview outcome, got {other:?}"),
    }
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn a_confirmed_setup_records_the_population_in_the_manifest() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    let outcome = runner
        .run(&request(&project_dir, "demo-app", true))
        .expect("run the setup");

    match outcome {
        AuthSetupOutcome::Applied { population, .. } => {
            assert_eq!(population.client_id, "realm-demo-app");
            assert!(population.issuer.ends_with("/realms/realm-demo-app"));
        }
        other => panic!("expected an applied outcome, got {other:?}"),
    }

    let manifest = manifest_of(&project_dir);
    let parsed: serde_yaml::Value = serde_yaml::from_str(&manifest).expect("parse the manifest");
    let auth = parsed
        .get("auth")
        .expect("the manifest records the auth block");
    assert_eq!(
        auth.get("provider").and_then(|value| value.as_str()),
        Some("fake")
    );
    assert_eq!(
        auth.get("client_id").and_then(|value| value.as_str()),
        Some("realm-demo-app")
    );
    assert!(
        !manifest.contains("password"),
        "the manifest must not carry a secret"
    );
}

#[test]
fn a_repeated_setup_does_not_duplicate_the_manifest_entry() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    runner
        .run(&request(&project_dir, "demo-app", true))
        .expect("first run");
    let first = manifest_of(&project_dir);
    runner
        .run(&request(&project_dir, "demo-app", true))
        .expect("second run");
    let second = manifest_of(&project_dir);

    assert_eq!(
        first, second,
        "the repeated setup must not change the manifest"
    );
    assert_eq!(second.matches("provider: fake").count(), 1);
}

#[test]
fn a_provider_answer_for_another_project_is_refused() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    // The pack answers with a project id that is not the requested one.
    let packs = temp.path().join("packs");
    let script = fs::read_to_string(packs.join("fake-pack")).expect("read the pack");
    let tampered = script.replace(
        "\"project_id\":\"'\"$project_id\"'\"",
        "\"project_id\":\"someone-else\"",
    );
    fs::write(packs.join("fake-pack"), tampered).expect("write the pack");

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    let error = runner
        .run(&request(&project_dir, "demo-app", true))
        .expect_err("the mismatched answer must be refused");

    assert!(
        error.to_string().contains("another request"),
        "unexpected error: {error}"
    );
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn a_provider_answer_that_carries_a_secret_is_refused() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    let packs = fake_pack(
        temp.path(),
        "fake-pack",
        &apply_response("ready", ",\"secret\":\"hunter2\""),
    );

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    let error = runner
        .run(&request(&project_dir, "demo-app", true))
        .expect_err("the secret answer must be refused");

    assert!(
        error.to_string().contains("secret material"),
        "unexpected error: {error}"
    );
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn a_provider_failure_leaves_the_project_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("failed", ""));

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    let outcome = runner
        .run(&request(&project_dir, "demo-app", true))
        .expect("run the setup");

    match outcome {
        AuthSetupOutcome::Blocked { reason, .. } => assert_eq!(reason, "failed"),
        other => panic!("expected a blocked outcome, got {other:?}"),
    }
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn two_projects_get_two_separate_populations() {
    let temp = TempDir::new().expect("temp dir");
    let first = project(temp.path(), "alpha-app");
    let second = project(temp.path(), "beta-app");
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let catalog = discover(&packs);
    let runner = AuthSetupRunner::new(catalog);
    runner
        .run(&request(&first, "alpha-app", true))
        .expect("first setup");
    runner
        .run(&request(&second, "beta-app", true))
        .expect("second setup");

    let first_manifest = manifest_of(&first);
    let second_manifest = manifest_of(&second);
    assert!(first_manifest.contains("realm-alpha-app"));
    assert!(second_manifest.contains("realm-beta-app"));
    assert!(!first_manifest.contains("realm-beta-app"));
    assert!(!second_manifest.contains("realm-alpha-app"));
}

#[test]
fn the_cli_lists_the_installed_packs_without_touching_a_project() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let output = std::process::Command::new(forge_bin())
        .arg("auth")
        .arg("provider")
        .arg("list")
        .arg("--packs")
        .arg(packs.to_string_lossy().to_string())
        .output()
        .expect("run forge");

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("fake"), "unexpected output: {text}");
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn the_cli_previews_without_writing_the_manifest() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let output = std::process::Command::new(forge_bin())
        .arg("auth")
        .arg("setup")
        .arg(project_dir.to_string_lossy().to_string())
        .arg("--provider")
        .arg("fake")
        .arg("--packs")
        .arg(packs.to_string_lossy().to_string())
        .arg("--state-dir")
        .arg(temp.path().join("state").to_string_lossy().to_string())
        .arg("--port")
        .arg("18090")
        .output()
        .expect("run forge");

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("Confirm with --confirm"),
        "unexpected output: {text}"
    );
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn the_cli_refuses_a_provider_that_is_not_installed() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let before = manifest_of(&project_dir);

    let output = std::process::Command::new(forge_bin())
        .arg("auth")
        .arg("setup")
        .arg(project_dir.to_string_lossy().to_string())
        .arg("--provider")
        .arg("absent-provider")
        .arg("--packs")
        .arg(temp.path().join("packs").to_string_lossy().to_string())
        .arg("--state-dir")
        .arg(temp.path().join("state").to_string_lossy().to_string())
        .output()
        .expect("run forge");

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("provider_unknown"),
        "unexpected output: {text}"
    );
    assert_eq!(
        manifest_of(&project_dir),
        before,
        "the manifest must not change"
    );
}

#[test]
fn the_json_output_names_the_contract_and_the_status() {
    let temp = TempDir::new().expect("temp dir");
    let project_dir = project(temp.path(), "demo-app");
    let packs = fake_pack(temp.path(), "fake-pack", &apply_response("ready", ""));

    let output = std::process::Command::new(forge_bin())
        .arg("--format")
        .arg("json")
        .arg("auth")
        .arg("setup")
        .arg(project_dir.to_string_lossy().to_string())
        .arg("--provider")
        .arg("fake")
        .arg("--packs")
        .arg(packs.to_string_lossy().to_string())
        .arg("--state-dir")
        .arg(temp.path().join("state").to_string_lossy().to_string())
        .arg("--port")
        .arg("18090")
        .output()
        .expect("run forge");

    assert_eq!(output.status.code(), Some(0));
    let payload: Value = serde_json::from_slice(&output.stdout).expect("the output is JSON");
    assert_eq!(payload["contract"], "forge-auth-setup/1.0");
    assert_eq!(payload["status"], "preview");
    assert!(payload["plan_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

use forge::governance::{
    check_project, load_config, save_provider_selection, GovernanceStatus, ProviderStatus,
    LOCAL_PROVIDER_ID,
};
use std::fs;
use tempfile::TempDir;

fn valid_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("forge.yaml"),
        "schema: 1\nproject:\n  id: local-demo\n  name: Local Demo\n  profile: rust-web\n",
    )
    .unwrap();
    dir
}

#[test]
fn no_configuration_uses_the_local_provider() {
    let project = valid_project();

    let config = load_config(project.path()).unwrap();
    assert_eq!(config.selected_provider(), LOCAL_PROVIDER_ID);

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.provider, LOCAL_PROVIDER_ID);
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert_eq!(observation.project_id, "local-demo");
    assert_eq!(
        observation.project_path,
        project.path().display().to_string()
    );
}

#[test]
fn switching_provider_does_not_touch_the_manifest() {
    let project = valid_project();
    let before = fs::read(project.path().join("forge.yaml")).unwrap();

    save_provider_selection(
        project.path(),
        "external",
        Some("missing-adapter"),
        true,
        100,
    )
    .unwrap();

    let after = fs::read(project.path().join("forge.yaml")).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        load_config(project.path()).unwrap().selected_provider(),
        "external"
    );
}

#[test]
fn missing_external_provider_is_unavailable_but_local_files_remain_usable() {
    let project = valid_project();
    save_provider_selection(
        project.path(),
        "external",
        Some("missing-adapter"),
        true,
        100,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Unavailable);
    assert!(observation.detail.unwrap().contains("missing-adapter"));
}

#[test]
fn disabled_external_provider_is_not_invoked() {
    let project = valid_project();
    save_provider_selection(
        project.path(),
        "external",
        Some("missing-adapter"),
        false,
        100,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Disabled);
}

#[test]
fn malformed_external_response_is_incompatible() {
    let project = valid_project();
    let adapter = project.path().join("adapter.sh");
    fs::write(&adapter, "#!/bin/sh\nprintf 'not-json'\n").unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        "external",
        Some(adapter.to_str().unwrap()),
        true,
        1000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Incompatible);
}

#[test]
fn unknown_external_status_is_incompatible() {
    let project = valid_project();
    let adapter = project.path().join("adapter.sh");
    fs::write(
        &adapter,
        r#"#!/bin/sh
printf '%s' '{"provider":"external","protocol_version":"0.1.0","project_id":"local-demo","status":"healthy"}'
"#,
    )
    .unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        "external",
        Some(adapter.to_str().unwrap()),
        true,
        1000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Incompatible);
}

#[test]
fn valid_external_response_is_normalized_and_redacted() {
    let project = valid_project();
    let adapter = project.path().join("adapter.sh");
    fs::write(
        &adapter,
        r#"#!/bin/sh
printf '%s' '{"provider":"external","protocol_version":"0.1.0","project_id":"local-demo","status":"pass","evidence":["token=ghp_123456789012345678901234567890123456"],"detail":"ok"}'
"#,
    )
    .unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        "external",
        Some(adapter.to_str().unwrap()),
        true,
        1000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert!(observation.evidence[0].contains("[REDACTED]"));
    assert!(!observation.evidence[0].contains("ghp_"));
}

#[test]
fn status_rendering_marks_external_failure_as_not_healthy() {
    let status = GovernanceStatus::from(ProviderStatus::Unavailable);
    assert!(!status.is_healthy());
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}

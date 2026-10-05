use forge::core::ForgeError;
use forge::governance::{
    check_project, load_config, resolve_known_adapter, save_provider_selection,
    save_provider_selection_with_root, GovernanceStatus, ProviderStatus, LOCAL_PROVIDER_ID,
    WORKSPACE_GOVERNANCE_PROVIDER_ID,
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
        5_000,
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
        5_000,
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
        5_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert!(observation.evidence[0].contains("[REDACTED]"));
    assert!(!observation.evidence[0].contains("ghp_"));
}

#[test]
fn an_adapter_that_never_reads_its_request_still_answers() {
    // An adapter is an external executable Forge does not own, and one is
    // entitled to answer without reading the request: it closes its own
    // standard input before answering. Forge must report what that adapter
    // actually answered, not `unavailable` because its request write lost the
    // race against the adapter's exit.
    let project = valid_project();
    let adapter = project.path().join("silent.sh");
    fs::write(
        &adapter,
        r#"#!/bin/sh
exec 0<&-
printf '%s' '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"local-demo","status":"pass","evidence":["adoption=adopted"],"detail":"answered without reading the request"}'
"#,
    )
    .unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(adapter.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert_eq!(observation.evidence, ["adoption=adopted"]);
}

#[test]
fn status_rendering_marks_external_failure_as_not_healthy() {
    let status = GovernanceStatus::from(ProviderStatus::Unavailable);
    assert!(!status.is_healthy());
}

#[test]
fn preset_selects_packaged_candidate_and_persists_absolute_path() {
    let project = valid_project();
    let before = fs::read(project.path().join("forge.yaml")).unwrap();
    let workspace = stage_workspace("adopted-clean.sh");

    let resolved = resolve_known_adapter(
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(workspace.path()),
        None,
    )
    .unwrap()
    .unwrap();
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(resolved.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    // Selection is persistence-only on the provider config: the manifest
    // survives byte-identical and the stored path is absolute.
    assert_eq!(fs::read(project.path().join("forge.yaml")).unwrap(), before);
    let config = load_config(project.path()).unwrap();
    let stored = config.provider.clone().unwrap();
    assert_eq!(stored.provider, WORKSPACE_GOVERNANCE_PROVIDER_ID);
    assert_eq!(
        stored.adapter.as_deref().unwrap(),
        resolved.to_str().unwrap()
    );
    assert!(resolved.is_absolute());

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.provider, WORKSPACE_GOVERNANCE_PROVIDER_ID);
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert!(GovernanceStatus::from(observation.status).is_healthy());
    // The adapter-reported revision passes through the v0.1.0 boundary.
    assert_eq!(
        observation.source_revision.as_deref(),
        Some("888d8058e7a2615e0c6525a533463e5029d56487")
    );
}

#[test]
fn audit_shaped_observations_map_to_normalized_statuses() {
    let cases = [
        (
            "adopted-clean.sh",
            ProviderStatus::Pass,
            "adoption=adopted",
            Some("888d8058e7a2615e0c6525a533463e5029d56487"),
        ),
        (
            "error-findings.sh",
            ProviderStatus::Fail,
            "DECLARATION_MISSING: adopted project must contain .project.json",
            Some("916f794204be9a26e20680dbd99527cde4afe14c"),
        ),
        (
            "adoption-gap.sh",
            ProviderStatus::Blocked,
            "adoption=unknown",
            Some("1c4177c5fdecaac837cea22f9a56adb31774c0cf"),
        ),
        (
            "unregistered.sh",
            ProviderStatus::Unknown,
            "PROJECT_UNKNOWN: project not found in registry: local-demo",
            None,
        ),
    ];
    for (fixture, expected_status, expected_evidence_or_detail, expected_revision) in cases {
        let project = valid_project();
        save_provider_selection(
            project.path(),
            WORKSPACE_GOVERNANCE_PROVIDER_ID,
            Some(fixture_path(fixture).to_str().unwrap()),
            true,
            10_000,
        )
        .unwrap();

        let observation = check_project(project.path()).unwrap();
        assert_eq!(
            observation.provider, WORKSPACE_GOVERNANCE_PROVIDER_ID,
            "{fixture}"
        );
        assert_eq!(observation.status, expected_status, "{fixture}");
        assert_eq!(
            GovernanceStatus::from(observation.status).is_healthy(),
            expected_status == ProviderStatus::Pass,
            "{fixture}"
        );
        let rendered = format!(
            "{} {:?}",
            observation.detail.clone().unwrap_or_default(),
            observation.evidence
        );
        assert!(
            rendered.contains(expected_evidence_or_detail),
            "{fixture}: {rendered}"
        );
        assert_eq!(
            observation.source_revision.as_deref(),
            expected_revision,
            "{fixture}"
        );
    }
}

#[test]
fn workspace_governance_evidence_is_redacted_and_bounded() {
    let project = valid_project();
    let adapter = project.path().join("noisy.sh");
    let long = "x".repeat(5_000);
    fs::write(
        &adapter,
        format!(
            "#!/bin/sh\nprintf '%s' '{{\"provider\":\"workspace-governance\",\"protocol_version\":\"0.1.0\",\"project_id\":\"local-demo\",\"status\":\"pass\",\"evidence\":[\"token=ghp_123456789012345678901234567890123456\",\"{long}\"],\"detail\":\"password: hunter2 {long}\"}}'\n"
        ),
    )
    .unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(adapter.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert!(observation.evidence[0].contains("[REDACTED]"));
    assert!(!observation.evidence[0].contains("ghp_"));
    assert!(observation.evidence[1].chars().count() <= 2_000);
    let detail = observation.detail.unwrap();
    assert!(detail.contains("[REDACTED]") && !detail.contains("hunter2"));
    assert!(detail.chars().count() <= 2_000);
}

#[test]
fn configured_sibling_removed_after_selection_is_unavailable() {
    let project = valid_project();
    let workspace = stage_workspace("adopted-clean.sh");
    let resolved = resolve_known_adapter(
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(workspace.path()),
        None,
    )
    .unwrap()
    .unwrap();
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(resolved.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();
    assert_eq!(
        check_project(project.path()).unwrap().status,
        ProviderStatus::Pass
    );

    // The sibling checkout disappears; every local artifact survives and
    // the plane classifies unavailable — never a PASS.
    fs::remove_file(&resolved).unwrap();
    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Unavailable);
    assert!(!GovernanceStatus::from(observation.status).is_healthy());
    let detail = observation.detail.unwrap();
    assert!(detail.chars().count() <= 2_000, "detail must stay bounded");
    assert_eq!(
        load_config(project.path()).unwrap().selected_provider(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID
    );
    let manifest = String::from_utf8(fs::read(project.path().join("forge.yaml")).unwrap()).unwrap();
    assert!(manifest.contains("local-demo"));
}

#[test]
fn known_provider_without_adapter_and_without_root_stays_no_adapter() {
    // Selection-time presets never rewrite evaluation semantics: a stored
    // selection without an adapter still reports "no adapter configured".
    let project = valid_project();
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        None,
        true,
        10_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Unavailable);
    assert!(observation
        .detail
        .unwrap()
        .contains("no adapter configured"));
}

#[test]
fn preset_selects_packaged_candidate_and_persists_workspace_root() {
    let project = valid_project();
    let workspace = stage_workspace("adopted-clean.sh");

    let resolved = resolve_known_adapter(
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(workspace.path()),
        None,
    )
    .unwrap()
    .unwrap();
    let root = workspace.path().canonicalize().unwrap();
    save_provider_selection_with_root(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(resolved.to_str().unwrap()),
        Some(root.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let config = load_config(project.path()).unwrap();
    let stored = config.provider.clone().unwrap();
    assert_eq!(
        stored.workspace_root.as_deref(),
        Some(root.to_str().unwrap())
    );

    // The evaluation still succeeds and the stored root survives reload.
    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Pass);
    assert_eq!(
        load_config(project.path())
            .unwrap()
            .provider
            .unwrap()
            .workspace_root
            .as_deref(),
        Some(root.to_str().unwrap())
    );
}

#[test]
fn stored_workspace_root_is_passed_to_the_adapter_environment() {
    // The root a preset was resolved from is handed to the adapter as
    // WORKSPACE_ROOT, so a later check with the host environment absent
    // reaches the same audit root the operator configured.
    let project = valid_project();
    let workspace = TempDir::new().unwrap();
    let candidate = workspace
        .path()
        .join("workspace-governance/scripts/forge_governance_adapter.py");
    fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    fs::write(
        &candidate,
        r#"#!/bin/sh
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"pass","evidence":["workspace=%s"],"detail":"env"}' "$pid" "${WORKSPACE_ROOT:-UNSET}"
"#,
    )
    .unwrap();
    make_executable(&candidate);
    let root = workspace.path().canonicalize().unwrap();
    save_provider_selection_with_root(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(candidate.to_str().unwrap()),
        Some(root.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.status, ProviderStatus::Pass);
    let expected = format!("workspace={}", root.display());
    assert_eq!(observation.evidence[0], expected);
    assert_ne!(observation.evidence[0], "workspace=UNSET");
}

#[test]
fn selection_without_root_passes_no_workspace_environment() {
    // Explicit adapters keep the prior, unchanged behavior: the adapter
    // inherits only the process environment Forge already had.
    let project = valid_project();
    let adapter = project.path().join("env.sh");
    fs::write(
        &adapter,
        r#"#!/bin/sh
req=$(cat)
pid=$(printf '%s' "$req" | sed -n 's/.*"project_id":"\([^"]*\)".*/\1/p')
printf '{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"%s","status":"pass","evidence":["workspace=%s"],"detail":"env"}' "$pid" "${WORKSPACE_ROOT:-UNSET}"
"#,
    )
    .unwrap();
    make_executable(&adapter);
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(adapter.to_str().unwrap()),
        true,
        10_000,
    )
    .unwrap();

    let observation = check_project(project.path()).unwrap();
    assert_eq!(observation.evidence[0], "workspace=UNSET");
}

#[test]
fn local_provider_rejects_a_workspace_root() {
    let project = valid_project();
    let refusal = save_provider_selection_with_root(
        project.path(),
        LOCAL_PROVIDER_ID,
        None,
        Some("/somewhere"),
        true,
        10_000,
    )
    .unwrap_err();
    assert!(matches!(refusal, ForgeError::GovernanceInvalid { .. }));
    assert!(refusal
        .to_string()
        .contains("does not accept a workspace root"));
}

#[test]
fn relative_workspace_root_is_refused_at_persistence() {
    let project = valid_project();
    let refusal = save_provider_selection_with_root(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some("/abs/adapter"),
        Some("relative/root"),
        true,
        10_000,
    )
    .unwrap_err();
    assert!(refusal.to_string().contains("must be an absolute path"));
}

#[test]
fn legacy_selection_without_root_still_loads() {
    // serde(default) keeps every pre-change providers.yaml file loadable.
    let project = valid_project();
    let dir = project.path().join(".forge");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("providers.yaml"),
        "provider:\n  provider: workspace-governance\n  adapter: /abs/path.py\n  enabled: true\n  protocol_version: 0.1.0\n  timeout_ms: 10000\n",
    )
    .unwrap();
    let config = load_config(project.path()).unwrap();
    assert_eq!(config.provider.unwrap().workspace_root, None);
}

#[test]
fn unresolved_preset_never_disturbs_the_stored_selection() {
    let project = valid_project();
    save_provider_selection(
        project.path(),
        "external",
        Some("previous-adapter"),
        true,
        10_000,
    )
    .unwrap();
    let before = fs::read(project.path().join(".forge/providers.yaml")).unwrap();

    // The refusal happens at resolution time, before any save.
    let refusal =
        resolve_known_adapter(WORKSPACE_GOVERNANCE_PROVIDER_ID, Some(project.path()), None)
            .unwrap()
            .unwrap_err();
    assert!(matches!(refusal, ForgeError::GovernanceInvalid { .. }));

    let after = fs::read(project.path().join(".forge/providers.yaml")).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        load_config(project.path()).unwrap().selected_provider(),
        "external"
    );
}

/// Copy an audit-shaped fixture into a fresh workspace root at the
/// packaged candidate path with the execute bit set.
fn stage_workspace(fixture: &str) -> TempDir {
    let workspace = TempDir::new().unwrap();
    let candidate = workspace
        .path()
        .join("workspace-governance/scripts/forge_governance_adapter.py");
    fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    fs::copy(fixture_path(fixture), &candidate).unwrap();
    make_executable(&candidate);
    workspace
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/governance-audit")
        .join(name)
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

/// An adapter that writes `bytes` of padding on stdout, inside a
/// `forge.governance.observation/0.1.0`-shaped JSON response.
///
/// `padding` lands in `metadata` so the test can assert the **whole** payload
/// crossed the pipe: an implementation that truncated instead of draining would
/// report a short string and fail.
#[cfg(unix)]
fn write_padding_adapter(path: &std::path::Path, bytes: usize) {
    fs::write(
        path,
        format!(
            r#"#!/bin/sh
req=$(cat)
printf '{{"provider":"workspace-governance","protocol_version":"0.1.0","project_id":"local-demo","status":"pass","evidence":["emitted-bytes={bytes}"],"metadata":{{"pad":"'
head -c {bytes} /dev/zero | tr '\0' 'x'
printf '"}}}}'
"#
        ),
    )
    .unwrap();
    make_executable(path);
}

/// An adapter that writes more than the kernel's 64 KiB pipe buffer still
/// answers.
///
/// This is the guard for the deadlock the boundary used to have. Before the
/// drain, Forge read a pipe only after the adapter exited, so an adapter that
/// wrote more than one pipe buffer blocked in `write(2)` on **every** run and
/// was reported `unavailable` with `adapter exceeded timeout`. It is
/// arithmetic, not a race, so the test could not pass before the fix.
///
/// The `timeout_ms` of 5 s is also the bound that keeps this guard from
/// hanging the suite: a regression costs 5 s and a failed assertion, never a
/// stuck run.
#[cfg(unix)]
#[test]
fn an_adapter_that_writes_more_than_one_pipe_buffer_still_answers() {
    const PADDING: usize = 200 * 1024; // 3x the 64 KiB pipe buffer, under the 256 KiB cap
    let project = valid_project();
    let adapter = project.path().join("chatty.sh");
    write_padding_adapter(&adapter, PADDING);
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(adapter.to_str().unwrap()),
        true,
        5_000,
    )
    .unwrap();

    let started = std::time::Instant::now();
    let observation = check_project(project.path()).unwrap();
    let elapsed = started.elapsed();

    assert_eq!(observation.status, ProviderStatus::Pass);
    assert_eq!(observation.evidence, [format!("emitted-bytes={PADDING}")]);
    assert_eq!(
        observation.metadata["pad"].as_str().map(str::len),
        Some(PADDING),
        "the whole payload must cross the pipe, not a truncated prefix"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "answering took {elapsed:?}: that is the deadline, not a drained read"
    );
}

/// Draining a pipe must not become buffering it: past the output cap, Forge
/// still refuses with the typed cap error rather than accepting the payload or
/// waiting for the deadline.
#[cfg(unix)]
#[test]
fn an_adapter_that_writes_past_the_output_cap_is_refused() {
    const PADDING: usize = 320 * 1024; // past MAX_ADAPTER_OUTPUT_BYTES
    let project = valid_project();
    let adapter = project.path().join("flooding.sh");
    write_padding_adapter(&adapter, PADDING);
    save_provider_selection(
        project.path(),
        WORKSPACE_GOVERNANCE_PROVIDER_ID,
        Some(adapter.to_str().unwrap()),
        true,
        5_000,
    )
    .unwrap();

    let err = check_project(project.path()).unwrap_err();
    let text = err.to_string();
    assert!(
        text.contains("adapter stdout exceeds 262144 bytes"),
        "expected the typed output-cap refusal, got: {text}"
    );
    assert!(
        matches!(err, ForgeError::GovernanceInvalid { .. }),
        "{err:?}"
    );
}

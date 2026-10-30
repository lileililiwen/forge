use super::*;
use std::collections::HashSet;

#[test]
fn matrix_covers_all_supported_profiles_in_catalog_order() {
    let ids = matrix_profile_ids();
    let expected: Vec<String> = list_profiles().iter().map(|p| p.id.clone()).collect();
    assert_eq!(ids, expected);
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
}

#[test]
fn strip_forge_removes_only_the_forge_directory() {
    let forge_dir = Path::new("/opt/forge/bin");
    let raw = std::env::join_paths([
        Path::new("/opt/forge/bin"),
        Path::new("/usr/local/cargo/bin"),
        Path::new("/usr/bin"),
    ])
    .unwrap();
    let stripped = strip_forge_from_path(&raw, forge_dir);
    let kept: Vec<PathBuf> = std::env::split_paths(&stripped).collect();
    assert_eq!(
        kept,
        vec![Path::new("/usr/local/cargo/bin"), Path::new("/usr/bin")]
    );
    let untouched = strip_forge_from_path(&raw, Path::new("/nowhere"));
    assert_eq!(untouched, raw);
}

#[test]
fn source_hash_is_stable_and_content_sensitive() {
    let files = vec![
        ("b.txt".to_string(), "two".to_string()),
        ("a.txt".to_string(), "one".to_string()),
    ];
    let first = source_hash_for(&files);
    let reordered = vec![
        ("a.txt".to_string(), "one".to_string()),
        ("b.txt".to_string(), "two".to_string()),
    ];
    assert_eq!(first, source_hash_for(&reordered));
    assert_eq!(first.len(), 64);
    let changed = vec![
        ("a.txt".to_string(), "one!".to_string()),
        ("b.txt".to_string(), "two".to_string()),
    ];
    assert_ne!(first, source_hash_for(&changed));
}

#[test]
fn forge_resolution_probe_detects_a_forge_on_path() {
    assert!(!forge_resolves_on_path(OsStr::new("")));
    let tmp = tempfile::TempDir::new().unwrap();
    let empty = std::env::join_paths([tmp.path()]).unwrap();
    assert!(!forge_resolves_on_path(&empty));
    let name = format!("forge{}", std::env::consts::EXE_SUFFIX);
    fs::write(tmp.path().join(name), b"fake").unwrap();
    let seeded = std::env::join_paths([tmp.path()]).unwrap();
    assert!(forge_resolves_on_path(&seeded));
}

#[test]
fn readiness_fixture_includes_the_committed_feed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let target = tmp.path().join("readiness-aspnet-web");
    let request = normalize_explicit(
        Some("aspnet-web"),
        Some("readiness-aspnet-web"),
        None,
        &[],
        &target,
        None,
    )
    .unwrap();
    let feed = render_feed_files(&request).unwrap();
    let names: Vec<&str> = feed.iter().map(|(rel, _)| rel.as_str()).collect();
    assert!(
        feed.iter()
            .any(|(rel, bytes)| rel.starts_with("packages/platform-feed/")
                && rel.ends_with(".nupkg")
                && !bytes.is_empty()),
        "an aspnet-web fixture must carry the committed NuGet feed: {names:?}"
    );
}

#[test]
fn unknown_profile_is_refused_before_any_fixture() {
    let err = run_profile_row("nosuch-profile", None).expect_err("unknown id must fail");
    assert_eq!(err.code(), "unknown-profile");
}

#[test]
fn planned_profile_is_refused_before_any_fixture() {
    let err = run_profile_row("rust-cli", None).expect_err("planned id must fail");
    assert_eq!(err.code(), "unsupported-profile");
}

#[test]
fn missing_toolchain_yields_unverified_never_passing() {
    let empty: HashSet<String> = HashSet::new();
    let row = run_profile_row("rust-web", Some(&empty)).expect("row must be returned");
    assert_eq!(row.result, ReadinessStatus::Unverified);
    assert!(row.build.is_none());
    assert!(row.test.is_none());
    assert!(row.toolchain_version.is_none());
    assert!(!row.source_hash.is_empty());
    assert!(row.reason.contains("cargo"));
    let report = summarize_matrix(vec![row]);
    assert!(!report.ready);
    assert_eq!(report.unverified, 1);
}

#[test]
fn matrix_filter_dedupes_and_refuses_unknown_ids() {
    let err = run_matrix(&["rust-web".to_string(), "nosuch".to_string()])
        .expect_err("unknown filter id must fail");
    assert_eq!(err.code(), "unknown-profile");
    let err = run_matrix(&["rust-cli".to_string()]).expect_err("planned filter must fail");
    assert_eq!(err.code(), "unsupported-profile");
}

#[test]
fn artifact_evidence_carries_version_and_checksum() {
    // Under `cargo test` the running executable is the harness, not
    // the artifact: target the sibling `forge` binary when present,
    // otherwise skip (the CLI contract test covers the shape).
    let harness = std::env::current_exe().expect("test harness path");
    let candidate = harness
        .parent()
        .and_then(|deps| deps.parent())
        .map(|debug| debug.join(format!("forge{}", std::env::consts::EXE_SUFFIX)));
    let binary = match candidate {
        Some(path) if path.is_file() => path,
        _ => return,
    };
    let evidence = artifact_evidence_for(&binary).expect("forge binary is a readable artifact");
    assert_eq!(evidence.contract, READINESS_CONTRACT_VERSION);
    assert_eq!(evidence.sha256.len(), 64);
    assert!(!evidence.binary_path.is_empty());
    assert!(!evidence.version_smoke.trim().is_empty());
}

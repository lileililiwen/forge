use forge::ci_evidence::{HostedCiEvidenceEnvelope, HostedCiRun};
use forge::gate::{GateAggregate, GateCheckState, GateEvidence};
use forge::hosted_ci::{
    bind_repository_id, ingest, observations, repository_binding, GitHubActionsClaims, IngestError,
    IngestOutcome,
};
use forge::registry::Registry;
use std::fs;
use std::path::Path;
use std::process::Command;

fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn project(root: &Path, db: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("forge.yaml"), "schema: 1\nproject:\n  id: sample-service\n  name: Sample Service\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n").unwrap();
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.email", "forge@example.test"]);
    run_git(root, &["config", "user.name", "Forge Test"]);
    run_git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/owner/repository.git",
        ],
    );
    let mut registry = Registry::open(db).unwrap();
    registry.register(root, None).unwrap();
}

fn envelope() -> HostedCiEvidenceEnvelope {
    let sha = "a".repeat(40);
    HostedCiEvidenceEnvelope {
        contract: "forge-hosted-ci-evidence/1.0".into(),
        project_id: "sample-service".into(),
        repository: "owner/repository".into(),
        run: HostedCiRun {
            provider: "github-actions".into(),
            workflow: "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main".into(),
            run_id: "123456789".into(),
            run_attempt: 1,
            job: "upload-hosted-evidence".into(),
            url: "https://github.com/owner/repository/actions/runs/123456789".into(),
        },
        source_revision: sha.clone(),
        gate: GateEvidence {
            contract: forge::gate::GATE_CONTRACT_VERSION.into(),
            project_id: "sample-service".into(),
            runtime: "forge".into(),
            profile: forge::gate::GateProfile::Full,
            runtime_version: Some("0.1.0".into()),
            revision: Some(sha),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: vec![forge::gate::GateCheck {
                id: "baseline".into(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-10-18T10:00:00Z".into(),
            dry_run: false,
            note: None,
        },
        observed_at: "2026-10-18T10:01:00Z".into(),
    }
}

fn claims() -> GitHubActionsClaims {
    GitHubActionsClaims {
        repository: "owner/repository".into(),
        repository_id: "12345".into(),
        run_id: "123456789".into(),
        run_attempt: "1".into(),
        workflow_ref: "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main".into(),
        sha: "a".repeat(40),
        event_name: "push".into(),
        git_ref: "refs/heads/main".into(),
    }
}

#[test]
fn binding_is_explicit_immutable_and_repository_ids_are_unique() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("forge.db");
    project(&tmp.path().join("project"), &db);
    assert_eq!(
        bind_repository_id(&db, "sample-service", "12345").unwrap(),
        "owner/repository"
    );
    assert_eq!(
        repository_binding(&db, "sample-service").unwrap(),
        Some(("owner/repository".into(), "12345".into()))
    );
    assert_eq!(
        bind_repository_id(&db, "sample-service", "12345").unwrap(),
        "owner/repository"
    );
    assert!(matches!(
        bind_repository_id(&db, "sample-service", "54321"),
        Err(IngestError::Conflict(_))
    ));
    assert!(bind_repository_id(&db, "sample-service", "000").is_err());
}

#[test]
fn opening_an_existing_registry_adds_missing_hosted_ci_tables() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("forge.db");
    drop(Registry::open(&db).unwrap());
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute("DROP INDEX hosted_ci_evidence_project_received_idx", [])
        .unwrap();
    connection
        .execute("DROP TABLE hosted_ci_evidence", [])
        .unwrap();
    connection
        .execute("DROP TABLE hosted_ci_project_bindings", [])
        .unwrap();
    drop(connection);

    drop(Registry::open(&db).unwrap());
    let connection = rusqlite::Connection::open(&db).unwrap();
    for table in ["hosted_ci_project_bindings", "hosted_ci_evidence"] {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "missing migrated table {table}");
    }
}

#[test]
fn ingest_is_append_only_idempotent_and_conflicting_replay_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("forge.db");
    project(&tmp.path().join("project"), &db);
    bind_repository_id(&db, "sample-service", "12345").unwrap();
    let first = match ingest(&db, envelope(), &claims()).unwrap() {
        IngestOutcome::Inserted(record) => record,
        IngestOutcome::Replayed(_) => panic!("first record was not inserted"),
    };
    let replay = match ingest(&db, envelope(), &claims()).unwrap() {
        IngestOutcome::Replayed(record) => record,
        IngestOutcome::Inserted(_) => panic!("retry inserted another record"),
    };
    assert_eq!(first.id, replay.id);
    assert_eq!(observations(&db, "sample-service", 20).unwrap().len(), 1);

    let mut changed = envelope();
    changed.gate.note = Some("changed report".into());
    assert!(matches!(
        ingest(&db, changed, &claims()),
        Err(IngestError::Conflict(_))
    ));
    assert_eq!(observations(&db, "sample-service", 20).unwrap()[0], first);
}

#[test]
fn simultaneous_identical_uploads_create_one_observation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("forge.db");
    project(&tmp.path().join("project"), &db);
    bind_repository_id(&db, "sample-service", "12345").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let outcomes = std::thread::scope(|scope| {
        (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                let db = db.clone();
                scope.spawn(move || {
                    barrier.wait();
                    ingest(&db, envelope(), &claims()).unwrap()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes.len(), 2);
    assert!(
        matches!(&outcomes[0], IngestOutcome::Inserted(_))
            ^ matches!(&outcomes[1], IngestOutcome::Inserted(_))
    );
    let records = observations(&db, "sample-service", 20).unwrap();
    assert_eq!(records.len(), 1);
    assert!(outcomes.iter().all(|outcome| match outcome {
        IngestOutcome::Inserted(record) | IngestOutcome::Replayed(record) =>
            record.id == records[0].id,
    }));
}

#[test]
fn ingest_requires_a_binding_and_exact_claim_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("forge.db");
    project(&tmp.path().join("project"), &db);
    assert!(matches!(
        ingest(&db, envelope(), &claims()),
        Err(IngestError::Invalid(_))
    ));
    bind_repository_id(&db, "sample-service", "12345").unwrap();
    let mut wrong_claims = claims();
    wrong_claims.sha = "b".repeat(40);
    assert!(matches!(
        ingest(&db, envelope(), &wrong_claims),
        Err(IngestError::Invalid(_))
    ));
    assert!(observations(&db, "sample-service", 20).unwrap().is_empty());
}

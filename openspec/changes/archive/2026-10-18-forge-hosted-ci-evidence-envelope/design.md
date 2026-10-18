# Design: hosted CI evidence envelope

## Architecture

Add `src/ci_evidence.rs` and export it from `src/lib.rs`. It is a Forge Core contract module and depends only on `crate::gate::GateEvidence`, Serde and existing `ForgeError` conventions. The schema is owned by the same module and serialized with a fixed contract discriminator. No sibling crate or repository is consumed.

```json
{
  "contract": "forge-hosted-ci-evidence/1.0",
  "project_id": "sample-service",
  "repository": "owner/repository",
  "run": {
    "provider": "github-actions",
    "workflow": "ci.yml",
    "run_id": "123456789",
    "run_attempt": 1,
    "job": "gate",
    "url": "https://github.com/owner/repository/actions/runs/123456789"
  },
  "source_revision": "<40 lowercase hex SHA>",
  "gate": { "contract": "0.1.0", "...": "existing GateEvidence fields" },
  "observed_at": "RFC3339 UTC timestamp"
}
```

`HostedCiEvidenceEnvelope::validate()` returns a typed error and enforces:

- exact envelope and embedded Gate contract versions;
- valid Forge project id; normalized `owner/repository` slug;
- lowercase full 40-character Git SHA, exactly equal to `gate.revision`;
- GitHub Actions provider, nonempty bounded workflow/job, decimal run ID, positive attempt;
- URL is HTTPS, host exactly `github.com`, and path matches the repository and run ID; reject credentials, query, fragment, alternate hosts and malformed path;
- RFC3339 timestamp; embedded Gate evidence has `dry_run == false`, matching project/revision, and a terminal aggregate (passed, blocked or failed; `unknown` is rejected as non-verdict evidence);
- bounded collection/string sizes and no unknown fields in envelope/run structs, preventing accidental credential/log payload growth.

The aggregate is a producer-reported Gate result. Validation proves internal consistency and format only. It does not prove GitHub authored the document, that the named repo belongs to the project, or that the runner result is current/trusted. Future ingestion must authenticate separately and match authenticated repository identity, registered project mapping, exact SHA, run identity, and freshness policy before storing or exposing an accepted observation. A passing uploaded record must not overwrite the local Gate evidence file or silently satisfy delivery preflight.

## Contract evolution

The contract identifier is immutable. Incompatible shape or semantics require a new major identifier; additive optional fields may be introduced only under a documented compatibility decision and validator tests. `src/ci_evidence.rs` is the maintained source of truth; `docs/ci-cd-system-model.md` references it. No duplicate JSON Schema is introduced until a non-Rust consumer needs one; the Rust serde model and fixtures provide the contract now.

## Errors and limits

Validation returns a typed `ForgeError::InvalidInput` with a bounded field-specific message. Deserialization rejects unknown fields and invalid enum/string forms. The validator never logs or returns the input body. It does not redact and retain arbitrary metadata: there are no arbitrary metadata, environment, logs, or token fields in this contract.

## UI/UX

`UI/UX: N/A` for the infrastructure contract only. There is no human actor or workflow in this package. The named consuming package is `forge-hosted-ci-evidence-ingest`, which must provide the authenticated workbench journey before hosted evidence becomes operator-facing product behavior.

## Verification

- Unit tests validate a full fixture and JSON round trip.
- Negative tests reject unknown versions/fields, wrong project/revision, SHA format/mismatch, dry-run, unknown aggregate, malformed run id/attempt, URL host/path/credential/query/fragment violations, and malformed timestamps.
- Existing Gate evidence serialization and tests remain unchanged.
- Strict OpenSpec validation checks the canonical requirement after archive.

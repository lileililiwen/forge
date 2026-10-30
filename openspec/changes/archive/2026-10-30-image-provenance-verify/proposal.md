# Change: image-provenance-verify

## Why

`forge build --image` writes a provenance record, and nothing reads it. The
record is currently write-only evidence: a Gate run, a release or an operator
cannot ask "is the image this revision produced still the image this record
claims", and a record left over from an earlier revision, a different project,
or an SBOM edited after the build reads exactly like a fresh one.

That is the gap the previous change named as its follow-up. The Gate already
binds a verdict to a project and a source revision; what it lacks is a stable,
non-interactive way to consume the record. A Gate check must be a process that
exits zero or non-zero and writes a bounded reason — it cannot reach into
Forge's types.

The missing piece is therefore a verify verb, not a new Gate engine: read the
record for the revision the Gate is checking, prove its identity, re-hash the
SBOM and fail when anything has drifted. A project binds it as a bound check
with a one-line script, the same way it binds any other declared check.

## What Changes

- **`forge build --verify-image` becomes the read-only verification.** It
  resolves the project's committed revision, reads
  `.forge/build/<project-id>/<sha12>.json`, and checks the record's contract,
  project, revision, digest shape and SBOM hash. It writes nothing.
- **A stale or foreign record is a refusal.** A record for another revision,
  another project, an unsupported contract or a digest that is not a `sha256`
  digest is `build-invalid`; an SBOM whose bytes no longer match the recorded
  hash is `build-failed`. The sentences name what drifted and the recovery
  (`forge build --image`).
- **The record becomes bindable as a Gate check.** The contract document states
  the two-line opt-in: a declared `image-provenance` check with
  `source: bound` and a script that runs `forge build . --verify-image`. Forge
  ships no default check and no pack change, so no existing project's Gate is
  widened.
- **The verb is exclusive.** `--verify-image` with `--image`, `--tag`,
  `--builder-bin`, `--os`, `--arch` or `--rid` is `build-invalid`: it verifies
  one record and starts no builder.
- **Nothing is recorded.** Verification returns the record it trusted; it does
  not rewrite it, does not write Gate evidence and does not invoke a provider.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/image/mod.rs` | `verify_image_build`, reusing the record, digest and SBOM rules |
| `src/cli/gitops.rs`, `src/main.rs` | `--verify-image` and its refusals |
| `docs/adapter-contracts/image-builder.md` | verification and the Gate opt-in |
| `README.md` | one line on the verify flag |
| `docs/ci-cd-system-model.md` | the record is consumable by a bound check |
| Existing image build tests | unchanged; verification is read-only |

Persistence: none. Verification writes no file and no registry row.

Failure boundaries: a missing record is `build-invalid` with the revision and
the recovery; a foreign or unsupported record is `build-invalid`; a changed
SBOM or a changed digest is `build-failed`. No builder is resolved or run.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `image-provenance-verify` | added | this package |
| `forge-build-image` | read only | its record shape is consumed, not changed |
| `gate-runtime-evidence` | read only | the verb is a normal bound command; the Gate engine is untouched |
| `forge-baseline` rule pack | read only | no check is added to the baseline |

## Non-Goals

- No new Gate check id, no baseline or pack change, and no Gate engine change.
  A project opts in; Forge does not widen every project's Gate.
- No registry read, no remote registry query and no re-pull of the image. The
  record and the local SBOM are the evidence.
- No signature verification. The record is Forge's own observation, not a
  signed attestation.
- No repair. A drifted record is refused; the operator rebuilds.

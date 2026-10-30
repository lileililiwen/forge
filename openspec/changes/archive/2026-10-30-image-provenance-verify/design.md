## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The record reader and its checks | `forge` | Yes. `src/image/`. |
| The CLI flag and its refusals | `forge` | Yes. `src/main.rs`, `src/cli/gitops.rs`. |
| The contract document and the Gate opt-in | `forge` | Yes. |
| The Gate engine, the baseline and the packs | `forge` | No. Untouched. |
| The project's `forge.yaml` check declaration | the project | No. The opt-in is documented, not imposed. |
| Image signing, registry access | external | No. |

## What Verification Proves

`forge build --verify-image` answers one question with the evidence Forge
already has: *does the record for the revision at `HEAD` still describe a build
that this project produced, and is the SBOM it names still the file it hashed?*

| Check | Rule | Code when it fails |
|---|---|---|
| the record exists for `HEAD` | `.forge/build/<project-id>/<sha12>.json` | `build-invalid` |
| the record is readable and typed | the `forge-build-image` document | `build-invalid` |
| the contract is compatible | family `forge-build-image`, major `0` | `build-invalid` |
| the record names this project and this revision | exact equality | `build-invalid` |
| the digest is a `sha256` digest | `sha256:` + 64 lower-case hex | `build-invalid` |
| the SBOM is still present and inside the project | the same path rule the build applied | `build-invalid` |
| the SBOM bytes are the recorded bytes | SHA-256 and byte length | `build-failed` |

The last row is the one that makes the record load-bearing: a record copied
onto a newer revision fails on identity, and an SBOM edited after the build
fails on the hash. Verification never rebuilds and never writes.

## The Gate Opt-in

A Gate check is a process. The project declares:

```yaml
verification:
  checks:
    - id: image-provenance
      source: bound
      root: project
      path: scripts/check-image-provenance.sh
```

and the script is one line:

```sh
#!/bin/sh
exec forge build . --verify-image
```

The Gate then binds the verdict to the same revision it always did, and the
check's evidence is the verb's bounded output. Forge ships neither the check
nor the script in the baseline: a project that does not build images must not
be asked for one.

## CLI

```text
forge build [PATH] --verify-image
```

- The flag is exclusive with `--image`, `--tag`, `--builder-bin`, `--os`,
  `--arch` and `--rid`; each combination is `build-invalid` with the reason.
- On success the JSON carries `image_build` (the record) and
  `verification: {verified: true, revision, digest}`; the human output says
  the record verified.
- On failure the process exits non-zero with the typed code, so a bound Gate
  check blocks like any other failing command.

## Compatibility

- The record shape does not change; the new verb reads the shape the previous
  change writes.
- No Gate evidence, registry, journal or manifest change.
- A project that never opts in sees no behavioural change.

## Verification

- unit tests for the identity, contract, digest and SBOM rules;
- CLI contract tests: a fresh record verifies; a moved `HEAD` fails; an edited
  SBOM fails; a tampered digest fails; a foreign record fails; a missing record
  fails; every exclusive-flag refusal;
- a real build-then-verify run, and the same run after a commit;
- the repository suite and the repository Gate.

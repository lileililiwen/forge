## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The image-builder contract and its validation | `forge` | Yes. `src/image/`. |
| The CLI flags and the record path | `forge` | Yes. `src/main.rs`, `src/cli/gitops.rs`. |
| The bounded runner's stdin entry point | `forge` | Yes. `src/native/`. |
| The reference adapters | `forge` | Yes, as reference material. |
| The contract document | `forge` | Yes. `docs/adapter-contracts/image-builder.md`. |
| The builders (`railpack`, `pack`, Docker) | their owners | No. Never installed by Forge. |
| The profile build command and `forge build` without `--image` | `forge` | No. Untouched. |
| The deploy lane's artifact input | `forge` | No. A later change. |
| Gate evidence | `forge` | No. A build record is separate evidence. |

## The Contract

`forge-image-builder/0.1.0`, one JSON document in, one out, on stdin/stdout.

Request:

```json
{
  "contract": "forge-image-builder/0.1.0",
  "project_id": "app",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "directory": "/workspace/app",
  "tag": "forge-app:0123456789ab",
  "dry_run": false
}
```

Response:

```json
{
  "contract": "forge-image-builder/0.1.0",
  "builder": "pack",
  "builder_version": "0.35.0",
  "status": "built",
  "image": "forge-app:0123456789ab",
  "digest": "sha256:1f0…",
  "sbom_path": ".forge/build/app/0123456789ab/sbom.cdx.json",
  "evidence": ["builder detected a Node application", "image exported"],
  "recovery": []
}
```

`status` is `built` or `failed`. A `failed` response is a build failure with
its evidence; a non-zero exit is the same outcome without a response. The
contract is read with `ContractVersion`: the family must be
`forge-image-builder` and the major must be `0`, so `0.1.0` and a future `0.2.0`
are both accepted and `1.0.0` is refused with both sides named.

## Validation

Forge refuses a response rather than recording a claim it cannot check:

| Claim | Rule | Why |
|---|---|---|
| `image` | equals the requested tag, bounded reference shape | an adapter may not build something else and call it this build |
| `digest` | `sha256:` followed by 64 lower-case hex | one digest vocabulary; no `sha512:` masquerade |
| `sbom_path` | relative, only normal components, resolves inside the project, exists, ≤ 8 MiB | a path is an operator's filesystem; it may not escape the project |
| `builder` | non-empty, ≤ 128 characters | provenance must name the tool |
| `builder_version` | optional, ≤ 128 characters | not every builder can report one |
| `evidence` | ≤ 20 lines, ≤ 800 characters each, redacted | the journal and the record stay bounded |

Forge computes the SBOM's SHA-256 itself. The adapter names the file; Forge
does not accept a hash the adapter supplies, because that would be one more
unchecked claim.

## The Record

`.forge/build/<project-id>/<sha12>.json`, written to a `.tmp` sibling and
renamed into place, so a reader never sees a half-written record:

```json
{
  "contract": "forge-build-image/0.1.0",
  "project_id": "app",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "tag": "forge-app:0123456789ab",
  "builder": "pack",
  "builder_version": "0.35.0",
  "digest": "sha256:1f0…",
  "sbom": {"path": ".forge/build/app/0123456789ab/sbom.cdx.json", "sha256": "…", "bytes": 4211},
  "evidence": ["…"],
  "observed_at": "2026-10-30T10:00:00Z"
}
```

The record is keyed by the project and the revision prefix, matching the
`<sha12>` convention the publish and delivery lanes already use. It carries no
absolute path: the SBOM path is project-relative and the directory is not
recorded.

## CLI

```text
forge build [PATH] --image [--tag <REF>] [--builder-bin <PATH>]
```

- `--image` selects the path. Without it, `forge build` behaves exactly as it
  did.
- `--tag` defaults to `forge-<project-id>:<sha12>`. A provided tag is validated,
  never rewritten.
- `--builder-bin` overrides `$FORGE_IMAGE_BUILDER_BIN`, which overrides
  `forge-image-builder` on `PATH`. The resolution is explicit and never
  installs anything.
- `--tag` or `--builder-bin` without `--image` is `build-invalid`.
- `--image` with `--os`, `--arch` or `--rid` is `build-invalid`: an image's
  platform belongs to the builder, and pretending Forge selected it would be a
  claim Forge cannot keep.

## Reference Adapters

Two Python 3 adapters under `adapters/image/`:

- `forge-image-builder-railpack` runs `railpack build <dir> --name <tag>`;
- `forge-image-builder-pack` runs `pack build <tag> --path <dir>`.

Both read the digest from the local image store with
`docker image inspect --format '{{index .RepoDigests 0}}'`, because neither
builder is guaranteed to print one. Both return no `sbom_path`: Forge supports
SBOM capture, and an adapter claims one only when it actually produced one.

They are reference material. The contract tests drive a recording adapter, and
the adapters themselves are documented as unexercised against a real builder on
this host, in the same way the SSH deploy adapter states its adoption
checklist.

## Verification

- unit tests for the tag default, the image-reference and digest shapes, and
  the SBOM path rule (relative, escaping, missing, oversized);
- CLI contract tests with a recording adapter: built with digest and SBOM,
  built without SBOM, failed status, non-zero exit, malformed response, wrong
  image, bad digest, escaping SBOM, missing adapter, and the flag refusals;
- a real run that writes the record and one that refuses without writing it;
- the repository suite and the repository Gate.

# Forge image-builder contract

`forge build --image` turns a source tree into a container image without Forge
owning a builder. The builder is an operator-installed adapter — `pack`,
Railpack, or anything else — that speaks one JSON document in and one out over
stdin/stdout.

The current contract is `forge-image-builder/0.1.0`. It is judged by the
shared extension rule: the family must be `forge-image-builder` and the major
must be `0`, so a newer minor is accepted and a different family or major is
refused with both sides named.

## Request

```json
{
  "contract": "forge-image-builder/0.1.0",
  "project_id": "example",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "directory": "/workspace/example",
  "tag": "forge-example:0123456789ab",
  "dry_run": false
}
```

`revision` is the project's committed Git `HEAD`; an image build is bound to a
revision, so a project without one is refused. `tag` is either the operator's
`--tag` or `forge-<project-id>:<sha12>`. The adapter builds that tag; a
response that names a different image is refused.

## Response

```json
{
  "contract": "forge-image-builder/0.1.0",
  "builder": "pack",
  "builder_version": "0.35.0",
  "status": "built",
  "image": "forge-example:0123456789ab",
  "digest": "sha256:1f0…",
  "sbom_path": ".forge/build/example/0123456789ab/sbom.cdx.json",
  "evidence": ["detected a Node application", "image exported"]
}
```

`status` is `built` or `failed`. A failed build is reported by the adapter as
a `status: failed` response with **exit 0**, so Forge records the adapter's
evidence; a non-zero exit means the adapter could not answer at all. Both
outcomes are `build-failed` and neither writes a record.

`builder` is required and names the tool; `builder_version` is optional.
`sbom_path` is optional and project-relative. `digest` is required on a built
response and must be `sha256:` followed by 64 lower-case hex characters.

## What Forge checks

Forge refuses a response rather than recording a claim it cannot check:

| Field | Rule |
|---|---|
| `contract` | family `forge-image-builder`, major `0` |
| `status` | `built` or `failed` |
| `builder` | non-empty, at most 128 characters |
| `builder_version` | optional, at most 128 characters |
| `image` | exactly the requested tag |
| `digest` | `sha256:` + 64 lower-case hex |
| `sbom_path` | relative, plain path components only, resolves inside the project, names a file, at most 8 MiB |
| `evidence` | at most 20 lines, at most 800 characters each, credential-shape redacted |

Forge computes the SBOM's SHA-256 itself. The adapter names the file; a hash
supplied by the adapter is not accepted.

## The record

A successful build writes `.forge/build/<project-id>/<sha12>.json` through a
staged file and an atomic rename:

```json
{
  "contract": "forge-build-image/0.1.0",
  "project_id": "example",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "tag": "forge-example:0123456789ab",
  "builder": "pack",
  "builder_version": "0.35.0",
  "digest": "sha256:1f0…",
  "sbom": {
    "path": ".forge/build/example/0123456789ab/sbom.cdx.json",
    "sha256": "…",
    "bytes": 4211
  },
  "evidence": ["…"],
  "observed_at": "2026-10-30T10:00:00Z"
}
```

The record carries no absolute path and is not Gate evidence. It does not make
a Gate check pass, and it does not feed the deploy lane; both are later
changes.

## Resolving the adapter

One order, and Forge never installs anything:

1. `--builder-bin <path>`;
2. `$FORGE_IMAGE_BUILDER_BIN`;
3. `forge-image-builder` on `PATH`.

A binary that cannot start is `build-invalid` with the variable named and the
reminder that Forge never installs a builder.

## Reference adapters

Two Python 3 adapters ship under `adapters/image/`:

| Adapter | Command it wraps | Digest source |
|---|---|---|
| `forge-image-builder-railpack` | `railpack build <dir> --name <tag>` | `docker image inspect` |
| `forge-image-builder-pack` | `pack build <tag> --path <dir>` | `docker image inspect` |

Both report `status: failed` when the builder fails or when the local image
store reports no digest, because Forge will not record a build without one.
Neither returns an `sbom_path`: Forge supports SBOM capture, and an adapter
claims one only when it actually produced one.

They are reference material, not a claim that either tool is installed on this
host. The contract tests drive a recording adapter; the reference adapters
themselves have not been exercised against a real builder, and adopting them
requires the operator to install the builder and Docker first.

## What this contract does not do

- No builder is bundled, downloaded, pinned or version-managed by Forge.
- No registry push, cache, signing or provenance attestation.
- No SBOM generation. Forge hashes and records an SBOM the builder produced.
- No `--os`/`--arch`/`--rid`. An image's platform belongs to the builder;
  `forge build --image` refuses those flags instead of claiming a platform it
  did not select.

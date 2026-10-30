# Change: forge-build-image

## Why

`forge build` runs the profile's build command and reports a target, but it
produces no artifact Forge can name. The deploy lane's artifact is a container
file the project already owns; nothing in Forge turns a source tree into an
image, and nothing records what image came out of which revision. The previous
`target-addressed-build` change stated this limit in as many words: "Nothing in
Forge produces a compiled artifact yet — the deploy lane's artifact is a
container file — so the build step does not yet feed the deploy step."

The Go PaaS landscape answers this with a build contract rather than a builder:
Cloud Native Buildpacks (`pack`), Railpack, and every commercial PaaS built on
them detect a source tree, build an image, and can emit an SBOM. Forge should
consume that contract, not reimplement it. Forge already delegates in exactly
this shape — `forge run compose` to Docker Compose, `forge run local` to
GitRunByKaru, deploy to per-kind adapters — and a builder is one more external
tool with its own version, flags and failure modes.

## What Changes

- **`forge build --image` becomes the image path.** It resolves the project
  revision, derives a default tag (`forge-<project>:<sha12>`), and invokes one
  operator-installed image-builder adapter through a versioned stdin/stdout
  contract. Forge never installs, downloads or vendors a builder.
- **The adapter contract says what a build is.** `forge-image-builder/0.1.0`
  takes a bounded request (project, revision, directory, tag) and returns a
  bounded result: builder identity, image reference, `sha256` digest, an
  optional SBOM path, and evidence. The contract is judged by the shared
  extension version rule, so a newer minor is accepted and a different family
  or major is refused.
- **Forge validates the claim.** The returned image must be the tag it asked
  for; the digest must be a `sha256` digest; an SBOM path must be relative, must
  stay inside the project and must name a file that exists; Forge hashes the
  SBOM itself instead of trusting the adapter's number. A response that fails
  any of these is refused, not recorded.
- **One provenance record per build.** `.forge/build/<project-id>/<sha12>.json`
  records the revision, tag, builder identity, digest, SBOM digest and the
  command's evidence. It is written atomically and is bound to the revision the
  build ran at. Forge does not write this into Gate evidence: a build record is
  its own evidence, and a Gate check that consumes it is a separate change.
- **Refusals are typed and early.** No adapter binary, a failed adapter, a
  malformed response, an unsafe SBOM path, and `--image` combined with
  `--os`/`--arch`/`--rid` are all refused before a record is written.
- **Two reference adapters ship.** `adapters/image/forge-image-builder-railpack`
  and `adapters/image/forge-image-builder-pack` map the contract onto the two
  builders' own commands and read the image digest from the local image store.
  They are reference material, exercised against recording builders, not a
  claim that either tool is installed.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `src/image/` | new: request, response, validation, record, runner |
| `src/native/` | one additive entry point: run a bounded command with stdin |
| `src/main.rs` | `build --image`, `--tag`, `--builder-bin` |
| `src/cli/gitops.rs` | dispatch, refusal for flag combinations, output |
| `src/build/` | unchanged; the non-image build path is untouched |
| `adapters/image/` | new: two reference adapters |
| `docs/adapter-contracts/image-builder.md` | new: the contract |
| `docs/ci-cd-system-model.md` | the build-to-image seam |
| Existing build tests | unchanged; the image path is a separate branch |

Persistence: one JSON record per build under `.forge/build/`, written by a
staged file and an atomic rename. No registry write, no journal row, no schema
change.

Failure boundaries: the adapter is resolved before any process starts; a
missing binary is `build-invalid` with the recovery (install it or set the
variable); a failed or timed-out adapter is `build-failed`; a response Forge
cannot trust is `build-invalid`. No partial record survives a refusal.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `forge-build-image` | added | this package |
| `target-addressed-build` | read only | the profile build path is untouched |
| `artifact-and-ci-baseline` | read only | its artifact is Forge's own binary, not a project image |
| `extension-point-contract` | read only | the version rule this contract follows |
| `single-delivery-lane` | read only | the deploy lane's artifact input is not changed |

## Non-Goals

- No builder bundled, downloaded or pinned by Forge. The adapter is operator
  configuration, like a deploy executor.
- No change to `forge build` without `--image`, to the profile catalog, or to
  the deploy lane's artifact input. Feeding a built digest into `deployment.
  artifact` is a later change.
- No SBOM generation. The contract accepts one; Forge hashes and records it;
  whether a builder emits one is the builder's and the adapter's business.
- No Gate evidence. A build record does not make a Gate check pass; a future
  check may consume it.
- No registry, cache, remote registry push or provenance signing.

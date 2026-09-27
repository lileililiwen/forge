# Linux build to Mac runtime

The Mac is a runtime provider, not a source or deployment-code repository.

## Ownership

- GitHub owns source code, review history, and release inputs.
- Forge on Linux owns checkout, validation, local container builds, release
  manifests, deployment orchestration, and evidence.
- The Mac owns only runtime state: pulled images, containers, volumes, logs,
  runtime configuration, and private secrets.
- Workspace Governance owns deployment policy and eligibility; Forge consumes
  that decision.

## Release flow

1. Forge checks out the selected Git revision on Linux.
2. Forge runs its local container release stage through
   `FORGE_CONTAINER_BIN`; GitHub Actions is not required.
3. The release manifest records the project, revision, container name, private
   Mac env-file path, ports, and an image pinned by `sha256` digest.
4. Forge invokes the Linux-side `mac-runtime` adapter through
   `FORGE_DEPLOYER_BIN`.
5. The adapter sends argument-array `ssh ... docker ...` operations to the Mac:
   pull the image, replace the named container, and start it.
6. Observation uses a read-only `docker inspect` operation.

The adapter must never rsync source, copy scripts, call Jenkins jobs, read the
Mac's secrets, or execute a shell script on the Mac. A dry run must not contact
the Mac.

## Configuration boundary

The manifest may contain a path such as
`/Users/allen/production/secrets/<project>/.env`, but not the contents of that
file. The path is runtime configuration, not source code. Secret rotation is a
Mac runtime operation and does not modify the release artifact.

The Mac remains one selectable provider. The `mac-runtime` target is an adapter
choice, not a global Forge deployment assumption; another provider can consume
the same release contract.

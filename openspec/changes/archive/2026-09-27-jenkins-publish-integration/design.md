# Design: jenkins-publish-integration

## 1. Implementation boundary

Forge remains the controller. Its existing `forge-deploy-executor/0.1.0`
contract invokes a Linux-side adapter supplied by `jenkins-local`.
`src/deploy/engine.rs` sets the executor working directory to the project root
so a relative runtime manifest is resolved locally. The adapter receives
`kind: mac-runtime`; `jenkins`, `sync`, and Mac-script paths are not part of
the new primary path.

## 2. Language and runtime

Forge uses Rust 1.87+. The Mac runtime adapter uses Python 3 standard library,
SSH, and Docker CLI on Linux. Forge's existing release `container` stage runs
the local image build/publish adapter. No GitHub Actions build is required.

## 3. Ownership and shared code

Forge owns the executor contract and persisted evidence. `jenkins-local` owns
the Linux-side `mac-runtime` adapter. Workspace Governance owns eligibility and
promotion metadata. The Mac owns secrets, volumes, logs, and generated runtime
state only. No repository is imported into another repository.

## 4. Behavioral model

1. Forge validates governance, source revision, release checks and artifact.
2. Forge builds/publishes the immutable image locally.
3. Forge invokes adapter `apply --target ... --kind mac-runtime`.
4. Adapter reads the release manifest locally and sends argument-array SSH/Docker
   operations to the Mac.
5. Adapter observes the named container and emits a conformant envelope.
6. Forge persists evidence; unavailable/failed runs preserve prior last-good
   state.

The release manifest contains `forge-runtime-release/0.1.0`, project id,
revision, immutable image reference, container name, Mac env-file path, and
published ports. It contains no secret values.

## 5. Contract and compatibility

`mac-runtime` is additive to target validation. Existing `local` and
`docker-compose` targets remain unchanged. The old `forge publish` source-sync
surface is retained only for compatibility and marked deprecated; it is not
used by the new runtime-only deployment path.

## 6. Failure and boundary policy

- malformed or traversing artifact: refuse before SSH;
- source/script-sync request: refuse before SSH;
- SSH/Docker unavailable: `unknown` observation and recovery detail;
- pull/run failure: failed apply, prior state preserved;
- dry-run: command planning only, no subprocess;
- unknown target: typed deploy-invalid error.

## 7. Verification oracle

Forge tests cover target validation and executor working-directory handoff.
Adapter tests cover manifest validation, exact command argv, dry-run, observe
read-only behavior, secret-path handling, and failure classification. Required
checks are Forge format/build/deploy tests, the jenkins-local Python suite,
strict OpenSpec validation, and `git diff --check`. A real Mac canary is not
claimed by these local checks.

## 8. Decision ledger

- Local image build/publish is Forge's existing release container stage.
- Registry pull is the first artifact transport; direct `docker save` streaming
  is deferred.
- Jenkins is not required for the new runtime adapter; the old Jenkins path is
  a migration/recovery surface.

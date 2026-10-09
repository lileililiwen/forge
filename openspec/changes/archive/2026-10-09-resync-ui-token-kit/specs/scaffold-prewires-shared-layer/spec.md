# scaffold-prewires-shared-layer (delta)

## ADDED Requirements

### Requirement: Vendored token mirror is revision-synced with the kit source

The vendored `platform-ui-web` mirror SHALL be one atomic record of one kit
source revision: the per-file bytes under `kits/`, the `kits/manifest.json`
records (source revision, released kit version, per-file digests), the
compiled-in descriptor digests and the registered kit version constant SHALL
all describe the same kit release. After a resync the vendored kit verifier
SHALL pass over the new bytes, and any disagreement between the mirrored
bytes and their manifest or descriptor records SHALL fail generation with
`kit-digest-mismatch` naming the file and both digests.

#### Scenario: Resynced mirror verifies

- **WHEN** the token artifacts are re-copied from a new kit release and the
  manifest, descriptor digests and kit version constant are updated together
- **THEN** `node kits/scripts/verify-tokens.mjs` passes over the vendored pair,
  the vendored-asset digest check passes, and a `react-web` render receipts the
  new bytes with their newly measured digests

#### Scenario: Half-applied resync fails closed

- **WHEN** mirrored bytes are updated but a manifest or compiled-in descriptor
  record still names the previous revision's digest
- **THEN** generation and Forge's own digest verification fail with
  `kit-digest-mismatch` naming the drifted file, the expected digest and the
  actual digest, and no project is staged

#### Scenario: Pinned version matches the generation version

- **WHEN** a `react-web` or `nextjs-web` project is scaffolded after the resync
- **THEN** the rendered `forge.yaml` `kit.version` and the ownership receipt
  `version` equal the kit release the vendored artifacts were generated at, and
  `forge kit upgrade --to platform-ui-web@<that version>` resolves against the
  compiled-in registry

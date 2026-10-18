# Pinned assets: `baseline-client@1.1.0`

These Forge-owned assets extend `baseline-client@1.0.0`. The GitHub Actions
workflow installs Forge `0.1.0` from the versioned Linux x86_64 GitHub Release
and verifies its SHA-256 sidecar before extraction. The CI entry point and
local pre-commit hook continue to run the same repository-wide Forge Gate.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/scripts/verify.sh` | `verify.sh` | `6635daaea17506d972af7497d5f9d5d1b9620ac63c8624997ee69fe25eec0fae` |
| `.standard/ci/verify.yml` | `ci/verify.yml` | `11b513ac727180c2a909bd0dfc94bcfc2d2f0badaae8eb44c7d79c2f931c524b` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `11b513ac727180c2a909bd0dfc94bcfc2d2f0badaae8eb44c7d79c2f931c524b` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `823d36ae671f8da9ff78ac7ba2dc74c36d26897f211d4f80b86fc1cf48903c81` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | _copied from 1.0.0_ |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these bytes after the pack version is released; create a new pack
version and update this provenance table when any asset changes.

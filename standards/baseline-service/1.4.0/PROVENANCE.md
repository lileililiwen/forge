# Pinned assets: `baseline-service@1.4.0`

These Forge-owned assets extend `baseline-service@1.3.0`. The GitHub Actions
workflow installs Forge `0.1.0` from the versioned Linux x86_64 GitHub Release
and verifies its SHA-256 sidecar before extraction. The service verifier and
Forge Gate remain separate required steps. The local pre-commit hook remains
the same repository-wide `forge gate .` command.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/ci/verify.yml` | `ci/verify.yml` | `66f2f7df6e1b2c1f9cb75f63a35bb7fc6dfa1cbabfa9a50c792dd3076f6d18df` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `66f2f7df6e1b2c1f9cb75f63a35bb7fc6dfa1cbabfa9a50c792dd3076f6d18df` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `97bba25b72fae702c1e6fb36dd8ea90062836099a73271f4360ad0607fd249e0` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | _copied from 1.3.0_ |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these bytes after the pack version is released; create a new pack
version and update this provenance table when any asset changes.

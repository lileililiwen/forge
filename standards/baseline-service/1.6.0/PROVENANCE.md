# Pinned assets: `baseline-service@1.6.0`

These Forge-owned assets extend `baseline-service@1.5.0`. Pre-commit runs the
Forge mandatory fast baseline. Hosted CI runs the full Gate profile and keeps
the existing authenticated evidence-upload boundary. The installer pins Forge
`0.1.1`, which provides the profile-aware CLI. Earlier snapshots remain
immutable.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/ci/verify.yml` | `ci/verify.yml` | `72c764d0ec0b9388fd50006e18f5f51aa52f336557ad54802f8716583510a162` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `72c764d0ec0b9388fd50006e18f5f51aa52f336557ad54802f8716583510a162` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `32efc179a59778e78b911e2b283628bec8e731970f1980c237cbaa6636722643` |
| `.standard/scripts/upload-hosted-ci-evidence.py` | `upload-hosted-ci-evidence.py` | `7738e550fae1d859d8e1ba87a26972fae828c8cf68849385bd1fefef70e89b82` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | `3bc5338a58ba201010dbff4b0c886ae758809f5142d8c333bc9798c3df674e87` |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these assets after release; create a new pack version when any
asset changes.

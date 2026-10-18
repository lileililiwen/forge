# Pinned assets: `baseline-client@1.3.0`

These Forge-owned assets extend `baseline-client@1.2.0`. Pre-commit runs the
Forge mandatory fast baseline. Hosted CI invokes the full Gate profile through
the pack's verification wrapper and preserves the authenticated evidence
upload boundary. The installer pins Forge `0.1.1`, which provides the
profile-aware CLI. Earlier snapshots remain immutable.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/scripts/verify.sh` | `verify.sh` | `b091622250ce152d4a8c01ae82143a0aa9956feaeb8e839c770e32f7da0906e4` |
| `.standard/ci/verify.yml` | `ci/verify.yml` | `506fd782522e4eb0227b1c7101e321d34969cc3fed9e863e2b98fa4dfe55724a` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `506fd782522e4eb0227b1c7101e321d34969cc3fed9e863e2b98fa4dfe55724a` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `51fd454d1833bf4779b297ce7597d27b342d584bce78c1d1f965e3b8829026b0` |
| `.standard/scripts/upload-hosted-ci-evidence.py` | `upload-hosted-ci-evidence.py` | `7738e550fae1d859d8e1ba87a26972fae828c8cf68849385bd1fefef70e89b82` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | `24810aa7dfb31665a946789dc292b5d548e97dc028f6f04eccb897989485fcb5` |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these assets after release; create a new pack version when any
asset changes.

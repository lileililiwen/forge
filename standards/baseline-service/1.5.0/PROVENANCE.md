# Pinned assets: `baseline-service@1.5.0`

These Forge-owned assets extend `baseline-service@1.4.0`. The CI workflow
preserves Forge Gate evidence and enables GitHub OIDC only in the dependent
hosted-evidence upload job. The pinned uploader verifies the GitHub Actions
runtime endpoint, requests the fixed `forge-ci-evidence` audience, and sends
the Gate record to a configured Forge ingest endpoint. The local pre-commit
hook and required Gate checks are unchanged from `1.4.0`.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/ci/verify.yml` | `ci/verify.yml` | `e0a2ab3962d6961aa864d9d2e15d09a0d1fb6aecf0ce197f24225e353f1ae076` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `e0a2ab3962d6961aa864d9d2e15d09a0d1fb6aecf0ce197f24225e353f1ae076` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `b08b398b9638f05c314989c81a676c17fd8fbf7422f2f141d01c269bce87ceb5` |
| `.standard/scripts/upload-hosted-ci-evidence.py` | `upload-hosted-ci-evidence.py` | `2c348bb600b7945b75aaf67d985db03112d964632bcd5cc4ceac9321fd8af1b1` |
| `.pre-commit-config.yaml` | copied from `baseline-service@1.4.0` | `073b7624db378ffc66b21c02ac600628e3eb642363898bf10d6dca02ffdb3ffd` |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these assets after release; create a new pack version when any
asset changes.

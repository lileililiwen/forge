# Pinned assets: `baseline-client@1.2.0`

These Forge-owned assets extend `baseline-client@1.1.0`. The CI workflow
preserves Forge Gate evidence and enables GitHub OIDC only in the dependent
hosted-evidence upload job. The pinned uploader verifies the GitHub Actions
runtime endpoint, requests the fixed `forge-ci-evidence` audience, and sends
the Gate record to a configured Forge ingest endpoint. The local pre-commit
hook and Gate-only verification wrapper are unchanged in behavior.

| Project-relative path | Source | SHA-256 |
|---|---|---|
| `.standard/scripts/verify.sh` | `verify.sh` | `c411e367cc5dafc9cde00a97cf0d55c5b0e2058b24eb8c939a4794acd6ef0bbc` |
| `.standard/ci/verify.yml` | `ci/verify.yml` | `bd4a51e7920e6621589a9ceaf17479b0a9103356f8db800a9b00d9d0b6dda27f` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `bd4a51e7920e6621589a9ceaf17479b0a9103356f8db800a9b00d9d0b6dda27f` |
| `.standard/scripts/install-forge.sh` | `install-forge.sh` | `ea1f46e1b4c6938f431e49836ea790df5c1e2420057a6f0fd310be30a7afb32d` |
| `.standard/scripts/upload-hosted-ci-evidence.py` | service uploader snapshot | `2c348bb600b7945b75aaf67d985db03112d964632bcd5cc4ceac9321fd8af1b1` |
| `.pre-commit-config.yaml` | copied from `baseline-client@1.1.0` | `073b7624db378ffc66b21c02ac600628e3eb642363898bf10d6dca02ffdb3ffd` |

The pack descriptor digest is computed from its complete sorted asset list.
Do not edit these assets after release; create a new pack version when any
asset changes.

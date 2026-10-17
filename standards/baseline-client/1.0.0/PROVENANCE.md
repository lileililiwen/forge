# Pinned assets: `baseline-client@1.0.0`

These assets are authored and versioned by Forge for the current client
profiles. CI and the local pre-commit hook run the same repository-wide Forge
Gate command. The workflow installs Forge revision
`d7c5f96c16fab8c50a9b40f80e55783b25a729af` immutably; release-based
installation is deferred until Forge publishes checksummed binaries.

| Project-relative path | Source | SHA-256 |
| --- | --- | --- |
| `.standard/scripts/verify.sh` | `verify.sh` | `09786dd0b198cc9c32872f3e38537d32a01b7a72bb6e13528379e01f4fae75b2` |
| `.standard/ci/verify.yml` | `ci/verify.yml` | `491e92eecb048aa97c539f0eae8054ae969cb326e321d07df3e2aeb9fe1bcaf7` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `491e92eecb048aa97c539f0eae8054ae969cb326e321d07df3e2aeb9fe1bcaf7` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | `a6da3e58ae646b83317e9391f753db5c92649d5d53bb24aa67e52812bd064474` |

The pack descriptor digests all rendered assets. Keep these bytes immutable
for this version; update the pack version and provenance table for any change.

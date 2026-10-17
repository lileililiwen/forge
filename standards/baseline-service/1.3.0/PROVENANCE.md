# Pinned assets: `baseline-service@1.3.0`

These assets are authored and versioned by Forge. They extend the immutable
`baseline-service@1.2.0` snapshot with CI and pre-commit entry points that run
the same Forge Gate command. The workflow installs the Gate-capable Forge
revision `d7c5f96c16fab8c50a9b40f80e55783b25a729af`; the revision is pinned so
the generated workflow does not follow a mutable branch. Forge release
publication and checksummed release installation remain future work.

| Project-relative path | Source | SHA-256 |
| --- | --- | --- |
| `.standard/ci/verify.yml` | `ci/verify.yml` | `7ff27950f082bcadf069b973c269ade0b1c56d8370090d01a9327594e9fb5e17` |
| `.github/workflows/forge-gate.yml` | `ci/verify.yml` | `7ff27950f082bcadf069b973c269ade0b1c56d8370090d01a9327594e9fb5e17` |
| `.pre-commit-config.yaml` | `.pre-commit-config.yaml` | `b980a7e7126196b6555d858eb24036ae3a9f029308a116b36aafab5de3c5526c` |

The pack descriptor carries the complete rendered asset set and its digest.
Do not edit these bytes after the version is released; create a new pack
version and update the descriptor when any asset changes.

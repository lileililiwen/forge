# Changelog

All notable changes to Forge are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Entries between `0.1.0` and here are not exhaustively recorded in this file;
[openspec/changes/archive/](openspec/changes/archive/) and `git log` are the
complete record.

### Fixed

- **Fresh clones did not build.** `.gitignore` excluded `/templates/`, but
  `src/generate/mod.rs` embeds `templates/rust-web-main-rs.txt` with `include_str!`,
  so the `rust-web` scaffold body was never committable. The rule is removed and the
  template is tracked.
- Stale documentation claims refreshed: change and spec counts in `README.md`,
  `ROADMAP.md` and `openspec/config.yaml`, and `HANDOFF.md` records that the seven
  committed-but-unarchived packages were consolidated and promoted, that the
  `contracts/` mirror gap is closed, and that the `main`-branch git rule is now
  written in `AGENTS.md`.

### Added

- Canonical spec coverage promoted for shipped behavior: bounded external-adapter
  execution with concurrent pipe drain and a bounded source revision lookup; one
  shared request-write boundary across every adapter, translator and delivery
  provider, where `BrokenPipe` is not a provider failure; run-time Studio test port
  windows outside the host ephemeral range; a contract parity gate that compares real
  vendored bytes and cannot report a pass it did not earn; and the public portfolio
  manifest emitted in the contracted wire shape
  (`platform.public-portfolio-manifest`, `1.0.0`, `rev_<n>`).

## [0.1.0] - 2026-09-26

### Added

- Initial Forge Core/CLI baseline: `forge.yaml` manifest, project/profile
  registries, `forge new`, `forge import`, `forge list`, `forge inspect`,
  `forge doctor`, feature lifecycle, upgrades, quality policy, specs,
  agent runtime, MCP surface, distribution, translation, release, deployment,
  sibling-integration packages and gate runtime evidence.
- Installable artifact baseline: `scripts/package.sh`,
  `scripts/checksum.sh`, `scripts/install.sh`, `scripts/smoke.sh`,
  `scripts/bump.sh`, `deny.toml` policy and the CI job graph in
  `.github/workflows/ci.yml`.

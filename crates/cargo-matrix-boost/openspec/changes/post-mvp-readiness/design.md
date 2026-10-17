# Design: Post-MVP readiness for cargo-matrix-boost

## Boundary

- **In scope:** repository-local documentation and evidence owned by this repository. `README.md`,
  new `docs/*.md`, new screenshot/GIF assets, and the OpenSpec change package.
- **Out of scope:** runtime code paths, CLI contract, report schema, CI workflow behavior, sibling
  repositories, and any shared/platform artifact.
- The only executable surface used is the already-shipped `cargo matrix-boost --demo` and the existing
  verification commands. Readiness is proven by reproducing evidence, not by adding features.

## Runtime and Commands

The authoritative commands for this repository (`.project.json` verification command plus the CI
workflow) are:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo install --path .
cargo matrix-boost --demo --no-open --output-dir target/matrix-demo
```

`--demo` is the only evidence generator. It writes `report.json` and `report.html` into the chosen
output directory and does not require a scanned project.

## Ownership

| Artifact | Owner | Notes |
| --- | --- | --- |
| `README.md` | This repository | Single entry point; no sibling edits. |
| `docs/*.md` | This repository | Authored by this change. |
| `docs/assets/*` (screenshots, `demo.gif`) | This repository | Generated from `--demo`; provenance command recorded next to the asset. |
| `src/**`, `Cargo.toml`, `.github/workflows/ci.yml` | This repository, unchanged | Referenced only; any needed change is a separate approved change. |
| `openspec/**` | This repository | Change package and promoted `readiness` spec. |

## Behavioral Table

| Surface | Current behavior | This change | Verification |
| --- | --- | --- | --- |
| CLI flags | `--demo`, `--no-open`, `--output-dir`, `--toolchains`, `--features`, `--exclude-features`, `--max-combinations`, `--command`, `--no-default`, `--concurrency` | Unchanged; documented only | `cargo matrix-boost --demo --no-open --output-dir ...` reproduces outputs |
| Demo mode | Builds a mock matrix and writes JSON + HTML | Unchanged | Output files exist and match screenshots |
| TUI grid | Renders feature × toolchain status live | Unchanged; captured in screenshot/GIF | Screenshot shows the same columns as `report.json` cells |
| HTML report | Self-contained interactive dashboard | Unchanged; captured in screenshot | HTML opens from `file://`; screenshot fields match report |
| README | Install + usage overview | Adds evidence and deep-doc links | Links resolve; quickstart commands run |
| CI workflow | fmt + clippy + test | Unchanged; documented as a recipe | Recipe uses existing flags only |

## Contract and Compatibility

- No public contract change. CLI arguments, exit behavior, `report.json` shape, and `report.html`
  layout MUST remain as shipped in v0.1.0.
- Documentation MUST describe the shipped version only; it MUST NOT advertise unshipped flags.
- The toolchain matrix document MUST reflect what the resolver and runner actually support
  (`stable`, `beta`, `nightly` via rustup; powerset or explicit feature sets), not aspirational support.
- If a screenshot or document would require a behavior change, the change is split (see Ledger).

## Failure Policy

- A screenshot or GIF that cannot be produced in the available environment MUST be recorded as
  **BLOCKED** with the exact failed command and output. Fabricated or hand-edited mock images are
  prohibited.
- Documents that claim a command works MUST be reproduced by running that command; a doc that cannot
  be reproduced is **FAIL**, not "done".
- Tests, clippy, and fmt MUST pass before completion; a passing skeleton is not readiness evidence.
- Sibling/shared-work decisions not made in this change MUST be listed in the Ledger, never guessed.

## Oracle (exact commands)

```bash
# Repository gates
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

# Evidence reproduction from a clean install
cargo install --path .
cargo matrix-boost --demo --no-open --output-dir target/matrix-demo
test -s target/matrix-demo/report.json
test -s target/matrix-demo/report.html

# Documentation links and OpenSpec strict validation
openspec validate post-mvp-readiness --strict --no-interactive
```

PASS requires every command to exit 0 and the two report files to be non-empty. Any non-zero exit or
missing file is FAIL/BLOCKED with the exact command and output recorded.

## Ledger (blocked, not guessed)

- **Interactive browser open:** not exercised in a headless environment; the recipe uses `--no-open`.
  Do not claim the auto-open path was verified.
- **TUI screenshot/GIF capture:** requires a real TTY and a recording tool (`asciinema`/`agg`/`vhs`/
  `ffmpeg`). Tool availability is not assumed; if unavailable, mark BLOCKED.
- **Windows/macOS evidence:** not required by this change; parity is not claimed.
- **Machine-readable error output:** if the error catalog needs structured exit codes or `--json`
  errors, that is a behavior change and belongs to a separate feature change, not this one.
- **Sibling/shared crate decisions:** not made here; default is keep-local.
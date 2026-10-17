# Design: Post-MVP readiness for cargo-audit-boost

## Boundary

- **In scope:** repository-local documentation, evidence, and metadata owned by
  this repository — `README.md`, new `docs/*.md`, new screenshot/GIF assets
  under `docs/assets/`, the `Cargo.toml` license field, and the OpenSpec change
  package plus `openspec/config.yaml`.
- **Out of scope:** runtime code paths (`src/**`), CLI contract, report schema,
  CI workflow behavior, sibling repositories, and any shared/platform artifact.
- The only executable surface used is the already-shipped `cargo-audit-boost
  --demo` and the existing verification commands. Readiness is proven by
  reproducing evidence, not by adding features.

## Runtime and Commands

The authoritative commands for this repository (`.project.json` verification
command plus the existing CI workflow) are:

```bash
cargo check --verbose
cargo test --workspace
cargo install --path .
cargo-audit-boost --demo
```

`--demo` is the only evidence generator. It runs a simulated build through the
TUI, auto-quits when stdout is not a TTY (`src/main.rs`), and writes
`target/build-scan/report.json` and `target/build-scan/report.html`
(`src/exporter.rs`). It does **not** open a browser.

## Ownership

| Artifact | Owner | Notes |
|---|---|---|
| `README.md` | This repository | Single entry point; no sibling edits. |
| `docs/*.md` | This repository | Authored by this change. |
| `docs/assets/*` (screenshots, `demo.gif`) | This repository | Generated from `--demo`; producing command recorded next to the asset. |
| `Cargo.toml` `license` field | This repository | Aligns metadata with the existing MIT `LICENSE`; no dependency change. |
| `src/**`, `.github/workflows/ci.yml` | This repository, unchanged | Referenced only; any needed change is a separate approved change. |
| `openspec/**` | This repository | Change package and promoted `readiness` spec. |

## Behavioral Table

| Surface | Current behavior | This change | Verification |
|---|---|---|---|
| CLI flags | `--demo`, plus trailing cargo args (defaults to `check`) | Unchanged; documented only | `cargo-audit-boost --demo` reproduces outputs |
| Demo mode | Simulated build drives the TUI, then writes JSON + HTML to `target/build-scan/` | Unchanged | Both files exist and are non-empty and match screenshots |
| TUI | Live Gantt, split detail sidebar, search, cached/bottleneck toggles | Unchanged; captured in screenshot/GIF | Screenshot columns match `report.json` `crates[]` |
| HTML report | Self-contained dashboard (Gantt, concurrency SVG, diagnostics, filter/sort, light/dark) | Unchanged; captured in screenshot | Opens from `file://`; fields match `report.json` |
| `report.json` | `total_duration_secs`, `total_crates`, `max_concurrency`, `success`, `bottlenecks`, `crates[]` | Unchanged; documented | `docs/profile-output.md` matches a fresh export |
| README | Install + usage overview | Adds evidence and deep-doc links | Links resolve; quickstart commands run |
| CI workflow | `cargo check --verbose`, `cargo test --verbose` | Unchanged; documented as a recipe | Recipe uses existing behavior only |
| `Cargo.toml` | No `license` field | Adds `license = "MIT"` | `LICENSE`, README, and Cargo metadata agree |

## Contract and Compatibility

- No public contract change. CLI arguments, exit behavior, `report.json` shape,
  and `report.html` layout MUST remain as shipped in v0.1.0.
- Documentation MUST describe the shipped version only; it MUST NOT advertise
  unshipped flags or `--no-open`-style options this tool does not have.
- `docs/profile-output.md` MUST describe the fields emitted by the current
  `ExportReport` in `src/exporter.rs`, not aspirational metrics.
- The MIT metadata addition is the only non-documentation edit and MUST NOT
  change dependencies or build output.
- If a screenshot or document would require a behavior change, the change is
  split (see Ledger).

## Failure Policy

- A screenshot or GIF that cannot be produced in the available environment MUST
  be recorded as **BLOCKED** with the exact failed command and output.
  Fabricated or hand-edited mock images are prohibited.
- Documents that claim a command works MUST be reproduced by running that
  command; a doc that cannot be reproduced is **FAIL**, not "done".
- Tests and `cargo check` MUST pass before completion; a passing skeleton is not
  readiness evidence.
- Sibling/shared-work decisions not made in this change MUST be listed in the
  Ledger, never guessed.

## Oracle (exact commands)

```bash
# Repository gates
cargo check --verbose
cargo test --workspace

# Evidence reproduction from a clean install
cargo install --path .
cargo-audit-boost --demo
test -s target/build-scan/report.json
test -s target/build-scan/report.html

# Documentation links and OpenSpec strict validation
openspec validate post-mvp-readiness --strict --no-interactive
```

PASS requires every command to exit 0 and the two report files to be non-empty.
Any non-zero exit or missing file is FAIL/BLOCKED with the exact command and
output recorded.

## Ledger (blocked, not guessed)

- **TUI screenshot/GIF capture under a non-TTY environment:** the demo
  auto-quits when stdout is not a TTY, and `crossterm` raw mode requires a real
  terminal. Capturing the TUI therefore needs a PTY recorder
  (`asciinema`/`agg`/`vhs`/`script`). Tool availability is not assumed; if a PTY
  recorder is unavailable, mark the asset BLOCKED with the exact command.
- **Headless CI invocation of the TUI:** if `cargo-audit-boost check` fails to
  enter raw mode in a non-TTY CI runner, the CI recipe MUST wrap the command in a
  PTY or profile only via `--demo`. Adding a `--no-tui` run mode would be a
  behavior change and belongs to a separate feature change, not this one.
- **Interactive browser open:** not applicable — this tool does not open a
  browser; do not document an auto-open path.
- **Windows/macOS evidence:** not required; parity is not claimed.
- **Machine-readable error output:** if the error catalog needs structured exit
  codes or `--json` errors, that is a behavior change and belongs to a separate
  feature change.
- **Sibling/shared crate decisions:** not made here; default is keep-local.

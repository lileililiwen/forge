# Design: Post-MVP readiness for cargo-sonar-boost

## Boundary

- **In scope:** repository-local documentation and evidence owned by this
  repository — `README.md`, new `docs/*.md`, new screenshot/GIF assets under
  `docs/assets/`, and the OpenSpec change package plus `openspec/config.yaml`.
- **Out of scope:** runtime code paths (`src/**`), CLI contract, report schema,
  scoring model, CI workflow behavior, `docs/publishing_guide.md`, sibling
  repositories, and any shared/platform artifact.
- The only executable surface used is the already-shipped `cargo sonar-boost
  --demo` and the existing verification commands. Readiness is proven by
  reproducing evidence, not by adding features.

## Runtime and Commands

The authoritative commands for this repository (`.project.json` verification
command plus the existing CI workflow) are:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --verbose
cargo install --path .
cargo sonar-boost --demo --no-open --output-dir target/sonar-demo
```

`--demo` is the only evidence generator. It builds a mock `SonarReport` from
`src/demo.rs`, writes `report.json` and `report.html` into the chosen output
directory (`src/exporter.rs`), and opens the report in a browser unless
`--no-open` is passed (`src/main.rs`). It does not require a scanned project.

## Ownership

| Artifact | Owner | Notes |
|---|---|---|
| `README.md` | This repository | Single entry point; no sibling edits. |
| `docs/*.md` | This repository | Authored by this change. |
| `docs/assets/*` (screenshots, `demo.gif`) | This repository | Generated from `--demo`; producing command recorded next to the asset. |
| `src/**`, `Cargo.toml`, `.github/workflows/ci.yml`, `docs/publishing_guide.md` | This repository, unchanged | Referenced only; any needed change is a separate approved change. |
| `openspec/**` | This repository | Change package and promoted `readiness` spec. |

## Behavioral Table

| Surface | Current behavior | This change | Verification |
|---|---|---|---|
| CLI flags | `--demo`, `--no-open`, `--output-dir` (default `target/sonar`), `--project-dir` | Unchanged; documented only | `cargo sonar-boost --demo --no-open --output-dir ...` reproduces outputs |
| Demo mode | Builds a mock report and writes JSON + HTML | Unchanged | Output files exist and match screenshots |
| Tool aggregation | clippy always run; deny/geiger/tarpaulin probed and skipped with a printed warning when absent | Unchanged; documented as a matrix | Matrix matches `src/analyzer.rs` probes and messages |
| Health score | Weighted 0–100 score and A–F grade from clippy/security/coverage/unsafe | Unchanged; documented | Screenshot gauge matches `report.json` `summary` |
| HTML report | Self-contained interactive dashboard (gauges, clippy browser, coverage, unsafe, light/dark) | Unchanged; captured in screenshot | Opens from `file://`; fields match `report.json` |
| README | Install + usage overview | Adds evidence and deep-doc links | Links resolve; quickstart commands run |
| CI workflow | `cargo fmt --check`, clippy `-D warnings`, `cargo test` | Unchanged; documented as a recipe | Recipe uses existing flags only |
| License metadata | `Cargo.toml` already `license = "MIT"` with readme/repository | Unchanged; verified | `LICENSE`, `Cargo.toml`, README agree |

## Contract and Compatibility

- No public contract change. CLI arguments, exit behavior, `report.json` shape,
  the scoring model, and `report.html` layout MUST remain as shipped in v0.1.1.
- Documentation MUST describe the shipped version only; it MUST NOT advertise
  unshipped flags.
- `docs/tool-availability.md` MUST reflect the exact probes and skip messages in
  `src/analyzer.rs` (`cargo <tool> --version`; per-tool "not installed" message).
- If a screenshot or document would require a behavior change, the change is
  split (see Ledger).

## Failure Policy

- A screenshot or GIF that cannot be produced in the available environment MUST
  be recorded as **BLOCKED** with the exact failed command and output.
  Fabricated or hand-edited mock images are prohibited.
- Documents that claim a command works MUST be reproduced by running that
  command; a doc that cannot be reproduced is **FAIL**, not "done".
- `cargo fmt`, clippy, and tests MUST pass before completion; a passing skeleton
  is not readiness evidence.
- Sibling/shared-work decisions not made in this change MUST be listed in the
  Ledger, never guessed.

## Oracle (exact commands)

```bash
# Repository gates
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --verbose

# Evidence reproduction from a clean install
cargo install --path .
cargo sonar-boost --demo --no-open --output-dir target/sonar-demo
test -s target/sonar-demo/report.json
test -s target/sonar-demo/report.html

# Documentation links and OpenSpec strict validation
openspec validate post-mvp-readiness --strict --no-interactive
```

PASS requires every command to exit 0 and the two report files to be non-empty.
Any non-zero exit or missing file is FAIL/BLOCKED with the exact command and
output recorded.

## Ledger (blocked, not guessed)

- **Interactive browser open:** not exercised in a headless environment; the
  recipe uses `--no-open`. Do not claim the auto-open path was verified.
- **Dashboard screenshot/GIF capture:** requires a real browser for the HTML
  screenshot and a screen recorder for the GIF. Screenshotting `report.html` can
  use a headless browser; the GIF needs a recorder. Tool availability is not
  assumed; if unavailable, mark BLOCKED with the exact command.
- **Tool-availability matrix reproduction:** verifying every skip message
  requires each optional tool to be absent at least once. If that state cannot be
  produced in the environment, the matrix entries MUST be marked unverified
  rather than presented as observed.
- **`cargo-tarpaulin` side effect:** `run_tarpaulin` writes its own output under
  `target/sonar`; the CI recipe MUST account for that path and MUST NOT claim the
  demo and a real run share a directory.
- **Windows/macOS evidence:** not required; parity is not claimed.
- **Machine-readable error output:** if the error catalog needs structured exit
  codes or `--json` errors, that is a behavior change and belongs to a separate
  feature change.
- **Sibling/shared crate decisions:** not made here; default is keep-local.

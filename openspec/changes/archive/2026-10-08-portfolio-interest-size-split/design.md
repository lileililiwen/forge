# Design: Portfolio interest size split

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  crate (library + binary). No sibling touched.
- **Module changed:** `src/portfolio/interest/mod.rs`
  (1,002 lines) → `mod.rs` (~785 lines) +
  `src/portfolio/interest/vocabulary.rs` (~240 lines).
- **Modules reused unchanged:** every other `src/` file and every
  `tests/` file. No `mod ...;` line in `src/lib.rs`,
  `src/main.rs`, or `src/portfolio/mod.rs` changes; only this one
  module gains a `pub mod vocabulary;` declaration plus `pub use`
  re-exports.
- **Must NOT change:** any public symbol path, refusal code,
  contract string (`forge-portfolio-interest/0.1.0`,
  `forge-portfolio-activation/0.1.0`), metric label, CLI output,
  JSON shape, journal column, route, catalog row, env var, or
  behavior. The four vocabulary enums and their `impl` blocks
  move with their bodies verbatim.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Build: `cargo build`.
  Test: the interest unit tests plus the portfolio/interest
  contract suites before and after, counts identical.
- Linux; no platform-specific code added or moved.

## 3. Ownership and shared code

The module follows the types-vs-records seam already described
in its own module doc ("what a snapshot *is*"):

```
src/portfolio/interest/mod.rs  →  mod.rs (doc, constants, key sets,
                                   refusal vocabulary, record shapes,
                                   helpers, unit tests)
                                  vocabulary.rs (PrivacyMode, Coverage,
                                   InterestMetric, SnapshotState +
                                   their impls, verbatim)
```

`mod.rs` declares `pub mod vocabulary;` and re-exports every
moved name:

```rust
pub use vocabulary::{Coverage, InterestMetric, PrivacyMode, SnapshotState};
```

so every existing caller path
(`crate::portfolio::interest::PrivacyMode`,
`crate::portfolio::interest::InterestMetric::ALL`, …) keeps
resolving without an import fix. The moved items need only
`use serde::Serialize;` in the new file (the `Serialize`
derives); nothing else is imported. The `// --- <area> ---`
section banners move with their enums.

## 4. User experience and interface

`UI/UX: N/A`. No CLI change, no API change, no frontend change,
no journal change, no env change. The only observable difference
is the file shape on disk, which the operator never sees.

## 5. Behavioral model

None — this is a pure move. Each enum and `impl` body is copied
verbatim. The only mechanical changes are the `pub mod`
declaration, the `pub use` re-exports, and the single
`use serde::Serialize;` in the new file (per §10 verbatim-move
rules).

## 6. Contract and compatibility

- No new or modified public symbol path. `cargo doc` output, the
  `forge --help` output, every `forge <verb> --help`, every
  `GET /v1/admin/...` envelope, every catalog row, every
  interest/activation record, every registry/journal row, every
  CLI exit code — all byte-identical.
- No `cargo test` regression: interest unit tests and the
  portfolio/interest contract suites report identical counts
  before and after.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A moved name misses its re-export | `cargo build` fails compile; fixed before commit |
| The new file misses an import | `cargo build` fails compile; fixed before commit |
| A caller needs an import fix | Only the import line changes; no logic change |
| A resulting file still exceeds the cap | Not the case here (~785 / ~240); otherwise split further |

## 8. Verification oracle

- **Build:** `cargo build` reports 0 errors
  (pre-existing warnings only).
- **Tests:** interest unit tests (`portfolio::interest`) plus the
  portfolio/interest contract suites report identical passed /
  failed / ignored counts before and after.
- **Gate:** `source-file-size` drops 48 → 47 (this is the first
  `src/`-side file of the grind); no new Gate failure.
- **Full pre-archive suite:** `cargo fmt`, `cargo fmt --check`,
  `node scripts/check-openspec-change-names.mjs`,
  `node scripts/check-spec-governance.mjs`,
  `openspec validate --all --strict --no-interactive`,
  `git diff --check`, `forge gate --dry-run` + `forge gate`.
- A task box is checked only with the command output for its
  assertion.

## 9. Follow-on roadmap (one file per future change)

This change is file 4 of the grind after the archived
`source-file-size-remediation` first slice
(`tests/kit_contract/` done) and files 1–3
(`tests/gate_contract/`, `tests/supervised_agent_contract/`,
`tests/portfolio_contract/` done). The remaining oversized files
stay as pinned in the parent
`openspec/changes/archive/2026-10-08-source-file-size-remediation/design.md`
§9 — including the still-oversized
`src/portfolio/interest/activation.rs` (1,054),
`src/registry/interest/mod.rs` (1,047), and
`src/portfolio/mod.rs` (1,019), which are explicitly NOT part of
this change. The next implementer picks the next single file from
that table without re-deciding the strategy.

## 10. Decision ledger

- **Resolved:** split `src/portfolio/interest/mod.rs` along its
  types-vs-records seam in this change. The four closed
  vocabularies are the coherent extraction: self-contained
  (only `serde::Serialize`), referenced as one group, and
  exactly large enough (~228 lines) to bring `mod.rs` under the
  cap while leaving the new file far under it.
- **Resolved:** keep every moved body verbatim with `pub use`
  re-exports, so no caller import changes (parent §10 rule).
- **Resolved:** do not touch `activation.rs`,
  `registry/interest/mod.rs`, or `portfolio/mod.rs` — one file
  per change per parent §9.
- **Blockers:** none.

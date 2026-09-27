# Design: Forge post-MVP readiness

## Boundary

This package owns audience-facing readiness for Forge's already-implemented
v0.1 surfaces: `README.md` command mapping, the install *plan* presentation, a
committed quickstart transcript/terminal capture, governance-provider prose,
and the `LICENSE` file matching the declared MIT expression. It owns no Forge
behavior.

In scope: `README.md`, a committed quickstart capture under `docs/`, the
`LICENSE` file (coordinated with `artifact-and-ci-baseline`), and link-only
touches to `docs/provider-evidence.md`/`docs/release-readiness.md`.

Out of scope: `src/**`, `Cargo.toml` (beyond making the declared license true),
registry schema, CLI flags, emitted documents, CI workflows and `.project.json`.
The real packaging scripts remain owned by the active `artifact-and-ci-baseline`
change.

## Runtime and commands

The documented runtime is the existing one; this package adds no runtime.

- Build: `cargo build` → `./target/debug/forge`.
- v0.1 surfaces: `forge.yaml`, the project/profile registries, and
  `forge import`, `forge list`, `forge inspect`, `forge new`, `forge doctor`
  (profiles: `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`,
  `python-service`).
- Governance: `local` provider is the default with no `.forge/providers.yaml`;
  `workspace-governance` is an optional preset.
- Authoritative verification: `cargo test --workspace` (recorded in
  `.project.json`) and `scripts/release-check.sh`.

## Ownership

| Concern | Owner | This package's role |
| --- | --- | --- |
| Manifest, registries, import/list/inspect/new/doctor | `src/core`, `src/registry`, `src/import`, `src/generate`, `src/doctor` | Describe and demonstrate; change nothing |
| Packaging scripts, `[workspace]`, `rust-version`, `deny.toml`, `CHANGELOG.md`, `LICENSE` | `artifact-and-ci-baseline` (active, unimplemented) | Present the plan and, if archived first, consume its `LICENSE` and commands; do not duplicate |
| Governance provider default and sibling boundary | `src/governance`, `governance-provider-contract` spec | Describe the keep-local default accurately |
| `workspace-governance` adapter and its execute bit | `workspace-governance` (sibling) | Document the preset and the sibling-owned prerequisite; never work around it |
| Gate runtime | `driftwatchdog` (sibling) | Do not touch; claim no gate pass |

## Behavioral table

| Condition | Behaviour |
| --- | --- |
| README documents a command behavior the binary does not have | Documentation defect; fix the text, never the code in this package |
| Documented command output diverges from a real run | Re-capture the transcript from the real binary; never hand-edit |
| Documentation implies packaging exists | Reject; state the plan and name `artifact-and-ci-baseline` as owner |
| `LICENSE` absent while `Cargo.toml` declares MIT | Readiness fails; `LICENSE` is created once, never in both changes |
| `workspace-governance` candidate is not executable | Document as a sibling-owned prerequisite with the explicit-adapter remedy; do not modify the sibling |
| No `.forge/providers.yaml` | Document the `local` default explicitly; do not require a sibling to run |

## Contracts and compatibility

- No command, flag, exit code, emitted document, registry schema or provider
  contract changes.
- Existing README sections (Status, MVP and delivery, Documentation quickstart,
  Optional governance providers, External DriftWatch check, Shared gate runtime
  evidence, Product boundaries) are preserved; content is mapped and extended.
- `LICENSE` is additive and matches the declared expression; no relicensing.
- The sibling boundary is unchanged: `workspace-governance` remains optional,
  the `local` provider remains the default, and no parent-directory or network
  search is introduced.

## Failure policy

- A missing `LICENSE` is a readiness failure, not a warning, and is resolved
  once with `artifact-and-ci-baseline`.
- A command map that misdescribes a surface blocks the README requirement until
  corrected from a real run.
- An install claim that overstates the plan is rejected.
- A sibling-owned prerequisite (adapter execute bit, uninitialized
  `driftwatch` store) is recorded in the ledger; this package does not change
  the sibling to obtain green.
- No gate pass and no deployment claim is made.

## Oracle (exact commands)

Run from the repository root unless noted.

```bash
# 1. Licensing
grep -n 'license' Cargo.toml          # expect MIT
test -f LICENSE

# 2. Build and demonstrate the v0.1 surfaces
cargo build
./target/debug/forge --version
./target/debug/forge import .          # or a scratch project
./target/debug/forge list
./target/debug/forge inspect <id-or-path>
./target/debug/forge new rust-web --name demo --out /tmp/forge-demo
./target/debug/forge doctor .

# 3. Governance keep-local default
./target/debug/forge governance list .
./target/debug/forge governance status .
# optional preset (sibling-owned execute bit required):
./target/debug/forge governance use workspace-governance . --workspace-root /path/to/workspace

# 4. Full regression
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
node scripts/check-openspec-change-names.mjs
openspec validate --all --strict --no-interactive
```

## Ledger (blockers not guessed)

- **Packaging ownership** — the real `scripts/package.sh`/`install.sh`/
  `checksum.sh` are owned by the active `artifact-and-ci-baseline`; this
  package's install section must not claim they exist until that change is
  archived. If they diverge, prefer this package consuming the implemented
  commands.
- **`LICENSE` single writer** — whichever of this package and
  `artifact-and-ci-baseline` is implemented first creates `LICENSE`; the other
  must consume it. Not resolved by guessing.
- **`workspace-governance` execute bit** — the sibling ships
  `scripts/forge_governance_adapter.py` as mode `100644`; the documented
  non-executable refusal is a sibling-owned prerequisite, and no workaround is
  invented here.
- **`driftwatch` store** — no `.driftwatch` store is initialized in this
  checkout, so no gate pass can be claimed; the sibling-owned action is
  recorded, not guessed.
- **Capture format** — asciinema `.cast`, an SVG render, or a plain transcript
  is chosen during implementation; the choice is recorded.
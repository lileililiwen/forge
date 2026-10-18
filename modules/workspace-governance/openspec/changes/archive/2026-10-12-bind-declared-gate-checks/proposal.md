# Proposal: Bind every declared gate check to a command derived from disk

## Why

Every repository in this workspace declares, in its own
`.ai-gate/gate.yaml`, which checks it intends to run. Most of those
declarations name **no command at all**. The Gate then reports
`UNBOUND` — "no command declared for this concern; no execution was
attempted" — and counts the project as failing.

This is ruling D18 in `AGENTS.md`: *a verifier that no gate runs does
not gate; wire every check into the gate that blocks, or it is
documentation.* Measured on 2026-10-04 against the 81 repositories under
`/home/paul/code`:

| Measurement | Value |
|---|---|
| Repositories | 81 |
| Repositories with a `.ai-gate/gate.yaml` | 76 |
| Required checks (`checks: <id>: true`) across those manifests | 421 |
| Required checks that already carry a `commands:` entry | 138 |
| **Required checks with no command** | **283** |
| Repositories with at least one such check | **69** |
| Repositories fully wired today | 7 |

The 283 unbound checks are concentrated in four concerns — `security`
69, `tests` 68, `build` 66, `repository` 61 — and spread over fifteen in
total. `labrys` is the reference model: its manifest binds all nine of
its concerns to real commands (`cargo build --workspace`,
`cargo test --workspace`, `cargo audit`, `git diff --check`, …).

So the fleet is not short of commands. It is short of the binding step
that would put each repository's own commands in its own manifest.

## Why a generator, and not 69 hand edits

Hand-editing 69 files produces 69 hand-maintained copies that drift the
moment a repository adds a second crate, a second solution, or a
renamed script. The binding has to be **derived from what is on disk**,
so it is regenerated rather than remembered.

A template repository is the same defect wearing a different hat: it
would write one repository's shape into 69 others. This change derives
each command from the repository's own files, or refuses.

## Why the declared `stack` cannot drive it

`.project.json` `stack` is wrong for a large minority of the fleet, and
the generator must not read it. Measured shapes:

| Declared `stack` | What is actually on disk | Repositories |
|---|---|---|
| `dotnet` | a `.sln` **or a `.slnx`** solution | 45 |
| `dotnet` | `.csproj` files and **no solution at all** | 3 |
| `rust` | `Cargo.toml` with `[workspace]` | 10 |
| `rust` | `Cargo.toml`, no `[workspace]` | 7 |
| `typescript` | `package.json` at the repository root | 9 |
| `typescript` | `package.json` nested, no root manifest | 2 |
| `python` | a detectable test runner | 8 |
| `flutter` | `pubspec.yaml` | 1 |
| `python` | Rust + Node + Go, no Python at the root | 1 |
| `rust` | Rust + Node, both at the root | 1 |

Two of these deserve naming, because a depth-limited search gets both
wrong:

- **19 repositories use `LoreMix.slnx`-style `.slnx` solutions and no
  `.sln` at all.** A `*.sln` glob finds nothing and the repository looks
  like an unstructured project. Every one of them pins
  `global.json` to SDK `10.0.400`, which reads `.slnx` natively.
- **`alltools-platform` declares `stack: python` and has no Python at
  the repository root**: its Rust workspace is under `rust/` and its
  Node tooling is at the root.

The conclusion is the design constraint: **detection walks the disk and
never reads the label.**

## What Changes

One generator, `scripts/bind_gate_commands.py`, which for every
repository:

1. **Detects** the real shape from disk — `*.sln` / `*.slnx` solutions,
   `*.csproj` trees, `Cargo.toml` (and whether `[workspace]` is
   present), `package.json` `scripts` keys, pytest configuration,
   `pubspec.yaml`, and security tooling the repository itself declares.
2. **Derives** one command per concern from the detected shape, never
   from `stack`.
3. **Binds only what the manifest already declares** in `checks:`. It
   adds no concern and removes none.
4. **Refuses rather than guesses.** An unknown shape, a `package.json`
   without the needed script, an absent test runner, or two equally
   plausible build entry points is recorded as `unresolved` with the
   reason and the evidence. A wrong command is worse than an honest gap.
5. **Leaves a durable debt record** — `gate-debt.json` at the governance
   root, one machine-readable entry per unresolved repository — so the
   next agent to inherit a repository cannot quietly inherit the gap.
6. **Runs dry by default.** `--apply` writes.

## The record is read: a new change waits on the debt

Writing the record is not enough. 22 repositories carry an unbound required
check today, and nothing stopped the next feature from starting on top of
them. The debt then sits still, and a cleanup push that never finishes is the
likely shape of the failure.

So the record is read by a second rule, `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`,
in `scripts/workspace_check.py`: a repository with debt that opens a **new**
change is an `ERROR`. The change that closes the debt is exempt, because a
repository that may not start the work which fixes its debt can never fix it.
And the changes that were *already* open when the rule landed are recorded once
in `gate-debt-baseline.json`, generated from disk — without that, the rule
fails the whole legacy fleet for the state it was written to stop growing, and
no action those repositories can take clears it.

Both halves are read, not recomputed — the unbound half from `gate-debt.json`
(set-equal to the gate's own unbound set by `GateOracleTest`), the
unarchived-change half from the same directory walk `validate_change_names`
uses. There is no second parser over manifests.

The rule is fail-closed and has no off switch. Its grandfathering and its exit
are two committed records in the shape of `rules-drift-baseline.json`:
`gate-debt-baseline.json` (`(project, change)`, the work that predates the
rule) and `gate-debt-exceptions.json` (`(project, change, reason)`, a
deliberate decision about new work). Both are loaded with the same refusals, and
there is no environment variable and no flag. A recorded exception is reported
at `WARNING` with its reason quoted, so a deliberate decision is visible in the
run rather than invisible in a pass.

The code is added to `GOVERNANCE_QUALITY_DECLARED_CODES` in `driftwatchdog`
in the same change. Without that, the concern filters the finding out and
reports `PASS` while this checker exits 1.

## What this deliberately does not do

**It does not invent a command for an ambiguous repository.** Six
repositories have two equally plausible entry points for the same
concern — `linglitch` has a .NET solution under `backend/` and a Node
`test` script under `frontend/`; `retakeKit` has a root solution *and* a
root `package.json` with a `test` script. Binding either one would
assert that the repository builds only one way, which is not what is on
disk. These are reported, not resolved.

**It does not bind a `build` for a pure-Python repository.** A pure
Python package has no build step to run. `python3 -m compileall` and
`python3 -m build` are both available answers and neither is what the
repository asked for, so the concern stays unresolved with both named.

**It does not mine prose.** `workspace-governance`'s own `product_quality`
command is written in its manifest `notes:` and its checker sits at
`scripts/product_code_quality.py`. A rule that reads notes and pairs them
with a filename is a guess wearing a rule's clothes; the concern is
instead bound from the file that exists, and only that.

**It does not touch the 7 already-wired repositories' commands.** An
existing `commands:` entry that disagrees with a derived one is
reported as a conflict and left exactly as it is.

## Impact

| Surface | Before | After |
|---|---|---|
| Required checks the gate calls unbound | 125 | **47** |
| Repositories with an unbound check | 53 | **24** |
| Repositories fully wired (of 76 with a manifest) | 23 | **52** |
| `placeholder-threshold` / `product-code-boundary` unbound | 39 / 39 | **0 / 0** |
| Sibling repositories written | 0 | **39** — the authorised set, `.project.json` and `.ai-gate/gate.yaml` only |
| Manifests hand-edited | 0 | 0 |
| Marker vocabulary | 8 tokens, 2 of them ordinary English | 6 tokens, all unambiguous |
| Test suite | `Ran 615 tests … OK` | `748 passed, 158 subtests passed` (see `tasks.md` §3.4, §7.8) |

The original measurement in *Why* put the figure at 283 unbound across 69
repositories. That count read `checks:` alone and missed the checks a
`profile:` schedules; §5.1 corrected it against the real binary to 125 across
53, and the numbers above are on the binary's footing throughout.


The final write is the one this change is authorised for and no more: 39
repositories, two files each, verified by hashing both files across all 81
repositories before and after. The 47 that remain unbound are reported in
`gate-debt.json`, and the record and the gate's own unbound set are set-equal
against the repository build of `driftwatchdog` — not by count. The
`driftwatchdog` on `PATH` is a stale 2026-10-01 build that answers 0 for the
whole fleet and is not an oracle.

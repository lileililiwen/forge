# Proposal: Derive a composite command for a polyglot repository

## Why

44 required checks across 22 repositories are unbound. 24 of them are the same
refusal, and the refusal is wrong.

`scripts/bind_gate_commands.py` returns **one** command per concern. When it
finds two equally plausible entry points it refuses, on the rule that *a wrong
command is worse than a listed gap*. That rule is right and it stays. But the
tool has only two answers — bind one, or refuse — and a polyglot repository
needs a third:

> Build the .NET backend **and** the npm frontend.

`crossify` holds `backend/CourseBridge.slnx` and `frontend/package.json`. The
gate currently refuses `build`, `security` and `tests` for all three, because
`dotnet build backend/CourseBridge.slnx` and `cd frontend && npm run build` are
two candidates of equal standing. But they are not two answers to one question.
They are two halves of one repository, and the correct command is both, joined:

    dotnet build backend/CourseBridge.slnx && cd frontend && npm run build

That is not a guess and it is not a coin flip. It is the only command that
builds the whole repository, and the gate's job is to build the whole
repository. Binding one half is the false green this workspace keeps finding: a
green `build` on a repository whose frontend was never built.

24 checks, in 11 repositories:

| Repository | Concerns refused for two entry points |
|---|---|
| `alltools-platform` | `build`, `security` |
| `catchen` | `tests` |
| `crossify` | `build`, `security`, `tests` |
| `hestiaLab` | `tests` |
| `linglitch` | `build`, `security`, `tests` |
| `opendockify` | `tests` |
| `orphevia` | `build`, `security`, `tests` |
| `reelio` | `tests` |
| `retakeKit` | `build`, `security`, `tests` |
| `rust-ecommerce` | `security` |
| `sisyphusfy` | `security` |

## What Changes

The binder composes commands for disjoint product surfaces when their
candidates come from distinct stacks, while preserving the existing dominance
rule and same-stack refusal. Each composite is stable, names its source
candidates, and executes each part in its own shell directory.

## What is not changed

The dominance rule stays. A root-level entry point still beats a nested one, so
a repository with `package.json` at the root and one in `packages/x/` still
resolves to the root without a composite. Only genuinely disjoint entry points
compose.

The refusal stays for the cases that are genuinely undecidable from disk. This
change does not touch the other 20 refusals:

- **no checker to run** (`accessibility`, `privacy`, `provenance`, `fixtures`,
  `schemas`) — 11 checks. The repository has no `scripts/check-*.py`, and
  inventing one is real work, not derivation.
- **no build step** (`alethefy`, `ariadex`, `jenkins-local`, `sisyphusfy`
  `build`) — 4 checks. A pure-Python package has nothing to build; the honest
  answer is a packaging decision by the owner.
- **several `.csproj` and no solution** (`aegify`, `hestiaLab`, `smotoox`) —
  6 checks. The fix is a `.sln` in the repository.
- **no Python security tool declared** (`jenkins-local`, `platform-contracts`,
  `workspace-governance`) — 3 checks. Declaring `pip-audit` is the owner's
  call.

Those 24 stay in the debt record with their reasons, which is exactly what the
record is for.

## The rule

Two or more candidates compose into one command **when every one of them must
run for the repository to be sound**. The test for that is not "there are two of
them" — it is *do they cover disjoint product surfaces?* Two candidates that
both answer `tests` for the same Rust workspace are redundant, not composite.
Two that answer `build` for a .NET backend and a Node frontend are composite,
because neither implies the other.

The distinction is made by the stack each candidate came from, which the
generators already know. A composite is only offered when the candidates span
**two or more distinct stacks**. Within one stack the existing dominance rule
still decides, because two `npm test` entry points in one repository is a
workspace layout question, not two halves of a product.

The joined command is ` && `-separated, in a stable order: the evidence's stack
order as the generators produce it, so two runs on an unchanged tree produce
byte-identical output.

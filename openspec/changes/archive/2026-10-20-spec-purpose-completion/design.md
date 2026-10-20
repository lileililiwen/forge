## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The fourteen canonical capabilities' `## Purpose` paragraphs | `forge` | Yes. `openspec/specs/`. |
| The guard that refuses a placeholder purpose | `forge` | Yes. `tests/spec_purpose_contract.rs`. |
| The requirements, scenarios and wording of every capability | `forge` | No. Read only. |
| The OpenSpec tool | its own package | No. Read only. |

## The Problem

The archive writes a placeholder into every capability it creates:

```js
// openspec's specs-apply.js
export function buildSpecSkeleton(specFolderName, changeName) {
    return `# ${specFolderName} Specification\n\n## Purpose\nTBD - created by archiving change ${changeName}. Update Purpose after archive.\n\n## Requirements\n`;
}
```

That skeleton is written only when the capability is **new**. When an archive
updates an existing capability it rewrites the requirements section and
leaves the Purpose alone. So a purpose written once stays written; a
placeholder stays a placeholder until somebody replaces it.

Fourteen never were:

| Capability | What it holds |
|---|---|
| `ci-cd-system-model-documentation` | the maintainer guide that traces the CI/CD system model |
| `developer-workflow-facade` | the facade that keeps the developer workflow on one path |
| `dotnet-platform-libs-monorepo` | the .NET platform libraries monorepo |
| `forge-governance-consumer-migration` | the migration of the governance consumers |
| `forge-native-gate` | the native Gate runtime |
| `forge-workspace-governance-in-tree` | workspace governance living inside the repository |
| `gate-declaration-migration` | the migration of the Gate declarations |
| `hosted-ci-evidence-envelope` | the hosted CI evidence envelope |
| `hosted-ci-evidence-ingest` | the hosted CI evidence ingestion |
| `local-runtime-entrypoints` | the local runtime entry points |
| `origin-repository-retirement` | the retirement of the origin repository |
| `rust-platform-libs-monorepo` | the Rust platform libraries monorepo |
| `source-file-size-gate` | the source-file size gate |
| `web-project-delivery-timeline` | the web project's delivery timeline |

The other 119 capabilities already state a purpose. The shortest of those is
81 characters, so a purpose is expected to be a paragraph, not a label.

## The Purpose

Each purpose is one paragraph, written from that capability's own
requirements. It says what the capability is for, in the product's own terms,
in the present tense — the same voice the capabilities that already have a
purpose use:

> Read-only maturity assessment through forge doctor: a stable finding
> inventory (PASS/WARN/FAIL/UNAVAILABLE) with evidence and remediation
> classes, target-gated L0–L4 applicability, and registry-observation
> staleness, never masking missing evidence as healthy.

The rules for writing one:

- One paragraph, 35–70 words, no bullet points and no headings inside it.
- Every claim is supported by a requirement in that file. Nothing is
  invented.
- The capability's own vocabulary — the requirement headers and the scenario
  wording — is reused rather than replaced with new terms.
- No "TBD", no "TODO", no "placeholder", no mention of the archive or the
  change that created the capability.
- It starts with the subject, not with "This capability".

## The Guard

The placeholder recurs, because every archive writes one. Two changes in this
programme had theirs replaced by hand; the next would have gone unnoticed.

The guard is a repository contract test, `tests/spec_purpose_contract.rs`.
It reads every `openspec/specs/*/spec.md` and refuses a capability whose
Purpose is:

- missing,
- the archive placeholder (or any line that starts with "TBD - created by
  archiving"),
- shorter than 60 characters — a label is not a purpose.

The test names the capability it refuses, so the failure says which file to
open and what to write. It is a Rust integration test because that is where
the repository's contracts live: the Gate's `tests` check runs the suite
(`scripts/check-tests.sh`) and CI's `test` job runs the same command, so one
test covers both.

The guard is not a red suite waiting to happen: the archive writes the
placeholder *after* the Gate has run, so a change that forgets to replace it
is caught by the next Gate run — before that next change ships, and while
the fix is one paragraph.

## User Experience and Interface

Nothing to type. A reader who opens one of the fourteen capabilities now finds
a sentence that says what it is for, above requirements that are unchanged.

## Behavioral Model

| Situation | Outcome |
|---|---|
| A capability's purpose is the archive placeholder | The contract test fails and names the capability. |
| A capability has no purpose section | The contract test fails and names the capability. |
| A capability's purpose is a single word | The contract test fails: a purpose is a paragraph. |
| A capability states a real purpose | The contract test passes. |
| A change archives a new capability | The archive writes a placeholder; the next Gate run refuses it until it is written. |

## Contract and Compatibility

The capability specifications keep their names, their requirement headers,
their scenarios and their wording. Only the `## Purpose` paragraph changes,
and it changes from a sentence that says nothing to a sentence that says
what the capability is for.

The new capability `spec-purpose-completion` is added. No existing capability
is added, removed, merged, split or renamed, so no consumer of the specs
sees a different contract.

## Failure and Boundary Policy

- **The guard reads only.** It reads the specification files and asserts on
  them. It writes nothing and changes no file.
- **The guard is fail-closed.** A specification it cannot read is a failure,
  not a skip. A missing directory is a failure too: an empty spec tree would
  otherwise pass silently.
- **The guard reads the repository's own specifications.** It resolves them
  from `CARGO_MANIFEST_DIR`, so it reads the tree it was built from and not
  whatever happens to be in the working directory.
- **No purpose is invented by the tooling.** The guard refuses; a person
  writes.

## Verification Oracle

```bash
# The Forge Core test suite
cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1

# The repository Gate
./target/debug/forge gate --dry-run
./target/debug/forge gate --timeout-secs 1800

# Strict OpenSpec validation and whitespace
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
git diff --check
```

A real run: `cargo test --test spec_purpose_contract` must fail while a
placeholder is in place and pass once the fourteen purposes are written.

## Implementation Plan

**The purposes.** The fourteen `## Purpose` paragraphs, one per capability,
written from that capability's requirements.

**The guard.** `tests/spec_purpose_contract.rs` (new): read every
`openspec/specs/*/spec.md`, assert the purpose exists, is not the archive
placeholder, and is a paragraph.

**The tests.** The guard's own cases: a capability with a real purpose
passes; one with the placeholder fails; one with no purpose fails; one with a
one-word purpose fails.

## Decision Ledger

- **Resolved:** the purposes are written by hand, not generated. A generated
  paragraph would restate the requirements in the generic shape OpenSpec's
  own prose templates use, which is exactly what a purpose must not be.
- **Resolved:** the guard is a Rust integration test, not a Gate check
  script. The Gate's check commands are generated by
  `bind_gate_commands.py` and the `product_code_quality.py` checker is
  vendored from the sibling workspace-governance module, so a new rule there
  would diverge from its source. A test needs no binding and runs in both the
  Gate and CI.
- **Resolved:** the guard refuses a purpose shorter than 60 characters. The
  shortest real purpose in the tree is 81, so the threshold catches a label
  without constraining a legitimate short paragraph.
- **Resolved:** only the fourteen placeholder capabilities are rewritten. The
  other 119 keep the purposes they have.
- **Implementation authorized (2026-10-20):** the owner directed the trunk
  repair programme and authorized this package in the approved order, after
  `lifecycle-observation-closure` closed the doctor's observation of the
  delivery.

## Requirement Traceability

| Requirement | Proposal outcome | UX flow | Design decision | Implementation boundary | Success/failure/boundary scenarios | Tasks | Verification oracle |
|---|---|---|---|---|---|---|---|
| Every capability states its purpose | The fourteen placeholders are replaced with real purposes | A reader learns what the capability is for | One paragraph, written from the requirements | `openspec/specs/` | A real purpose | 3.1 | The guard test |
| The purpose is the only thing that changes | The requirements are untouched | The contract below the purpose is unchanged | Paragraph-only edit | `openspec/specs/` | The requirements are identical before and after | 3.1 | The diff |
| The placeholder cannot survive unnoticed | The contract test refuses it | The failure names the capability | A test, not a bound checker | `tests/spec_purpose_contract.rs` | A placeholder; no purpose; a label | 3.2–3.3 | The guard test |

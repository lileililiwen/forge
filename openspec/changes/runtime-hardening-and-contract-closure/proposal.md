# Proposal: runtime-hardening-and-contract-closure

## Why

Seven change packages were implemented, verified and committed on `main` without
being archived, because each one recorded `openspec archive` as "deliberately not
run: the other changes are parked". That is inverted: archive is the per-change
terminator that removes a change from flight, so waiving the one-active-change rule
removed the only forcing function and six changes each cited the other five.

The queue jammed at `contract-parity-gate-real-digests` tasks §4.4, which left
`check-spec-governance.mjs` failing on
`openspec/specs/scaffold-prewires-shared-layer/spec.md` as an unowned blocker. That
failure was fixed by `5019d12`, and no later session re-ran the check. The blocker
was gone; the reasons to skip archive were not.

Splitting along discovery order also split mechanisms that are shared in code:

- `src/governance.rs` was edited by three of the five code commits
  (`21a9566`, `5d5f103`, `e3a156b`) and `src/process.rs::write_request` now carries
  the rule for four call sites, so two packages specify one requirement.
- `f078a4c` deleted the ~100 lines `5d5f103` had written into
  `tests/studio_preview_contract.rs` and moved the logic into the shared
  `tests/support/studio_ports.rs`, so two packages specify one requirement and one
  of them describes an implementation that no longer exists.

Archiving them separately would have promoted two near-duplicate requirements into
`runtime-hardening-and-test-isolation` and written the broken-pipe rule twice, at
two different scopes. This package merges them first: one requirement per mechanism,
one archive, so the canonical specs state what the shipped code does.

## What Changes

Implementation is already committed. This package consolidates the specification
record for it and closes the archive step.

| Absorbed change | Commit | Requirement promoted here |
|---|---|---|
| `governance-adapter-request-write-race` | `21a9566` | Adapter request-write race is not a provider failure (merged with the shared-boundary rule below) |
| `governance-adapter-bounded-process-run` | `5d5f103` | Bounded adapter subprocess run; Bounded source revision lookup |
| `studio-preview-contract-port-range` | `5d5f103` | Studio test port ranges are chosen at run time outside the host ephemeral window (merged) |
| `adapter-request-write-boundary` | `e3a156b` | the request-write rule widened from governance to every adapter, translator and delivery boundary, at `src/process.rs::write_request` |
| `studio-test-port-range` | `f078a4c` | the same port requirement, widened to all four Studio targets and `tests/support/studio_ports.rs` |
| `contract-parity-gate-real-digests` | `c138f08` | the parity gate requirements and the modified digest-pinned vendored contract set |
| `manifest-wire-contract-shape` | `619b945` | The public manifest is emitted in the contracted shape |

Merges applied, and what was preserved:

- **Request write.** One requirement in `governance-provider-contract`, wording from
  `adapter-request-write-boundary` (one shared boundary, four call sites, never
  discarded at a call site) carrying all four unioned scenarios. It is NOT also filed
  under `runtime-hardening-and-test-isolation`.
- **Studio test ports.** One requirement in `runtime-hardening-and-test-isolation`,
  body from both deltas (run-time window outside the ephemeral range, every port
  bindable when chosen, assertions computed from the configured base, listeners held
  for the collision test, per-target candidate index) carrying the union of six
  scenarios.
- Everything else is carried verbatim: the bounded adapter run (5 scenarios), the
  bounded revision lookup (3), the port allocator refusal invariant (3), the parity
  gate (1 modified + 5 added, 20 scenarios), the manifest wire shape (5).

## BFS Impact Map

| Surface | Affected fact |
|---|---|
| `src/governance.rs` | `run_adapter` drains both pipes concurrently, waits on `recv_timeout`, bounds `git_revision` by the provider `timeout_ms`; `write_adapter_request` delegates to the shared rule |
| `src/process.rs` | `write_request(stdin: &mut impl Write, request: &[u8])` is the one request-write boundary; `BrokenPipe` is `Ok(())` |
| `src/publish/providers.rs`, `src/docs/mod.rs`, `src/delivery/hermora.rs` | each calls the shared rule and maps any other error to its own typed refusal after kill and reap |
| `src/portfolio/share/manifest.rs` | `wire_manifest_revision` emits the contracted strings; internal revision stays an integer everywhere |
| `scripts/contract-parity.sh` | compares vendored bytes against the resolved source, counts comparisons, exits non-zero on zero comparisons or an unresolvable source |
| `tests/support/studio_ports.rs` | shared run-time port selection for four Studio targets |
| `contracts/**` | mirror re-synced; `contracts/schemas/public-portfolio-manifest.schema.json` present |
| canonical specs | `governance-provider-contract`, `runtime-hardening-and-test-isolation`, `site-studio-preview-refinement`, `platform-contract-consumption`, `portfolio-share` |
| operators | a share approval made before the manifest encoding change is refused until re-preview and re-approve |

## Capabilities

No new capability id. Existing specs gain requirements:

- `governance-provider-contract` — bounded adapter execution and a request-write race
  that does not become a provider failure.
- `runtime-hardening-and-test-isolation` — test port windows chosen at run time.
- `site-studio-preview-refinement` — a refused port allocation never kills a listener.
- `platform-contract-consumption` — a parity gate that compares real bytes and cannot
  report a pass it did not earn.
- `portfolio-share` — the emitted manifest satisfies the pinned contract.

## Non-goals

- **No implementation change.** The code is committed; this package only consolidates
  and closes the specification record.
- **The pipe-buffer deadlock shape at the three non-governance boundaries.**
  `src/publish/providers.rs:589`, `src/docs/mod.rs:684` and
  `src/delivery/hermora.rs:180` drain stdout only after `try_wait` reports exit and
  cap nothing. Misclassification ("timed out") rather than a hang; no adapter in this
  repository produces more than ~10 KiB. Still open; see `design.md` §5.
- **Unifying the two bounded-run implementations.** `src/process.rs::spawn_with_timeout`
  (50 ms poll, no stdin write, `String` errors) is not reused by `run_adapter`, which
  needs the 256 KiB `GovernanceInvalid` cap and a typed error taxonomy.
- **The other seventeen `try_wait` loops in the tree.**
- **`Text file busy (os error 26)`.** Root-caused to a fork/exec write-descriptor race
  in test fixture staging; no fix fits one change. Recorded in `HANDOFF.md`.
- **Narrowing or widening the shared contract vocabularies** for `visibility`,
  `status_evidence` and `id` length. A product decision, its own change.
- **Retrying, sleeping, `#[ignore]`ing or weakening any assertion.**

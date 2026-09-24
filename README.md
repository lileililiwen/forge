# Forge

Forge is a language-agnostic developer control plane and software assembly platform. It coordinates heterogeneous projects, resolves reusable software parts, generates ordinary source deterministically and uses AI for interpretation and unresolved project-specific work.

## Status

Implemented baseline with five archived audit/foundation follow-ups and eight archived sibling-integration packages (newest `gate-runtime-evidence`). The repository began with [Requirements & Product Design v0.3](requirement.md); it now ships a Rust Core/CLI workspace (`src/`, `cargo build` produces `./target/debug/forge`) with a SQLite-backed registry, 37 archived OpenSpec changes and promoted canonical specs under [openspec/specs/](openspec/specs/), plus contract and cross-surface test suites. The requirements document version is not a delivered Forge release. The last audit found one MCP test that depends on a read-only host registry; native profile matrix, packaging/CI and real provider round trips remain separately qualified, with per-cycle evidence recorded in [HANDOFF.md](HANDOFF.md).

## MVP and delivery

v0.1 is deliberately limited to `forge.yaml`, project/profile registries, and `forge import`, `forge list`, `forge inspect`, `forge new`, `forge doctor`. Its profiles are `aspnet-web`, `rust-web`, `nextjs-web`, `flutter-app`, and `python-service`. These commands are available from this checkout via `cargo build`.

v0.2 adds features and upgrades; v0.3 integrates DriftWatch, specs and existing agent infrastructure; v0.4 exposes mature MCP operations; v0.5 adds repository distribution, translations, releases and deployment. Advanced components, UI patterns, AI planning, identity, analytics, API and portal are implemented as later changes (`forge component`, `forge ui-pattern`, `forge intent`, `forge identity`, `forge analytics`, `forge api serve`, `forge portal dashboard|view`). Machine-facing checker emission is available as `forge check` (see [external DriftWatch checker](docs/external-checker.md)).

See the [dependency-ordered roadmap](ROADMAP.md), [complete section coverage](docs/requirements-coverage.md), [architecture](docs/architecture.md), and [current handoff](HANDOFF.md).

## Documentation quickstart

With Node.js and the OpenSpec CLI available (validated here with OpenSpec 1.6.0):

```sh
node scripts/check-openspec-change-names.mjs
openspec list
openspec status --change core-manifest-registry
openspec validate --all --strict --no-interactive
```

Foundation toolchain (established by `core-manifest-registry`, see
[ADR 0001](docs/adr/0001-foundation-toolchain.md)): Rust stable with
`rusqlite` bundled (no system SQLite required).

```sh
cargo fmt --check
cargo build        # produces ./target/debug/forge
cargo test
cargo clippy --all-targets -- -D warnings
```

There is no Forge installation packaging yet.

## Optional governance providers

Forge is standalone by default. With no `.forge/providers.yaml`, the built-in
`local` provider validates the project's canonical `forge.yaml`; no sibling
repository, network service, account, or external binary is required.

```sh
forge governance list .
forge governance status .
forge governance use local .
forge governance use workspace-governance . --workspace-root /path/to/workspace
forge governance use workspace-governance . --adapter /path/to/adapter
forge --format json governance status .
```

`workspace-governance` is a known provider with a packaged adapter preset:
`--workspace-root` (or `FORGE_WORKSPACE_ROOT`) names the workspace/portfolio
root whose `workspace-governance/scripts/forge_governance_adapter.py` is the
packaged adapter. Selection verifies the candidate is an existing executable
regular file, refuses otherwise while naming the exact candidate path, and
never searches parent directories or the network. The resolved adapter path
and the workspace root are stored under `.forge/providers.yaml`, so later
checks do not depend on the environment; the root is re-supplied to the
adapter as `WORKSPACE_ROOT` exactly as the sibling documents its own
invocation. An explicit `--adapter` always wins over the preset.

External providers use the versioned `0.1.0` JSON adapter contract and are
optional. Provider failures are reported as `unavailable` or `incompatible`
observations and do not disable local Forge workflows. Provider selection is
stored under `.forge/`, not in `forge.yaml`, and switching providers preserves
the manifest and registry identity.

## External DriftWatch checker

Any Driftwatchdog-monitored project can gate on Forge's read-only assessment
evidence by registering `forge check` as an external checker. The command
prints one protocol-compatible alerts document on stdout, mutates nothing,
journals nothing, and never drives DriftWatch itself unless the operator
explicitly passes `--include-policy`. See
[docs/external-checker.md](docs/external-checker.md) for the envelope
contract and the `driftwatch.toml` registration snippet.

## Shared gate runtime evidence

Driftwatchdog's Gate owns plan resolution, blocking policy, its own run
history and exit semantics. `forge gate [TARGET]` resolves the runtime a
project declares (`FORGE_GATE_BIN` explicitly, else `.project.json`
`verification.gate_runtime`, else the ordered `driftwatchdog` → `driftwatch`
probe), executes the real `gate --format json` through one bounded
argument-array invocation, and journals a revision-bound record at
`.forge/gate/<project-id>/evidence.json`. A parseable status document is
evidence whatever the exit code — a blocked gate records `blocked` and
exits non-zero, mirroring the sibling — while an unresolvable binary, a
timeout or an unparseable answer stays honestly `unavailable` with prior
evidence untouched. `forge gate . --dry-run` rehearses through the
runtime's side-effect-free plan preview and persists or journals nothing.

```sh
forge gate .
forge gate . --dry-run
forge gate status .
forge --format json gate <project-id-or-path>
```

Doctor's `gate-evidence` finding, the release `gate` check kind and the
`gate-runtime` provider row consume the same record: stale evidence never
satisfies a verification claim, an absent one reads `unverified` and never
a pass, and captured output is credential-redacted and host-path-scrubbed.
No gate tool is exposed over MCP or the API, and no gate pass is claimed
for this repository — the real run honestly reports
`gate-runtime-unavailable` until the sibling store (`driftwatch init`) is
initialized in this checkout. See
[docs/provider-evidence.md](docs/provider-evidence.md) for the probe
boundary and `tests/fixtures/gate/NOTES.md` for the verbatim sibling
captures.

## Product boundaries

- Deterministic templates, packages, codemods and migrations precede AI generation.
- Generated projects must build and operate through their native tools without Forge.
- The product model stays independent of language, framework, AI vendor, IDE and host.
- Forge coordinates DriftWatch, the existing PTY agent manager, content and analytics tools; it does not replace them.
- Project maturity is evidence-based and optional to advance; L0 experimentation is valid.
- Forge is not a programming language, low-code runtime, IDE, CMS, Git host, AI model, universal runtime, Kubernetes replacement or generic CI/CD replacement.

The full authoritative brief remains in [requirement.md](requirement.md). [AGENTS.md](AGENTS.md) defines the project contribution entry point.

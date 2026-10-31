## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| Binary resolution | `forge` | Yes |
| The checkout layout (`target/{debug,release}`) | the Cargo workspace | read only |
| The packaged archive | `artifact-and-ci-baseline` | No |
| Doctor's finding status | `doctor-claim-evidence` | read only |

## Resolution Order

1. `FORGE_DRIFTWATCH_BIN` — an explicit operator choice, used verbatim.
2. `PATH` — `driftwatchdog`, then `driftwatch`, executable files only.
3. The source checkout — `CARGO_MANIFEST_DIR/target/{debug,release}/driftwatchdog`,
   executable only.
4. `Unavailable` — the reason names all three attempts.

The checkout step is last so an operator's installed runtime and an explicit
override always win. It is only reachable when Forge was built from a checkout
that still contains the built workspace member, which is exactly the case the
false `UNAVAILABLE` described.

## Why Not Only PATH

The adapter was written for the sibling-integration era, when Driftwatchdog was
an external install. Now the workspace builds it. Requiring a developer to
prefix `PATH` with `target/debug` makes the default `forge doctor .` result a
false negative, and CI would need the same manual step.

## Why Checkers, Not the Gate

`policy_surface_args` used to run `gate --format json` for any project with a
`gate.toml` or `.ai-gate/gate.yaml`. On this repository that ran the full Gate
(build plus the suite) inside `forge doctor`, which exceeded the 60-second
adapter timeout. The Gate already has its own evidence finding and its own
verb; the policy plane is the configured Driftwatch checkers. The adapter now
always runs `check --dry-run --format json`, which writes nothing and returns in
milliseconds.

A checker report with zero configured checkers is not an unavailable plane. It
is an empty configuration: the run succeeded and had nothing to judge. The
report carries `checkers`, and the verdict distinguishes `NotConfigured`
(WARN) from `Unjudged` (a configured rule that did not apply, still
UNAVAILABLE). The release lane records `NotConfigured` as a disabled check.

## Why Not Package It

The release archive is the Forge control plane, not the policy runtime. Shipping
a second binary would double the archive and create a version-coupling surface
the archive does not have today. An installed Forge without a runtime still
reports the policy plane unavailable, honestly.

## Failure Boundaries

- An override path that does not exist still fails exactly as before; the
  checkout fallback is not consulted.
- A non-executable file in `target/debug` is skipped, and the release path is
  tried.
- The finding remains `UNAVAILABLE` when no runtime exists, so a host cannot
  become healthy by the fallback alone.

## Verification Strategy

1. `cargo test -p forge --lib policy::` covers PATH preference and the new
   checkout fallback.
2. `forge doctor .` reports the policy plane from `target/debug/driftwatchdog`
   (pass or warning, not unavailable) and the verdict is checked.
3. The repository suite, Gate, formatting and OpenSpec validation pass.

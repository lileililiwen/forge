# Proposal: A cross-cutting shared capability is declarable in every profile

## Why

`capability-adoption-gate` made shared-capability adoption measurable as a
three-way join: the capability registry (`capabilities.json`), a pinned
dependency in the consumer's own manifests, and a `consumes[]` record in the
consumer's `.project.json`. The record is written under
`capabilities[<capability-id>]`.

But `validate_capabilities` reports WARNING `CAPABILITY_UNSUPPORTED` for any
declared capability id that is not in the project profile's
`supported_capabilities` list, and **no profile lists the shared capability
ids the registry binds**. A consumer that records a lawful, correctly-evidenced
adoption of `telemetry`, `rate-limiting`, `idempotency`, or `feature-flags` is
therefore reported as an anomaly, and a profile owner must edit a profile
before any consumer may document adoption.

Measured, not argued. A `dotnet-product` declaration that records
`capabilities.telemetry` with a resolving `consumes[]` entry produces exactly
one finding, reproduced by invoking the checker directly:

```
WARNING CAPABILITY_UNSUPPORTED capability 'telemetry' is not declared as
supported by the project profile
```

The four ids share one property the seven product-shaped ids do not: they are
**cross-cutting**. Telemetry, rate-limiting, idempotency, and feature-flags
apply to a provider, a product, a library, and a tool alike. A profile list
that cannot admit them turns a correct record into a warning.

This blocks the first real consumer: `argoscope` cannot adopt
`Platform.Observability`/`Platform.Web.Telemetry` and record the required
`consumes[]` edge without the spurious warning.

## What

Add the four cross-cutting shared capability ids — `telemetry`,
`rate-limiting`, `idempotency`, `feature-flags` — to the
`supported_capabilities` list of every profile that declares one.

Nothing else changes. `CAPABILITY_UNSUPPORTED` keeps its purpose: a capability
id the profile genuinely does not support still warns. The existing negative
control (`billing` under `dotnet-library`) is preserved and re-pinned.

## What this deliberately does not do

- **It does not add the registry's role-specific ids** (`persistence-postgres`,
  `health-checks`, `web-application-factory`, `postgres-test-fixture`). They
  describe a hosting, persistence, or test-harness role rather than a
  cross-cutting concern, and are admitted on demand when a consumer records an
  adoption. `supported_capabilities` stays an intentional, maintainer-owned
  list rather than a mirror of the registry.
- **It does not change the checker.** The union-with-registry alternative was
  rejected in `design.md` §4: it makes the profile list redundant, changes the
  meaning of a hook that reads no registry today, and removes the intentional
  review step the list exists to force.
- **It does not touch any sibling repository and does not edit
  `capabilities.json`.** The registry already binds `telemetry` to
  `dotnet-platform-libs`; no posture, provider, or graduation changes.

## Impact

| Surface | Before | After |
|---|---|---|
| Profiles | 12 lists, 7 product ids each (narrower for library/governance) | same lists plus the 4 cross-cutting ids |
| `dotnet-product` declaring `telemetry` | 1 `CAPABILITY_UNSUPPORTED` WARNING | no finding |
| Negative control (`billing` under `dotnet-library`) | WARNING | WARNING (unchanged) |
| `capabilities.json` | 9 registered capabilities | unchanged |
| Fleet audit | `errors=279 warnings=20` | unchanged: the change removes only a warning a consumer chose to incur |
| Declarations written | — | none; 0 sibling files touched |

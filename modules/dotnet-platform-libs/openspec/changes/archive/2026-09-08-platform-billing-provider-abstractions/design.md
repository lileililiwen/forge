# Design

`Platform.Billing` owns normalized workflows and depends on existing billing contracts,
idempotency, eventing, jobs, and core time. `IBillingProvider` exposes checkout, portal,
subscription lookup, and raw webhook verification/normalization. `IBillingEventProjector`
updates consumer-owned subscription/entitlement stores through explicit interfaces.

Applications register a plan catalog and map local plans to provider references outside the
platform package. Provider events are deduplicated by provider name plus provider event ID,
then normalized before projection. Entitlement checks are deterministic and expose reasons,
expiry, usage, and provider state.

Use in-memory stores for demos and test doubles for deterministic tests. Persistent stores and
provider adapters are separate changes.

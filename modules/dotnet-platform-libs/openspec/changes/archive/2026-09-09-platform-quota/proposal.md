## Why

The existing billing contracts can check usage limits, but sibling applications also need atomic reservation, settlement, and release semantics for expensive or asynchronous work. The starter and Crossify both contain this logic, yet it is embedded in product plans and entities.

## What Changes

- Add generic quota dimensions, windows, usage snapshots, and reservation lifecycle contracts.
- Support check, reserve, settle, release, and idempotent operation semantics.
- Add deterministic in-memory implementation and testing builders.
- Define an optional bridge to billing entitlements without owning plans, prices, or ledgers.
- Add concurrency and failure tests for oversubscription and repeated operations.

## Capabilities

### New Capabilities

- `platform-quota`: provider-neutral quota checks and usage reservation lifecycle.

### Modified Capabilities

- None.

## Impact

- New `Platform.Quota` and `Platform.Quota.Testing` packages.
- Public quota and reservation APIs; no product plan or persistence model.
- May depend on `Platform.Core` and billing contracts only through a narrow optional integration seam.

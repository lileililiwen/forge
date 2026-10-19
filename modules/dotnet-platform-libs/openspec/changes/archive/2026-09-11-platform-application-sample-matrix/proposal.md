## Why

The current sample proves the composite starter path, but it does not show the incremental adoption decisions that distinguish this platform from the complex starter kit. A small matrix of focused samples would make package boundaries, ownership, and rollback behavior executable and reviewable.

## What Changes

- Add focused conformance samples for minimal web, EF Core, identity, tenancy, and one provider adapter.
- Keep each sample small, independently buildable, and explicit about package references.
- Add a matrix test/documentation layer that verifies incremental adoption and package ownership boundaries.

## Capabilities

### New Capabilities

- `sample-matrix`: focused application samples demonstrating incremental platform adoption.

### Modified Capabilities

- None.

## Impact

Adds sample projects, test fixtures, documentation, and solution entries. Production packages and application business models are unaffected.

## Non-Goals

- No production application or full-stack demo.
- No React, Aspire, Docker, Terraform, billing product, catalog, tickets, or chat sample.

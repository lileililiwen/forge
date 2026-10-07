# Staged product adoption

Repeatable adoption stages for .NET business products. Each product opts into
platform packages independently. No product is described as migrated without
native project evidence (restore, build, test, and gate runs inside that
product's own repository).

The shared contract that wires a product to the platform lives in
[`docs/platform-consumer-adoption.md`](platform-consumer-adoption.md): the
anonymous public feed, package-source mapping, exact
`PlatformPackageVersion`, optional user-level local source, and clean-machine
verification. The old source bootstrap is compatibility-only.
The .NET 10 baseline that every product must adopt first is described in
[`docs/dotnet10-migration-contract.md`](dotnet10-migration-contract.md).

## Stages (every product)

1. **Inventory** — Run the adoption diagnostics against the product
   (`Platform.Adoption.Tool` with `--project-dir`) and record the per-package
   evidence levels (`Absent`, `Configured`, `Incompatible`, `Unverified`,
   `Verified`). A package without native verification evidence stays
   `Unverified` at most; it is never called production-ready. Install the
   canonical `NuGet.config` mapping and one exact `PlatformPackageVersion`
   before adding platform references.
2. **Pilot** — Adopt one package in a non-critical service with an exact
   version pin. Copy the canonical `NuGet.config` and run the consumer
   conformance fixture against the pilot's services.
3. **Verify** — Promote `Unverified` to `Verified` only with native evidence:
   the product's own tests and gates covering registration, replacement,
   health, and failure safety.
4. **Expand** — Promote the package to other services one at a time with the
   same smoke test in each. Record each promotion in the product's
   `CHANGELOG.md`.
5. **Rollback** — Re-pin `PlatformPackageVersion` to the previous exact version
   (`scripts/consumer-upgrade-rollback.sh` pattern). Uninstalling a package
   never affects application-owned users, roles, migrations, provider
   credentials, or UI, which stay application-owned throughout.

## Per-product starting points

| Product | Suggested first package | Notes |
|---|---|---|
| `chinago` | `Platform.Core` + `Platform.Testing` | Contracts and test toolkit first; providers stay application-owned. |
| `arivio` | `Platform.Core` + `Platform.Testing` | Same contracts-first staging. |
| `ploutify` | `Platform.Billing.Contracts` | Normalized subscription/entitlement snapshots; provider IDs and invoice rules stay in the product. |
| `fotofy` | `Platform.Storage` + `Platform.Storage.Local` | Provider-neutral storage; credentials and buckets stay in the product. |
| `smotoox` | `Platform.Mailing` | Mail contract with application-owned templates and provider keys. |
| `cvunify` | `Platform.Identity.Contracts` | Identity lifecycle against the existing identity store, which remains the source of truth. |
| `stylify` | `Platform.Web` + `Platform.AspNetCore` | Web runtime conventions; routes and auth schemes stay application-owned. |

The table lists starting points only. Actual adoption is confirmed exclusively
by per-product native evidence; this document makes no migration claim for
any product.

## Rollback (every product)

- Remove the `Platform.*` package reference or re-pin the previous version.
- Delete copied fixture projects; no platform source copy ever exists in the
  product.
- Application stores, migrations, credentials, and UI are untouched by both
  adoption and rollback.

# Platform sample matrix

Five focused samples proving incremental adoption: each one restores
only its declared platform capabilities, owns its application state,
and removes cleanly. `samples/matrix.json` is the machine-readable
index; `tests/Platform.SampleMatrix.Tests` verifies every entry
against the projects on disk.

## Stages

| Stage | Sample | Platform references |
| --- | --- | --- |
| 1 minimal web | `samples/Platform.MinimalWeb.Sample` | `Platform.Starter` |
| 2 EF Core | `samples/Platform.EfCore.Sample` | `Platform.Persistence.EfCore` |
| 3 identity | `samples/Platform.Identity.Sample` | `Platform.Identity.Contracts` |
| 4 tenancy | `samples/Platform.Tenancy.Sample` | `Platform.Tenant.Lifecycle`, `Platform.Tenant.Lifecycle.Contracts` |
| 5 provider adapter | `samples/Platform.ProviderStorage.Sample` | `Platform.Storage`, `Platform.Storage.Local` |

Start with the smallest matching stage and add the next one only
when the application needs that capability. Every sample targets
`net10.0`, builds independently (`dotnet build` on its own project
restores nothing outside its declared references), and carries a
README with ownership and rollback notes.

## Verification

```bash
dotnet test tests/Platform.SampleMatrix.Tests -c Release
```

Per-sample suites cover the intended wiring: root plus `/live` for
stage 1, migration apply plus insert/query for stage 2, login
success/401/400 for stage 3, ordered two-step provisioning for
stage 4, and local upload/download/metadata/delete for stage 5. The
metadata test asserts each `matrix.json` entry matches its project
(package references, framework, unpacked marker, README, rollback).

## Environment-blocked semantics

All five stages are deterministic: no network, no credentials, no
Docker. A live provider check (for example S3 against a real bucket)
would live outside this matrix and, when unavailable, must report
blocked rather than green — the same classification the adoption
tooling uses for feed and Docker probes.

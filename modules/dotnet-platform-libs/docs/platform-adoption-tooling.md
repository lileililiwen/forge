# Platform adoption tooling

Read-only diagnostics for independent subprojects: SDK compatibility,
package pinning, test-only boundaries, and project quality signals for
an explicit target directory.

## Commands

The tool lives at `tools/Platform.Adoption.Tool` and consumes the
provider-neutral `Platform.Adoption` diagnostic core. It never modifies
the inspected target.

```bash
dotnet run --project tools/Platform.Adoption.Tool -- doctor --project-dir /abs/path/to/consumer
dotnet run --project tools/Platform.Adoption.Tool -- inventory --project-dir /abs/path/to/consumer --json
dotnet run --project tools/Platform.Adoption.Tool -- conformance --project-dir /abs/path/to/consumer
dotnet run --project tools/Platform.Adoption.Tool -- preview --project-dir /abs/path/to/consumer
```

- `doctor` runs every source check and prints the resolved target
  first, then one line per check plus a summary.
- `inventory` lists discovered projects and package references.
- `conformance` runs the pinning and test-boundary checks only.
- `preview` lists proposed package alignment edits without writing
  files. There is no apply mode: add references manually or through a
  reviewed patch, and roll back by removing them.

## Options

- `--project-dir <absolute-path>` (required): the target directory.
  Relative paths and missing directories exit 64. Nothing is inferred
  from the current working directory.
- `--json`: machine-readable output with target, check identifier,
  status, evidence, and remediation or rerun guidance; no secrets.
- `--include-environment`: run environment-dependent checks.
- `--feed-url <url>`: probe feed reachability as an environment check.
- `--check-docker`: probe Docker daemon availability as an
  environment check.
- `--expected-platform-version <version>` (default `0.1.0`): the exact
  `Platform.*` pin that preview and outdated warnings compare against.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | No source failures and no environment-blocked checks. |
| `1` | At least one source check failed. |
| `2` | No source failures, but at least one check was environment-blocked. |
| `64` | Usage error (unknown command, missing `--project-dir`, unusable target). |

CI can fail on source failures (`1`) while counting blocked checks
(`2`) separately. Blocked results always carry a bounded rerun
instruction and never claim the consumer source is invalid.

## Checks

| Check | Result |
| --- | --- |
| `sdk` | Pass when `global.json` pins an SDK version; warning otherwise. |
| `solution` | Pass when a solution is discoverable; warning otherwise. |
| `projects` | Fails when no C# projects are discovered. |
| `central-packages` | Pass with central version management; warning otherwise. |
| `platform-pinning` | Fails on floating `Platform.*` versions, warns on unpinned or outdated pins, proposes exact preview edits. |
| `test-boundary` | Fails when a production project references a `*.Testing` package. |
| `nullable-warnings` | Pass with `Nullable` and `TreatWarningsAsErrors`; warning otherwise. |
| `environment-feed` | Blocked when the configured feed is unreachable (opt-in). |
| `environment-docker` | Blocked when the Docker daemon is unreachable (opt-in). |

Evidence is secret-free: relative paths, identifiers, versions, and
exception type names only. Fixtures covering minimal, adopted,
misconfigured, and environment-blocked consumers live under
`tests/Platform.Adoption.Tests/Fixtures/`.

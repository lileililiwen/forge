# Release governance

Production projects under `src/` are packable platform packages. Samples,
tests, and templates are excluded from the package inventory. Package metadata,
symbols, embedded sources, deterministic builds, and the repository README are
defined in `Directory.Build.props`.

## Versioning and compatibility

The default package version is `0.1.0`; a release must pass an explicit
`Version` or `VersionPrefix` override. Use SemVer: additive compatible changes
are minor, fixes are patch, and reviewed breaking changes are major. Breaking
changes require migration notes and a reviewed API-baseline update in the same
change. The selected public contract baseline is checked by:

```bash
./scripts/check-public-api.sh
./scripts/check-public-api.sh --update  # only in a reviewed release change
```

Record user-visible changes in `CHANGELOG.md` before publishing. Package
signing and SBOM generation are release-environment responsibilities; their
evidence must be attached to the release and unavailable evidence is not a
pass. No workflow publishes packages or changes versions without an explicit
release trigger.

The manual Forge-root `platform-dotnet-release` workflow publishes the
unchanged `Platform.*` IDs to the anonymous static NuGet v3 feed at
`https://lileililiwen.github.io/forge/nuget/index.json`. Sleet is pinned by
`dotnet-tools.json`. Publication refuses different bytes for an existing
ID/version, validates the feed, then deploys through GitHub Pages. Consumers
need no token. The consumer template also retains the former
`dotnet-platform-libs` feed as a fallback source for previously published
versions. Its remote Pages branch was unavailable during the monorepo import,
so this migration does not claim those bytes have been copied into Forge.
The repository setting **Pages → Source → GitHub Actions** must be enabled
before the first real dispatch; local workflow validation is not evidence
that this remote setting or endpoint is live.

## Verification

```bash
./scripts/quality-gate.sh
./scripts/conformance.sh
./scripts/audit-packages.sh
```

The default package source is the Huawei Cloud NuGet v3 mirror, selected for
mainland-China build reliability. Override it for an approved internal feed or
the official source without editing the repository:

```bash
NUGET_SOURCE=https://api.nuget.org/v3/index.json ./scripts/audit-packages.sh
```

The source is used for restore, packed-consumer verification, and vulnerability
metadata lookup. The audit uses NuGet restore-time scanning for all direct and
transitive dependencies at high severity. A mirror that does not expose current
vulnerability metadata must be treated as `UNVERIFIED`; it is never silently
accepted as a clean audit.

The conformance fixture consumes packed local NuGet artifacts, not project
references. The audit script reports `AUDIT_STATUS=UNVERIFIED` when its service
cannot provide evidence and exits non-zero unless explicitly overridden for a
non-release diagnostic run.

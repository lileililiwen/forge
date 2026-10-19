## Why

The platform is intended for adoption by 26+ independent applications, but the repository currently has no CI workflow that proves packed artifacts, API compatibility, security scanning, or release quality. A shared library can therefore compile locally while still being unsafe or difficult to consume.

## What Changes

- Add CI gates for restore, build, test, strict OpenSpec validation, packaging, and package smoke consumption.
- Add package metadata, versioning, changelog, and supported-framework documentation rules.
- Add API compatibility and dependency/vulnerability checks for production packages.
- Add reproducible package, symbol, source-link, and optional signing/SBOM configuration.
- Add an upgrade test from a previous package version and a rollback/release checklist.
- Do not publish packages or change package versions automatically without an explicit release trigger.

## Capabilities

### New Capabilities

- `platform-release-governance`: reproducible, validated, and consumer-tested package releases.

### Modified Capabilities

- None.

## Impact

Affected files include `.github/workflows/`, `Directory.Build.props`, `Directory.Packages.props`, package metadata, release documentation, and a packed-artifact consumer fixture. This changes repository delivery behavior, not runtime library APIs.

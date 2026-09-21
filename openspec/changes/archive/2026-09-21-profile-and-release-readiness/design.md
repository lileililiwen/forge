## Context

Forge advertises native-buildable generated projects but the current evidence
is distributed across unit tests and handoff notes. There is no checked-in CI
workflow or packaging/install contract. A release gate must therefore make
profile/toolchain evidence explicit and reproducible.

## Goals / Non-Goals

**Goals:** matrix coverage for `aspnet-web`, `rust-web`, `nextjs-web`,
`flutter-app`, `python-service`, and supported `react-web`; deterministic
fixture generation; CI parity; minimal artifact verification.

**Non-Goals:** building unavailable toolchains inside Forge, downloading
third-party runtimes implicitly, or making specialist profiles selectable.

## Decisions

The profile descriptor remains the source of build/test commands. A matrix
runner generates into disposable directories, removes Forge from PATH for the
native phase, captures toolchain versions, command, exit status, source hash
and timestamp, and classifies each row as passed, failed or unverified. CI
fails on a supported profile that is expected by the selected runner but does
not pass; unsupported host/toolchain rows are visible as unverified and cannot
be used as release evidence.

The first package artifact is the platform-native Forge binary produced by
Cargo, with checksums and `forge --version` smoke evidence. Packaging formats
and publication destinations remain bounded by existing release/distribution
contracts.

## Requirement and scenario coverage

- **R1 — Native profile evidence:** success, failure and unavailable-toolchain
  classifications for every advertised profile.
- **R2 — Reproducible release gate:** CI/local parity and artifact smoke checks.

## Failure and compatibility

Existing profile IDs, generated layouts and native commands remain compatible.
A failed or missing row never becomes `verified`; adding a profile requires a
descriptor, template, native commands and evidence before promotion.

## Migration Plan

No application-data migration. Add checked-in fixtures/workflow and release
metadata. Existing handoff evidence is retained as historical evidence and
reclassified where it does not meet the new matrix.

## Verification Strategy

Run the matrix locally for available toolchains, execute the same commands in
CI, build the binary, verify checksum/version/install behavior, and run the
full Rust quality suite plus strict OpenSpec validation.

## Open Questions

The supported CI runner images and signing authority are deployment decisions;
the package must record them before implementation rather than assuming a
provider or secret store.

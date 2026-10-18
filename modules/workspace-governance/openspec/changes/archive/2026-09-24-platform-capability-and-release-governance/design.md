# Design: Capability and release governance

## Capability manifest

Projects may declare optional capabilities such as identity, admin, tenancy,
billing, jobs, storage, and release artifact type. A capability describes
intent and ownership; it does not prove implementation. The audit reports
declared, configured, verified, and blocked states separately.

## Release evidence

The common evidence record is producer-neutral and includes project ID,
revision, version, toolchain identity, artifact digest, SBOM reference,
signature/provenance status, checks performed, and publication status. Missing
external evidence remains unavailable or unverified.

## Ownership

Workspace Governance owns vocabulary and policy. Driftwatchdog executes local
commands. Platform repositories own implementations. Jenkins-local owns
deployment execution. Products retain native build, test, and release
commands.

## Verification

Use synthetic declarations and evidence fixtures, then run workspace audit,
auto-gate, registry/schema tests, and strict OpenSpec validation. No project is
promoted to deployable from declarations alone.

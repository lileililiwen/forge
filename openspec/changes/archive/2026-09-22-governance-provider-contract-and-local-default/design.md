# Design: Standalone-first governance providers

## Ownership and boundaries

Forge Core owns the normalized project model, local registry, operation
journal, provider configuration, local observations, and provider selection.
An external provider owns its own registry, policy engine, gate runtime, and
provider-specific evidence. Forge imports observations; it does not adopt the
external provider's storage or authority model.

The built-in local provider is always available. An external provider is an
optional adapter selected by configuration or an explicit CLI command. A
provider failure affects external observations only unless the user explicitly
requests a provider-required policy check.

## Contract

The provider protocol is versioned independently from provider names. The
The first operation is `check`; the contract leaves room for future
`discover`, `inspect`, and `read_evidence` operations without coupling Core to
provider-specific models. Provider requests and responses use structured JSON
over an executable adapter boundary or an equivalent in-process adapter. Core
receives:

- provider identifier and protocol version;
- project identity and canonical path;
- observation status (`pass`, `fail`, `blocked`, `unknown`, `unavailable`, or
  `stale`);
- observed timestamp and source revision when available;
- evidence references and redacted findings;
- opaque provider metadata that Core does not interpret.

Provider-specific status values must be mapped to the normalized vocabulary;
unknown values are rejected as incompatible rather than treated as healthy.

## Configuration and switching

Provider configuration is optional and lives outside the canonical
`forge.yaml` project contract, in a versioned `.forge/providers.yaml` file.
The configuration identifies a provider, adapter command or adapter kind,
protocol version, timeout, and whether it is enabled. It contains no secrets.

The local provider is selected when no external provider is configured. The
selection command changes provider configuration only; it does not rewrite the
Forge registry, project manifest, or external provider state. Imported
observations retain provider, protocol version, source revision, and timestamp
so a provider switch cannot make old evidence appear current.

The first implementation exposes `governance list`, `governance status`,
`governance use <provider>`, and `governance inspect`. Existing local commands
remain valid without these commands or their configuration.

## Data flow and mutation boundary

1. Core resolves the selected provider and validates its configuration.
2. Core constructs a typed request with an explicit project target.
3. The adapter runs with argument arrays, bounded input/output, and a timeout.
4. Core validates the response and binds it to the requested project and source
   revision.
5. Core stores a timestamped observation and operation outcome.
6. Core exposes the normalized observation through CLI, MCP, API, and portal.

The provider contract is observation-oriented in this change. It does not
grant an adapter mutation authority over Forge projects. Future mutating
operations require separate capabilities, explicit plans, confirmation,
idempotency, and ownership receipts.

## Failure and compatibility behavior

- No provider configuration: use local provider.
- Disabled provider: report `disabled`; do not invoke it.
- Missing adapter: report `unavailable` with the adapter name and remediation.
- Timeout or malformed response: report `unavailable` or `incompatible` with
  bounded diagnostic evidence; do not write a healthy observation.
- Project identity mismatch: reject the observation without attaching it to a
  different project.
- Provider-declared stale observation: preserve it with `stale` status and its
  original source timestamp.
- Provider replacement: retain old observations as historical records and use
  only the newly selected provider for current status.

## Migration and compatibility

Existing manifests and registries remain valid because provider configuration
is optional. Existing doctor, provider, deployment, API, MCP, and portal
contracts continue to operate through Core. Existing provider integrations can
map into the new contract without requiring their source models to become
public Core types.

The Workspace Governance adapter is a separate optional implementation. It may
consume `projects.json`, `.project.json`, and checker reports, but those files
must not be parsed by the local provider or required by the base Forge binary.

## Verification strategy

Contract tests cover local success, no configuration, provider selection,
disabled and missing providers, malformed responses, timeout, stale evidence,
identity mismatch, replacement, and persistence across restart. Cross-surface
tests prove the same normalized observation is rendered by CLI, MCP, API, and
portal. Adapter tests use fixtures and do not claim live Workspace Governance
evidence. Native local workflows must pass when all external adapters and
network access are unavailable.

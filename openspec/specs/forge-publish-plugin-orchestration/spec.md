# forge-publish-plugin-orchestration Specification

## Purpose
Forge is the only user-facing publish system. Manual project/folder CLI
input and verified GitHub push deliveries enter the same publish engine,
the provider is a switchable external JSON-line process implementing
`forge-publish-provider/0.1.0`, and OpenPanel and Jenkins remain optional
providers that can be enabled or disabled independently without rewriting
the contract.
## Requirements
### Requirement: One publish engine

Forge SHALL route manual project, manual folder, and verified GitHub push
triggers through the same `PublishRequest` and publish state machine.

#### Scenario: Manual project publish

- **WHEN** an operator runs `forge publish --project <id>`
- **THEN** Forge resolves the registered project and invokes the selected
  enabled provider through the publish contract

#### Scenario: Manual folder publish

- **WHEN** an operator runs `forge publish --folder <path>`
- **THEN** Forge captures the folder revision and uses the same publish path
  without requiring a separate deployment implementation

#### Scenario: Verified push publish

- **WHEN** Forge receives a valid GitHub push event for an enabled project
- **THEN** Forge publishes the exact full commit SHA through the same engine

### Requirement: Switchable providers

Forge SHALL allow each provider to be listed, enabled, disabled, and inspected.
Disabled providers MUST NOT be invoked for new publish runs.

#### Scenario: Disable OpenPanel

- **WHEN** an operator disables the OpenPanel provider
- **THEN** new OpenPanel publish requests are refused before provider execution

#### Scenario: Enable Jenkins compatibility

- **WHEN** an operator enables the Jenkins provider
- **THEN** Forge may select it for projects explicitly configured for Jenkins

### Requirement: Provider contract

Forge SHALL validate the `forge-publish-provider/0.1.0` request and response
contract, including provider identity, operation id, revision, terminal status,
health state, and redacted evidence.

#### Scenario: Malformed provider response

- **WHEN** a provider returns invalid JSON or a different contract version
- **THEN** Forge records a failed run and does not claim deployment health

### Requirement: Idempotent push retries

Forge SHALL derive an operation identity from project, provider, revision, and
trigger id and SHALL return the existing terminal result for duplicate events.

#### Scenario: Duplicate GitHub delivery

- **WHEN** the same signed push delivery is received twice
- **THEN** Forge invokes the provider at most once for that operation identity

### Requirement: Provider isolation

Forge SHALL preserve provider-local secrets, source handling, and transport
details. A disabled or unavailable provider MUST NOT prevent Forge startup or
manual publishing through another enabled provider.

#### Scenario: OpenPanel unavailable

- **WHEN** the OpenPanel provider executable is missing
- **THEN** Forge reports OpenPanel unavailable while another provider remains
  listable and selectable

### Requirement: Extended plugin capability vocabulary and kinds

The plugin registry SHALL accept the closed capability set `topic,description,homepage,language,publish,delivery,gate,quality,agent,contract,analytics` and the kinds `metadata,delivery,gate,quality,agent,contract`, keeping `Metadata|Delivery` behavior unchanged and reporting anything outside the set as `Invalid` advertising nothing.

#### Scenario: New kinds resolve

- **WHEN** a descriptor names `kind: gate|quality|agent|contract` with a matching closed capability
- **THEN** `forge plugins list` reports that kind as `Ready` (when enabled and executable) with the advertised capabilities

#### Scenario: Unknown stays invalid

- **WHEN** a descriptor names an unknown kind or a capability outside the closed set
- **THEN** the record is `Invalid` naming `unknown plugin kind` or the `closed set` and advertises no capabilities

### Requirement: Builtin resolution with providers override

The registry SHALL resolve a plugin id from the `providers.yaml` descriptor,
then from the workspace declaration of the project the id belongs to when that
project declares it, then from `kits/manifest.json`, and only then from a
builtin table; it SHALL report which source answered each resolution. A
lower-precedence source SHALL never override a higher-precedence one, so a
declaration is never silently replaced by a compiled-in guess and a builtin is
never preferred to a declared value. A field the answering source does not state
SHALL be filled from the next source that states one, never overridden by it.
When a configured descriptor and a declaration disagree about an id, the
registry SHALL refuse with a typed error naming the id and both values rather
than choosing one. An id that nothing declares SHALL still resolve through the
builtin table unchanged.

#### Scenario: Declaration answers first

- **WHEN** a project declares `driftwatchdog` with kind `gate` and capability
  `gate`, and no descriptor names it
- **THEN** the row resolves from the declaration and reports the declaration as
  its source, with the same kind and capabilities the builtin table would give

#### Scenario: Builtin maps an undeclared id

- **WHEN** no declaration, descriptor or kit manifest names an id the builtin
  table knows
- **THEN** it resolves from the builtin table and reports the builtin as its
  source

#### Scenario: Descriptor and declaration disagree

- **WHEN** `providers.yaml` carries a `plugins:` descriptor for an id the project
  also declares, and the two disagree on kind or capabilities
- **THEN** the registry refuses naming the id and both values, and resolves no
  row for that id

#### Scenario: Builtin maps known ids

- **WHEN** no descriptor, declaration or kit manifest names `driftwatchdog`,
  `cargo-clippy`, `sisyphusfy|ariadex|mnemora`, `platform-contracts`,
  `labrys|openpanel|jenkins-local`, or `argoscope|devloom`
- **THEN** they resolve to `gate[gate]`, `quality[quality]`, `agent[agent]`,
  `contract[contract]`, `delivery[delivery]`, and `metadata[analytics]`
  respectively, each reporting the builtin table as its source

#### Scenario: Descriptor overrides builtin

- **WHEN** `providers.yaml` carries a `plugins:` descriptor for an id the
  builtin table also names, and the project declares nothing that contradicts it
- **THEN** the descriptor kind and capabilities win, the row reports the
  descriptor as its source, and the builtin is ignored for that id

#### Scenario: No declaration at all

- **WHEN** the project declares no interfaces and no capabilities
- **THEN** resolution falls through to the descriptor, manifest and builtin
  sources exactly as before, and nothing is reported as declared

### Requirement: Gate routing beside metadata routing

The registry SHALL expose `gate_plugins()` routing Ready gate plugins beside the existing `metadata_plugins()` routing.

#### Scenario: Gate routing selects ready gate plugins

- **WHEN** the registry holds a Ready enabled `gate` plugin and a disabled one
- **THEN** `gate_plugins()` returns only the Ready enabled entry in registry order

### Requirement: Cap grouping facade over flat commands

Forge SHALL offer `forge cap list|inspect <cap>|add|run` grouping the flat commands (`gate/check/doctor/readiness` under gate, `agent/studio/intent` under agent, `contract/component/standard` under contract, `delivery/deploy/publish` under delivery), keeping every old command as a working alias with no behavior change.

#### Scenario: Cap list derives from vocabulary plus live states

- **WHEN** `forge cap list` runs
- **THEN** it lists every `CAPABILITIES` entry with the live Ready plugin count from `forge plugins list` states

#### Scenario: Cap inspect names plugins and flat commands

- **WHEN** `forge cap inspect gate` runs
- **THEN** it names the Ready gate plugins and the mapped flat commands `gate, check, doctor, readiness`

#### Scenario: Old flat commands still work

- **WHEN** `forge gate --dry-run`, `forge agent list`, `forge contract list`, or `forge delivery status` runs
- **THEN** each behaves exactly as before the cap facade existed

### Requirement: A boundary declaration is readable or refused

Forge SHALL validate a boundary declaration before resolving anything from it,
and SHALL refuse with a typed error naming the offending field when: the
declaration cannot be parsed; an interface `mechanism` is outside the accepted
set; a non-sibling interface names a contract without a version; or a `consume`
or `observe` interface carries no digest. An absent or empty interface list SHALL
be valid and SHALL be treated as "nothing declared", never as an error, so a
project that declares no boundary keeps working exactly as before.

#### Scenario: Unknown mechanism refused

- **WHEN** a declaration names an interface mechanism outside the accepted set
- **THEN** Forge refuses naming the field, the value and the accepted set, and
  resolves nothing from that declaration

#### Scenario: Undigested consumption refused

- **WHEN** a `consume` interface carries no digest
- **THEN** Forge refuses naming that interface and the missing digest

#### Scenario: Empty declaration is silent

- **WHEN** a project's declaration has no interfaces
- **THEN** Forge reports no error and continues with the remaining sources

### Requirement: Forge declares its own interfaces

Forge SHALL declare the boundaries it provides and consumes in its own project
declaration, with each entry's mechanism, entry point and contract version
derived from the existing versioned-surface inventory rather than restated, so
the declaration and `forge contract list` cannot disagree. Where a contract
version does not exist yet, the declaration SHALL record the boundary without a
version — as an `integrations[]` provider when the schema's versioned-contract
rule leaves no lawful interface for it — rather than invent one, and SHALL leave
the missing version visible to governance. Forge's retained copies of another
project's bytes SHALL each be declared as a consumption edge naming the provider,
the contract, the version and the digest already recorded in its vendored
manifest.

#### Scenario: Derived values agree with the inventory

- **WHEN** the declaration's contract versions are compared with the versioned
  surface inventory
- **THEN** every version present in the inventory appears identically in the
  declaration, and a version absent from the inventory is absent from the
  declaration

#### Scenario: A missing version is declared, not invented

- **WHEN** a boundary has no contract version today
- **THEN** the declaration records the boundary without a version (as an
  `integrations[]` provider, because the schema requires a versioned contract on
  every interface) and governance reports the gap

#### Scenario: Retained bytes are declared

- **WHEN** Forge keeps a copy of another project's artifact
- **THEN** a consumption entry names its provider, contract, version and the
  digest of the retained bytes — the value already recorded in
  `contracts/manifest.json` or `kits/manifest.json` where one pins the copy, and
  the copy's own sha256 otherwise — and no copy remains unaccounted for


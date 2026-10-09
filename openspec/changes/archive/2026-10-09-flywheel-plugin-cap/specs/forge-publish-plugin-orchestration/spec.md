# forge-publish-plugin-orchestration (delta)

## ADDED Requirements

### Requirement: Extended plugin capability vocabulary and kinds

The plugin registry SHALL accept the closed capability set `topic,description,homepage,language,publish,delivery,gate,quality,agent,contract,analytics` and the kinds `metadata,delivery,gate,quality,agent,contract`, keeping `Metadata|Delivery` behavior unchanged and reporting anything outside the set as `Invalid` advertising nothing.

#### Scenario: New kinds resolve

- **WHEN** a descriptor names `kind: gate|quality|agent|contract` with a matching closed capability
- **THEN** `forge plugins list` reports that kind as `Ready` (when enabled and executable) with the advertised capabilities

#### Scenario: Unknown stays invalid

- **WHEN** a descriptor names an unknown kind or a capability outside the closed set
- **THEN** the record is `Invalid` naming `unknown plugin kind` or the `closed set` and advertises no capabilities

### Requirement: Builtin resolution with providers override

The registry SHALL resolve known plugin ids from a builtin table consulting `kits/manifest.json` first, with the `providers.yaml` descriptor overriding the builtin when present.

#### Scenario: Builtin maps known ids

- **WHEN** no descriptor names `driftwatchdog`, `cargo-clippy`, `sisyphusfy|ariadex|mnemora`, `platform-contracts`, `labrys|openpanel|jenkins-local`, or `argoscope|devloom`
- **THEN** they resolve to `gate[gate]`, `quality[quality]`, `agent[agent]`, `contract[contract]`, `delivery[delivery]`, and `metadata[analytics]` respectively

#### Scenario: Descriptor overrides builtin

- **WHEN** `providers.yaml` carries a `plugins:` descriptor for an id the builtin table also names
- **THEN** the descriptor kind and capabilities win and the builtin is ignored for that id

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

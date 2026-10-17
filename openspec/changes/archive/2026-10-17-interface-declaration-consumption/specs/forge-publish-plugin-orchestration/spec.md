# forge-publish-plugin-orchestration Specification (delta)

## MODIFIED Requirements

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

## ADDED Requirements

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

# doctor-claim-evidence Specification

## Purpose
Truthful doctor statements: a finding and a control state only what the doctor actually read, the registry observation reports an unknown registration state as unknown instead of claiming the project is unregistered, the callers say whether they consulted the registry, and the drift and structure statements name the checks behind them.
## Requirements
### Requirement: A finding and a control state only what the doctor read

Forge SHALL state in a finding's evidence, in a finding's detail and in a
control's requirement text only the checks the doctor performed and the
facts it read. Forge SHALL NOT credit evidence a check did not read, SHALL
NOT claim a comparison it did not make, and SHALL report a fact it cannot
know as unknown rather than asserting it.

#### Scenario: The structure control names its checks

- **WHEN** doctor assesses a project's structure
- **THEN** the `L1-structure` control names the manifest and the profile's
  build definition, and does not claim a comparison with the profile's
  layout

#### Scenario: The drift evidence names what it compared

- **WHEN** doctor runs on a project whose manifest declares a runtime
  language that agrees with its profile's language
- **THEN** the `dependency-drift` finding names the manifest's runtime
  language and the profile's language, and does not credit dependency
  evidence it did not read

#### Scenario: A manifest without a runtime language

- **WHEN** doctor runs on a project whose manifest declares no runtime
  language
- **THEN** the `dependency-drift` finding states that no manifest language
  was compared

### Requirement: The registry observation reports what the doctor knows

Forge SHALL distinguish a registry observation that says the project is
registered, an observation that says the project is not registered, and
the absence of an observation. A fresh observation SHALL pass, a stale
observation SHALL warn that it is stale, an observation that says the
project is not registered SHALL warn, and the absence of an observation
SHALL be reported as not applicable with evidence naming the unknown. The
doctor SHALL NOT state that a project is not registered when it holds no
observation. The maturity control that mirrors the registration state
SHALL be applicable only when the doctor holds an observation, and SHALL
name the unknown when it does not.

#### Scenario: No observation leaves the registration state unknown

- **WHEN** doctor runs on a project and the caller supplies no registry
  observation
- **THEN** the `registry-observation` finding is reported as not applicable
  with evidence naming the unknown, and it does not claim that the project
  is not registered

#### Scenario: An observation that says the project is not registered still warns

- **WHEN** doctor runs on a project and the caller supplies an observation
  that says the project is not registered
- **THEN** the `registry-observation` finding warns that the project is
  not registered

#### Scenario: A stale observation still warns as stale

- **WHEN** doctor runs on a project and the caller supplies an observation
  that predates the manifest
- **THEN** the `registry-observation` finding warns that the observation
  is stale

#### Scenario: The registration control agrees with the finding

- **WHEN** doctor assesses a project at L3 or above and the caller supplies
  no registry observation
- **THEN** the `L3-registry` control is reported as not applicable with
  evidence naming the unknown, and the project is not failed for it

### Requirement: The registry callers say which situation they are in

A caller that consults the registry read-only SHALL supply an observation
that says the project is not registered when the registry holds no record
for the project, so that the doctor reports the state the caller read. A
caller that holds no observation SHALL leave the registration state unknown
to the doctor.

#### Scenario: The CLI reports a project the registry does not hold

- **WHEN** the CLI runs doctor for a project that the registry does not
  hold a record for
- **THEN** the caller supplies an observation that says the project is not
  registered, and the `registry-observation` finding warns

#### Scenario: The MCP method reports a project the registry does not hold

- **WHEN** the MCP doctor method runs for a project that the registry does
  not hold a record for
- **THEN** the caller supplies an observation that says the project is not
  registered, and the `registry-observation` finding warns

### Requirement: The doctor's claims are pinned

Forge SHALL pin the doctor's claims in the repository's contract check, so
that a claim cannot name a check the doctor does not perform and a caller
cannot stop saying which registration state it read.

#### Scenario: The contract check reads the claims

- **WHEN** the repository's contract check reads the doctor's findings,
  controls and registry callers
- **THEN** it fails naming the claim when a claim names a check the
  doctor does not perform, and when a caller cannot say whether the
  registry was consulted

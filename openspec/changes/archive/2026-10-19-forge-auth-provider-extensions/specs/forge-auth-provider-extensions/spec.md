# forge-auth-provider-extensions Specification

## Purpose

Forge had no way to give a generated application its own end-user sign-in
population. The only identity surfaces were Forge's own administrator login and
the per-project administrator OIDC session, and both are operator surfaces. An
application owner had to stand up an identity provider by hand and wire it into
the generated application. Decision `D9` in the workspace governance register
now requires a provider-neutral, replaceable, project-scoped answer. This
capability gives an owner one command that prepares the end-user identity of
one project, keeps the provider replaceable behind a versioned contract, and
never binds a provider vendor into Forge Core or into an application.

## ADDED Requirements

### Requirement: Application auth setup is explicit and separate from Forge admin identity

Forge SHALL expose a project end-user auth setup operation distinct from Forge-wide administrator login and from the project-scoped Forge admin OIDC session. Listing, inspecting or previewing a provider pack SHALL NOT mutate a project, create a provider user, or change application auth behavior. The operator's own identity SHALL NOT be selectable as a product end-user population.

#### Scenario: Owner opts in

- **WHEN** an owner runs `forge auth setup <project>` for a project that has a readable manifest
- **THEN** Forge prepares the end-user identity of that project through a provider pack, and no part of Forge's administrator identity is used for the product end users

#### Scenario: A pack is merely listed

- **WHEN** an owner runs `forge auth provider list` or `forge auth provider inspect <id>`
- **THEN** Forge names the installed packs, their modes and whether their executables are present, and no project file, provider user or application auth behavior changes

#### Scenario: The operator signs in separately

- **WHEN** a Forge operator uses `forge identity`, or a project admin uses its configured admin OIDC
- **THEN** that authentication stays independent from any product end-user population this capability creates

#### Scenario: The workspace administrator provider is not a product population

- **WHEN** the owner asks for a product end-user population
- **THEN** Forge selects only from the installed provider packs, and never selects the operator's administrator provider for the product population

### Requirement: Auth setup uses preview and explicit confirmation

Forge SHALL validate the target project and the selected provider, produce a read-only plan that names every change, and require explicit confirmation of that exact plan before it mutates anything. The apply SHALL be bound to the plan that was previewed, and a plan whose digest no longer matches SHALL be refused. Without confirmation, Forge SHALL write nothing.

#### Scenario: The owner previews

- **WHEN** an owner runs `forge auth setup <project>` without `--confirm`
- **THEN** Forge prints the provider, the planned changes and the plan digest, tells the owner to confirm, and no project file is written

#### Scenario: The owner confirms

- **WHEN** an owner runs `forge auth setup <project> --confirm` and the provider applies the confirmed plan
- **THEN** the population is recorded in the project manifest and the outcome names the provider, the population and the issuer

#### Scenario: The owner declines

- **WHEN** a preview is not confirmed
- **THEN** the outcome is reported as declined, the provider is not applied, and the project manifest is unchanged

#### Scenario: The confirmed plan is not the planned one

- **WHEN** the provider answers an apply with a plan digest that is not the confirmed digest
- **THEN** Forge refuses the setup and writes nothing to the project manifest

### Requirement: Provider packs are replaceable and independently bounded

Forge SHALL talk to an identity provider through a versioned request/response contract, and a pack SHALL be an independent process. Forge Core SHALL NOT embed provider code, SHALL NOT call a vendor API, and SHALL NOT fall back to a different provider when the selected one is missing or unusable. A contract with an unknown major version SHALL be refused, never adapted. The packs SHALL live outside Forge Core's own crate, in a module with its own workspace and its own Gate.

#### Scenario: The selected pack is not installed

- **WHEN** the owner selects a provider that is not installed
- **THEN** the setup is blocked with the reason `provider_unknown` and a recovery instruction, and no other provider is used instead

#### Scenario: The pack is not built yet

- **WHEN** the selected pack is installed but its executable is missing
- **THEN** the setup is blocked with the reason `provider_not_built` and a recovery instruction

#### Scenario: The pack speaks another contract version

- **WHEN** a pack answers with a contract whose major version Forge does not know
- **THEN** Forge refuses the answer and does not interpret it

#### Scenario: A second provider is added later

- **WHEN** another pack is installed and the owner selects it for a project
- **THEN** the setup runs through that pack, and the application code and the Forge Core code are unchanged

### Requirement: Provider plan and application change are safe

Forge SHALL treat the provider's plan as side-effect free, SHALL refuse an answer that names a different provider, operation, project or project revision, and SHALL refuse an answer that carries secret material. A failed apply SHALL leave the project manifest unchanged, SHALL NOT delete provider data, and SHALL report the recovery instruction. A setup that ends in any refusal or block SHALL write nothing to the project manifest.

#### Scenario: The answer is for another project

- **WHEN** a pack answers with a different project id, operation id or project revision than the request carried
- **THEN** Forge refuses the answer and writes nothing

#### Scenario: The answer carries a secret

- **WHEN** a pack answers with a field that carries a secret value, including in a field the contract does not define
- **THEN** Forge refuses the answer, and the secret is not written to the project manifest, the log or the command output

#### Scenario: The provider fails after it changed something

- **WHEN** the provider reports a failure during the apply
- **THEN** the setup is blocked with the provider's reason and recovery instruction, the project manifest is unchanged, and the provider's own data is left in place

#### Scenario: The provider cannot be used with its stored credential

- **WHEN** a provider runtime does not accept the credential the provider holds
- **THEN** the setup is blocked with the reason and the exact recovery command, nothing is written to the project manifest, and the runtime and its users are not removed

#### Scenario: The plan is repeated

- **WHEN** the owner runs the same confirmed setup twice
- **THEN** the second run reports the same population and the project manifest holds one auth entry, not two

### Requirement: Product end-user populations remain project-isolated

Forge SHALL give each project its own end-user population, and SHALL NOT share an account, session, role or permission between projects. A local provider pack SHALL derive the population from the project id, so two projects never share a realm, a client or an issuer. A pack that manages a hosted project SHALL refuse a project that is shared with another product. A provider's role claims SHALL NOT grant an application permission by itself.

#### Scenario: Two projects are prepared

- **WHEN** the owner prepares the end-user identity of two different projects
- **THEN** each project records its own population id, issuer and client id, and the two populations do not share a realm, a client or an issuer

#### Scenario: A hosted project is shared

- **WHEN** the owner points the hosted-project pack at a project that another product already uses
- **THEN** the pack refuses it and the setup does not proceed

#### Scenario: The provider grants a role

- **WHEN** the provider returns a role claim for a product end user
- **THEN** the recorded configuration grants no application permission on its own, and the application decides what the role means

### Requirement: The recorded provider configuration is standards-based and fails closed

Forge SHALL record a standard OpenID Connect configuration for the application: an issuer, a client id, a callback path, the requested scopes and the provider's registration link. Every URL in an answer SHALL be HTTPS, or plain HTTP on a loopback host, and a registration link SHALL live on the issuer's own origin. Every credential in a recorded configuration SHALL be a reference, never a value.

#### Scenario: The configuration is recorded

- **WHEN** a setup is confirmed
- **THEN** the project manifest records the provider, the population id, the issuer, the client id, the callback path, the scopes, the registration link and the secret references, and no secret value

#### Scenario: The answer carries an unsafe URL

- **WHEN** a pack answers with a plain HTTP issuer that is not a loopback host
- **THEN** Forge refuses the answer and writes nothing

#### Scenario: The registration link points elsewhere

- **WHEN** a pack answers with a registration link on a host other than the issuer's origin
- **THEN** Forge refuses the answer and writes nothing

#### Scenario: The provider cannot register users

- **WHEN** the pack cannot prove that the provider can register an end user
- **THEN** the setup stays blocked, and no registration link is recorded

### Requirement: The setup reports a usable outcome to the project owner

Forge SHALL report the outcome of a setup as one of `preview`, `applied`, `blocked` or `declined`, with the reason and, when the setup did not complete, a recovery instruction. The human output and the machine output SHALL name the same outcome, and the machine output SHALL carry the contract identifier `forge-auth-setup/1.0`.

#### Scenario: A complete setup

- **WHEN** a setup is applied
- **THEN** the output names the provider, the population, the issuer and the manifest path, and the machine output reports the status `applied`

#### Scenario: A blocked setup

- **WHEN** a setup cannot be completed
- **THEN** the output names the reason and the recovery instruction, and the machine output reports the status `blocked`

#### Scenario: A preview

- **WHEN** a setup is previewed
- **THEN** the output names the planned changes and the plan digest, and the machine output reports the status `preview` with the plan digest

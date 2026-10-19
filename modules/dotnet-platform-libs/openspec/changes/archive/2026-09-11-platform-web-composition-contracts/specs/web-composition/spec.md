## ADDED Requirements

### Requirement: Explicit module contract
The web composition package MUST expose a module contract with a stable name, deterministic order, service-registration hook, middleware hook, and endpoint-mapping hook.

#### Scenario: Minimal module registration
- **WHEN** an application registers one module
- **THEN** its service and endpoint hooks can be invoked through the package extensions

### Requirement: Explicit registration only
The package MUST require consumers to register module types explicitly and MUST NOT scan all loaded assemblies or register Mediator, validators, persistence, or product services implicitly.

#### Scenario: Unregistered assembly
- **WHEN** an assembly contains a module type that was not passed to registration
- **THEN** the module is not instantiated or mapped

### Requirement: Deterministic and isolated composition
The package MUST order modules by declared order and stable name, reject duplicate module names or types, and keep registry state scoped to the application service provider rather than process-global state.

#### Scenario: Duplicate and parallel hosts
- **WHEN** duplicate modules are registered or two hosts are built in the same process
- **THEN** duplicate registration fails deterministically and each host retains only its own module set

### Requirement: Opt-in pipeline integration
Middleware and endpoint integration MUST be opt-in and MUST invoke each registered module at most once per pipeline or endpoint-mapping call.

#### Scenario: Consumer controls adoption
- **WHEN** a consumer registers modules but does not call the mapping extension
- **THEN** no module middleware or endpoints are added

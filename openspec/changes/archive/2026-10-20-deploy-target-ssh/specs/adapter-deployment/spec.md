## MODIFIED Requirements

### Requirement: Explicit adapter deployment

Forge SHALL implement deploy with project and target selection through versioned Docker Compose, local, Jenkins, runtime and SSH adapters using validated artifact and configuration inputs. A deploy target kind SHALL declare the target fields a target of that kind must state, and the configuration validator SHALL refuse a target that does not state them, naming the kind and the fields. A deploy target kind MAY name its own adapter binary, and the adapter for a target SHALL be resolved from the kind's own binary, then the shared binary, then the kind's default.

#### Scenario: Explicit adapter deployment success

- **WHEN** an authorized deployment selects a compatible target
- **THEN** the adapter applies the planned artifact and records the target receipt

#### Scenario: Explicit adapter deployment failure

- **WHEN** a target, credential reference or artifact is missing or incompatible
- **THEN** preflight fails before remote mutation

#### Scenario: Explicit adapter deployment boundary

- **WHEN** multiple targets exist and no default or explicit target is set
- **THEN** Forge requires selection rather than choosing an arbitrary server

#### Scenario: A target does not state what its kind needs

- **WHEN** a selected target omits a field that its deploy target kind
  declares as required
- **THEN** preflight fails before remote mutation, and the refusal names the
  kind and the missing fields

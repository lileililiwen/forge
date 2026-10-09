# github-cli-project-workflows (delta)

## ADDED Requirements

### Requirement: Create registers a missing project when asked

`forge project github create` SHALL accept `--register-if-missing`, SHALL validate the local `forge.yaml` manifest and register the canonical project path before creating the remote when the project is not yet registered, and SHALL report the registration outcome without changing the default path.

#### Scenario: Create with register-if-missing on an unregistered checkout

- **WHEN** `forge project github create <dir> --repo <owner/name> --confirm [--push-source --confirm] --register-if-missing` runs for an existing directory with a valid `forge.yaml` that is not in the registry
- **THEN** Forge validates the manifest, registers the project, creates the remote via `gh repo create`, optionally pushes the source, and reports `registered=true`

#### Scenario: Invalid manifest is refused before any remote write

- **WHEN** `--register-if-missing` is set but the directory has no valid `forge.yaml`
- **THEN** Forge refuses with a typed invalid error naming the manifest reason and performs no `gh` mutation

#### Scenario: Default create path is unchanged

- **WHEN** `create` runs without `--register-if-missing`
- **THEN** registry behavior is exactly as before and the envelope reports `register_if_missing=false registered=false`

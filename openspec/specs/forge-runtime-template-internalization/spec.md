# forge-runtime-template-internalization Specification

## Purpose
Provide Forge-owned, version-1 compatible runtime templates through the Forge CLI and portable release bundle without requiring a Workspace Governance checkout.
## Requirements
### Requirement: Forge exposes the versioned runtime template renderer

Forge SHALL expose `forge runtime-template list`, `inspect`, `render`, and `check`, forwarding renderer arguments, standard streams, and exit status while using the bundled version-1 registry and assets.

#### Scenario: List built-in templates

- **WHEN** an operator runs `forge runtime-template list`
- **THEN** Forge lists the same version-1 template IDs, versions, required variables, and generated files without a Workspace Governance checkout

#### Scenario: Render preserves version-1 output

- **WHEN** an operator renders a supported template with valid inputs into a compatible declared project
- **THEN** Forge produces the same files and `.project.json` runtime metadata as the version-1 renderer

#### Scenario: Renderer failure remains visible

- **WHEN** the renderer rejects invalid input, unsafe paths, secrets, conflicts, malformed declaration, or invalid Compose
- **THEN** Forge returns a nonzero result and preserves the renderer's refusal details and does not claim successful verification

### Requirement: Bundle resources resolve without sibling discovery

Forge SHALL resolve renderer resources from an explicit `FORGE_RUNTIME_TEMPLATES_HOME`, the development checkout, or the installed `share/forge/runtime-templates` location in that order, SHALL refuse an invalid explicit override, and SHALL NOT probe a sibling checkout, parent directory, or network. The version-1 bundle manifest SHALL pin every renderer and template asset by relative path and SHA-256, and the renderer SHALL refuse a missing, extra, or digest-mismatched asset.

#### Scenario: Installed resource resolution

- **WHEN** the Forge binary is installed with its runtime template bundle under the selected prefix
- **THEN** the command resolves the bundled renderer and assets and succeeds without the source checkout

#### Scenario: Invalid explicit resource override

- **WHEN** `FORGE_RUNTIME_TEMPLATES_HOME` is set to a directory without the renderer and version-1 registry
- **THEN** Forge reports the exact invalid bundle and does not fall back to another location

#### Scenario: No bundle and no sibling checkout

- **WHEN** no valid bundle exists and the sibling repository is absent
- **THEN** Forge reports the missing resource as unavailable and never searches parent paths or the network

### Requirement: Forge releases carry the runtime template bundle

Forge package and install operations SHALL include the renderer and all registry-declared template files under the documented share directory and SHALL validate completeness before reporting package success.

#### Scenario: Complete package and install

- **WHEN** Forge is packaged and installed into an explicit prefix
- **THEN** the binary and every version-1 renderer resource are installed under the documented layout with checksum verification before installation writes

#### Scenario: Incomplete template registry

- **WHEN** a template registry references a missing or undeclared asset
- **THEN** package validation fails with the template ID and missing path and emits no successful release archive

#### Scenario: Modified bundled renderer asset

- **WHEN** a bundled file differs from its Forge-owned manifest digest or an unlisted file is added
- **THEN** the renderer refuses to serve the bundle and package validation fails with the affected relative path

## ADDED Requirements

### Requirement: Installable template pack
The repository MUST produce a valid `dotnet new` template package that can be installed from a local package and used to generate an application in a clean directory.

#### Scenario: Generate minimal application
- **WHEN** a consumer installs the pack and runs the documented short-name command
- **THEN** a named application with source, project, configuration, and test files is generated

### Requirement: Small default output
The default template MUST generate only a minimal web application, platform package references, application-owned source, and an optional test project; it MUST NOT include React, Aspire, Docker, Terraform, product modules, or platform source copies.

#### Scenario: Inspect generated tree
- **WHEN** a generated application tree is listed
- **THEN** it contains no full-stack starter-kit infrastructure or product module directories

### Requirement: Explicit variants and valid substitutions
Template symbols MUST support the documented optional capabilities and project-name substitution, reject invalid combinations, and produce compilable project files with exact package versions.

#### Scenario: Optional capability variant
- **WHEN** a consumer enables a documented capability symbol
- **THEN** only the matching references and registration code are generated and the project builds

### Requirement: Detached ownership
The generated application MUST NOT reference the template project or require the template package at runtime.

#### Scenario: Uninstall after generation
- **WHEN** the template package is removed after generation
- **THEN** the generated application still restores, builds, and runs from its own source

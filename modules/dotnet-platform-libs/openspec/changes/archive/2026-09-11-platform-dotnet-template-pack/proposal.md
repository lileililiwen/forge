## Why

The current platform scaffold is useful as reference material but cannot be installed with `dotnet new`. The starter kit's installable template is powerful but brings the complexity this platform is intended to avoid. A small detachable template would make incremental adoption practical for ordinary subprojects.

## What Changes

- Add an installable template-pack project for a minimal platform-backed application.
- Provide a small API, optional test project, central package pinning, and explicit capability switches.
- Generate owned source with no dependency on the template package after creation.

## Capabilities

### New Capabilities

- `dotnet-template-pack`: installable minimal application template for platform consumers.

### Modified Capabilities

- None.

## Impact

Adds a packable template project, template metadata, generated sample validation, and documentation. It does not alter runtime packages.

## Non-Goals

- No React clients, Aspire AppHost, Docker/Terraform, product modules, or full-stack starter architecture.
- No CLI update mechanism in this change.

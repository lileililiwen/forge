# Proposal: Platform application starter experience

## Why

Independent demo applications repeatedly recreate project structure, service registration,
configuration, test hosting, frontend setup, and runtime middleware. The platform needs a
Spring-Boot-like entry point while preserving small independently adoptable packages.

## Scope

Add composite `Platform.Starter` registration, a `dotnet new` template or equivalent scaffold,
sample host, configuration contract, test-host defaults, package composition documentation, and
adoption guides for existing applications.

## Non-goals

- no monolithic replacement for every platform package;
- no automatic activation of billing, identity, tenants, or providers;
- no forced folder layout for existing applications;
- no automatic migration of all repositories.

## API impact

Adds `AddPlatformApplication`, `UsePlatformApplication`, starter options, template files,
sample host APIs, and generated project conventions.

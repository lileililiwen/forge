# Proposal: Versioned project model and CLI foundation

## Why

[The product brief](../../../requirement.md) §1, §2, §3, §4, §5, §6, §7, §34, §37, §38, §39, §44, §45, §47 requires versioned project model and cli foundation. This bounded change supplies that outcome without claiming the rest of the platform is implemented. Target: v0.1.

## What Changes

- **Versioned manifest contract:** Forge will validate schema-versioned forge.yaml with project identity, profile, current and target maturity, runtime, versioned features, quality, AI, deployment, distribution and documentation metadata; unsupported versions and ambiguous manifest sources will fail without mutation.
- **Registry and CLI identity:** Forge will expose list and inspect through shared Core contracts and persist project ID, name, path, Git repository, primary and mirror remotes, stack, profile, maturity, schema/platform versions, features, deployment target, runtime, last commit, quality, agent and documentation observations.
- **Independent command contract:** Forge will provide CLI help, version reporting and structured errors independently of GUI, AI or network services, with nonzero exit codes for failed operations.

## BFS Impact Map

- **Capabilities and flows:** Versioned project model and CLI foundation; acceptance outcomes are specified in [the capability delta](specs/core-manifest-registry/spec.md).
- **Modules, contracts and persistence:** Core, Registry, CLI; manifest schema and local registry; future adapters and transports.
- **Callers:** CLI first; only stable Core behavior is eligible for later MCP, API and portal exposure. Every available caller must retain equivalent validation and outcomes.
- **Dependencies and configuration:** None; this establishes the foundation.
- **Failure and boundary behavior:** Database and manifest writes cannot share one transaction; journal reconciliation and never report a partial write as successful. Unsupported schema versions must not be rewritten.
- **Tests:** scenario fixtures for each named requirement, including successful execution, rejected input/dependency failure and boundary behavior; cross-caller and persistence checks where touched.
- **Compatibility/security/privacy:** preserve stack-neutral contracts, project ownership and ordinary source; validate external input and redact secrets. Changes to remote authority are never implicit.
- **Unaffected scope:** Runtime infrastructure, profiles, generation, HTTP, MCP, or a requirement for generated projects to depend on Forge.

## Capabilities

### New Capabilities

- `core-manifest-registry`: Versioned project model and CLI foundation.

### Modified Capabilities

None. This adds a distinct capability over the declared prerequisites; canonical specs do not exist yet.

## Non-goals

Runtime infrastructure, profiles, generation, HTTP, MCP, or a requirement for generated projects to depend on Forge. This package is planning-only and does not authorize implementation or external operations.

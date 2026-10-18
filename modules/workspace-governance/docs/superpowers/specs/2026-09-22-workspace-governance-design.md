# Workspace Governance Design

## Goal

Create a language-neutral coordination project for repositories in a workspace
selected at invocation time; do not encode a user's filesystem prefix.
It owns project identity, governance profiles, evidence vocabulary, and deployment declarations.
It does not own product source, language runtimes, or deployment execution.

## Ownership model

- `workspace-governance`: policy, registry, schemas, bootstrap templates, and mandatory checks.
- `forge`: future portfolio control plane and registry consumer.
- `driftwatchdog`: shared quality/evidence gate runtime.
- `sisyphusfy`: agent iteration runtime.
- `ariadex`: human-supervised runtime control.
- `dotnet-platform-libs`: reusable .NET contracts and adapters.
- `jenkins-local`: Jenkins and Mac deployment execution.

## Contracts

Each managed repository has a `.project.json` declaration. The registry uses stable project IDs,
stack profiles, lifecycle state, verification commands, gate ownership, and deployment metadata.
The mandatory checker validates both the registry and each selected project. Jenkins consumes the
generated deployment manifest only after the governance check passes.

## Safety boundaries

- Missing mandatory checks fail closed.
- Planning, build, test, integration, provider, runtime, and release evidence remain distinct.
- Secrets are never stored in the registry or deployment manifest.
- The governance project does not copy shared gate runtimes or installers into product repositories.
- Deployment is an explicit Jenkins action; governance does not deploy by itself.

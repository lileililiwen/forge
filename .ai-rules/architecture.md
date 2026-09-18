# Architecture rules

Load for new modules, public contracts, cross-cutting behavior, persistence or integration design. Follow [architecture and decision boundaries](../docs/architecture.md) and the authoritative [brief](../requirement.md).

- Keep a modular monolith with shared Core rules and thin CLI/MCP/API transports.
- Keep product contracts stack-neutral; resolve recommended implementation choices in the foundation ADR.
- Version manifest, profile, feature, component and transport contracts.
- Separate desired configuration from timestamped observations and verified outcomes.
- Prefer deterministic owned assets; keep generated projects independent of Forge.
- Delegate existing quality, agent, analytics and content functions through adapters.
- Use explicit operation identity, preconditions, ownership and recovery for mutations.
- Do not introduce early portal, advanced planner, SSO or remote lifecycle work into v0.1.

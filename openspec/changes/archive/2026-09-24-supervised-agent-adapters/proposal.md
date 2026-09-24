# Proposal: Wire Forge agent sessions to the real supervised runtimes

## Why

`agent-runtime-workflows` deliberately returns `unsupported` for
pause/takeover because "the existing PTY-based manager is not wired into
this build" (`src/agent/mod.rs`). The workspace already contains that
manager and its iteration layer: Ariadex is a human-supervised tmux runtime
for OpenCode/Codex with `start`/`stop`/`resume` and durable handoff state,
and Sisyphusfy is a fresh-session task supervisor with independent
verification and structured `--json` results. Requirement.md §21 mandates
integrating the existing PTY manager rather than rewriting it and §32 maps
it to the Agent Runtime plane. Until Forge delegates to one of them, its
agent surface cannot honestly implement the brief's core operations.

## What Changes

- Add an `ariadex` agent provider adapter: `forge agent start/status/
  resume/restart` map onto the sibling's documented CLI in the target
  project's directory; pause/takeover report the sibling's real capability
  (attach guidance) instead of a dead-end `unsupported`.
- Add a `sisyphusfy` execution adapter for the spec-execution path:
  `forge agent run-spec` may hand the generated spec's task files to
  `sisyphusfy` and ingest its `--json` outcome as the session's verification
  evidence.
- Both binaries are optional external executors resolved through ordered
  probes (`FORGE_ARIADEX_BIN`/`FORGE_SISYPHUSFY_BIN`, then PATH names);
  absence keeps today's explicit `unavailable`/`unsupported` semantics.
- Session records persist the backing runtime id so status/inspect can name
  where the truth lives; all transitions journal as before.

## BFS Impact Map

- **Capabilities:** `agent-runtime-workflows` (provider set, transition
  mapping, session provenance).
- **Users and flows:** operators running bounded spec work through a
  supervised runtime; portal/CLI session inspection.
- **Contracts/data/persistence:** AgentSession gains an optional
  `backing` descriptor; no schema rewrite; sessions under `.forge/agents/`
  stay Forge-owned pointers, never copies of the runtime's state.
- **Integrations/configuration:** two external binaries behind env/PATH;
  no vendoring, no protocol re-invention (consumes documented CLI output).
- **Callers:** CLI `forge agent …`; `run_agent` MCP tool dispatches the
  same Core path; API/portal unaffected.
- **Failure/boundary behavior:** runtime missing → unavailable with
  recovery note; runtime present but session id unknown to it → disconnected,
  session file preserved; JSON outcome unparsable → unverified, never done.
- **Tests:** adapter fixtures (stub binaries emitting real output shapes),
  transition matrices per provider, session preservation on failure.
- **Dependencies:** none of the other proposed changes.
- **Compatibility/security/privacy:** existing OpenCode/Codex bundled
  adapters keep their behavior; argument arrays never shells; no credential
  in session records.

## Capabilities

- `agent-runtime-workflows`: Forge can delegate session lifecycle and spec
  execution to the workspace's existing supervised runtimes and report
  their outcomes honestly.

## Non-goals

- Rewriting or vendoring Ariadex/Sisyphusfy behavior in Forge.
- A unified cross-runtime session bus or web control UI.
- Exposing pause/takeover where the backing runtime genuinely lacks them
  (mapping to its real verbs only).
- Changing `run_tests`/`commit`/`push` contracts.

Source: requirement.md §21, §22, §32, §41.

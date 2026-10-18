# Proposal: Route UI work through the shared UI convention

## Why

The workspace has one real UI-layer consumer, `somedanote`, while 55 UI-bearing
repositories have no UI conformance declaration and 15 have a measured
divergent accessibility primitive. The shared library and read-only checker
already exist, but the governance skill does not route UI work to them and a
check that is not bound into a repository Gate does not block drift.

This change makes the existing rule discoverable to agents and gives an
operator a safe, one-repository-at-a-time command to bind the check into that
repository's own Gate. It does not claim that other repositories have adopted
the shared packages.

## What Changes

- Add one UI routing row to `skills/workspace-governance/SKILL.md` and a
  focused reference covering the checker, adoption declarations, finding
  meanings, real stack-specific paths, and existing policy decisions.
- Add `scripts/bind_ui_gate.py`, a dry-run-by-default, idempotent binder that
  can update only the explicitly named repository's `.ai-gate/gate.yaml`.
- Add `docs/ui-convention.md` for maintainers and operators.

The adoption checker in `scripts/ui_conformance.py`, its finding vocabulary,
baseline, and shared UI packages remain owned by their existing projects and
are not changed here.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `ui-convention-guidance` | Agents and operators can discover the UI rule and safely bind its check to one repository Gate | Workspace Governance, Markdown and Python 3 | Skill route, reference, docs, and `bind_ui_gate.py` CLI contract | Existing `scripts/ui_conformance.py`, `ui-conformance` Gate ID, `.ai-gate/gate.yaml` rule-pack shape | Strict OpenSpec validation, skill-reference checker, binder unit tests, and a scratch-repository dry-run/apply/idempotence sequence |

The package is one outcome owned by Workspace Governance. It does not include a
consumer migration, project-generator default, or UI package implementation;
each has a different repository owner and acceptance oracle. Those follow-up
packages are dependency ordered: finish this governance route and binder, then
update Forge's project generator, then migrate named existing consumers in
their own repositories. No sibling source is edited by this package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| `dotnet-platform-libs` | `ui/README.md`, `scripts/check-ui-conformance.sh`, `ui/scripts/verify-ui-feed.mjs` | Versioned UI artifacts, feed digest verifier, and workspace-wide primitive/conformance check | Does not explain the rule through the governance skill or bind the check into each product's own Gate | `dotnet-platform-libs` owns packages and feed; each consumer owns its manifest and pages | **adopt** |
| Workspace Governance | `scripts/ui_conformance.py`, `scripts/bind_gate_commands.py`, `scripts/bind_verification_command.py` | UI finding vocabulary, existing Gate adapters, and comment-preserving per-repository binders | UI checker is not routed from the agent front door; UI check is not bound by a dedicated one-repository binder | Workspace Governance owns policy/checker/binder; consumer Gate remains local | **extend shared owner** |
| Forge | `src/generate/mod.rs`, `openspec/specs/scaffold-prewires-shared-layer/spec.md` | New-project generation and digest-pinned vendored kit assets | Project scaffolding defaults are a separate Forge release and are not implemented here | Forge owns its generator and generated project contract | **keep local** |

`keep local` for Forge means its generator changes stay in Forge; it does not
justify duplicating the UI packages or the UI checker in Forge.

## User Experience and Interface Impact

The actors are an AI coding agent preparing a UI change and an operator
maintaining a repository Gate. The agent discovers the convention through the
workspace-governance skill routing table, reads the reference, then runs the
read-only checker. The operator can run the binder in dry-run mode, review the
proposed two manifest entries, and explicitly apply them to one repository.
Conflicts and unknown manifest shapes are reported without writing.

The interface is a Markdown routing/reference flow plus the `bind_ui_gate.py`
CLI. It has no browser surface. CLI output names the repository and one of
`BOUND`, `ALREADY_BOUND`, `CONFLICT`, `REFUSED`, or `NO_MANIFEST`; JSON output
supports automation. Keyboard accessibility follows the invoking terminal.
Responsive browser behavior and assistive-technology UI review are not
applicable because this change renders no web or mobile interface.

## BFS Impact Map

| Surface | Impact |
|---|---|
| Agent discovery | Add one routing row and a reference that resolves to existing checker vocabulary and real adoption files |
| Gate binding | Add a dry-run default and explicit `--apply`; one invocation may write only the named repository's gate manifest |
| Contract | `checks.ui-conformance: true` and a repository-relative `commands.ui-conformance` command in the recognized governance `rule_pack` manifest |
| Failure behavior | Missing or unknown manifest shape is refused; conflicting existing entries are preserved; repeated apply is byte-identical |
| Security and portability | No shell execution, environment-variable path, absolute path, or workspace/fleet traversal is introduced |
| Existing consumers | No consumer source is edited; `somedanote` is documented only as the already existing worked example |
| Tests and verification | Unit fixtures cover the binder; skill-reference validation and strict OpenSpec validation cover routing and package structure |
| Compatibility | Existing UI finding codes, baseline entries, package versions, consumer manifests, and Gate semantics are unchanged |

## Capabilities

- `ui-convention-guidance` — make the shared UI convention discoverable and
  bind its existing conformance check safely to one named repository.

## Non-goals

- Do not change `scripts/ui_conformance.py`, its seven finding codes, or
  `ui-conformance-baseline.json`.
- Do not migrate or bind any consumer repository as part of this change.
- Do not edit Forge, `dotnet-platform-libs`, or product source.
- Do not publish the feed, create a Flutter artifact, or make a fleet-wide
  write.
- Do not claim conformance for a repository until its own checker runs.

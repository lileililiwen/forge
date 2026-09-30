# Proposal: Site studio with live preview and scoped refinement

## Why

Forge has deterministic project generation, validated Intent, controlled
agent adapters, and an in-process Rust/maud UI. It does not yet provide the
interactive website-building loop that turns a prompt into a running preview
and lets the user refine the result without manually coordinating files,
processes, and screenshots.

## What Changes

- Add a versioned, project-owned `forge.app.yaml` site specification,
  separate from the existing infrastructure `forge.yaml` manifest.
- Add a Studio module to the existing Forge Core that parses the AppSpec,
  validates it without writing project files, and persists it through the
  existing operation journal.
- Add a bounded preview session lifecycle (start/stop, port reservation,
  bounded profile-runner spawn, kill-on-timeout, bounded log capture) that
  delegates to the profile runner's allowlisted argv (`npm run dev`, with
  `FORGE_STUDIO_RUNNER_BIN` as the controlled-stub override) and inherits
  `PATH` without a shell. The long-lived API owns the live session; the
  short-lived CLI runs a bounded readiness probe and tears its own child
  down, so no process is left detached.
- Add a refinement record route that validates the request and persists a
  `studio.refine` journal entry; the controlled-agent adapter wiring is a
  follow-up cycle (the data-flow contract is delivered here, the runtime
  hook is recorded as deferred in `tasks.md`).
- Add a minimal Studio read-only browser page to the existing API UI and
  five versioned `/v1/projects/{id}/studio/*` routes (CLI parity verified
  by cross-surface tests). The CLI mirrors the revision-bound save at
  `forge studio spec <project> --confirm yes --expected-revision <rev>`
  so an operator can create the session and run the bounded start probe
  without the API host; without `--confirm yes` the command stays
  read-only.
- Add schema fixtures (`schemas/app-spec-v1.json` and
  `tests/fixtures/app-spec/{valid,invalid}/*`) and a unit/contract/cross-
  surface test matrix that locks the typed errors and the bounded process
  contract.

## Package Boundary and Split Assessment

This package ends at a reviewed, runnable preview **and** its journaled
refinement record. Publishing and provider lifecycle are separate outcomes
and are owned by the delivery workflow. MVP supports the existing
`react-web` profile for static, client-rendered sites; backend/auth/
database profile extensions are follow-up work. The split prevents Studio
from duplicating Forge publish lifecycle or OpenPanel deployment state.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `site-studio-preview-refinement` | Create and iteratively refine a running site preview | Forge, Rust + browser JS | `forge.app.yaml`, Studio API/UI, preview session | validated Intent, deterministic generation, controlled agents | Browser workflow plus preview/session contract tests |

Dependency order: existing OpenPanel provider facade → Hermora onboarding
API → Forge project-to-production workflow. Studio is independently
adoptable; it does not depend on the delivery surface.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge API UI | `src/api/ui/{routes,render,data}.rs` | Rust/maud browser UI, existing auth and origin checks | No site editor or preview session | Forge release | **extend shared owner** |
| Forge generation/Intent/agents | profile registry, `src/agent/` | validated Intent, deterministic profile assets, supervised agent adapters | No project-bound preview lifecycle | Forge release | **adopt** |
| workspace runtime templates | `workspace-governance/templates/runtime` | portable runtime template ownership | Site authoring and preview are not deployment-template concerns | Workspace Governance release | **adapt through a generic adapter** |
| OpenPanel | `forge-publish-provider/0.1.0` contract | staged deployment, health, rollback | Provider facade remains an independent OpenPanel change | OpenPanel release | **defer to delivery workflow** |
| Hypora | Fake Door validation | approved demand evidence can inform a later Studio prompt | No import contract is in this package | Hypora release | **defer** |

## BFS Impact Map

- **Actors:** project owner, Forge UI/API, profile runner (a bounded child
  process whose argv is profile-owned), CLI caller, agent adapter (future).
- **State:** `draft` AppSpec → `saved` (persisted with the session at
  `<project>/.forge/studio/session.json`) → preview `starting` → `ready`
  → `failed` / `stopping` → `stopped`; refinement records a new
  `studio.refine` journal row and a monotonic revision counter (`r1`,
  `r2`, ...).
- **Persistence:** the saved AppSpec is embedded in the session record
  at `<project>/.forge/studio/session.json` (single file, schema-major
  versioned);
  preview state is journaled through the existing `operations` table with
  `kind ∈ {studio.spec.save, studio.preview.start, studio.preview.stop,
  studio.refine}` and bounded detail redaction. **No new SQLite tables
  or migrations.**
- **Security:** every `/v1/projects/{id}/studio/*` and
  `/ui/studio/{id}` request is
  bearer-token gated and project-scoped (same posture as the existing
  `/v1/projects/{id}/*` and `/ui/projects/{id}/*` surfaces). Profile
  runners are invoked through `process::spawn_with_timeout` with a
  bounded argv derived from the profile descriptor; no shell, no
  user-supplied command, no path escape. Bounded stderr (2 KiB
  truncation marker) and bounded stdout log capture (1 MiB per session)
  with `policy::redact_credentials` applied before journal or API
  output.
- **Failure:** invalid spec, unsupported profile, busy port, startup
  timeout, agent refusal, process exit, and path escape produce typed
  failures (`studio-invalid-spec`, `studio-unsupported-profile`,
  `studio-port-unavailable`, `studio-start-timeout`,
  `studio-revision-conflict`, `studio-project-scope`) without claiming
  a preview is ready.
- **Compatibility:** existing `forge.yaml`, `/ui` pages, API routes,
  project artifacts, publish records, and standalone CLI behavior
  remain valid. No new CLI top-level command — Studio lands under the
  existing `forge studio <verb> <project>` subcommand, mirroring the
  `forge delivery` and `forge governance` posture. No new dependency.
- **Verification:** schema fixtures (valid/invalid/boundary), unit tests
  for AppSpec parsing, contract tests for CLI behaviour with a fake
  profile runner (allowlisted argv, timeout kill, port collision
  safety, log cap/redaction, idempotent stop), cross-surface tests for
  CLI/API parity on success and typed refusal, and a UI smoke that
  confirms the Studio page renders the spec and preview status.

## Capabilities

- **New:** `site-studio-preview-refinement`.
- **Modified:** `control-plane-portal`, `deterministic-project-generation`,
  and `agent-runtime-workflows` only through additive Studio consumers.

## Non-goals

- No arbitrary framework/profile generation; no backend, authentication,
  PostgreSQL, or external service provisioning in this first slice.
- No deployment, DNS, billing, public sharing, or visitor analytics.
- No replacement of the agent runtime, quality Gates, or OpenPanel.
- No Codex-owned branding claim; ChatGPT Sites is an external UX
  benchmark.

## Explicit deferred work (recorded in tasks.md)

The bounded package delivers the data-flow contract and a runnable local
preview. The follow-up work below is recorded so the boundary is honest
but the cycle still ships:

1. The same-origin preview iframe proxy that surfaces the running site
   through Forge on a reserved URL (this cycle reserves the port and
   records the state, the actual HTTP proxy lands in a follow-up that
   cannot conflict with the existing listener because the proxy lives
   on its own port, not under the API).
2. The Studio page's interactive browser controls (prompt form,
   refinement textarea, file picker). The current page renders the
   spec and preview state read-only.
3. The `studio.refine` journal row → controlled-agent adapter runtime
   hook (`agent::apply_transition` with a `refine` transition). The
   data-flow contract is delivered; the runtime hook is a follow-up
   that lands alongside an agent contract bump.
4. Browser smoke through a Node toolchain; the sandbox this package was
   implemented in does not have Node installed, so the smoke is
   recorded as `not run` and pinned as the verification gap to close
   on a Node-equipped runner.
5. The native `react-web` profile scaffold verified against `npm
   install && npm run dev`. The current contract test uses a fake
   profile runner on a controlled `PATH`; the native verification is
   the same gap as the graduation and delivery packages and lands
   when the live toolchain is present.
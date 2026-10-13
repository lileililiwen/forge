# Design: Forge never reports agent activity it did not produce

## 1. The defect, precisely

`apply_transition` splits on `session.provider.is_supervised()`
(`src/agent/transitions.rs:126`). The supervised arm delegates to
`ariadex_transition` and takes the state from a real `ariadex status --json`
report. The other arm reaches two functions:

`start_or_resume` (`transitions.rs:181-209`), serving `Start`, `NewSession`
and `Resume`:

```rust
let binary = session.provider.binary();
let available = command_on_path(binary);
if !available { return Err(ForgeError::AgentUnavailable { … }); }
let evidence = vec![ format!("provider: {…}"), format!("binary: {binary}"), … ];
let note = format!("{} recorded for session `{}`; provider `{}` is present and the recorded session is the source of truth", …);
Ok((SessionState::Active, evidence, note, next_step))
```

`restart` (`transitions.rs:211-237`) checks that the prior state is
`Active | Paused | Idle` and then returns `SessionState::Active`.

Neither spawns anything. `command_on_path` is `fs::is_file` over the `PATH`
entries (`src/agent/sessions.rs:440-450`) — a filename existence probe. The
complete set of things Forge observes on this path is: a file exists, and a
state string was in the record. From that it asserts *a session is running*.

Both providers are bundled PATH-probed adapters by Forge's own documentation
(`src/agent/model.rs:36-40`): "`opencode`/`codex` are the bundled PATH-probed
adapters". `is_supervised()` (`model.rs:66-70`) is false for both. And the same
file already states the rule being broken at `transitions.rs:509-510`:
"A state the runtime did not report is never synthesized."

## 2. Decision: option (a) — report `unsupported`, keep the operations exposed

### 2.1 The evidence

**`openspec/specs/agent-runtime-workflows/spec.md:8`** — the only canonical
spec line in `openspec/specs/**` that names OpenCode or Codex at all
(verified: `rg -l -i 'opencode|codex' openspec/specs/ --glob '*.md'` returns
this one file, and inside it this one line):

> Forge SHALL expose start, pause, takeover, resume, restart and new-session
> operations for **supported** OpenCode/Codex adapters, and route spec run to
> the selected project and active spec.

Three words carry the requirement: **expose**, **operations for**, and
**supported**. The requirement is about the *surface*, not about a running
process. "Supported" is the qualifier that lets a provider be exposed while
being honestly unable to run.

**`spec.md:44-50`** — the requirement that *does* demand a real backing
runtime is scoped to the supervised provider by name, twice:

> ### Requirement: Supervised runtime delegation
>
> Forge SHALL delegate agent session lifecycle to the workspace's existing
> supervised runtimes (an Ariadex-class session manager, a Sisyphusfy-class
> iteration supervisor) through ordered binary resolution and bounded,
> argument-array invocation, and SHALL attribute every reported state to the
> backing runtime rather than synthesizing it.

and its only success scenario (:52-57) is conditioned on the provider:

> #### Scenario: Session starts on the backing runtime
>
> - **WHEN** an operator starts an agent session selecting **the supervised
>   provider** on a host with the runtime installed
> - **THEN** the session record names the runtime, its handle and the
>   transition journal shows the delegated verb

No scenario anywhere says a bundled adapter must start a process.

**`spec.md:65-70`** — "Truthful transition mapping" is already unconditional
and already forbids exactly this:

> Forge SHALL expose only transitions the backing runtime genuinely supports;
> unsupported requests SHALL report the operator's real control path (attach
> guidance or the runtime's own pause primitive) and SHALL **never claim a
> state the runtime did not report**.

**`spec.md:15-18`** — the failure scenario already names the honest outcome
for this class of request:

> #### Scenario: Managed agent sessions failure
>
> - **WHEN** the manager disappears or **the requested transition is
>   unsupported**
> - **THEN** Forge reports **failed, disconnected or unsupported with recovery
>   guidance**

**`src/agent/model.rs:84-90`** — the state vocabulary already documents
`Unsupported` for this case:

> `Unsupported` means the requested transition is not available on the
> current provider

**`src/agent/transitions.rs:100-112`** — the function doc already claims the
supervised half holds the line and the bundled half "returns an explicit
unsupported state rather than simulating success"; the code only kept that
promise for `pause`/`takeover`.

**Non-canonical, for completeness.** `requirement.md:900-905` ("Supported
agents **may include**: OpenCode, Codex, future CLI agents") frames the
providers as a menu, not a mandate. `README.md:408` — "Forge coordinates
DriftWatch, the existing PTY agent manager, content and analytics tools; it
does not replace them" — names the PTY manager as the integration point that
is not wired in. `ROADMAP.md:52` maps `supervised-agent-adapters` to §21/§22/§32:
the real provider delegation work is the supervised one.

### 2.2 Why not (b)

Option (b) — "make them really spawn" — is rejected on three independent
grounds, any one of which is sufficient:

1. **No canonical spec asks for it.** Every scenario that requires a real
   process is scoped to "the supervised provider". Implementing PTY spawns for
   the bundled adapters would be new capability that no requirement authorizes,
   and `AGENTS.md` requires an OpenSpec change whose *spec* authorizes it —
   the delta would have to invent a requirement, not promote one.
2. **It contradicts a standing repository rule.** `AGENTS.md:50` — "Do not
   replace DriftWatch, the existing PTY manager or external
   content/analytics." `requirement.md:1358-1364` maps "OpenCode PTY manager →
   Agent Runtime" as an **existing** system that "should retain [its]
   identity". Spawning `opencode` from Forge would be the replacement that
   rule forbids.
3. **The surface does not exist.** There is no PTY manager wired into this
   build; `transitions.rs:144-146` and `:150-152` say so in as many words:
   "the existing PTY-based manager is the integration point but is not wired
   into this build". Option (b) is therefore not a small change to a described
   adapter — it is building the missing integration, which is a separate
   change with its own spec.

**Decision: (a).**

## 3. The invariant

> Forge SHALL report `SessionState::Active` for a session only when a real
> process was started or a live status was confirmed for it. It SHALL NOT
> synthesize, infer or carry forward an `active` claim from a recorded state
> or from the presence of a binary on `PATH`. Not behind a flag, not behind a
> feature toggle, not behind an environment variable.

Applied to this file, the invariant has exactly two witnesses:

| Witness | Where | What it requires |
|---|---|---|
| `invoke_runtime` | `transitions.rs:369-464` | a `Command::spawn` that returned `Ok(child)` |
| `map_ariadex_status` | `transitions.rs:511-545` | a live `ariadex status --json` document that said so |

The bundled path has neither. So the bundled path has no `Active`.

## 4. Before/after state matrix, per provider × transition

`Active` never appears in the "after" column. "typed refusal" is the
`Err(ForgeError::AgentUnavailable)` path (unchanged code, corrected text).

### 4.1 `AgentProvider::Opencode` (`!is_supervised()`)

| Transition | Before | After | Change |
|---|---|---|---|
| `start` | `active` if `opencode` on PATH, else typed refusal | `unsupported` if on PATH, typed refusal if absent | **state fixed**; evidence now carries the probe result and the "no adapter" detail; next_step names the real control path |
| `resume` | `active` if on PATH, else typed refusal | identical to `start` | **state fixed** |
| `new_session` | `active` if on PATH, else typed refusal | identical to `start` | **state fixed** |
| `restart` | `active` (prior-state guard only) | `unsupported` | **state fixed** |
| `pause` | `unsupported` | `unsupported` | none |
| `takeover` | `unsupported` | `unsupported` | none |
| `run-spec` (no supervisor) | `active` via `Start` | `unsupported` | **state fixed** |
| `run-spec --provider sisyphusfy` | real supervisor run | real supervisor run | none |
| `status` / `list` | render the record | render the record | none |
| `live_runtime_status` | `None` | `None` | none (already honest) |

### 4.2 `AgentProvider::Codex` (`!is_supervised()`)

Identical row for row to §4.1 with `codex` substituted. Both bundled providers
share one code path (`apply_transition`'s non-supervised arm); the only
difference in the whole matrix is the binary name in the evidence line.

### 4.3 `AgentProvider::Ariadex` (`is_supervised()`) — **byte-identical**

| Transition | Before | After | Change |
|---|---|---|---|
| `start` | real `ariadex start`, then `ariadex status --json` decides | identical | **none** |
| `resume` | real `ariadex resume`, then status decides | identical | **none** |
| `new_session` | `ariadex stop` then `ariadex start`, then status decides | identical | **none** |
| `restart` | `ariadex stop` then `ariadex start`, then status decides | identical | **none** |
| `pause` | real `ariadex pause`, then status decides | identical | **none** |
| `takeover` | guidance only; `ariadex attach` named; no hijack | identical | **none** |
| `run-spec` (no supervisor) | refused: ariadex owns its scheduler | identical | **none** |
| missing runtime binary | typed refusal listing resolution attempts | identical | **none** |
| version probe off-surface | `unsupported`, "no runtime state was claimed" | identical | **none** |
| unreadable status document | `disconnected` + bounded redacted raw | identical | **none** |
| stored handle absent from live report | `disconnected` + "loss recorded" | identical | **none** |
| `status` live block | `ariadex status --json` render | identical | **none** |
| `backing` pointer | runtime + handle + adapter version | identical | **none** |

The Ariadex matrix is reproduced in full so the non-regression claim is
checkable line by line, not asserted. Its tests
(`tests/supervised_agent_contract/ariadex.rs`, 12 tests, and
`tests/supervised_agent_cross_surface.rs`) are **not** edited by this change.

## 5. Implementation shape

### 5.1 `start_or_resume`

The PATH probe is kept — it is a real observation and its result is worth
recording — but it stops being the authorization for a state. The function
returns the existing `unsupported()` tuple, with the probe result and the
capability gap folded into the `detail` evidence string:

```
provider: codex
transition: start
binary: codex (present on PATH)
detail: forge spawns no process for the bundled `codex` adapter in this build; \
        the binary is an availability signal, not a session. use `--provider ariadex` \
        for a session whose state comes from a real runtime
```

`unsupported()` supplies the state, the "stays in state `…`" note and the
next step, so the shape matches the `pause`/`takeover` answers byte for byte
in structure. `next_step` is overridden by `unsupported()`'s generic wording
("adopt a provider that exposes this transition or record a manual status"),
which is the honest advice for both bundled providers.

### 5.2 `restart`

The pre-check is **kept and widened**. It currently accepts
`Active | Paused | Idle`; after this change a bundled session's steady state
is `Unsupported`, so leaving the guard as-is would make `start` → `restart` a
dead end that errors with "restart requires `idle`/`active`/`paused`".
`Unsupported` and `Disconnected` are both explicitly non-terminal
(`model.rs:84-90`: `Unsupported` = "not available on the current provider",
`Disconnected` = "the manager is gone"), which is the same rule the supervised
arm already states ("restart requires a non-terminal session",
`transitions.rs:715`). The guard therefore accepts the five non-terminal
states and refuses `Stopped`, and the refusal text is corrected to name them.

**Rejected:** deleting the guard entirely. A `Stopped` session is terminal by
contract (`model.rs:86-90`); restarting one must stay a refusal.

**Rejected:** leaving the guard at three states. That is the dead end above,
and it would make the honest answer to `restart` be an error rather than the
`unsupported` state the spec names.

### 5.3 The missing-binary refusal

Kept as a typed `agent-unavailable` (exit 1) — `spec.md:59-63` ("the command
reports unavailable listing the resolution attempts") still describes this
class of failure, and no state claim is made here, so nothing about it is a
fabrication. Only the reason text changes: the old "install the codex
provider or rerun on a host that has it" is a false lead, because installing
`codex` would leave the operator with the same `unsupported` answer. The new
text states both facts — the binary is absent **and** this build has no
adapter for it, so installing it is not the fix.

**Rejected:** collapsing the absent case into `unsupported` too, for a single
uniform answer. That would turn a hard, actionable, currently-honest failure
into a soft one and would change the exit code the failure scenario at
`spec.md:59-63` describes.

### 5.4 `src/api/admin/agent_identity.rs:210`

One note string. Before: "recorded session state is the live surface for this
provider; no runtime probe exists." — which tells a browser operator that the
record *is* live, i.e. that something is running. After: the record is named as
the only surface and explicitly not an observation. The pinned assertion in
`tests/web_agent_identity_readonly_contract.rs:280` requires the substring
`recorded`, which the new text keeps, so that test is unchanged.

### 5.5 Untouched, deliberately

`ariadex_transition`, `invoke_runtime`, `resolve_binary_path`,
`map_ariadex_status`, `live_runtime_status`, `disconnected_outcome`,
`run_spec_sisyphusfy`, `command_on_path`, `probe_provider`, `model.rs`,
`src/cli/agent.rs`, `src/cli/commands.rs`, `src/gate/**`, `src/contract/**`,
`src/api/command_catalog/**`.

## 6. Tests

| Test | Kind | Asserts |
|---|---|---|
| `agent::tests::start_on_a_bundled_provider_never_claims_active` | new | start/new_session/resume on `opencode` and `codex` return `Ok` only as `unsupported`; the recorded evidence names the probe result and the no-adapter reason; the note says the session stays `unsupported`; `next_step` is present |
| `agent::tests::restart_on_a_bundled_provider_never_claims_active` | new | `restart` from each non-terminal state returns `unsupported`; `Stopped` still refuses with `agent-unavailable`; the spec binding survives |
| `agent::tests::no_transition_reaches_active_without_a_process_or_a_live_report` | new | matrix over `is_supervised()` × every transition: no non-supervised provider × any transition ever yields `Active`; the only reachable `Active` in the module is the supervised one |
| `agent::tests::start_records_state_for_present_or_missing_provider` | **rewritten** | the old body asserted `assert_eq!(outcome.state, SessionState::Active)` for the present-binary arm — it encoded the bug. New body: `unsupported` when the binary is present, `agent-unavailable` when absent |
| `agent::tests::restart_preserves_prior_state_and_records_active` | **rewritten** | the old body asserted `assert_eq!(outcome.state, SessionState::Active)` — it encoded the bug. New name `restart_preserves_the_prior_record_without_claiming_active`; asserts `unsupported`, one recorded transition, spec id preserved |
| `agent_contract::start_records_session_with_provider_and_project_association` | **rewritten** | the old assertion `s == "active" \|\| s == "unsupported"` *accepted* the fabricated state. New: the state is `unsupported` or the command is a typed refusal, and never `active`; the session id, project id and provider are still recorded and both files are still written |
| `agent_contract::restart_preserves_prior_state_and_records_active` | **rewritten** | asserted `active` outright. New: `unsupported`, session id and spec binding still preserved |
| `agent_contract::pause_/takeover_…unsupported…` | unchanged | already honest; now shares one code path with start/resume/restart |
| `supervised_agent_contract/legacy_session.rs` (3) | unchanged | the non-regression proof for the Ariadex arm |
| `supervised_agent_contract/ariadex.rs` (12), `sisyphusfy.rs` (9), `native_toolchain.rs` (5) | unchanged | Ariadex/sisyphusfy non-regression |
| `web_agent_identity_readonly_contract` | unchanged | the reworded note keeps the `recorded` substring |

## 7. Operator-visible consequence

Stated plainly, because it is a real reduction in claimed capability even
though it is a correction:

- `forge agent start --session s --provider codex` **never** reports `active`
  in this build. It reports `unsupported` (exit 0) or, when `codex` is not on
  `PATH`, `agent-unavailable` (exit 1) naming both facts.
- `forge agent resume`, `restart` and `new-session` behave the same for
  `opencode`/`codex`.
- `forge agent run-spec <project> --session s` on a bundled session now
  returns `unsupported` instead of a fabricated `active`.
- `forge agent status` / `list` still render whatever the record says;
  `forge agent status` never adds a `live:` block for a bundled provider
  (already true).
- `forge agent start … --provider ariadex` is **unchanged** and remains the
  only way to obtain a session state backed by a real runtime.
- Sessions recorded by an earlier build are not rewritten; a stored
  `state: "active"` reads back as stored, and the transition log keeps its
  history.

## 8. Gate and evidence

`forge gate` is the orchestrator's for this cycle (per the task brief), so no
Gate verdict is claimed here; the HANDOFF evidence table records the
delegation and `forge gate --dry-run` is run as a safe rehearsal. Local
verification is the lightweight set named in the brief:
`cargo build`, `cargo fmt --check`, `cargo clippy`,
`cargo test --lib agent`, `cargo test --lib gate`,
`cargo test --test agent_contract`,
`cargo test --test supervised_agent_contract`,
`cargo test --test agent_runtime_workflows_cross_surface`,
`cargo test --test supervised_agent_cross_surface`,
`cargo test --test web_agent_identity_readonly_contract`,
`node scripts/check-openspec-change-names.mjs`,
`node scripts/check-spec-governance.mjs`,
`openspec validate --all --strict --no-interactive`, `git diff --check`.
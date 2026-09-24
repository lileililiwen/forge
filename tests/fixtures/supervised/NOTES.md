# Supervised runtime fixture notes (`supervised-agent-adapters`)

Exact command/output contracts recorded from the sibling sources on
this host (2026-09-24) as required by the change design. The stub
binaries in `tests/supervised_agent_contract.rs` emit these shapes.

## ariadex (`/home/paul/code/ariadex`, binary `ariadex`)

* The project directory is always the invocation cwd; there is no
  `--project` flag on lifecycle verbs (`src/ariadex/cli.py`:
  `_project_dir() -> Path.cwd()`). Forge adapters therefore set
  `current_dir` to the session's project path.
* `ariadex --version` → `ariadex 0.1.0` with optional `+g<short-sha>`
  and `-dirty` build suffixes (`cli.py` version action,
  `upgrade.describe_build`).
* Durable coordination modes: `AUTO`, `MANUAL`, `PAUSE` (uppercase;
  `state.py MODES`). Daemon record statuses: `running|stopping|stopped`;
  the `--json` daemon view exposes `alive: bool`, `mode`, `session`.
* `ariadex start` (headless, no tty) → exit 0, prints
  `daemon started (pid <pid>, endpoint .ariadex/daemon.sock); observing
  durable state` and `managed runtime started; daemon owns provider,
  watcher, and widget`. Not initialized → stderr `error: project is
  not initialized; run \`ariadex init\` first ...` exit 1. With zero
  active OpenSpec changes → `no active OpenSpec changes; provider not
  started` exit 0 (no daemon is spawned).
* `ariadex status --json` → one JSON document, optionally followed by
  plain `blocker <id>: <desc>` text lines. With a live daemon the
  document is `{"daemon": {"alive": true, "mode": "AUTO", "session":
  "<12hex handle>", ...}}`; without one the local form is
  `{"mode": "AUTO", "session": "<12hex handle>", "agent": ..., ...}`.
  Uninitialized project → exit 1, stderr `missing runtime state at
  .ariadex/state.json; run \`ariadex init\` first`.
* `ariadex pause [--json]` → exit 0 idempotent; JSON `{"changed": ..,
  "coordination": .., "mode": "PAUSE", "note": .., "ok": true, "via":
  "pause"}`.
* `ariadex resume [--json]` → from PAUSE: exit 0, mode returns to
  `AUTO` (`"mode is already AUTO; no change made"` from AUTO, exit 0);
  from MANUAL: stderr `error: resume rejected from MANUAL: only a
  PAUSED project may resume (use \`takeover\` or \`auto\` from other
  modes)` exit 1.
* `ariadex takeover` → MANUAL mode (automatic input disabled until
  `ariadex auto`); Forge maps takeover to guidance only and never
  invokes `ariadex attach` (which `execvp`s
  `tmux attach-session -t ariadex-<sid>` and would hijack the
  operator terminal).
* Session handle: `uuid4().hex[:12]` generated at `ariadex init` and
  reported by the status surface; the tmux name is `ariadex-<sid>`.
* Duplicate-owner `start --json` prints TWO back-to-back JSON
  documents; no-live-daemon manual-action refusals print a compact
  one-line `{"error": .., "ok": false}`. Only the FIRST complete JSON
  document is ever consumed.

## sisyphusfy (`/home/paul/code/sisyphusfy`, binary `sisyphusfy`)

* **Live-verified contract correction (2026-09-24):** the high-level
  `run <change>` positional resolves `openspec/changes/<change>/tasks.md`
  or project-root `tasks.md` candidates only (`config.discover_task_path`)
  — it does NOT accept a `.forge/specs/<id>/tasks.md` path
  (`run` answers `{"error": "change not found: <path>"}` exit 1).
  Forge therefore delegates through the documented low-level loop
  verb, which takes an explicit task file:
  `sisyphusfy loop --task-path .forge/specs/<id>/tasks.md --json
  --adapter <provider>`. Passing `--adapter` lets the sibling's own
  adapter registry own the agent CLI grammar (opencode|codebuddy|
  generic; an unknown adapter name exits 2 with argparse usage and no
  JSON, which Forge classifies `unverified`).
* `loop --json` prints exactly one JSON document on stdout (progress
  goes to stderr and is disabled in JSON mode).
* Outcome document keys (`LoopResult.to_dict`): `stop_reason`,
  `iterations`, `run_records`, `final_task_path`, plus `verification`
  with `status` (a run classification: `success|failure|timeout|...`
  or `skipped`) and `source` (`configured|discovered|unavailable`).
* `stop_reason` vocabulary: `complete|max_iterations|timeout|
  interrupted|blocked|agent_failed|unchanged_state|verification_failed|
  models_exhausted|adapter_error|dry_run|command_not_found|
  context_budget_exceeded`.
* Exit codes: 0 **iff** `stop_reason == "complete"`, else 1. Blocked
  iterations carry `blocked_reason` (marker lines, e.g.
  `NEED_PERMISSION`); agent failures carry `agent_error.{name,message}`.
* Live round-trip shape (real binary, agent absent from PATH):
  `{"stop_reason": "command_not_found", "iterations": 1, "run_records":
  [{"iteration": 1, "result": {"command": ["opencode", "run"],
  "exit_status": 127, "classification": "command_not_found", ...}}]}`
  exit 1 → Forge verdict `partial` with the named reason.
* Verification "runs exactly once per productive iteration and always
  before completion is accepted" (`loop.py`); agent claims are not
  proof. `verification.status == "skipped"` means not verified.
* No `--version` flag exists on this binary; Forge does not
  version-probe the supervisor and instead consumes the outcome
  document shape directly.

## Verdict mapping implemented by `src/agent`

| Supervisor report | Forge verdict |
| --- | --- |
| exit 0 + `complete` + verification `success` | `done` |
| `blocked` / `max_iterations` / `timeout` / `interrupted` / `agent_failed` / `unchanged_state` / `models_exhausted` / `adapter_error` / `command_not_found` / `context_budget_exceeded` | `partial` (supervisor reason named) |
| `verification_failed`, `dry_run`, `complete` without passing verification, exit/document disagreement, unparsable document, bounded-wait exceeded | `unverified` (never `done`) |

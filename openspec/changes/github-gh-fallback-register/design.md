# Design: GitHub gh-fallback and create register-if-missing

## 1. Decisions

- **Adapter first, `gh` fallback second (chosen).** `cmd_github_observe`/`cmd_github_propose` keep the exact adapter path (binary + `FORGE_GITHUB_TOKEN` checks) when `binary_available()`. Only when no adapter binary resolves and `GhCli::from_env().binary_available()` does the `gh` fallback run. Alternative (prefer `gh` always) rejected: the versioned adapter contract stays authoritative when configured.
- **Fallback observe is `gh repo view --json` (chosen).** Fields `nameWithOwner,description,repositoryTopics,defaultBranchRef,isArchived,primaryLanguage`; parsed in Rust into `GithubObservation{contract,host,repository,state:Current,...}` with `source_revision=None`, redacted description/note, topics deduplicated via the existing bound. Failure maps through `classify_failure` (auth-required/forbidden/not-found/rate-limited/timeout/unavailable/failed). Alternative (shell `--jq` shaping) rejected: jq availability varies; Rust parsing is deterministic.
- **Fallback propose is single-`topic=` direct only (chosen).** Gate: `mode==direct` + non-empty `--confirm` + exactly one change with `field==topic`. Runs `gh repo edit <repo> --add-topic <topic>`; success returns `ProposeOutcome{mode:"direct",state:Current,note:"direct topic update via gh (confirmation verified locally; value not logged)"}` — the confirm echo is the local equality check, the token value never enters the note/log. PR mode and multi-field/other-field direct requests return the original `github-adapter-unavailable` so behavior is unchanged. Alternative (all fields via `gh repo edit`) rejected: description/homepage flags differ in review semantics; keep the fallback minimal and reviewable.
- **`--register-if-missing` is create-only and explicit (chosen).** New `#[arg(long="register-if-missing")] register_if_missing: bool` on `GithubCommands::Create`, threaded through `cmd_project_github` → `cmd_github_cli_create`. When true: canonicalize the project dir, open the registry read-write, `inspect` by id-or-path — miss → `Manifest::load_from_dir` (validates `forge.yaml`) → `Registry::register` → proceed to `run_create`. JSON gains `register_if_missing` + `registered`; human gains `registered=<bool>`. Without the flag nothing changes (unregistered dirs already work; registered ones refresh nothing). Alternative (auto-register always) rejected: implicit registry writes violate the explicit-operation rule.
- **Portal change is controls-only (chosen).** `controls_for(Repositories)` gains `forge project github observe <owner/repo>`, `... propose <owner/repo> --set topic=<t> --mode direct --confirm <token>`, `... create <path> --repo <owner/name> [--push-source --confirm] [--register-if-missing]`; no new section id, no status change, existing `portal-invalid` and roll-up behavior intact. Alternative (new section) rejected: twelve-section contract is stable; clients assert it.
- **Web change is one appended Delivery card (chosen).** `frontend/index.html` gains `wb-card` `github-metadata` with topics `<ul id="github-topics">` + `<p id="github-topics-alt">` text alternative, propose form (repo text, field select, value text, pull-request/direct radios, password confirm field), create form (project/repo text, private/public radios, push-source + register-if-missing + action-confirm checkboxes), `role="status"` result div. `frontend/app.js` `initGithubMetadata()` renders topics from the existing maintain `github` payload (dev-highlight for `dev-`/`forge-` prefix, case-insensitive), validates locally, previews the exact CLI strings, never fetches a new endpoint. Alternative (new API routes) rejected: no server change needed; CLI hints are honest and deterministic.

## 2. Implementation boundary

- **Files changed:**
  - `src/github/gh_fallback.rs` (new) — builders + runners + topic-only gate + redaction.
  - `src/github/mod.rs` — `pub mod gh_fallback;` + re-exports.
  - `src/cli/commands_ops.rs` — `Create.register_if_missing` flag.
  - `src/cli/project.rs` — observe/propose fallback branches + flag threading.
  - `src/cli/github.rs` — create register-if-missing path + `registered` envelope.
  - `src/portal/sections.rs` — repositories controls.
  - `frontend/index.html`, `frontend/app.js`, `frontend/styles.css` (appended block only).
  - `tests/github_gh_fallback_register_contract.rs` (new).
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`, `specs/*/spec.md` deltas).
- **Modules reused unchanged:** `GithubAdapter`/`GhCli` resolution, `spawn_with_timeout`, `redact_credentials`, `classify_failure`, `normalize_observation`, `Registry::register`, `Manifest::load_from_dir`, portal roll-up/validation, frontend request/error-summary helpers, `forge gate` runtime.
- **Must NOT change:** adapter contract version, `gh` allowlist for auth/clone/pr, registry schema, API routes, portal section ids, contrast/touch/motion tokens.

## 3. Language and runtime

Rust (stable, `cargo fmt`/`cargo build`/`cargo test`) for CLI/portal; vanilla JS (ES2021, no build) + vanilla CSS for `frontend/`. Verification: `cargo test --test github_gh_fallback_register_contract`, full `cargo test`, `forge gate --dry-run` + bounded `forge gate`, `openspec validate --all --strict --no-interactive`, `git diff --check`.

## 4. Failure modes

- No adapter + no `gh` → original `error[github-adapter-unavailable]` (observe/propose).
- Fallback `gh` auth missing → typed `github-cli-auth-required`; not-found/forbidden/rate-limited/timeout map via `classify_failure`; stderr redacted.
- Direct propose without `--confirm`, with empty confirm, multi-field, or non-topic field on fallback → `error[github-invalid]` or `error[github-adapter-unavailable]` (PR/other-field), no write.
- Create `--register-if-missing` with missing dir → `github-cli-invalid`; invalid `forge.yaml` → `github-cli-invalid` naming the manifest reason; id/path collision → existing `IdCollision`/`PathCollision` typed error; nothing pushed on refusal.
- Tokens/credentials never enter stdout/stderr/note/log: every fallback note is a fixed string; description/topics pass through `redact_credentials`; tests assert absence.

## 5. Accessibility / responsive

Labels on every input (`<label class="field">` + `field-hint`/`field-error`), native radios/checkboxes (keyboard operable), error summary `tabindex="-1"` with focus move, result `role="status"`, topics list is a real `<ul>` with a text alternative (`3 topics: a, b, c`), dev topics get `.topic-dev` background + `aria-label` suffix "dev topic" (color never the only signal), card stacks under the existing `workbench-grid` responsive rules, 44px targets via existing `.button`/`.field` rules.

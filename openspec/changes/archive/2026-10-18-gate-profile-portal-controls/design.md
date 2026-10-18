# Design: Gate profile workbench controls

## Implementation boundary

Forge Rust API and static browser UI only: `src/api/admin/gate.rs`,
`frontend/index.html`, `frontend/app.js`, and their existing Gate control
contract/browser tests. Extend the current `POST
/v1/admin/projects/{id}/gate/run`; do not add a route or alter CLI, hosted ingest,
project manifests, sibling repositories, or persisted evidence format.

## Language and runtime

Rust stable using existing Serde/clap `GateProfile`; browser code is the existing
vanilla JavaScript in `frontend/`. Use the repository's established Cargo test
targets for API contracts and browser scripts/harness for rendered-flow evidence.
The API request is capped at 8 KiB, must be a JSON object with only the supported
optional `profile` string, and defaults absent `profile` to `GateProfile::Full`.
Malformed JSON, unknown fields, non-string values and values other than `fast` or
`full` return 400 before project resolution/execution side effects. The parsed
profile is assigned to `GateConfig.profile`.

## Data flow and state

1. The workbench selector starts at `full`; the existing action reads its value.
2. The browser posts `{ "profile": "fast"|"full" }` to the existing session-
   gated route and disables the action while the request is pending.
3. The server validates the exact request shape, creates the existing full
   configuration, overwrites only its typed `profile`, then invokes existing
   `run_gate`.
4. Existing successful evidence persistence and journal behavior remain. The
   response's `GateEvidence.profile` is rendered by the shared status renderer.
5. Status GET and hosted observations render profile from existing evidence.
   Historical local evidence without a profile deserializes as `full` under the
   established evidence contract; an unrecognized projected value is displayed
   as unavailable.
6. A validation or runtime failure leaves the prior evidence visible and reports
   the existing safe error/notice.

## Compatibility and security

Omitting `profile` is a full-profile request for existing clients. The evidence
contract is unchanged because profile is already persisted and serialized.
Authorization, project ID resolution, process execution, secret scrubbing,
atomic persistence and fast-evidence hosted rejection remain as they are.
Request values are enum-parsed, never interpolated into shell input.

## UI details

Use a native `<select>` with an explicit visible `<label>` and explanatory help:
`Fast: required baseline only`; `Full: baseline and enabled project checks`.
Default selection is Full. Keep the existing rerun button, pending feedback and
live outcome region. Display profile as a text fact on local evidence and in each
hosted observation; unrecognized values render `Profile unavailable`. The narrow
workbench layout must allow grid and lifecycle values to shrink and wrap, and the
Quality card and profile control must remain inside the viewport. Verify keyboard
access, visible focus, screen-reader label/help association, and reflow at 320
CSS px.

## Verification oracles

- Rust API contract: omitted/full compatibility, explicit fast and full in
  execution evidence, unknown/invalid JSON rejected without changing evidence,
  authenticated access still required.
- Browser contract/runtime: selector default/labels and request payload; returned
  profile displayed for local and hosted evidence; pending, failure and success
  states preserve the existing honest evidence model.
- Browser accessibility: keyboard select/action, accessible label and focus, and
  no page-level horizontal scrolling at 320 CSS px.
- Repository workflow: targeted tests, formatting, OpenSpec strict validation,
  `git diff --check`, Gate dry-run and bounded full Gate run.

# Design: authenticated hosted CI Gate evidence ingest

## Implementation boundary

Repository `/home/paul/code/forge`; Rust 1.88+ modular monolith. Extend `src/ci_evidence.rs`, `src/registry/{schema,store,model}.rs`, `src/api/{model,router,handlers_core}.rs`, `src/api/admin/` and `src/command/` (locate actual command modules); add a `src/hosted_ci/` module for OIDC verification, persistence orchestration and projections. Extend `frontend/index.html`, `frontend/app.js`, styles and browser harness. Add new immutable snapshots `baseline-service@1.5.0` and `baseline-client@1.2.0`, promote defaults only after their scripts and workflow are complete. Do not change local Gate evidence semantics, delivery policy, unrelated projects or historical snapshots.

## Language and runtime

Rust 1.88+, edition 2021, serde, rusqlite 0.32, reqwest 0.12 blocking/rustls, jsonwebtoken 9.3.1, SQLite, Python 3 standard library in generated upload scripts, and vanilla HTML/CSS/JavaScript. Use RS256-only verification; do not implement cryptography. Commands: `cargo fmt --all -- --check`, focused Rust integration tests, `cargo test --test hosted_ci_oidc_contract`, relevant workbench/browser checks, `python3 -m py_compile` for the uploader, `node --check frontend/app.js`, strict OpenSpec validation, `forge gate --dry-run`, and bounded full `forge gate`. Use the repository's existing Chromium harness and accessibility checker.

## Ownership and shared code

Forge owns the `hosted-ci-evidence/1.0` contract, GitHub OIDC verifier, registry rows and API/UI. Existing `reqwest` and `GateEvidence` are reused; `jsonwebtoken` performs fixed RS256 validation. Sibling projects consume generated workflows and the pinned uploader script only. No extraction to a sibling `/lib`: only Forge currently owns this lifecycle, and a Forge-source dependency would break project portability. Forge's installed CLI version is pinned by standard snapshot; the wire contract is versioned independently.

## User experience and interface

Actor: Forge operator with a valid admin session. Entry is the existing project workbench Quality checks card. The card keeps local Gate result first, then a clearly labeled **Hosted CI observations** section. It has:

- Repository binding state: unconfigured or `owner/name` plus numeric GitHub repository ID. An explicit `Bind repository` form accepts a positive decimal ID; backend checks the slug against registered HTTPS/SSH GitHub remote and refuses conflicting existing binding. The ID is immutable in this capability; no browser reset is provided.
- Hosted history, newest observation first, capped at 20 rows. Each row shows workflow/job, GitHub run link, run and attempt, short SHA, aggregate, observed time, and current revision freshness (`current`/`stale`/`unknown`). API history retains the complete validated envelope, including per-check statuses and safe bounded notes.
- Loading/success/empty/unavailable/error/permission states use existing component patterns, `role=status`/`aria-live`, labels, visible keyboard focus, no color-only status, and responsive wrapping. Sort is newest-first and no client side secret/token state is stored.
- No automatic refresh loop. Reload uses the existing workbench refresh path; binding is a separate explicit action.

Browser oracle: a signed-in operator sees the unbound empty state on one project, then sees a bound hosted record, run URL and stale state on another project after browser reload/navigation. API tests cover the bind mutation and accepted ingest separately. Check keyboard form access, live announcements, contrast, 390px overflow and console/network errors. API tests cannot replace the rendered UI checks.

## Behavioral model

| State/event | Preconditions | Effect | Replay/failure |
|---|---|---|---|
| Bind ID | Admin session; registered project; exactly one canonical GitHub slug in `git_remote`; positive numeric repo ID; not already bound | Set immutable `github_repository_id` | Same ID is idempotent; different ID conflicts; no automatic replacement |
| Ingest | Valid envelope; valid signed OIDC token; issuer/audience/time valid; claims match envelope; exact registered slug and immutable repo ID match | Insert one append-only row in transaction | Same run/attempt and identical digest returns existing record; same identity with different digest returns 409 |
| Read | Admin session and registered project | Return max 20 most recent observations plus freshness against current registered revision | Missing rows is successful empty list; storage failure is unavailable, never empty success |
| Key fetch | Fixed GitHub metadata/JWKS endpoints; HTTPS only; bounded connect/total timeout and response size | Cache keys by `kid` for bounded TTL; refresh once for unknown `kid` | Network/parse/key failure is 503 and does not write; never fall back to unverified claims |

JWT must validate signature with RS256 only, `iss=https://token.actions.githubusercontent.com`, expected audience `forge-ci-evidence`, `exp`, `nbf` when present, and required claims `repository`, `repository_id`, `run_id`, `run_attempt`, `workflow_ref`, `sha`, `event_name`, `ref`. Token audience is requested from Actions runtime using the fixed literal, not supplied by workflow input. Claim `repository` and numeric `repository_id` match both the envelope and project binding. `run_id`/attempt/workflow-ref/SHA match the envelope; workflow reference must be the generated `.github/workflows/forge-gate.yml` on a branch, and event must be `push`. The job name is informational because it is not a standard signed claim; uniqueness is therefore one record per project/run/attempt. Ignore `sub` format because GitHub allows subject customization. Require SHA to equal the full lowercase source revision. The report stores claims needed for audit but never the JWT.

OIDC audience verification is separate from admin session auth. `POST /v1/ci/github/evidence` is the sole public workload route; it accepts bearer ID token and a bounded JSON envelope. `GET /v1/admin/projects/{id}/ci/hosted` and `PUT /v1/admin/projects/{id}/ci/repository` are Forge admin-session routes. The ingest identifies project only through signed repo claims + registered slug/immutable ID; it never trusts an arbitrary project path or body project ID as authority.

## Contract, persistence, compatibility

Use the existing closed `HostedCiEvidenceEnvelope` JSON body. Upload route responds 201 for insert and 200 for exact replay with `{contract, project_id, id, duplicate, received_at}`; no raw token or path. Errors: 400 malformed body, 401 missing/invalid OIDC, 403 unbound/mismatched trusted repository or claim mismatch, 404 unknown matching project, 409 replay conflict, 413 body limit, 503 JWKS unavailable. Responses are bounded and do not echo document/token.

Add a separate `hosted_ci_project_bindings` table keyed by project ID with globally unique repository ID and normalized slug, plus a `hosted_ci_evidence` table with internal stable ID, project ID, run/attempt/job, repository/repository ID, workflow, SHA, observed timestamp, aggregate, envelope JSON, canonical envelope SHA-256 digest and received timestamp. Unique `(project_id, run_id, run_attempt)`; index `(project_id, received_at DESC)`. Envelope size is capped at 256 KiB despite API's larger general body limit. Keep one transaction per bind/insert; digest comparison happens under the same immediate transaction to serialize concurrent retries. Historical rows survive project re-registration and do not alter local Gate `.forge/gate` evidence. Old projects default unbound.

The pinned standard script `.standard/scripts/upload-hosted-ci-evidence.py` runs only inside GitHub Actions. It loads the single `.forge/gate/<project-id>/evidence.json` artifact produced by Gate, derives bounded run metadata from GitHub environment, requests an ID token from `ACTIONS_ID_TOKEN_REQUEST_URL` with `ACTIONS_ID_TOKEN_REQUEST_TOKEN` using the fixed audience, then POSTs the envelope to `FORGE_CI_EVIDENCE_URL`. It refuses missing variables, wrong project/revision, rehearsal evidence, nonterminal aggregate or HTTP error; it never prints the token or response body. The endpoint must be HTTPS. Retry only 429/5xx and transport timeouts with bounded exponential backoff; exact replay is safe.

Generated workflows run a `contents: read` verification job without OIDC permission, then a dependent `upload-hosted-evidence` job with `id-token: write` only and `needs: verify`. The verify job exposes whether a Gate evidence file exists and uploads only `.forge/gate/*/evidence.json`; `include-hidden-files: true` is scoped to that file path because Forge stores evidence under a hidden `.forge` directory. It preserves the record even when Gate blocks. The uploader job runs only for a completed, non-cancelled branch push with a Gate record and configured endpoint; blocked observations are retained while the verification job remains failed. Pull requests have an explicit visible skip notice and receive no OIDC job. If `FORGE_CI_EVIDENCE_URL` is absent, the verify job logs a visible skip and no OIDC job runs. New snapshots leave historical pack versions unchanged.

## Failure, security and recovery

Fail closed for invalid signature/alg/kid/issuer/audience/exp/nbf, absent required claims, repository slug or ID mismatch, run/attempt/workflow/SHA mismatch, unknown project, absent binding, malformed envelope, unsafe configuration URL, body/key response over limits, unknown duplicate conflicts, SQLite lock/write failure, and JWKS unavailability. No persistence occurs before all verification and binding checks. Never log Authorization, OIDC request token, complete request headers, or untrusted body. Redact response details and keep only validated envelope. Admin-only reads are scoped by validated project id and normal Forge session/origin guard. Public ingest has body and network timeout bounds and does not disclose project existence before a valid signature.

## Verification oracle

1. Deterministic RSA fixtures validate good token, wrong audience, `alg=none`, expired token, missing claims, and mismatched repository ID/slug/run/attempt/workflow/SHA; the verifier pins its JWKS URL, response size, timeout, key type and cache policy in code.
2. Registry tests prove additive hosted-table initialization on an existing DB, immutable binding, old project unbound status, append-only reads, exact replay, conflict replay and two concurrent same-key posts.
3. Route tests prove unauthenticated OIDC ingest success only with valid signed claims, browser session cannot ingest, bearer app token cannot read admin routes, admin session can bind/read, route classifier/CORS and error codes.
4. Uploader tests mock the HTTP opener, validate exact envelope and auth headers, fixed audience and bounded timeout; ensure stdout/stderr contain no token. The script independently checks missing environment, failed Gate, non-HTTPS endpoint and retry bounds.
5. Workflow contract tests assert verification has no `id-token: write`, upload job is dependency-gated, terminal blocked Gate evidence is preserved, pull requests have no token permission and a visible skip, and endpoint absence remains non-failing/visible.
6. Browser flow and accessibility checks as above; strict OpenSpec, formatting and local Gate as repository mandates.

## Decision ledger

- Resolved: GitHub Actions is the only issuer/provider in v1; audience is fixed; immutable repository ID is operator-bound, not first-run auto-enrolled; hosted observations do not authorize delivery.
- Resolved: use current SQLite registry and existing Forge workbench; no external storage or sibling code.
- Assumption: GitHub Actions ID token exposes `repository_id`, `run_attempt`, `workflow_ref`, `sha`, `event_name`, and `ref`; the job name is not a signed claim and remains descriptive only.
- Deferred: automated repository ID discovery, token revocation lists, other CI providers, retention/cleanup and release/check-run status writes. No unresolved design blockers.

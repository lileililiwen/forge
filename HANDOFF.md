current_spec: standard-pack-registry-and-snapshots

# Forge handoff

## Current state

`portfolio-interest-snapshots` implemented, verified and archived on
2026-09-28 as `2026-09-28-portfolio-interest-snapshots`; its three
requirements (Forge accepts only privacy-safe aggregate snapshots,
snapshot evidence is immutable and idempotent, comparisons preserve
window semantics) were promoted into
[openspec/specs/interest-snapshots/spec.md](openspec/specs/interest-snapshots/spec.md).
The implementation closes every Section-1 BFS, Section-2 DFS,
Section-3 BFS and Section-4 verification task in the proposal.

**Domain** (`src/portfolio/interest/`). The package is split by
concern so each file answers one question, matching
`src/portfolio/share/*`: `mod.rs` owns the vocabularies, bounds and
record shapes, `validation.rs` owns the single `validate_snapshot`
gate, `compare.rs` owns the comparison and trend projections.
`PrivacyMode` (`exact-count|lower-bound|undeclared`), `Coverage`
(`complete|partial`), `SnapshotState` (`accepted|superseded`),
`Freshness` (`current|stale`) and the five-count `InterestMetric`
allowlist (`unique_visitors`, `completed_public_workflows`,
`returning_visitors`, `outbound_cta_clicks`, `paid_interest_events`)
are closed enums. `paid_interest_events` is an aggregate signal,
deliberately **absent** from the `PAYMENT_KEYS` refusal list, and
never a basis for granting access.

The load-bearing rule is the **closed key set**: a record may carry
only `SNAPSHOT_KEYS` (nine keys) and a `metrics` object may carry only
metric labels, so an identity, raw-event, payment or credential field
is refused *by construction* rather than by review, and a refusal
names the field and the rule and never echoes the value. The four
deny lists are `IDENTITY_KEYS`, `RAW_EVENT_KEYS`, `PAYMENT_KEYS` and
`CREDENTIAL_KEYS`; an email-shaped value and a raw URL carrying a
query are refused on shape as well as on name; every refusal,
rejection and finding passes `redact_interest_text`, which applies
`policy::redact_credentials` and then drops address-shaped tokens. A
metric map that is all zeros is refused unless the source declared
`coverage: complete` — a partially measured window reporting zero is a
collection gap, and storing it would turn a broken provider into a
standing statement that nobody was interested.

**Persistence** (`src/registry/interest/`). Three additive tables
(`portfolio_interest_snapshots`, `portfolio_interest_metrics`,
`portfolio_interest_findings`) applied by
`apply_portfolio_interest_migration` inside its own explicit
`BEGIN IMMEDIATE` … `COMMIT` batch with `ROLLBACK` on any failure —
deliberately *separate* from the portfolio and share batches so a
rollback in any domain leaves the others intact. `projects`,
`operations` and every portfolio/share table are untouched, and the
interest domain writes **no** `operations` journal row. Metrics are
rows, not a JSON blob: there is no `payload`/`events`/`metadata`
column for a free-form value to hide in. Snapshots are append-only —
only `accepted` → `superseded` ever changes, and only when the source
names the revision it replaces. Identity
(`project_id, source, source_revision, window_start, window_end`) is a
`UNIQUE` constraint: an exact repeat is an idempotent `AlreadyPresent`,
and a *changed* payload under an already-attested identity is a
conflict rather than a merge.

**Orchestration** (`src/portfolio/interest_report.rs`). The single
door both transports enter. `import_snapshots` is deliberately
per-record: a batch stores its sound records and reports the rest with
an index, a field, a stable code and a reason. Windows are UTC
half-open, so `2026-09-01..09-08` and `2026-09-08..09-15` are adjacent
and both storable, while any same-source overlap is refused until the
source declares `replaces_source_revision`. `compare_projects` and
`interest_trend` read *current* snapshots only, so a superseded
revision never answers the same question twice; `project_interest`
(`show`) is the history view and keeps them. Freshness is derived from
the window end at read time and never stored, so widening the bound
never rewrites an observed value.

**CLI.** `forge portfolio interest import|list|show|compare|trend|audit`.
`import` takes a path or `-` for stdin, bounded to 1 MiB before it is
parsed. A batch whose records are **all** refused is a typed
`portfolio-interest-invalid` whose reason names each refused record's
index, code and reason (bounded to five and 200 chars each) — the
`forge fleet online` precedent for "a report with zero successes is a
failure the shell can see". The reason text has already been scrubbed,
so nothing echoes.

**JSON API.** `GET|POST /v1/projects/{id}/interest`,
`GET /v1/interest/{compare,trend,audit}` — all behind the existing
`authorize()`, and **all** requiring `admin:access`, the reads
included: which projects draw interest, and which draw none, is the
private half of a portfolio decision. The import route injects the
project from the path and refuses a body that names a different one,
and records the **authenticated session subject** as the importer, not
a client-claimed actor. Two new typed errors:
`portfolio-interest-invalid` → 400, `portfolio-interest-conflict` →
409. `percent_decode` handles the comma-separated project list.

The pointer advances to `standard-pack-registry-and-snapshots` — the
next active change in `openspec list` whose declared prerequisites
(the profile registry, generation and the runtime template contract)
are all archived and whose oracle ("render/diff/upgrade contract
tests") is locally verifiable. `fleet-live-rollout` remains active but
is deliberately **not** selected: its acceptance is live 20/20 green on
the Mac, and the previous cycle recorded that the Mac has no fleet yet,
so it cannot be completed or honestly evidenced from this checkout.

## Verification evidence (portfolio-interest-snapshots, 2026-09-28)

- `cargo fmt --all -- --check`: PASS for every touched file
  (`src/api/mod.rs`, `src/core/mod.rs`, `src/main.rs`,
  `src/portfolio/mod.rs`, `src/registry/mod.rs`,
  `src/portfolio/interest/{mod,validation,compare}.rs`,
  `src/portfolio/interest_report.rs`, `src/registry/interest/mod.rs`,
  `tests/support/{mod,interest}.rs`,
  `tests/portfolio_interest_{cli,api,cross}_contract.rs`). The
  pre-change baseline carries formatting drift in
  `src/gate/evidence.rs`, `src/publish/{fleet,jenkins}.rs`,
  `src/portfolio/share/validation.rs`, `tests/gate_contract.rs`,
  `tests/gate_cross_surface.rs` and
  `tests/publish_queue_status_contract.rs`; `cargo fmt` touched them
  incidentally and those edits were reverted with `git checkout --`, so
  the drift is preserved exactly as the prior cycles left it.
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: verified against the
  stashed baseline with `git stash push -u -- src tests` and a
  location-by-location `comm` diff — **12 baseline locations, 12 after,
  zero new**. The pre-existing locations are
  `src/api/ui/auth.rs:166-167`, `src/gate/evidence.rs` (229, 425, 826,
  827, 862), `src/portfolio/share/validation.rs` (13, 392),
  `src/publish/fleet.rs:51` and `src/publish/mod.rs:634-636`. Three
  clippy errors this change *did* introduce were fixed rather than
  suppressed: two "this loop never actually loops" in the interest
  query parsers, rewritten into the `parse_share_limit` shape, and one
  needless borrow in `validate_window`.
- `cargo test --workspace --all-targets -- --skip
  rust_scaffold_builds_and_tests_with_native_toolchain`: **78 result
  groups, 1772 tests, 0 failed**; the change adds no new long-running
  native test. New supervised suites: 34 unit tests across
  `src/portfolio::interest::{mod,validation,compare}` and
  `src/registry::interest`, 16 `tests/portfolio_interest_cli_contract.rs`,
  13 `tests/portfolio_interest_api_contract.rs` and 10
  `tests/portfolio_interest_cross_surface.rs`. Notable coverage: closed
  vocabularies; the strict envelope (wrong/absent contract, stray
  envelope key, non-object record, empty batch); all four refusal
  classes plus an unknown field named and never echoed; a closed metric
  object; negative / fractional / string / list / object / over-bound
  values; the naive-timestamp, inverted, empty and 731-day window
  refusals; self-replacement; UTC normalization and canonical metric
  order; the all-zero-complete vs all-zero-partial rule; half-open
  adjacency vs overlap; idempotent repeat vs changed-payload conflict;
  declared replacement superseding only the named revision; the
  registry proving additive idempotent migration, a hand-built
  pre-change registry carrying forward with its project *and* journal
  rows intact, an interrupted batch rolling back with no partial table
  and the next open retrying cleanly, an unreadable state reading as
  the one that never compares, and metrics stored as rows in canonical
  order; the schema-column privacy assertion; stored metric names read
  back against the allowlist; refused imports never erasing evidence;
  two sources over the same instants never summed (and `150` never
  appearing anywhere); the private portfolio projection, public share
  manifest and fleet list carrying no interest data; and no
  `operations` row written.
- Live binary smoke (CLI), against a two-project local registry:
  `forge portfolio interest --help` advertises all six subcommands;
  a four-record batch reports `3 accepted, 0 already present, 0
  superseded, 1 refused` and the JSON report names the refused record's
  index, field and code; `compare` on two windows renders
  `comparable across windows: false`, both windows, per-row
  `[source / source_revision / privacy_mode / freshness]`, and the two
  notes ("will not rank or total across windows", "lower-bound is not
  an exact count"); `show` renders both `accepted` windows, their
  metrics, `stale windows: 2` and the persisted refusal; `trend`
  renders both points with `lower-bound`/`stale` labels. Refusals:
  an overlapping record reports
  `record 0 [interest-overlap-refused]: source \`github-analytics\`
  already reported the window … count the shared days twice` (exit 1,
  stdout empty); a `card_number` metric reports
  `interest-payment-refused`; an `email` metric reports
  `interest-identity-refused` with the address absent from every
  finding; an unknown project reports `unknown-project` and stores
  nothing; `--stale-after-days 0` and `501`-record audit limits are
  typed refusals. Widening `--stale-after-days` from 1 to 365 moved
  `stale windows` from 1 to 0 while leaving the stored value identical.
- Live binary smoke (HTTP), `forge api serve --bind 127.0.0.1 --port
  18971` with a minted session: `/healthz` stays byte-identical; every
  interest route without a bearer is 401 (including
  `GET /v1/interest/audit?limit=0`, which is 401 rather than a 400
  disclosure); `/v1/projects` and `/ui` without a bearer are 401; a
  mixed batch over `POST /v1/projects/alethefy/interest` returns
  `accepted 1 rejected [(1, 'interest-identity-refused')] actor
  release-bot`; a session minted for `alethefy` presented to
  `POST /v1/projects/forge/interest` is 403 `api-project-mismatch`; the
  projection returns `measured True snapshots 1 refusals 1`; `compare`
  returns `rows 1 total None freshness stale privacy exact-count`; the
  trend returns one point; `audit` returns the refusal and **zero**
  occurrences of the address; an unknown metric is a 400
  `api-invalid` naming the allowlist; an overlap over HTTP is reported
  per record; and `GET /v1/projects/alethefy/portfolio` and
  `GET /v1/share/manifest` contain zero occurrences of the interest
  metrics, source revision or `interest` key.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
  **No new dependency** was added and `Cargo.toml`/`deny.toml` are
  unmodified; the closure is unchanged.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 50 passed,
  0 failed (50 items) pre-archive and 50 passed, 0 failed post-archive
  with the promoted `interest-snapshots` spec (+3 requirements);
  `git diff --check`: PASS.
- **Blocked, honestly recorded:** no analytics provider was contacted
  and no provider credential was used. Every fixture in this package is
  a local JSON document or an in-process call; there is no adapter
  subprocess, no `FORGE_*_BIN` override and no network request in the
  interest code path. This package is the *consuming* half of an
  analytics pipeline: the provider that produces the aggregate, its
  collection policy and its credentials stay outside Forge, and Forge
  claims nothing about how a figure was produced beyond the
  `privacy_mode` and `coverage` the source declared. Real provider
  runtime collection remains external evidence that does not exist yet.
  The package also fixes no schema in `platform-contracts`: the import
  envelope is Forge's own
  (`forge-portfolio-interest/0.1.0`), and no cross-repo fixture
  validation is claimed.
- Pointer state: `portfolio-interest-snapshots` archived (`14/14` tasks
  evidenced, `4/4` artifacts complete). `openspec list` shows three
  active changes (`standard-pack-registry-and-snapshots`,
  `fleet-live-rollout`, `portfolio-activation-readiness`); the pointer
  advances to `standard-pack-registry-and-snapshots`, whose
  prerequisites are archived and whose oracle is local, while
  `fleet-live-rollout` stays active but unselected because it requires
  live target access. `portfolio-activation-readiness` is an
  **implementation-ready, planning-only** change authored in this cycle
  to track the deferred product-owned activation follow-up. It has no
  implementation and its 23 tasks are unchecked, but nothing is left for
  an implementer to decide: `design.md` pins the exact domain types and
  field names, the eight-reason vocabulary and its fixed order, the
  candidate-selection and no-fallback rule, the evaluation control flow,
  the CLI grammar with defaults and bounds, the exact human layout, the
  exact JSON and API shapes, the `fleet online` print-then-error gate
  exit pattern, the one additive error code, the file-by-file boundary,
  a ten-row calibration table, and a named unit/CLI/API/cross-surface
  test list. It claims nothing.
- Two deferred items this package could not close are now registered
  where an operator will look rather than only in an archived task list.
  The external one is recorded in
  [docs/provider-evidence.md](docs/provider-evidence.md) under
  "Portfolio interest import consumption" with its dated `not-run`
  status and a concrete exact next action: the analytics adapter
  boundary exposes only `health`, so a **producer**, not a consumer, is
  what is missing, and it is the provider that must add the verb. The
  product-owned one is recorded in the [ROADMAP](ROADMAP.md) deferred
  choices and in the planning-only change above. Neither is Forge
  implementation work, which is why neither became an implementation
  change: putting analytics collection or billing into Forge would
  violate the package boundary this change just established.
- No shared Gate Runtime is configured; no Gate pass is claimed. No
  PostgreSQL, multi-user, SSO or remote-synchronization readiness is
  claimed; no product database, visitor identity, payment record or
  external analytics host was contacted, read or written.

## Current state

`portfolio-share-publish` implemented, verified and archived on 2026-09-29
as `2026-09-28-portfolio-share-publish`; its four requirements (admin-defined
explicit share record, exact-approval publication, deterministic and
idempotent publication, public output excluding private data) were promoted
into
[openspec/specs/portfolio-share/spec.md](openspec/specs/portfolio-share/spec.md).
The implementation closes every Section-1 BFS, Section-2 DFS, Section-3 BFS
and Section-4 verification task in the proposal.

**Domain** (`src/portfolio/share/`). The package is split by concern so each
file answers one question, matching the `src/api/ui/*` and `src/publish/*`
convention: `mod.rs` owns the vocabularies and record shapes,
`validation.rs` owns secret detection and the public URL/text/evidence rules,
`manifest.rs` owns the contracted document and its SHA-256, `publish.rs` owns
the local and adapter publishers, `audit_types.rs` owns the approval and
attempt rows. `src/portfolio/publication.rs` is the single orchestration path
both transports enter. Lifecycle (`validated|approved|published|superseded|rejected`),
visibility (`public|unlisted`), showcase status
(`planned|demo|beta|stable|archived|unknown` — deliberately *not* an
availability claim) and publication outcome (`published|failed|unknown`) are
closed enums. Public URLs must be `https://`, free of embedded credentials and
non-default ports, on a fully qualified public host, and free of a path
segment from the closed private list (`admin`, `account`, `settings`,
`internal`, …), of `.`/`..`, of percent-encoding, and of credential-bearing
query keys. Secret detection reuses `policy::redact_credentials` as its first
line and adds a well-known token-prefix scan; a rejected value is **refused,
not redacted**, and the finding never echoes it.

**Manifest.** The hashed part is `ManifestBody` (`schema_family`,
`schema_version`, `manifest_revision`, `projects[]`) sorted by stable project
id with surfaces sorted by `(label, url)`; the published document adds
`generated_at` and `manifest_sha256`. That split is what makes "same records →
same bytes and hash" and "document carries an emission time" both true. An
empty catalog is valid and publishable. `status_evidence` is a closed
string-valued object (five allowed keys) — there is no arbitrary metadata map.

**Persistence** (`src/registry/share/`). Five additive tables
(`portfolio_share_records`, `portfolio_share_surfaces`,
`portfolio_share_findings`, `portfolio_share_approvals`,
`portfolio_share_publications`) applied by `apply_portfolio_share_migration`
inside its own explicit `BEGIN IMMEDIATE` … `COMMIT` batch with `ROLLBACK` on
any failure — deliberately *separate* from the portfolio batch so a rollback in
either domain leaves the other intact. `projects`, `operations` and every
portfolio table are untouched, and the share domain writes **no** `operations`
journal row.

**CLI.** `forge portfolio share set|remove|show|list|preview|approve|publish|reconcile|audit`.
Every mutation is project-scoped, validated before the write, and idempotent
where an identity is provided. `preview` is read-only.

**JSON API.** `GET|POST /v1/projects/{id}/share`,
`POST /v1/projects/{id}/share/remove`, `GET /v1/share/manifest`,
`POST /v1/share/{approve,publish,reconcile}`, `GET /v1/share/audit` — all
behind the existing `authorize()`, and **all** requiring `admin:access`,
preview included: previewing the candidate manifest reveals which projects an
operator considers publishable, which is itself private. Two new typed errors:
`portfolio-share-invalid` → 400, `portfolio-share-conflict` → 409.
`authorize()` now returns the authenticated session subject, which is what the
audit trail records instead of a claimed actor.

**Publication.** `forge portfolio share publish --target <path>` writes the
approved document locally (write-then-rename; identical bytes reconcile rather
than republish). `--adapter <path>` selects the optional credential-injected
executable, which receives exactly `{contract, operation_key,
manifest_revision, manifest_sha256, target, document}` on stdin — never a
registry query, a session or a credential — answers
`forge-portfolio-share-adapter/0.1.0`, and is killed at a bounded timeout
(`FORGE_PORTFOLIO_SHARE_TIMEOUT_SECS`, 1..=3600).

The pointer advances to `portfolio-interest-snapshots` — the next sibling
whose two declared prerequisites (share-publish, and the analytics adapter) are
now both archived.

## Verification evidence (portfolio-share-publish, 2026-09-29)

- `cargo fmt --all -- --check`: PASS for every touched file
  (`src/portfolio/mod.rs`, `src/portfolio/publication.rs`,
  `src/portfolio/share/{mod,validation,manifest,publish,audit_types}.rs`,
  `src/registry/mod.rs`, `src/registry/portfolio.rs`, `src/registry/share/{mod,audit}.rs`,
  `src/api/mod.rs`, `src/core/mod.rs`, `src/main.rs`,
  `tests/support/share.rs`,
  `tests/portfolio_share_{cli,api,cross}_contract.rs`). The pre-change
  baseline carries formatting drift in `src/gate/evidence.rs`,
  `src/publish/{fleet,jenkins}.rs`, `tests/gate_contract.rs`,
  `tests/gate_cross_surface.rs` and `tests/publish_queue_status_contract.rs`;
  `cargo fmt` touched them incidentally and those edits were reverted with
  `git checkout --`, so the drift is preserved exactly as the prior cycles
  left it.
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: identical to the stashed
  baseline — the same 10 pre-existing locations
  (`src/gate/evidence.rs`, `src/publish/fleet.rs`, `src/publish/mod.rs`).
  **Zero new clippy errors.** (An earlier `cargo fmt` pass also reformatted the
  six drifted files listed above; they were reverted before the final run.)
- `cargo test --workspace --all-targets -- --skip
  rust_scaffold_builds_and_tests_with_native_toolchain`: **75 result groups,
  1699 tests, 0 failed**; the change adds no new long-running native test.
  New supervised suites: 4 `src/portfolio::share` modules (24 tests —
  closed vocabularies; public URL scheme/host/port/path/query rules; secret
  shapes refused without echo; bounded control-free text; surface splitting
  and duplicate refusal; sorted normalization; the closed evidence block;
  deterministic ordering and hash across record order; a title change moving
  the hash; empty catalog valid; rejected/private/duplicate records omitted
  with findings; the document embedding hash and emission time; the manifest
  carrying no private key; bounded operation keys and actors; a local
  publisher that reconciles identical bytes and refuses an oversized
  manifest; a wedged adapter killed at its budget; a missing adapter command
  refused), 11 `src/registry::share` (additive idempotent migration; a
  hand-built pre-change registry migrates forward with its project *and*
  journal rows intact; an interrupted share batch rolls back and leaves
  neither a partial table nor a broken prior registry, with the next open
  retrying cleanly; unknown project changes no share state; a refused write
  persists a finding without the value; a refused edit stops serving the
  previous record; revisions bump; surfaces are replaced not accumulated;
  withdrawal; a project without a record is never listed; approval needs the
  exact current hash; an edit after approval needs a new one; an empty catalog
  is approvable; a second approval supersedes; one operation key reserves
  once and refuses a different manifest; a published attempt moves the
  approval and the records; a later publication supersedes the prior set; an
  unreconciled attempt blocks; only `unknown` reconciles; the audit answers
  what was public and by whom), 19 `tests/portfolio_share_cli_contract.rs`,
  8 `tests/portfolio_share_api_contract.rs` and 2
  `tests/portfolio_share_cross_surface.rs`.
- Live binary smoke (CLI), against a two-project local registry: `preview` on
  an untouched fleet renders `project count: 0`; a `--surface
  "Admin=https://alethefy.example.com/admin/settings"` write returns
  `error[portfolio-share-invalid] … surface url path segment \`admin\` is a
  private or administrative surface and cannot be published; no share state
  was changed` (exit 1) and `share show` then prints the persisted
  `share-write-refused` finding while still reporting the project as absent
  from the catalog; the accepted write reports `validated at revision 1 with 2
  public surface(s)`; approving a wrong hash is refused naming both hashes;
  publishing before approval is refused; editing after approval makes publish
  fail with `the share records changed after revision 1 was approved` and
  writes no file; re-approving and publishing succeeds; a retry under the same
  operation key reports `already present; no second publication`. The
  published artifact is exactly the contracted document —
  `schema_family public-portfolio-manifest`, `schema_version 1`,
  `manifest_sha256` matching the approved hash, one project with only
  `id/title/summary/category/source_url/visibility/showcase_status/featured/
  demo_url/surfaces`. `share audit` renders revision 2 `published` and
  revision 1 `superseded`, each with its hash, actor and project count, one
  publication row, and `unreconciled: none`.
- Live binary smoke (HTTP), `forge api serve --bind 127.0.0.1 --port 18933`
  with a minted admin session: every share route without a bearer is 401
  (including `GET /v1/share/audit?limit=0`, which is 401 rather than a
  400 disclosure); `/healthz` stays byte-identical; `/v1/projects` still
  demands a real session (401); `/ui` without a bearer is 401 and with one is
  200; `GET /v1/projects/alethefy/share` on an unshared project is
  `{"shared":false}` rather than a blank entry; an admin surface over HTTP is
  a 400 `portfolio-share-invalid` naming the rule; an accepted write returns
  `validated beta True 1`; preview → approve → publish records
  `actor release-bot` on both the approval and the publication; `GET
  /v1/share/audit` renders `unreconciled: null`; the private
  `/v1/projects/alethefy/portfolio` projection contains **zero** occurrences of
  the share source URL.
- Adapter evidence: `an_adapter_never_receives_a_credential_or_a_registry_query`
  captures the adapter's stdin and asserts the exact key set and that the
  handed-over document equals the approved canonical manifest;
  `an_adapter_answering_the_wrong_contract_is_refused` and
  `a_failing_adapter_does_not_create_a_false_success_and_retry_reconciles`
  (a wedged stub killed at its 1 s budget) prove an adapter failure records a
  `failed` attempt, keeps the approval and retries under the same key without a
  second row. All adapters are **local executable stubs**.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 50 passed, 0 failed
  (50 items) pre-archive and 50 passed, 0 failed post-archive with the
  promoted `portfolio-share` spec (+4 requirements); `git diff --check`: PASS.
- **Blocked, honestly recorded:** `platform-contracts` still carries
  `public-portfolio-manifest` as an unimplemented proposal — there is no
  `schemas/public-portfolio-manifest.schema.json` on disk — so the
  cross-repo schema-fixture validation is **blocked, not passed**. Forge pins
  the family name and major version from that proposal's design and enforces
  the field set itself. No credential-injected GitHub Pages adapter was run,
  so **no external publication is claimed**; no GitHub, Cloudflare or other
  external host was contacted.
- Pointer state: `portfolio-share-publish` archived (`16/16` tasks evidenced,
  `4/4` artifacts complete). `openspec list` shows the three remaining
  proposals (`portfolio-interest-snapshots`,
  `standard-pack-registry-and-snapshots`, `fleet-live-rollout`); the pointer
  advances to `portfolio-interest-snapshots`, whose declared prerequisites
  (share-publish, analytics adapter) are now both archived.
- No shared Gate Runtime is configured; no Gate pass is claimed. No
  PostgreSQL, multi-user, SSO or remote-synchronization readiness is claimed;
  every publication target in the evidence above is a local file or a local
  executable stub.

## Current state

`portfolio-metadata-and-review` implemented, verified and archived on
2026-09-29 as `2026-09-28-portfolio-metadata-and-review`; its four
requirements (store horizontal portfolio metadata, enforce relation
integrity, import source-owned evidence as snapshots, project portfolio
data through the portal) were promoted into
[openspec/specs/portfolio-metadata-and-review/spec.md](openspec/specs/portfolio-metadata-and-review/spec.md).
The implementation closes every Section-1 BFS, Section-2 DFS,
Section-3 BFS and Section-4 verification task in the proposal.

**Persistence.** Eight additive tables (`portfolio_projects`,
`portfolio_tags`, `portfolio_project_tags`, `portfolio_relations`,
`portfolio_goals`, `portfolio_goal_projects`, `portfolio_reviews`,
`portfolio_evidence_snapshots`) live in `src/registry/portfolio.rs` and
are applied by `apply_portfolio_migration` inside one explicit
`BEGIN IMMEDIATE` … `COMMIT` batch with `ROLLBACK` on any failure.
`projects` and `operations` are untouched: the registry gains **no
column** and no row type is shared, so a registry written before this
package migrates forward with its project and journal rows intact and a
failed create leaves no partial portfolio table.

**Domain** (`src/portfolio/mod.rs`). Lifecycle
(`incubating|building|validating|operational|paused|archived`),
confidence (`unknown|low|medium|high`), relation types
(`depends-on|duplicate-of|shares-domain-with|replaces|consumes|optional-provider`)
and evidence states (`observed|stale|unavailable|invalid|not-run`) are
closed enums. Tag names are bounded lowercase kebab; notes, goals and
source labels are bounded and control-free; evidence payloads must be a
JSON **object**, are bounded to 64 KiB, and pass through
`policy::redact_credentials` before they are stored. Expiry can only
*downgrade* `observed` to `stale`: an `unavailable`, `invalid` or
`not-run` source stays exactly as reported and never reads as a pass.
An absent user-owned field stays `None` — an unclassified project is
never reported as `incubating`.

**CLI.** `forge portfolio tag add|remove|list`,
`relation add|remove|list`, `review set|list`, `goal add|link|list`,
`evidence import|list`, `show <project>`. Every mutation is
project-scoped, validated before the write, and idempotent where an
identity is provided.

**JSON API.** `GET /v1/projects/{id}/portfolio` (read) plus
admin-gated `POST …/portfolio/{tags,relations,reviews,evidence}` behind
the existing `authorize()`; a new typed `portfolio-invalid` maps to
400. A session minted for project A presented to project B is refused
with `api-project-mismatch` before any write.

**Portal.** `GET /ui` gains a tag/lifecycle/confidence filter form and
three portfolio columns; an out-of-vocabulary filter is a typed 400
rather than a silently widened list. `GET /ui/projects/{id}` gains the
portfolio projection (classification, blocker, next action, tags,
goals, relations in both directions, source-attributed evidence table,
review history) and a write form behind
`POST /ui/projects/{id}/portfolio`, which repeats the bearer /
`Origin` / form-token checks the republish POST already established.
Imported evidence has **no** browser write path — source-owned
snapshots are append-only — and no UI surface edits a repository file
or a provider record.

The pointer advances to `portfolio-share-publish` — the next sibling
whose proposal declares a dependency on portfolio metadata (its other
prerequisites, the manifest contract and the analytics adapter, are
separate packages) — so the operator's next cycle has the right
`current_spec`.

## Verification evidence (portfolio-metadata-and-review, 2026-09-29)

- `cargo fmt --all -- --check`: PASS for every touched file
  (`src/portfolio/mod.rs`, `src/registry/portfolio.rs`,
  `src/registry/mod.rs`, `src/core/mod.rs`, `src/lib.rs`,
  `src/main.rs`, `src/api/mod.rs`, `src/api/ui/{data,render,routes}.rs`,
  `tests/portfolio_contract.rs`, `tests/portfolio_ui_contract.rs`).
  The pre-change baseline carries formatting drift in
  `src/gate/evidence.rs`, `src/publish/{fleet,jenkins}.rs`,
  `tests/gate_contract.rs`, `tests/gate_cross_surface.rs` and
  `tests/publish_queue_status_contract.rs`; `cargo fmt` touched them
  incidentally and those edits were reverted, so the drift is
  preserved exactly as the prior cycles left it.
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: identical to the
  stashed baseline — the same 10 pre-existing locations
  (`src/api/ui/auth.rs:166-167`, `src/gate/evidence.rs`,
  `src/publish/fleet.rs`, `src/publish/mod.rs`), verified by
  `git stash push -u -- src tests` and re-running. **Zero new clippy
  errors.**
- `cargo test --workspace --all-targets -- --skip
  rust_scaffold_builds_and_tests_with_native_toolchain`: 72 result
  groups, 1625 tests, 0 failed (three consecutive clean runs; the
  change adds no new long-running native test). New supervised suites:
  17 `src/portfolio` unit tests (closed lifecycle/confidence/relation/
  evidence vocabularies; no relation type permits a self link; bounded
  tag/note/colour/source validation; evidence object-bound +
  redaction + size cap; expiry downgrades `observed` and never
  upgrades an absence; filter matching requires a declared value;
  `parse_filter` refuses unknown keys and out-of-vocabulary values),
  20 `src/registry::portfolio` unit tests (additive idempotent
  migration; a hand-built pre-change registry migrates forward with
  its rows intact; `an_interrupted_migration_rolls_back_and_keeps_the_prior_registry_usable`
  fails the batch mid-way through a decoy table and proves no partial
  table survives and the next open retries cleanly; unknown project
  changes no portfolio state; duplicate tag leaves one link;
  dependency relation idempotent and visible in both directions; self
  relation and unknown-target relation refused; append-only snapshots
  with redaction; malformed/non-object payload, bad timestamp and an
  inverted freshness bound refused; expired bound reads `stale` while
  the stored status is preserved; unavailable never reads healthy;
  goals unique and linkable; fleet filter combines user and tag
  fields), 23 `tests/portfolio_contract.rs` CLI/API tests and 20
  `tests/portfolio_ui_contract.rs` browser tests.
- Known flake, honestly recorded: on one of four full-suite runs
  `tests/governance_contract.rs::workspace_governance_evidence_is_redacted_and_bounded`
  failed under parallel load (it shells out to a fixture adapter with a
  10 s budget). It passes in isolation and in the three subsequent full
  runs; the governance path is untouched by this change.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
  **No new dependency** was added — `maud` arrived with the prior
  `portal-web-ui` cycle; the dependency closure is unchanged.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 50 passed,
  0 failed (50 items) pre-archive and 50 passed, 0 failed post-archive
  with the promoted `portfolio-metadata-and-review` spec (+4
  requirements); `git diff --check`: PASS.
- Live binary smoke (CLI): `forge portfolio tag add`,
  `review set --confidence/--lifecycle/--next-action/--blocker`,
  `relation add --to --type --note`, `goal link`, three
  `evidence import` calls (observed / expired-observed / unavailable),
  then `portfolio show alethefy` renders lifecycle `building`,
  confidence `high`, blocker, next action, the `outgoing depends-on`
  relation, three goals/relations rows, and `governance stale` /
  `runtime unavailable` beside `driftwatchdog observed` — every row
  carrying its own source system and source revision. Refusals:
  `error[unknown-project]` (unknown target, exit 1),
  `error[portfolio-invalid]` (self relation, malformed payload, exit 1)
  each ending "no portfolio state was changed".
- Live binary smoke (HTTP): `forge api serve --bind 127.0.0.1 --port
  18911` then `curl`. `/ui` without a bearer is 401 HTML;
  `/ui?tag=platform&lifecycle=building` renders only
  `/ui/projects/alethefy` and echoes `Filtered by
  tag=platform&amp;lifecycle=building.`;
  `/ui?lifecycle=shipped` is a 400 `portfolio-invalid` page naming the
  vocabulary; `/ui/projects/alethefy` renders `driftwatchdog`, `a1b2c3`,
  `governance`, `stale`, `unavailable`, `platform`, `building`,
  `depends-on`, `Edit portfolio metadata` and the "Forge stores what
  the source reported; it does not run the check" attribution note.
  Browser writes: cross-origin POST is 403 `ui-origin-mismatch`, no
  bearer is 401 `api-unauthorized`, an authorized same-origin POST is
  200 `Saved portfolio metadata` / `tagged with tooling` /
  `confidence medium` and the tag, confidence and lifecycle are
  confirmed through the CLI afterwards. `/healthz` stays byte-identical
  JSON; `/v1/projects` still demands a real session (401);
  `Accept: application/json` on `/ui` still returns the unchanged
  `api-routing` envelope.
- Pointer state: `portfolio-metadata-and-review` archived
  (`11/11` tasks evidenced, `4/4` artifacts complete). `openspec list`
  shows the four remaining proposals (`portfolio-share-publish`,
  `portfolio-interest-snapshots`, `standard-pack-registry-and-snapshots`,
  `fleet-live-rollout`); the pointer advances to
  `portfolio-share-publish`, whose declared prerequisite (portfolio
  metadata) is now archived. `portfolio-interest-snapshots` stays
  blocked behind `portfolio-share-publish`.
- No shared Gate Runtime is configured; no Gate pass is claimed. No
  PostgreSQL, multi-user, SSO or remote-synchronization readiness is
  claimed; provider evidence here is fixture-only and no external
  provider was contacted.

## Current state

`portal-web-ui` implemented, verified and archived on
2026-09-29 as `2026-09-28-portal-web-ui`; its four
requirements (browser project list with latest journal
publish state, project detail with evidence, confirm-gated
republish from the browser, dependency closure explicitly
registered) were promoted into
[openspec/specs/portal-web-ui/spec.md](openspec/specs/portal-web-ui/spec.md).
The implementation closes every Section-2 DFS and
Section-3 BFS task in the proposal: `forge api serve` now
serves `GET /ui` (fleet project list with per-project
journal `publish` state), `GET /ui/projects/{id}` (project
detail with manifest, maturity, doctor summary and
journal rows), and `POST /ui/projects/{id}/publish`
(confirm-gated republish that re-passes Core gates).
Renderer: `maud 0.27` — JSX-shaped compile-time HTML, the
closest in-Rust equivalent to the deferred graphical
portal framework (ASP.NET Core / Next.js) — added with
exactly three new transitive crates (`maud` + `maud_macros`
+ `itoa`), all MIT/Apache-2.0 (already allow-listed) and
registered in `deny.toml` with the rationale. No new
persistence, no new ports, no new auth surface. The
existing JSON API envelopes stay byte-identical
(`Accept: application/json` returns the unchanged
envelope); the CLI / MCP / portal CLI outputs are
untouched. The pointer advances to
`portfolio-metadata-and-review` — the next sibling UI
package whose proposal declares a dependency on
`portal-web-ui` (plus `forge-independent-project-inventory-fleet`
and `forge-publish-plugin-orchestration`, both already
archived) — so the operator's next cycle has the right
`current_spec`.

## Verification evidence (portal-web-ui, 2026-09-29)

- `cargo fmt --check` (touched files): PASS for
  `src/api/ui/{mod,render,auth,data,routes}.rs`,
  `src/api/mod.rs`, `tests/portal_ui_contract.rs`.
  Pre-existing formatting drift in `src/gate/evidence.rs`,
  `src/publish/{fleet,jenkins}.rs`, `tests/gate_*`,
  `tests/publish_queue_status_contract.rs` was preserved
  per AGENTS.md (those files were not touched by this
  change).
- `cargo build`: PASS.
- `cargo clippy --lib -- -D warnings`: 4 pre-existing
  errors in `src/gate/evidence.rs`, `src/publish/fleet.rs`,
  `src/publish/mod.rs` — none touched by this change.
  Zero new clippy errors introduced.
- `cargo test --workspace --all-targets -- --skip
  rust_scaffold_builds_and_tests_with_native_toolchain`:
  full suite runs to completion; the new suites pass:
  18 `api::ui::{auth,routes}::tests` unit tests + 12
  `tests/portal_ui_contract.rs` HTTP contract tests
  (auth refused, empty registry, 200 with bearer, 404
  unknown id, plan preview without confirm, confirmed
  republish journals `publish.ui` row, cross-origin POST
  refused, form token mismatch refused, escape matrix on
  adversarial ids, JSON content-type falls through).
- `cargo deny check`: advisories ok, bans ok, licenses ok,
  sources ok. The three new crates (`maud`, `maud_macros`,
  `itoa`) are all MIT/Apache-2.0; allow-list covered by the
  pre-existing entries plus the rationale comment in
  `deny.toml`.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 50
  passed, 0 failed (50 items). `git diff --check`: PASS.
- Live binary smoke: `forge api serve --bind 127.0.0.1
  --port 18765` followed by `curl` against `/healthz`
  (200 JSON, unchanged), `/ui` without bearer (401 HTML),
  `/ui` with `Authorization: Bearer …` (200 HTML, 1437
  bytes), `/ui/projects/no-such-app` (404 HTML),
  `/ui/projects/<script>` (400 — id rejected before render),
  `/ui/projects/alethefy/publish` without `confirm=yes`
  (200 plan page). `/v1/operations/{id}` and
  `/v1/projects` JSON envelopes unchanged (still demand
  bearer tokens; content-type still `application/json`).
- Pointer state: `portal-web-ui` is the only active change
  for which implementation evidence is recorded; the
  `current_spec` line is set to `portal-web-ui` and points
  at the work in progress. The change is **not yet
  archived**; archive + commit + HANDOFF advance are the
  next step.
- No shared Gate Runtime is configured; no Gate pass is
  claimed.

## Refinement history (2026-09-28 → 2026-09-29)

The proposal's "no new dependency, framework-free HTML" line
was refined after the operator pushed for a real web
framework (hand-rolled typed `Element` trees were tried
first and rejected). The refined decision: maud (compile-
time JSX-shaped HTML, MIT/Apache-2.0, zero runtime
reflection, inline templates). The refinement is documented
in `openspec/changes/portal-web-ui/{proposal,design,tasks,
specs/portal-web-ui/spec}.md` and reflected in this HANDOFF
section.

## Current cycle: portal-web-ui (2026-09-28 spec refinement)

The `portal-web-ui` proposal/design/tasks/spec were refined
in place before implementation to reflect three
operator-confirmed decisions:

1. **Renderer choice: `maud` (compile-time HTML, JSX-shaped).
   Replaces the typed hand-rolled `Element` tree that an
   earlier draft attempted.** The closure adds exactly three
   crates (`maud` + `maud_macros` + `itoa`), all
   MIT/Apache-2.0 (already allow-listed), each registered in
   `deny.toml` with its reason. Askama was considered and
   rejected (separate template files, heavier dep tree, less
   JSX-shaped); a hand-rolled typed renderer was attempted
   first and rejected for lack of layout/component reuse.
2. **v0 scope reduction: per-row live liveness probing in the
   list page is out of scope.** The original proposal asked
   the UI to SSH-probe the Mac per render; that requires
   target access the loopback UI process does not have and
   duplicates the `fleet-liveness-status` package's surface.
   The list page consumes the journal `publish` row state
   instead (registry-only, no SSH). Live liveness stays
   reachable through `forge fleet online` and the API JSON.
   `portfolio-metadata-and-review` and other later UI
   packages can extend the list view against the same
   template if the target-access story changes.
3. **Auth/CSRF: bearer-token re-check + `Origin` header
   check, no cookie session.** The existing API auth is
   bearer-token based, so cookie-CSRF is not in scope. The
   POST re-checks the token against the form's hidden token
   and refuses cross-origin POSTs whose `Origin` does not
   match the loopback bind address. Equivalent protection to
   the original proposal's same-session form token, with
   zero new auth surface.

`node scripts/check-openspec-change-names.mjs`: PASS.
`openspec validate --all --strict --no-interactive`: 50
passed, 0 failed after the refinement.

## Current state

`fleet-liveness-status` implemented, verified and archived on 2026-09-28 as
`2026-09-28-fleet-liveness-status`; its three requirements (read-only
fleet online verdicts, served router rules as route ground truth,
application answers count as online) were promoted into
[openspec/specs/fleet-liveness-status/spec.md](openspec/specs/fleet-liveness-status/spec.md).
The implementation closes every Section-3 BFS and Section-4 verification
task in the proposal: `forge fleet online` joins per `compose_ready`
roster entry the target container state (read through the existing SSH
transport with a `FORGE_PUBLISH_SSH_TARGET` override that also lets a
test stub take the probe directly), the served `platform/Caddyfile`
hosts parsed on the controller (nav host and `:80` fallback excluded),
and one bounded HTTPS probe (bounded `curl -s -m N --max-filesize 65536`
— no new dependency, `cargo deny` closure unchanged) into a typed
verdict of `ONLINE` / `DOWN` / `NO-ROUTE` / `NOT-DEPLOYED` /
`UNAVAILABLE` / `SKIPPED` and a typed `forge-fleet-liveness/0.1.0`
JSON document plus a human table. Every captured string passes
`policy::redact_credentials`; the router's unknown-hostname fallback
body and 502/503/504 never read `ONLINE`; non-ready entries are
reported with their own classification and never probed; the command
exits non-zero unless every probed host is `ONLINE` and writes no
journal row, no registry row, no target file. Roster flags mirror
`forge publish fleet` exactly (`--inventory` beats `--fleet-registry`,
legacy compatibility adapter is preserved, every declared entry
classified once and reported — `compose_ready` is the only probed
class). `--timeout-secs` is bounded `1..=120`; `--dry-run` renders the
plan and never contacts the target or any origin. The MCP `tools/list`,
portal, and `forge list` surfaces stay unchanged: no new tool, no new
route, no new portal section, no new journal kind beyond the existing
`publish` rows; the registry's `operations` table gains no columns.

`openspec list` shows four remaining proposals
(`fleet-live-rollout`, `portal-web-ui`, plus the `portfolio-*` and
`standard-pack-registry-and-snapshots` candidates). The pointer
advances to `fleet-live-rollout` — the next sibling package whose
proposal declares a dependency on the just-archived liveness report —
so the operator's next cycle has the right `current_spec`.

## Verification evidence (fleet-liveness-status, 2026-09-28)

- `cargo fmt --all -- --check`: PASS for the touched files
  (`src/main.rs`, `src/fleet/mod.rs`, `src/fleet/online.rs`,
  `tests/fleet_online_contract.rs`); the pre-change baseline
  carries formatting drift in `src/gate/evidence.rs`,
  `src/publish/fleet.rs`, `src/publish/jenkins.rs`,
  `tests/gate_contract.rs`, `tests/gate_cross_surface.rs`,
  `tests/publish_queue_status_contract.rs` — none touched by this
  change (verified by `git checkout --` on those files and re-running
  `cargo fmt --check`).
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS for the touched
  files. The seven pre-existing `-D warnings` errors live in
  `src/gate/evidence.rs`, `src/publish/mod.rs`, `src/publish/fleet.rs`,
  and the gate/publish test suites — none touched by this change.
- `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS — 69 result groups, 0 failed; the change adds no new
  long-running native test. New supervised suites: 22
  `src/fleet/online` unit tests (verdict matrix:
  `NO-ROUTE`/`NOT-DEPLOYED`/`DOWN` on gateway errors, `DOWN` on
  connection failure, `DOWN` on router fallback body,
  `ONLINE` on application 404, `UNAVAILABLE` on probe error,
  redaction of secret-shaped probe error detail;
  Caddyfile host parser: nav/fallback exclusion, empty-file
  handling, exact host match; `match_container` prefix match,
  bounded length, secret redaction; SSH argv shape matches the
  publish adapter for both the real `ssh <target>` and the
  local-path stub override; `summary` counts every state plus
  the `SKIPPED` non-probed bucket; non-`compose_ready` entries
  bypass the classifier so their classification surfaces
  verbatim) plus 14 `tests/fleet_online_contract.rs` CLI tests
  (help advertises every roster flag including `--inventory`,
  `--fleet-registry`, `--workspace-root`, `--domain`,
  `--timeout-secs`, `--dry-run`; `--dry-run` renders the plan
  without contacting the target; `--timeout-secs 0` refused
  with a typed out-of-bounds; empty roster refuses with
  `publish-invalid`; `NO-ROUTE` when the served Caddyfile has
  no rule; `NOT-DEPLOYED` when the served Caddyfile has a rule
  but `docker ps` is empty; `SKIPPED` for `compose_missing`
  and `source_unavailable` entries; `SKIPPED` for
  `source_unavailable` keeps the entry in the report and exits
  non-zero; secret-shaped bytes in `docker ps` are redacted in
  stdout and stderr; the served Caddyfile without a rule for a
  compose-ready entry yields `NO-ROUTE` and the HTTP probe is
  skipped; an entry with a routed Caddyfile and a running
  container probes `ONLINE`; the read-only guarantee — journal
  row count and registry bytes are identical before and after
  the run, and the fake ssh stub is not mutated).
- Live binary smoke: `forge fleet online --help` advertises
  `--inventory`/`--fleet-registry`/`--workspace-root`/`--domain`/
  `--timeout-secs`/`--dry-run`; `--dry-run` against an empty
  inventory prints the plan and exits 0; a real probe against
  `ssh mac` on the operator's Mac returns
  `error[publish-invalid]: probe \`ssh mac cat
  /srv/platform/Caddyfile\` failed: cat: ...: No such file or
  directory` and exits 1 (the Mac has no fleet yet — the typed
  refusal names the missing path verbatim and the operator's
  follow-up is the real rollout, not this change). The
  `inventory show` / `publish fleet` paths and every
  `forge fleet list|status|inspect` path stay byte-identical
  (the existing `tests/inventory_contract.rs` and
  `tests/fleet_contract.rs` continue to pass without edits).
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 50 passed,
  0 failed pre-archive and 50 passed, 0 failed post-archive with
  the promoted `fleet-liveness-status` spec (+3 requirements);
  `git diff --check`: PASS.
- Pointer state: `fleet-liveness-status` archived (`15/15` tasks
  evidenced). `openspec list` shows the four remaining proposals
  above; the pointer advances to `fleet-live-rollout`.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`sibling-cwd-publish` implemented, verified and archived on 2026-09-28 as
`2026-09-28-sibling-cwd-publish`; its six requirements (cwd-discovered
single publish, shared provider engine, explicit flag precedence,
revision discipline, typed failures without silent fallback, plugin
boundary preservation) were promoted into
[openspec/specs/sibling-cwd-publish/spec.md](openspec/specs/sibling-cwd-publish/spec.md).
The implementation closes every Section-2 DFS and Section-3 BFS task in
the proposal except the two optional verification tasks (native
scaffold build and live jenkins-local round trip): `forge publish` with
no `--project`/`--folder` now discovers the project from cwd
(`.project.json:id` > `forge.yaml:project.id` > directory basename),
validates the kebab/snake id, captures `HEAD` as a 40-hex revision
through the same `validate_revision` gate, and delegates to
`publish_via_provider_dir` — the exact provider path as `--folder`
(typed `forge-publish-provider/0.1.0` contract, bounded `1800s`
invocation, secret redaction, additive `revision`/`build_status`/
`run_status`/`container_identity`, `operations` `publish` journal row
with `queue_id=None`, visible via `forge deploy status --project <id>`).
`--cwd <PATH>` overrides the process cwd for scripting; `--project` and
`--folder` bypass discovery entirely; malformed `.project.json`,
invalid basename, non-hex revision and unknown/disabled provider all
return typed `publish-invalid` with empty stdout and zero provider
spawn; no parent-directory walk or sibling-directory scan is performed.
Fleet mode (`forge publish fleet --inventory <path>`) and `forge
inventory show [SOURCE]` are unchanged — every inventory entry still
classifies `compose_ready`/`compose_missing`/`invalid`/
`source_unavailable` and only `compose_ready` invokes a provider.
`jenkins-local` remains a Forge-side plugin configured via
`FORGE_PUBLISH_PROVIDER` / `.forge/providers.yaml`; siblings never call
`project-action.sh` directly. `forge publish --help` now advertises the
bare invocation and `--cwd`. No new registry columns or tables.

`forge-independent-project-inventory-fleet` implemented, verified and
archived on 2026-09-28 as
`2026-09-28-forge-independent-project-inventory-fleet`; its five
requirements (portable inventory source, complete inventory reporting,
container fleet publishing, public port routing, standalone ownership)
were promoted into
[openspec/specs/forge-independent-project-inventory-fleet/spec.md](openspec/specs/forge-independent-project-inventory-fleet/spec.md).
The implementation closes every Section-3 BFS and Section-4
verification task in the proposal: `forge inventory show [SOURCE]` and
`forge publish fleet --inventory <path>` consume the
`forge-project-inventory/0.1.0` contract; a local file is distinguished
from an external adapter by file extension (`.json` → local, otherwise
a `PATH`-discoverable executable); the external adapter is bounded by
`INVENTORY_ADAPTER_TIMEOUT_SECS = 300`, a `MAX_INVENTORY_BYTES` cap,
and the shared `policy::redact_credentials` pass, so a hostile adapter
cannot pin Forge. Every declared entry receives exactly one explicit
classification — `compose_ready`, `compose_missing`, `invalid`, or
`source_unavailable` — from `InventorySnapshot::classify` and is
surfaced in the fleet report under the `entries` and
`skipped_entries` arrays; nothing is silently omitted. Only
`compose_ready` entries invoke a provider, and the report carries the
`inventory_subdomain` (`<project>.<domain>`) only when
`runtime == web` AND `public_http == true`. Non-web runtimes, database
ports, Redis ports, Jenkins ports, and private worker ports never
receive a Cloudflare / Caddy public route — `InventoryEntry::subdomain`
returns `None` for any other shape, and contract validation refuses
`public_http = true` for non-web runtimes at parse time. The legacy
`--fleet-registry` flag and `$FORGE_WORKSPACE_REGISTRY` env stay as a
compatibility adapter (`legacy_inventory_snapshot`) so the
seven-project handoff keeps working without a sibling checkout during
migration; the workspace-governance relocation does not require
Forge code or fixed paths to change — the only shared helper reads
the operator-declared registry path verbatim. The MCP `tools/list`,
portal, and `forge list` surfaces stay unchanged: no new tool, no new
route, no new portal section, no new journal kind beyond the existing
`publish` rows; the operations journal gains no new columns — the
inventory classification lives in the CLI report, not in SQLite.

`openspec list` reports no active changes; the `current_spec` pointer
is removed. All 29 baseline and audit changes plus the ten
sibling-integration packages (orders 30 through 39) are archived and
promoted to their canonical specs.

## Verification evidence (forge-independent-project-inventory-fleet, 2026-09-28)

- `cargo fmt --all -- --check`: PASS for the touched files
  (`src/publish/inventory.rs`, `src/publish/mod.rs`, `src/main.rs`,
  `tests/inventory_contract.rs`); the pre-change baseline carries
  formatting drift in unrelated files, which is out of scope
  (verified by stashing the patch and re-running).
- `cargo build`: PASS.
- `cargo clippy --all-targets`: PASS for the touched files; the
  pre-change baseline carries six `-D warnings` errors in
  `src/gate/evidence.rs`, `src/main.rs`, `src/publish/mod.rs`,
  `src/publish/fleet.rs`, `src/publish/jenkins.rs`,
  `tests/gate_contract.rs`, `tests/gate_cross_surface.rs`,
  `tests/publish_queue_status_contract.rs`, `tests/publish_contract.rs`
  — none touched by this change (verified by stashing the patch and
  re-running; 9 errors on the baseline vs. 0 new errors introduced).
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS — full suite runs to completion with no FAILED entries;
  the change adds no new long-running native test. New supervised
  suites: 20 `src/publish/inventory` unit tests (`runtime_class_parses_known_values`,
  `validates_minimal_document`, `refuses_wrong_contract`, `rejects_missing_required_fields`,
  `rejects_non_hex_revision`, `rejects_unknown_runtime`,
  `rejects_public_http_without_runtime_web`,
  `rejects_public_port_without_public_http`, `rejects_duplicate_ids`,
  `malformed_entries_named_with_reason`, `classify_marks_compose_ready_when_file_present`,
  `classify_marks_compose_missing_when_no_compose_field`,
  `classify_marks_compose_missing_when_file_absent`,
  `classify_marks_source_unavailable_when_path_missing`,
  `classify_never_routes_non_web_runtime`,
  `classify_sorts_by_id_for_deterministic_fleet_sequence`,
  `load_local_round_trips_minimal_document`,
  `load_local_refuses_oversized_file`, `load_local_refuses_missing_file`,
  `invoke_external_refuses_missing_executable`) plus 9
  `tests/inventory_contract.rs` CLI tests (help advertises
  `SOURCE`/`--domain`; missing source refused with `publish-invalid`;
  every entry reported with an explicit classification;
  `compose_ready` for the staged alethefy + worker, `source_unavailable`
  for the un-staged forge + worker source paths; non-web runtime
  never receives a subdomain; wrong contract refused; malformed
  revision surfaces as `invalid` with a reason; external adapter
  executable consumed through `invoke_external`; `publish fleet
  --inventory` advertises the flag and refuses with zero
  `compose_ready`).
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 41 passed,
  0 failed pre-archive and 41 passed, 0 failed post-archive with the
  promoted `forge-independent-project-inventory-fleet` spec (+5
  requirements); `git diff --check`: PASS.
- Pointer state: `forge-independent-project-inventory-fleet` archived
  (`15/15` tasks evidenced). `openspec list` reports no remaining
  active changes; the `current_spec` pointer is removed.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Mac canary remains sibling-owned: the Mac-side Docker, port-registry,
  Caddy renderer, and Cloudflare tunnel refresh live in
  `jenkins-local` and were not modified here; Forge only computes the
  `inventory_subdomain` projection that the jenkins-local Caddy
  renderer consumes. The full Mac end-to-end (77 projects published
  through one wildcard tunnel) is a sibling-owned follow-up.

## Verification evidence (sibling-cwd-publish, 2026-09-28)

- `cargo fmt --check`: PASS for the touched files (`src/main.rs`,
  `tests/sibling_cwd_publish_contract.rs`); the pre-change baseline
  carries formatting drift in unrelated files, which is out of scope.
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: baseline has 4 pre-existing
  `-D warnings` errors (`src/gate/evidence.rs`, `src/publish/mod.rs`
  shape issues) — none touched by this change (verified by stashing
  the patch and re-running).
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS — parallel run has 1 flaky API/healthz race (`Connection refused`);
  all suites green in isolation. New supervised suite: 12
  `tests/sibling_cwd_publish_contract.rs` CLI tests
  (`publish_help_advertises_cwd_and_bare_publish`,
  `cwd_publish_discovers_project_json`,
  `cwd_publish_falls_back_to_forge_yaml`,
  `cwd_publish_falls_back_to_directory_basename`,
  `cwd_explicit_flag_overrides_process_cwd`,
  `cwd_folder_flag_beats_cwd_discovery`,
  `malformed_project_json_is_typed_failure`,
  `invalid_basename_cwd_is_typed_failure`,
  `cwd_non_hex_revision_is_refused_before_provider`,
  `cwd_disabled_provider_is_refused`,
  `cwd_publish_persists_and_visible_via_deploy_status`,
  `project_json_takes_precedence_over_forge_yaml`) plus
  12 `src/publish/inventory` + 9 `tests/inventory_contract.rs`
  still green.
- Live sibling smoke: `forge publish --help` advertises `--cwd`;
  bare `forge publish --dry-run --provider jenkins` from
  `alethefy` (or `forge` itself) discovers `project_id=alethefy`
  and `revision` 40-hex without a provider spawn; explicit
  `--folder` still beats cwd.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 41 passed,
  0 failed pre-archive and 42 passed, 0 failed post-archive with the
  promoted `sibling-cwd-publish` spec (+6 requirements);
  `git diff --check`: PASS.
- Pointer state: `sibling-cwd-publish` archived (`17/19` tasks
  evidenced; 4.2 native scaffold build and 4.4 live jenkins-local
  round trip are optional follow-ups when toolchains/siblings are
  available). `openspec list` reports no remaining active changes;
  the `current_spec` pointer is removed.
- No shared Gate Runtime is configured; no Gate pass is claimed.

verified and archived on 2026-09-28 as
`2026-09-28-forge-publish-observability-revision-containers`; its
three requirements (phase-visible publish lifecycle, revision-bound
container identity, phase-bounded progress events) were promoted
into
[openspec/specs/forge-publish-observability-revision-containers/spec.md](openspec/specs/forge-publish-observability-revision-containers/spec.md).
The implementation closes every Section-3 BFS and Section-4
verification task in the proposal: the provider contract now
requires an exactly 40-character hexadecimal Git revision (the
empty/`unknown` fallback that shipped in
`forge-publish-plugin-orchestration` is refused at parse time with
`RevisionShape` and a clear operator-facing message), every
`PublishProviderResponse` carries additive
`revision`/`build_status`/`run_status`/`container_identity` fields
validated against the bounded
`succeeded`/`failed`/`not_started`/`unknown` vocabulary and a
`forge-<project>-<sha12>` Compose identity, and the
`publish.progress` event classifier enforces the
`phase=build|run|complete` vocabulary — legacy phase names from
earlier sibling providers (`preflight`, `transfer`, `verify`,
`build-and-run`, `routing`, `completed`) are refused as
`Malformed` and surfaced to the operator instead of silently
accepted. The operations journal gains four additive columns
(`revision`, `build_status`, `run_status`,
`container_identity`) via `apply_migrations`, idempotent against
pre-change registries; every SELECT shares one
`row_to_operation_entry` helper so the new projection fields
flow through every query (`journal_entries`,
`recent_operations`, `operations_for_project`,
`operations_for_queue`, `operation_by_idempotency`, `operation`).
`cmd_publish_provider` validates the revision before invoking
the provider, persists the additive phase evidence on every
terminal response, and synthesizes the canonical container
identity from the recorded revision when the legacy provider
omits it. `cmd_publish_fleet` threads the same phase evidence
through the fleet summary and the per-project journal rows
through the new `record_queue_publish_phase` writer (a
`PublishPhaseEvidence<'a>` builder keeps the function signature
under the clippy 7-arg limit). `forge deploy status` JSON and
human output now expose every additive field per entry, the
`filter_publish_deploy` predicate also matches `publish.github`
rows so the API path is visible through the same projection, and
the GitHub push path (`handle_github_push` in `src/api/mod.rs`)
calls `update_operation_phase` to stamp the additive fields onto
the reserved row before responding with the same fields in the
202 envelope. The sibling provider
(`jenkins-local/adapters/forge-publish-provider.py`) is updated
in lock-step: a new `compose_project_name(project, revision)`
helper mirrors the Forge side, the Compose project identity now
travels as `forge-<project>-<sha12>` (replacing the prior
`jenkins-<project>` literal), the terminal `response(...)`
carries `revision`/`build_status`/`run_status`/`container_identity`
additively, `execute_publish` tracks `build_status` and
`run_status` state and emits progress events with the new
`phase=build|run|complete` taxonomy, and the live `capabilities`
and `preflight` round-trips through the sibling preserve the new
fields end-to-end. The MCP `tools/list`, portal and `forge list`
surfaces stay unchanged: no new tool, no new route, no new
portal section, no new journal kind beyond the existing
`publish` and `publish.github` rows; the registry's `operations`
table gains four additive columns and one new writer pair, no
columns removed.

## Verification evidence (forge-publish-observability-revision-containers, 2026-09-28)

- `cargo fmt --all -- --check`: PASS for the touched files
  (`src/publish/providers.rs`, `src/registry/mod.rs`, `src/main.rs`,
  `src/api/mod.rs`, `tests/publish_contract.rs`,
  `tests/publish_observability_contract.rs`); the pre-change
  baseline carries formatting drift in `src/gate/evidence.rs`,
  `src/publish/fleet.rs`, `src/publish/jenkins.rs`,
  `src/publish/mod.rs`, `tests/gate_contract.rs`,
  `tests/gate_cross_surface.rs`, `tests/publish_queue_status_contract.rs`
  — none touched by this change (verified by stashing the
  diff and re-running).
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS against the
  pre-change baseline (the seven pre-existing `-D warnings`
  errors live in `src/gate/evidence.rs`, `src/main.rs`,
  `src/publish/mod.rs`, `src/publish/fleet.rs`,
  `src/publish/jenkins.rs`, `tests/gate_contract.rs`,
  `tests/gate_cross_surface.rs` — none touched by this change;
  verified by stashing the patch and re-running).
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS — full suite runs to completion with no FAILED entries;
  the change adds no new long-running native test. New
  supervised suites: 13 `src/publish/providers` unit tests
  (`validate_revision_accepts_full_hex_sha`,
  `validate_revision_rejects_short_long_non_hex_blank`,
  `parse_request_rejects_short_and_non_hex_revision`,
  `compose_project_name_uses_first_twelve_hex_chars`,
  `response_phase_fields_validate_vocabulary`,
  `response_rejects_malformed_build_run_status`,
  `response_rejects_overlong_or_empty_container_identity`,
  `response_rejects_malformed_echo_revision`,
  `classify_progress_rejects_legacy_phase_names`,
  `classify_progress_rejects_unknown_status`,
  plus three existing tests re-pinned to the new phase
  vocabulary); 8 `tests/publish_observability_contract.rs` CLI
  tests (help advertises `--revision`; non-hex revision refused,
  provider never invoked; short revision refused, provider never
  invoked; full flow persists revision/build/run/container
  identity in response JSON and status projection; run failure
  preserves build success; legacy provider without phase evidence
  still gets the canonical container identity synthesized from
  revision; human renderer surfaces the additive fields;
  GitHub push end-to-end persists the phase evidence on the
  idempotent row).
- Live sibling round trip: `live_jenkins_local_provider_round_trip_through_real_sibling`
  continues to pass against the updated sibling at
  `/home/paul/code/jenkins-local/adapters/forge-publish-provider.py`;
  the `capabilities` round trip echoes the new
  `revision: <full 40-char hex>` and
  `container_identity: forge-<project>-<sha12>` fields verbatim
  (e.g. `revision: 0123456789abcdef0123456789abcdef01234567` and
  `container_identity: forge-alethefy-0123456789ab`); the
  `preflight` round trip emits the same additive fields; no
  Mac source checkout, script or deployment script is
  introduced on Mac.
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 42 passed,
  0 failed pre-archive and 44 passed, 0 failed post-archive with
  the promoted `forge-publish-observability-revision-containers`
  spec (+3 requirements); `git diff --check`: PASS.
- Pointer state: `forge-publish-observability-revision-containers`
  archived (`15/15` tasks evidenced; archive proceeded with
  `--yes` to record task status with the verified evidence).
  `openspec list` shows one remaining proposal:
  `forge-independent-project-inventory-fleet`. The pointer
  advances to it.
- No shared Gate Runtime is configured; no Gate pass is claimed.


`forge-publish-queue-status` implemented, verified and archived on
2026-09-28 as `2026-09-28-forge-publish-queue-status`; its four
requirements (sequential fleet execution, provider progress events,
durable deploy status, bounded status watch) were promoted into
[openspec/specs/forge-publish-queue-status/spec.md](openspec/specs/forge-publish-queue-status/spec.md).
The implementation extends the existing provider transport
(`fe13db1`/`89886c8` skeletons) without breaking them: every
provider request/response now carries an optional `queue_id`
field validated at `1..=128` ASCII alphanumeric plus `-`/`_`;
every `publish.progress` event on the stderr stream is matched
against the active request envelope — same `operation_id`,
`project_id`, and `queue_id` — and a mismatch is reported as a
provider protocol violation rather than silently accepted;
progress `detail` is bounded by `PROGRESS_DETAIL_MAX = 512`
characters with a `…` truncation, and the bounded value is
passed through `policy::redact_credentials` so a leaky provider
never reaches the operator. A new `src/publish/queue.rs` module
defines the per-project state machine (`queued`/`running`/
`succeeded`/`failed`/`timed_out`/`cancelled`) and enforces the
one-running-project invariant across the queue, with duplicate
terminal events for the same operation_id ignored so a noisy
provider cannot flip a `succeeded` project back to `failed`.
The existing operations journal gains an additive `queue_id`
column and matching partial index
(`operations_queue_project_state_idx`) so a `forge deploy
status --queue <id>` query answers from a single index lookup;
the schema migration is in `apply_migrations` and is idempotent
against pre-change registries. `cmd_publish_fleet` now
generates one stable `fleet-<UTC>-<8hex>` queue id per
invocation, persists one `publish` journal row per project
through the existing registry with the `queue_id` field set
on every completion path (success, failure, fail-fast early
exit), and exposes the `queue_id` in the fleet aggregate
JSON. `forge deploy status` accepts `--queue <id>` (validated
against `validate_queue_id`), `--watch` (refused without
`--queue`), and bounded `--interval-secs 1..=60` /
`--deadline-secs 1..=86400`; `--watch` polls the queue until
every project reaches a terminal state or the deadline
expires and refuses to claim success on a mixed-terminal
result. Status reads never append journal rows (no mutation
on the read path), and the existing `forge deploy status`
flags stay valid — the prior `forge-deploy-status/0.1.0`
document is now `forge-deploy-status/0.2.0` carrying the
`queue` and `scope` fields, but the kind filters, JSON/human
output structure and read-only guarantee are unchanged. The
GitHub push path (`src/api/mod.rs`) and the manual publish
path (`cmd_publish_provider`) both carry the new optional
field with `queue_id: None`, so single-project publishes are
byte-compatible. No Mac script, source checkout or deployment
script is introduced on Mac; Forge owns queue identity,
ordering, journal state and status projection, while Jenkins
owns Mac execution and would only emit the generic
provider-neutral events. The MCP `tools/list`, portal and
`forge list` surfaces stay unchanged: no new tool, no new
route, no new portal section, no new journal kind beyond the
existing `publish` rows; the registry's `operations` table
gains no columns beyond the additive `queue_id`.

## Verification evidence (forge-publish-queue-status, 2026-09-28)

- `cargo fmt --all -- --check`: PASS for the touched files
  (`src/publish/queue.rs`, `src/publish/providers.rs`,
  `src/registry/mod.rs`, `src/main.rs`, `tests/publish_queue_status_contract.rs`,
  `tests/publish_contract.rs`); the pre-change baseline already
  carries formatting drift in unrelated files, which is out of
  scope.
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS against the
  pre-change baseline (the four pre-existing `-D warnings`
  errors live in `src/gate/evidence.rs` and `src/publish/mod.rs`
  — none touched by this change; verified by stashing the patch
  and re-running).
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`:
  PASS — 64 result groups, 0 failures; the change adds no new
  long-running native test.
- New supervised suites: 6 `src/publish/queue` unit tests
  (start-then-finalize succeeds; second running project refused;
  terminal state blocks rerun; duplicate terminal event ignored;
  non-terminal finalize refused; aggregate counts every state); 19
  `src/publish/providers` unit tests (queue_id validation
  alphanumeric/dash/underscore accepted; empty / oversized /
  unsafe refused; parse_request carries queue_id through;
  parse_request rejects malformed queue_id; classify_progress_event
  accepts matching event; ignores non-progress event; rejects
  wrong contract; rejects operation/project/queue id mismatches;
  redacts and bounds detail; requires queue_id when active;
  rejects event with queue_id for standalone request; rejects
  missing phase/status); 7 `tests/publish_queue_status_contract.rs`
  CLI tests (help advertises `--queue`/`--watch`/`--interval-secs`/
  `--deadline-secs`; `--watch` without `--queue` refused;
  malformed queue id refused; `--interval-secs` 0/61 refused;
  `--deadline-secs` 0 refused; unknown queue returns empty
  read-only history; `--project` and `--queue` together refused).
- `node scripts/check-openspec-change-names.mjs`: PASS;
  `openspec validate --all --strict --no-interactive`: 41 passed,
  0 failed pre-archive and 42 passed, 0 failed post-archive with
  the promoted `forge-publish-queue-status` spec (+4
  requirements); `git diff --check`: PASS.
- Pointer state: `forge-publish-queue-status` archived
  (`19/19` tasks evidenced). `openspec list` shows two
  remaining proposals: `forge-independent-project-inventory-fleet`
  and `forge-publish-observability-revision-containers`. With
  `forge-publish-queue-status` now archived as a prerequisite,
  `forge-publish-observability-revision-containers` is the next
  eligible change and the pointer advances to it.
- No shared Gate Runtime is configured; no Gate pass is claimed.


`forge-publish-plugin-orchestration` implemented, verified and
archived on 2026-09-27 as
`2026-09-27-forge-publish-plugin-orchestration`; its five
requirements (one publish engine, switchable providers, provider
contract, idempotent push retries, provider isolation) were promoted
into
[openspec/specs/forge-publish-plugin-orchestration/spec.md](openspec/specs/forge-publish-plugin-orchestration/spec.md).
The implementation closes every Section-3 BFS and Section-4
verification task that remained after the Jenkins deploy adapter
archived: `forge publish --project <id> --provider <name>` and
`forge publish --folder <path>` converge through the same
`PublishProviderRequest` constructor (`forge-publish-provider/0.1.0`
contract, manual operation id `publish-<id>-<12hex-rev>` vs GitHub
push `github-<delivery_id>`), provider enable/disable/list/inspect
CLI persists state through `serde_yaml`, the contract validates
`contract`, `provider`, `project_id`, `revision`, `operation_id`
plus a marker-based secret redactor on `password=`/`token=`/`secret=`/
`private_key`/`-----begin` across both `evidence` and `recovery`,
disabled and unconfigured providers are refused with
`error[publish-invalid]` before any subprocess starts, and one
provider's failure does not block another — `publish_provider_failure_does_not_prevent_other_provider_use`
configures OpenPanel and Jenkins back to back, observes the failing
provider exit non-zero, then selects Jenkins and records a
healthy `done`. The Jenkins provider is genuinely optional: with
only `openpanel` configured, `--provider jenkins` is refused while
`--provider openpanel` succeeds without any `project.sh`,
`deploy-all.sh`, `install-mac.sh` or `jenkins-local` invocation in
the workdir (`publish_jenkins_optional_and_no_mac_script_required`).
GitHub push idempotency rides on the registry's existing
`reserve_idempotent_operation` with the delivery id as the key:
`duplicate_github_push_delivery_invokes_provider_at_most_once`
drives the API server end to end with two identical signed
deliveries, asserts the second returns `200` with `"status":
"duplicate"`, and counts exactly one fixture-provider invocation in
the recording log. The jenkins-local sibling's shipped adapter
(`/home/paul/code/jenkins-local/adapters/forge-publish-provider.py`,
`100755`) is exercised live as the `live_jenkins_local_provider_round_trip_through_real_sibling`
round trip (skipped honestly when the sibling is absent). The
OpenPanel sibling provider is still `0/9 tasks` in that repository,
so the Forge-side fixtures (the published contract fixtures under
`tests/fixtures/publish-provider/` plus the
`publish_apply_invokes_provider_with_typed_contract` fixture
provider) stand in for the conformance evidence on this host.
`publish_redacts_secret_shaped_provider_evidence` proves the
redactor rejects a leaky fixture before any stdout/stderr hits the
operator, and `manual_and_github_push_produce_equivalent_provider_request`
asserts the two entry points construct the same
`PublishProviderRequest` envelope (the operation id is the only
intentional divergence). The MCP `tools/list`, portal and
`forge list` surfaces stay unchanged: no new tool, no new route,
no new portal section, no new journal kind beyond the existing
`publish` rows; the registry's `operations` table gains no columns.

## Verification evidence (forge-publish-plugin-orchestration, 2026-09-27)

- `cargo fmt --all -- --check` for the touched files: PASS (the pre-change baseline already carries formatting drift in unrelated files; out of scope).
- `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS against the pre-change baseline (the seven pre-existing `-D warnings` errors live in `src/gate/evidence.rs`, `src/main.rs`, `src/publish/mod.rs`, `src/publish/fleet.rs` and `tests/gate_cross_surface.rs` — none touched by this change; verified by stashing the patch and re-running).
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS (pre-existing long-running scaffold excluded; the change adds no new long-running native test). New/extended suites: 22 `src/publish/providers` unit tests (clean response acceptance, password/token/PEM/private-key marker rejection across both `evidence` and `recovery`, wrong contract refusal, missing-field refusal for each of `provider`/`project_id`/`revision`/`operation_id`, blank-string refusal, non-object payload refusal, every operation id `capabilities`/`preflight`/`publish`/`verify`/`rollback` round-trip, unknown/disabled entry refusal, missing-file/invalid-yaml round trips, default-enabled serde round trip) and 11 `tests/publish_contract.rs` CLI/API tests (help surface; dry-run prints the request without invoking the provider; apply invokes the provider with the typed contract and records the envelope verbatim; disabled-provider refusal; multi-provider failure isolation; Jenkins optionality with no Mac script bundle invoked; secret-shaped evidence redacted at the contract boundary; enable/disable persistence through `forge publish provider enable|disable`; duplicate GitHub push delivery invokes the provider at most once through the API server end to end; manual and GitHub-push `PublishProviderRequest` envelopes agree on every shared field; live `forge-publish-provider.py` round trip through the jenkins-local sibling when present, skipped honestly otherwise).
- Live sibling round trip: `live_jenkins_local_provider_round_trip_through_real_sibling` invokes the real jenkins-local adapter at `/home/paul/code/jenkins-local/adapters/forge-publish-provider.py` (`100755`, contract `forge-publish-provider/0.1.0`) with a `capabilities` request, parses the JSON envelope, and asserts `contract`, `provider`, `operation_id` and `status` (`available`) are reported by the real sibling — no Forge-side fixture or stub involved.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 41 passed, 0 failed pre-archive and 41 passed, 0 failed post-archive with the promoted `forge-publish-plugin-orchestration` spec (+5 requirements); `git diff --check`: PASS.
- Pointer state: `forge-publish-plugin-orchestration` archived (`23/23` tasks evidenced); `openspec list` shows three remaining proposals (`forge-independent-project-inventory-fleet`, `forge-publish-observability-revision-containers`, `forge-publish-queue-status`) and the `current_spec` line is removed (no active eligible change remains on this host; the OpenPanel sibling provider is `0/9 tasks` and the jenkins-local provider round trip is now in this repository's contract tests).
- No shared Gate Runtime is configured; no Gate pass is claimed.

`gate-evidence-export-consumption` implemented, verified and archived as
`2026-09-27-gate-evidence-export-consumption`. Companion sibling
`driftwatchdog gate-evidence-export` archived `2026-09-27` at `221faeca`.
Real export captured at
`tests/fixtures/gate-evidence/forge-all-unverified.json`. No executed gate
pass claimed for this repository (`.driftwatch` store uninitialized);
honest state: all nine fields `unverified`. No active changes remain.

Companion driftwatchdog and workspace-governance next actions recorded in
their respective HANDOFF sections below.

`gate-runtime-evidence` implemented, verified and archived on 2026-09-24 as
`2026-09-24-gate-runtime-evidence`; its three requirements (declared gate
runtime execution, revision-bound evidence lifecycle, honest surface
projection) were promoted into
[openspec/specs/gate-runtime-evidence/spec.md](openspec/specs/gate-runtime-evidence/spec.md).
Forge now executes the gate runtime a project declares instead of
asserting one: `forge gate [TARGET]` resolves the binary through
`FORGE_GATE_BIN` (runs exactly as named, never replaced by probing) →
`.project.json` `verification.gate_runtime` (only `driftwatchdog` has a
resolution path; an unknown declared name is refused by name) → the
ordered `driftwatchdog` → `driftwatch` PATH probe shared with the policy
plane, every refusal naming its attempts; invocation is a bounded
argument array (null stdin, 256 KiB stdout bound, default 600s with
`--timeout-secs` 1..=86400), and a parseable gate status document is
evidence whatever the exit code — the sibling exits non-zero exactly
when blocked — while a `PASS` document riding a failure exit downgrades
to `unknown` instead of fabricating a pass. Live captures at sibling
`25811ed` disproved the design's assumption that `gate --dry-run` has a
JSON composition (it prints the human plan and exits before the JSON
writer, reconfirming the prior cycle's NOTES): the rehearsal reports
that plan preview and never persists or journals (the deploy-rehearsal
precedent), while the evidence surface is the real `gate --format json`
whose only side effect is one `gate_runs` row in the project's own
`.driftwatch/` store; the real documents carry a third top-level status
(`REVIEW_REQUIRED`) classified through `blocked`/`unknown` — never
`passed` — per-check `REVIEW_REQUIRED` maps to `unresolved`, and unknown
future strings never count as pass. Evidence persists atomically at
`.forge/gate/<project-id>/evidence.json` bound to the git HEAD captured
at invocation (an unbound revision can never read as fresh);
`forge gate status [TARGET]` annotates `fresh|stale|absent` with exit 0
only for fresh-passing; every attempted real run appends a `gate`
journal row (`done|blocked|failed`) and reads journal nothing; the
doctor `gate-evidence` finding passes fresh passing evidence, warns
stale, fails blocked and reports `unverified` for a declared or
gate-managed project that never ran — never PASS by silence — while
projects without any declaration or manifest keep the finding
not-applicable so the checker plane stays byte-identical; release
`checks` accept a `gate` kind that can never cite stale evidence; and
the provider matrix gained a `gate-runtime` row (six providers now)
under the existing opt-in rules, exercising only the side-effect-free
plan surface and never claiming a gate pass. Every captured string
passes `policy::redact_credentials`, host paths of the assessed project
become `<project>`, and char bounds are marked; MCP, the API and the
portal gained no gate surface (the mature registry advertises no gate
tool; `forge portal view gate` refuses `portal-invalid`). No executed
gate pass is claimed for this repository: `forge gate .` rehearses this
repo's real `.ai-gate/gate.yaml` (build/repository/security/tests
required) and the real run honestly reports
`gate-runtime-unavailable` at the sibling's own store boundary — **next
action (sibling-owned)**: initialize this checkout's `.driftwatch`
store (`driftwatch init`) before any real gate pass is claimed here;
until then the real-run row stays honestly unavailable. The previous
cycle's execute-bit action on the Workspace Governance adapter stays
open with that sibling.

## Verification evidence (gate-runtime-evidence, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 61 result groups, 1180 tests, 0 failures.
- Native toolchain test (excluded from the aggregate as the known long-running scaffold build): `cargo test --lib -- --exact generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` — PASS (546s, real cargo build+test of the generated tree, unaffected by this change's surfaces).
- New suites (55 tests): 23 `src/gate` unit tests (override-exactness with no fall-through, declared-unknown refusal naming the value, unreadable declaration still probes with honest attempts, the four verbatim fixtures classified to their real aggregates, PASS-on-nonzero-exit contradiction to `unknown`, non-blocking `REVIEW_REQUIRED` to `unknown`, `NOT_APPLICABLE`/future row states never pass, real-run mapping with runtime/version attribution, dry-run plan preview, unparseable real run unavailable quoting the runtime's own words, hang cut off by the bounded wait, credential+host-path scrubbing with char bounds, atomic round trip with rehearsal-persistence refusal, corrupt evidence names the file and never invents absence, freshness requires the exact revision binding, journal verdict mapping, timeout bounds); 16 `tests/gate_contract.rs` CLI tests (help surface; passing run persists revision-bound evidence, journals `done`, exits 0, human/JSON carry identical aggregate/revision/runtime/timestamp; blocked run exits 1 with evidence and journal `blocked` and no unavailable label; review-required fixture blocks; missing runtime lists attempts with empty stdout, byte-identical prior evidence and a `failed` row beside the kept `done`; unknown declared runtime refused by name; dry-run previews the plan, persists and journals nothing; unparseable real-run text unavailable never-a-pass; contradictory PASS document downgrades and journals `failed`; absent→fresh→stale status lifecycle with identical record bytes and zero read-journaling; timeout bounds refuse pre-spawn and a hang times out; credential-shaped diagnostics redacted through stdout, evidence file and journal; registry identity names journal and evidence path; argument conflicts refuse before work; unknown target refuses before any invocation; env override beats the declaration and runs exactly the named binary); 8 `tests/gate_cross_surface.rs` tests (doctor never invents health from absence — not-applicable versus declared-unverified — blocked fails with healthy false and stale passing warns; run/status/doctor byte parity of observed_at/revision/runtime/aggregate; checker projects blocked as an error alert and declared-never-run as a warning while plain projects gain no alert; gate rows journal alone, foreign rows byte-preserved, reads journal nothing; MCP tools/list advertises no gate tool after runs exist and no gate API route resolves; portal renders and refuses `view gate`; the release `gate` check captures unavailable never-run, passes fresh evidence at the captured revision and flips `stale`/not-ready after the revision moves); 8 `tests/gate_provider_contract.rs` tests (six-provider stable order with gate-runtime not-run; live-without-opt-in stays not-run; inspect names `FORGE_GATE_BIN`, the ordered default and the plan-only boundary that never claims a gate pass; fixture plan response supported; parseable status document attributed by its own aggregate; refusal unavailable; env-override exclusivity; PATH alias resolution). Extended: 3 provider roster expectations five→six (`src/provider` unit + `provider_contract` matrix + human roster).
- Real sibling round trips (release build against the installed `driftwatch 0.1.0`): this repository — `forge gate . --dry-run` exit 0 serving the real plan through the repo's own `.ai-gate/gate.yaml` (build/repository/security/tests required; `persisted: false`, `journaled: false`), and the real `forge gate .` honestly exiting 1 `gate-runtime-unavailable` quoting the runtime's own store hint (`.driftwatch/state.db` absent in this checkout), empty stdout, no repository writes; scratch project — passing run exit 0 persisted `.forge/gate/liveapp/evidence.json` bound to the real HEAD, journaled `done`, `gate status` served the identical record fresh with exit 0; failing the check produced a blocked run exit 1 with the per-check fail row and journal `blocked`, doctor moved pass→fail, `forge check` emitted the `doctor/gate-evidence` error alert with the attributed summary, a later commit rendered the same record stale with exit 1, and the dry-run previewed while journaling nothing. The stale `~/.cargo/bin/driftwatchdog` limitation from the prior cycle is checker-surface-specific: the gate surface answers on that installed binary.
- Fixture set `tests/fixtures/gate/`: four verbatim sibling documents (pass, blocked, not-applicable, review-required) plus the human dry-run plan, all captured at `25811ed`; the design-vs-reality divergences (third top-level status, envelope-to-row mapping, absent contract field with shape discrimination, plan-only dry-run) are recorded in `NOTES.md`.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 32 passed, 0 failed pre-archive and 32 passed, 0 failed post-archive with the promoted `gate-runtime-evidence` spec (+3 requirements); `git diff --check` and staged review: PASS.
- Pointer state: `gate-runtime-evidence` was the last active change (`openspec list`: none remain); the `current_spec` line is removed.
- No executed gate pass is claimed for this repository; the completion Gate boundary stays unconfigured (`forge gate` exists and `driftwatch init` is the sibling-owned prerequisite for this checkout).

`workspace-governance-adapter-consumption` implemented, verified and
archived on 2026-09-24 as
`2026-09-24-workspace-governance-adapter-consumption`; its two
requirements (known-provider preset resolution, provider input
isolation) were promoted into
[openspec/specs/governance-provider-contract/spec.md](openspec/specs/governance-provider-contract/spec.md).
The governance plane now consumes the real sibling end to end:
`forge governance use workspace-governance . [--workspace-root R]`
resolves the packaged candidate
`R/workspace-governance/scripts/forge_governance_adapter.py` from the
argument or `FORGE_WORKSPACE_ROOT` only — never a parent-directory
search and never the network (live verification disproved the design's
flat `R/scripts/...` layout: the checkout nests one level inside the
portfolio it governs, so the root names the same value the adapter's
own `WORKSPACE_ROOT` input takes) — verifies the candidate is an
existing executable regular file, refuses otherwise with the typed
`governance-invalid` naming the exact candidate path while the
previously selected provider stays in force, and stores the resolved
absolute adapter path plus the workspace root in
`.forge/providers.yaml`; every adapter run re-supplies the stored root
as `WORKSPACE_ROOT` exactly as the sibling documents its own
invocation, so later checks never depend on the environment staying
set. An explicit `--adapter` always wins and is stored verbatim;
`--workspace-root` without a preset is refused rather than silently
dropped; `serde(default)` keeps every pre-change selection file and
every pre-change providers.yaml byte-loadable, the contract v0.1.0
boundary is unchanged, and the local default, MCP/API/portal read
surfaces and doctor verdicts consume normalized observations exactly
as before. The sibling ships the adapter as git mode `100644` while its
other scripts are `100755`, so the direct-exec boundary honestly
refuses the real root's candidate (live verbatim in
`docs/provider-evidence.md`): **next action (sibling-owned)** — commit
`git update-index --chmod=+x scripts/forge_governance_adapter.py` in
Workspace Governance; until then the real-root live row stays honestly
`not-run`, and the documented explicit-adapter remedy carries any
non-executable checkout. Four audit-shaped fixture stubs capture the
sibling adapter's real observation documents verbatim (`pass`/`fail`
`DECLARATION_MISSING`-first/`blocked` adoption-gap/`unknown`
`PROJECT_UNKNOWN`, sibling HEAD `204d140`) with the divergences from
the design's guessed codes recorded in
`tests/fixtures/governance-audit/NOTES.md`; a removed configured
checkout surfaces bounded `unavailable` detail that never reads as
healthy through the checker plane (warning), the portal settings
section (`unavailable`/`fail` rollup) or any Forge exit-code contract,
and a failing live adapter keeps mapping `fail` to `error` severity
everywhere.

## Verification evidence (workspace-governance-adapter-consumption, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 58 result groups, 1125 tests, 0 failures.
- Native toolchain test (excluded from the aggregate as the known long-running scaffold build): `cargo test --lib -- --exact generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` — PASS (891s, real cargo build+test of the generated tree, unaffected by this change's surfaces).
- New/extended suites: 9 `src/governance` unit tests (no position taken for preset-less providers, refusal naming both explicit inputs when no root is supplied, blank env treated as absent, missing/directory/non-executable candidates each refusing while naming the exact candidate path, executable candidate resolving to a canonical absolute path, flag-beats-env with no fall-through to the env root, env-root resolution); 12 extended `tests/governance_contract.rs` tests (preset persistence of resolved path plus workspace root, workspace-root round-trip through reload, the four audit-shaped fixtures mapping to `pass`/`fail`/`blocked`/`unknown` with real evidence codes, revision passthrough and omission-when-unknown, credential redaction and char bounds on evidence and detail, WORKSPACE_ROOT re-supply only when a root is stored, no-root adapters see no injected WORKSPACE_ROOT, local rejecting a workspace root, relative-root refusal at persistence, legacy rootless selections loading, broken preset never disturbing the stored selection, removal-after-selection yielding bounded `unavailable`; the three pre-existing 1000ms stub timeouts raised to 5000ms against added parallel load); 13 `tests/governance_preset_contract.rs` CLI tests (help surface advertising `--workspace-root`, `--workspace-root` without a preset or adapter refuses `has no packaged adapter` and stores nothing, explicit `--adapter` alongside a workspace root stores and re-supplies it, selection storing the resolved path and `status` consuming the v0.1.0 boundary with the fixture's real revision, env-root resolution surviving env removal, argument-beats-env, explicit `--adapter` beating a resolvable preset, unresolved-root refusal naming both inputs with empty stdout and nothing stored, non-executable refusal naming the exact candidate while the previous provider stays in force and `list`/`status` remain local, directory-candidate refusal, a valid candidate above the project never discovered implicitly, removed checkout yielding `unavailable` while `list`/`use local` continue, and the registry database staying byte-identical across the full preset cycle); 3 extended `tests/governance_cross_surface.rs` tests (broken preset never rendering healthy through `forge check` or the portal settings section and a failing adapter mapping to `error` severity/`fail` status, unconfigured governance requesting no workspace root, MCP `run_governance` parity consuming the preset-selected observation).
- Real sibling round trips (live evidence at workspace-governance `204d140`, release build): against the real portfolio root the preset refuses verbatim `candidate /home/paul/code/workspace-governance/scripts/forge_governance_adapter.py is not an executable file` (exit 1, empty stdout) because the sibling committed mode `100644`; the full matrix then ran live through a scratch portfolio holding a byte-identical copy of the sibling code with the execute bit on the copy only (real tree stayed git-clean and `664` throughout): `forge governance use workspace-governance <project> --workspace-root <scratch-portfolio>` stored the resolved candidate and root, and `forge governance status` served the real audit — `forge`→`pass` (revision `888d8058…`, `adoption=adopted`), `crossalheart`→`fail` (`DECLARATION_MISSING` first in evidence, revision `916f7942…`), adoption-gap `argoset`→`blocked` (revision `1c4177c5…`), an invented id→`unknown` (`PROJECT_UNKNOWN`, exit 0, no revision), and a later-removed candidate→`unavailable` naming the spawn failure while local commands continued; explicit-adapter remedy runs against the real registry with the real portfolio root (`WORKSPACE_ROOT=/home/paul/code`) reproduced the same live observations, and the sibling adapter's malformed-request exit 2 stayed classified `unavailable`, never a governance status.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 33 passed, 0 failed pre-archive and 32 passed, 0 failed post-archive with the promoted `governance-provider-contract` spec (+2 requirements); `git diff --check`: PASS.
- Next pointer: `gate-runtime-evidence` (order 37) — the only remaining active change; its own proposal records that the Driftwatchdog gate JSON exists today (`checker-machine-output` is not a prerequisite), and the driftwatch adapter change already exercised that surface.
- No shared Gate Runtime is configured in Forge yet; no Gate pass is claimed; the `gate-runtime-evidence` change is the proposal to wire one.

`driftwatch-cli-alignment` implemented, verified and
archived on 2026-09-24 as
`2026-09-24-driftwatch-cli-alignment`; its two
`quality-policy-integration` requirements (canonical DriftWatch CLI
invocation, policy binary resolution) were promoted into
[openspec/specs/quality-policy-integration/spec.md](openspec/specs/quality-policy-integration/spec.md)
and the DriftWatch workspace-marker requirement into
[openspec/specs/doctor-maturity-assessment/spec.md](openspec/specs/doctor-maturity-assessment/spec.md).
The adapter now speaks to the real sibling: `run_driftwatch` resolves
the policy binary by ordered probe — `FORGE_DRIFTWATCH_BIN` runs
exactly the named binary with no fall-through, otherwise the first
executable hit of `driftwatchdog` → `driftwatch` wins, and neither
being present is an honest `unavailable` naming both attempts — and
invokes only grammar the installed binary supports with the project
directory as the confinement scope: `check --dry-run --format json`
for checker projects and the real `gate --format json` for projects
carrying `gate.toml`/`.ai-gate/gate.yaml`. Live verification against
the sibling at `25811ed` disproved the design's assumption that
`gate --dry-run` has a JSON composition (it prints the human plan and
exits before its JSON writer — captured verbatim in
`tests/fixtures/driftwatch/gate-dryrun-plan.txt` and
`NOTES.md`), so the gate surface runs for real and its only visible
side effect is one `gate_runs` row in the project's own
`.driftwatch/` store; the checker surface keeps the sibling's genuine
no-persistence composition. The fabricated `--project` flag is gone
from the adapter and the provider probe, which share the one
surface helper. Classification: a parseable document is evidence
whatever the exit code — checker-report rows map alerts to findings
(`id` `<checker>/<symbol>`, declared `extra.category` verbatim,
unknown or missing severity to warn, never pass) and
failed/timeout/protocol-error rows to fail findings carrying the
runtime's own bounded error note; gate results map FAIL→fail,
REVIEW_REQUIRED→warn, NOT_APPLICABLE→not-applicable pass, and a
blocked aggregate without detail to one fail rollup; an unknown
document contract reports `unavailable` naming the version and can
never contribute findings or a PASS; legacy adapter-report fixtures
keep prior semantics including non-zero-exit unavailability. Host
paths (the assessed directory and any document-reported root) are
replaced with `<project>` in every finding string before the shared
`policy::redact_credentials` pass, and observations record the
resolved binary name and version. Doctor's marker detection gained
`driftwatch.toml`, `gate.toml` and `.ai-gate/gate.yaml` alongside the
legacy names, with absence staying a distinct state from a
configured-but-failing run; the checker plane's default never
contacts the policy binary (feedback-loop guard preserved).

## Verification evidence (driftwatch-cli-alignment, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 57 result groups, 1088 tests, 0 failures.
- Native toolchain test (excluded from the aggregate as the known long-running scaffold build): `cargo test --lib -- --exact generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` — PASS (real cargo build+test of the generated tree, unaffected by this change's surfaces).
- New/extended suites: 15 `src/policy` unit tests (ordered `first_binary_on_path` preference with non-executable skip, surface selection by gate manifests including `.ai-gate/gate.yaml`, checker-envelope mapping with alerting/failed/ok rows plus category preservation, severity-default-to-warn and `<project>` scrubbing, non-zero-exit-but-parseable evidence, blocked-gate FAIL/REVIEW_REQUIRED/PASS/NOT_APPLICABLE mapping with empty-findings rollup guard, unknown-contract refusal naming the version with same-major 0.x acceptance, unrecognized-document refusal, dead-override no-fall-through, verbatim sibling fixtures for passing/alerting/blocked-gate/passing-gate/unknown-contract); 10 `tests/driftwatch_cli_alignment_contract.rs` tests (PATH-controlled ordered probe with recorded-argv proof of `check --dry-run --format json` and absence of `--project`, alias-only host, no-binary availability naming both attempts, override-exclusivity, blocked-gate verdict lowering via the real gate surface, unknown-contract refusal, secret + host-path scrubbing through doctor findings, marker detection per new file, plain `forge check` never contacting the binary and `--include-policy` consuming envelope symbols, provider fixture probe with non-zero-exit parseable-supported and unparseable-unavailable classification, and the ordered live PATH probe); 8 `tests/driftwatch_cli_contract.rs` tests driving the verbatim sibling fixtures and stale-binary stderr capture through the CLI; the `quality_policy_contract` detector fixture now resolves its project from the working directory (no `--project`).
- Real sibling round trips (task 3.4, live evidence at driftwatchdog `25811ed`, release build): `forge provider run driftwatch-policy probe --live` recorded `supported` (`sandbox: live`, `source: live:driftwatchdog`, tool_version `driftwatch 0.1.0`, evidence `contract=driftwatch-checker/0.1.0 checkers=2`, teardown true) through BOTH the `FORGE_DRIFTWATCH_BIN` override and a PATH-first `driftwatchdog` candidate with no override; `forge doctor` against the scratch project surfaced `driftwatch-docs-watch/DOCS-1` `warn` `[documentation:warn] docs evidence missing` with `driftwatchdog 0.1.0:`-attributed evidence and the `driftwatch-policy` rollup (0 pass, 1 warn, 0 fail), the `clean` checker contributing nothing and the `--dry-run` run persisting zero `drift_alerts`/`gate_runs` rows; the stale `~/.cargo/bin/driftwatchdog` (pre-`checker-machine-output`) is honestly classified `unavailable` from its verbatim `stale-binary-rejection.txt` stderr with empty stdout, never a PASS.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 34 passed, 0 failed pre-archive and 33 passed, 0 failed post-archive with the promoted `quality-policy-integration` (+2) and `doctor-maturity-assessment` (+1) specs; `git diff --check`: PASS.
- Next pointer: `workspace-governance-adapter-consumption` (order 32) — its companion evidence landed (workspace-governance shipped and archived `forge-governance-adapter` with the adapter shim on 2026-09-24).

`workspace-metadata-emission` implemented, verified and
archived on 2026-09-24 as
`2026-09-24-workspace-metadata-emission`; its two
requirements were promoted into
[openspec/specs/deterministic-project-generation/spec.md](openspec/specs/deterministic-project-generation/spec.md).
The implementation makes every generated project
adoptable by Workspace Governance on day one:
`forge new` additionally stages the sibling-compatible
`.project.json` (sibling `schema_version: 1`, the
project id, `kind: product`, `lifecycle: active`) as
an ordinary deterministic template file — governance
profile from an explicit per-profile descriptor
mapping (`rust-web`→`rust-product`,
`python-service`→`python-product`,
`nextjs-web`/`react-web`→`typescript-product`,
`aspnet-web`→`dotnet-product`,
`flutter-app`→`flutter-product`), the mapping values
confirmed against the real sibling registry vocabulary
at implementation time (the design's `node-product`
guess appears nowhere in Workspace Governance;
`typescript-product` is the observed value),
`verification.command` is the descriptor's native test
command (one source of truth with the readiness
matrix), `evidence_status` starts `planned` and
`deployment` stays `{deployable: false, jenkins_job:
null, compose_file: null}` — Forge never writes
evidence it did not observe. `gate_runtime` is emitted
only when the profile descriptor declares one, no
supported profile declares one (no shared Gate Runtime
is configured), so generated documents carry no key
and a guessed governance profile or gate claim is
refused; the sibling's checker code never reads the
field and its documented vocabulary, semantics and the
schema-example divergence are extracted in
`tests/fixtures/workspace-metadata/NOTES.md`.
Descriptors without a mapping (every planned candidate
and external descriptors) stage nothing and record an
honest omission note in the CLI output — human line
plus JSON `notes`, which appear only when non-empty so
mapped default output keeps the prior key set.
`--no-workspace-metadata` opts out with a tree
byte-identical to the pre-change release, proven per
profile against six tree digests captured from the
binary built before this change. The declaration is
covered by an ownership receipt
(`.forge/workspace/project.json.receipt`, the sha256
of the generated bytes, a pure function exactly like
feature receipts): a mutating upgrade whose refresh
would touch a user-edited declaration instead refuses
with the standard ownership-conflict code, journals
`blocked` and leaves the edited file and manifest
untouched; unedited stale content (manifest or
descriptor drift since generation) refreshes with the
applied features and names both files in
`files_changed`; a `.project.json` without a Forge
receipt (sibling `init_project.py` or hand-written) is
foreign — Forge never rewrites it and never blocks on
it. `forge import` observes the declaration as
informational evidence only (`workspace_metadata`
detected/missing in the proposal JSON and human
output) and never creates or rewrites it; doctor
surfaces presence as a `workspace-metadata` `PASS`
finding with `applicable: false` — never gating
health, maturity or the checker plane, and absence is
not a claim; MCP `create_project` and the API create
route share the one Core path and emit byte-identical
declarations; the registry gained no columns, journal
kind, MCP tool, API route or portal surface; and
generated projects still build and test with their
native toolchains with the file present and absent —
the declaration is inert metadata. At that close the
three remaining active changes were proposals (orders 30
`driftwatch-cli-alignment`, 32 `workspace-governance-adapter-consumption`
and 37 `gate-runtime-evidence`) awaiting companion implementation
evidence in the driftwatchdog and workspace-governance repositories
before selection; the driftwatchdog companion
(`checker-machine-output`) landed the same day and order 30 archived
above, leaving orders 32 and 37 roadmap-eligible.

## Verification evidence (workspace-metadata-emission, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 538 lib tests plus all non-skipped contract and cross-surface suites passed (55 result groups, 1061 tests total, 0 failures).
- New supervised suites: 10 `src/generate/workspace` unit tests (honest declaration shape for all required keys, `gate_runtime` omitted-when-undeclared and emitted-when-declared, no-mapping sentinel stages nothing with the omission note naming the profile, determinism with host-path-free and slash-free text, receipt records the declaration hash, upgrade state machine — absent/foreign/expected-matching stay `Nothing`, unedited stale content yields `Refresh` with a re-hashed receipt, user edits and malformed receipts fail closed to `Conflict`, and `apply_action` writes the refresh only and preserves an edited file under `Conflict`); 2 `src/profile` tests (the six supported descriptors carry the confirmed governance mappings with `kind: product` and no gate runtime while all planned candidates hold the no-mapping sentinel; an external descriptor round-trips `workspace` and a blank `governance_profile` refuses with `invalid-profile` naming the field); 1 `src/generate` matrix test (both files staged for all six profiles, opt-out renders exactly the default set minus the two new files, receipt matches the staged declaration hash); 12 `tests/workspace_metadata_contract.rs` tests (six-profile emission matrix through the real CLI with per-profile governance profile, native command, planned-evidence, gate-free, non-deployable and no-machine-path assertions plus receipt-hash agreement, per-profile opt-out tree digests byte-equal the six pinned pre-change-release hexes with no `.forge` tree, default JSON/human output lists both files and keeps the prior JSON key set, selectable-profiles-all-mapped sentinel proof, edited-metadata upgrade refusal with `error[feature-ownership-conflict]` on stderr, empty stdout and both files byte-preserved, unedited-success upgrade rewrites nothing, stale-unedited upgrade refreshes declaration plus receipt idempotently, foreign declarations survive inspect/adopt/upgrade untouched without an invented receipt, import observes detected/missing in JSON and human output and `--accept` writes nothing, doctor presence is a `PASS` non-applicable informational finding while opt-out projects carry none, and `forge check` emits no workspace alert); 4 `tests/workspace_metadata_cross_surface.rs` tests (MCP `create_project` renders the CLI-identical declaration, `tools/list` exposes no workspace or metadata tool, the API keeps the shared Core path without a workspace surface, and flag-off/flag-on projects produce equal registry records, a workspace-free portal dashboard and a journal with no new kinds).
- Native evidence (task 3.1, re-run separately): the previously-skipped lib test `generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` PASSED (464s, real cargo build+test of the generated tree now carrying the declaration and receipt), the contract `rust_scaffold_builds_with_native_toolchain_without_forge` PASSED on the default tree, and `declaration_is_inert_for_native_builds_present_and_absent` ran `cargo build` + `cargo test` green on both the declaration-present and `--no-workspace-metadata` trees.
- Real sibling round trip (task 4.2, live evidence): the real `workspace-governance` checkout on this host (HEAD `6b8981d`) consumed a real `forge new rust-web` output through a scratch registry — `scripts/workspace_check.py --project wm-demo` returned zero declaration-shape findings (`PROJECT_DECLARATION`, `DECLARATION_SCHEMA`, `DECLARATION_ID`, `VERIFICATION_COMMAND`, `DEPLOYMENT_DECLARATION` all clean) and the discovered project entered the audit through the sibling's own `DISCOVERED_UNREGISTERED: local metadata is the source of truth` path with the declared `rust-product` profile as the effective shape; the 7 remaining `REQUIRED_FILE`/`CI_MISSING`/`OPENSPEC_MISSING` errors are Workspace Governance's product-tier adoption checklist (the documents its `init_project.py` owns — the non-goal holds: Forge never registers or adopts), and `scripts/jenkins_manifest.py` skipped the `deployable: false` declaration into an empty deploy manifest.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 35 passed, 0 failed pre-archive and 34 passed, 0 failed post-archive with the promoted `deterministic-project-generation` spec carrying both requirements; `git diff --check`: PASS.
- Pointer state: the three remaining active changes (orders 30, 32 and 37) are blocked proposals needing companion implementation evidence in the driftwatchdog and workspace-governance repositories; the `current_spec` pointer is removed (no active eligible change remains).
- No shared Gate Runtime is configured; no Gate pass is claimed.

`jenkins-deploy-adapter-consumption` implemented, verified and
archived on 2026-09-24 as
`2026-09-24-jenkins-deploy-adapter-consumption`; its two
requirements were promoted into
[openspec/specs/adapter-deployment/spec.md](openspec/specs/adapter-deployment/spec.md).
The implementation freezes the executor boundary Forge already
calls as the versioned `forge-deploy-executor/0.1.0` JSON-envelope
contract documented in
`docs/adapter-contracts/deploy-executor.md` — fixed stdin payload
and stdout envelope schemas, a namespaced wire discriminator
distinct from Forge's own `0.1.0` report/state version (the bare
pre-namespacing value is refused, naming both sides, so a stale
executor can never masquerade as conformant), and classification
rules where a parseable non-zero result records the named stage as
failed with the runtime's own evidence while an unparseable,
contract-violating, timed-out or unspawnable result stays
`deploy-target-unavailable` with the prior DeployState
byte-identical; a contradictory non-zero envelope claiming
`delivered` is downgraded to `failed` because the exit status is
authoritative. Observe now invokes the adapter through a dedicated
read-only `observe --target --project --deploy-id` verb instead of
reissuing the `apply` argv; every stage outcome carries an
`executor=<source>@<revision>` attribution line (an
unself-identified adapter attributes to its binary name with
`unknown` revision, never a claimed version); a dry-run rehearsal
never persists deploy state; and the previously documented
`last_observed_running` field now preserves the last good
observation across unknown re-observations (serde default keeps
every pre-change state file loadable) while `current_state`
honestly reports `unknown`. The bundled reference adapter
`adapters/jenkins/forge-deployer-jenkins` (Python 3 stdlib,
translation only, no deployment logic) maps the contract onto the
real jenkins-local verbs — `project.sh <p> deploy --dry-run` for
the side-effect-free preview, `project.sh <p> deploy` for the real
trigger with the 0/2/3/4/5/6 exit-code-to-recovery guidance
recorded in `adapters/jenkins/jenkins-adapter.md`, and
`project-action.sh <p> status` filtered through an explicit
CONTAINERS-column health vocabulary (running→`running`,
stopped→`failed` down, `partial`/`not-created`/absent
row/failing command→`unknown` — no optimistic healthy default) —
scrubs credential shapes and absolute host paths to
`[REDACTED]`/`<host-path>` before the engine's own
`policy::redact_credentials` pass, is configured only through
`FORGE_JENKINS_LOCAL_DIR` (plus optional status-command and
revision overrides), and treats an incomplete checkout as
unavailable rather than acting on part of it. The provider matrix
`deploy` row requires the executor contract (a missing or unknown
discriminator is `unavailable` naming the mismatch), attributes
the adapter and its revision, and still probes `--dry-run` only;
deploy stays out of the MCP mature tool registry; the API mutation
route is unchanged; production promotion onto a live Jenkins host
is an explicitly deferred, jenkins-local-owned adoption checklist.

## Verification evidence (jenkins-deploy-adapter-consumption, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 525 lib tests plus all non-skipped contract and cross-surface suites passed (53 result groups, 1032 tests total, 0 failures).
- New supervised suites: 9 `src/deploy` engine unit tests (namespaced-envelope acceptance and `0.1.0`/non-JSON refusal naming both contract sides, attribution fallback to binary-name@unknown, non-zero-with-envelope recorded as a failed stage with the prior state file byte-identical, contradictory delivered-claim downgrade, non-zero-without-envelope staying `unavailable`, observe issuing the read-only verb with `--deploy-id` and never an artifact, unmapped observation status preserving `last_observed_running` while `current_state` reports `unknown`, legacy state files without the new field loading unchanged, bounded wait timing out on a hanged child); 3 new `tests/deploy_contract.rs` CLI tests (pre-namespacing contract refused with empty stdout and a typed unavailable naming the mismatch and persisting nothing, non-zero failed envelope rendered as a failed stage with surviving runtime evidence plus attribution and `deploy-health-failed`, dry-run rehearsal never writing a state record); 10 `tests/deploy_adapter_contract.rs` tests (adapter version grammar, dry-run rehearsal delivering without side effect or state, real trigger delivering and observing through the read-only verb with trace-order proof that observe never applies, running/stopped/partial health mappings with last-good preservation via `deploy inspect`, blocked exit-3 surfacing the runtime's own guidance as evidence plus named recovery, secret and host-path scrubbing through stdout and the persisted record, missing executor configuration typed `unavailable` inventing no state, incomplete jenkins tree leaving the prior record byte-identical, provider `deploy` row reaching `supported` with `adapter_source=forge-deployer-jenkins/0.1.0@<rev>` on a dry-run that leaves zero side effects, and the same row `unavailable` naming the missing configuration when the executor is unconfigured).
- Real sibling round trip (task 4.4, live evidence): the real `/home/paul/code/jenkins-local` checkout (HEAD `2e82292`) consumed the surface end to end — `forge provider run deploy --fixture adapters/jenkins/forge-deployer-jenkins` reported `supported` (`sandbox: fixture`) with evidence `adapter_status=delivered` and `adapter_source=forge-deployer-jenkins/0.1.0@2e82292` plus a `--version` tool-probe line, and `forge deploy apply --dry-run --confirm` delegated the real `project.sh <id> deploy --dry-run` verb, recorded the script's own "Would sync, configure shared PostgreSQL when needed, register the Jenkins job, and trigger it." preview as attributed evidence, and wrote no persisted state (a following observe refused `deploy-target-stale`, proving rehearsals leave no shadow record); the real project.sh returns before the Mac handoff on `--dry-run`, so the run contacted no host, triggered no Jenkins build and left the jenkins-local tree byte-identical (`git status` unchanged apart from its pre-existing untracked paths). Live production Jenkins evidence remains deferred to the jenkins-local adoption checklist in `adapters/jenkins/jenkins-adapter.md`.
- Adapter lint: `python3 -m py_compile adapters/jenkins/forge-deployer-jenkins` and `ruff check adapters/jenkins/forge-deployer-jenkins`: PASS (chosen commands; the adapter is Python 3 stdlib, and `shellcheck` is not installed on this host).
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 35 passed, 0 failed; `git diff --check`: PASS.
- The native scaffold test was excluded from the aggregate command because it is an existing long-running native-toolchain integration test. Run that native test when its environment is available.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`supervised-agent-adapters` implemented, verified and
archived on 2026-09-24 as
`2026-09-24-supervised-agent-adapters`; its three requirements were
promoted into
[openspec/specs/agent-runtime-workflows/spec.md](openspec/specs/agent-runtime-workflows/spec.md).
The implementation adds the supervised `ariadex` session provider and
the `sisyphusfy` run-spec supervisor to `src/agent`: ordered binary
resolution (`FORGE_ARIADEX_BIN`/`FORGE_SISYPHUSFY_BIN` env override
first — trusted only when the pin names an existing file, then the
PATH name — with the attempt list reported on absence) and bounded
argument-array invocation with null stdin so an interactive sibling
can never hijack the operator terminal (15s lifecycle probes, 900s
supervised loop). Every `start`/`pause`/`resume`/`takeover`/`restart`/
`new-session` transition on an ariadex session runs the sibling's real
verb in the project directory and the claimed state is taken only from
a bounded `ariadex status --json` probe afterwards — a live daemon
reporting AUTO/PAUSE maps to `active`/`paused`, while MANUAL, a stale
or absent daemon, an unreadable document, or a refused verb maps to
`disconnected` with the runtime's own bounded, redacted report as
evidence, never a synthesized `active`; a `--version` probe mismatch
(documented grammar `ariadex <major.minor…>`) refuses delegation as
`unsupported` naming the observed surface; `takeover` records the
`ariadex attach` operator path as guidance and never invokes the
sibling's exec-ing terminal verb; `restart`/`new-session` run
stop-then-start through the sibling's verbs. Sessions persist the
optional `backing {runtime, handle, adapter_version}` pointer (serde
default keeps every pre-change session file loadable) and `forge agent
status` renders a read-only live probe block whose unknown stored
handle surfaces `disconnected` without mutating the record. `forge
agent run-spec [TARGET] --provider sisyphusfy` hands the bound spec's
`.forge/specs/<id>/tasks.md` to the real low-level loop verb (`loop
--task-path <file> --json --adapter <provider>`, so the supervisor
itself owns the agent CLI grammar and refuses adapters it does not
know) and journals `done` only on a clean exit with `complete` plus a
`success` verification, `partial` with the sibling's named
stop_reason/blocked_reason for incomplete loops, and `unverified` —
never done — for `verification_failed`, `dry_run`, verification-less
completion, exit/document disagreement, bounded-wait overrun or an
unparsable document. Bundled `opencode`/`codex` adapters keep their
recorded-state behavior (pause/takeover stay PTY-honest `unsupported`
and spawn nothing); `sisyphusfy` is a supervisor id and never a
session provider; the MCP `run_agent` dispatch accepts the same
provider vocabulary, hoists the run-spec verdict into the shared
envelope and journals the real verdict, and the API agents route
accepts `ariadex` for peer parity; the portal and checker stay
read-only surfaces that never spawn either runtime; and every captured
runtime-output fragment passes `policy::redact_credentials` plus the
300-char evidence bound before it reaches the session file, the
transports or the journal.

## Verification evidence (supervised-agent-adapters, 2026-09-24)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 516 lib tests plus all non-skipped contract and cross-surface suites passed (50 integration binaries, 1010 tests total, 0 failures).
- New supervised suites: 10 `src/agent` unit tests (legacy session file without `backing` deserializes unchanged, backing round-trip, supervised provider vocabulary, ordered resolution env-then-PATH with attempt lists plus dead-pin fallthrough, first-JSON-document extraction across trailing `blocker` lines and duplicate-owner double documents, version-probe grammar, status mapping that never claims `active` without a live daemon across AUTO/PAUSE/MANUAL/stale/local/junk, sisyphusfy verdict classification incl. complete-with-skipped-verification → `unverified` and exit/document disagreement → `unverified`, evidence redaction and char bound); 29 `tests/supervised_agent_contract.rs` tests (help surfaces, bundled pause stays PTY-`unsupported` and spawns nothing, legacy session renders, delegated start records backing runtime/handle/version and journals them, uninitialized runtime → `disconnected` never `active`, runtime absent lists resolution attempts with empty stdout while the bundled vocabulary stays runtime-free, real pause/resume primitives with transition-history preservation, refused resume from MANUAL records the runtime's truth, takeover is attach guidance that never invokes the exec-ing verb, restart stop-then-start ordering, stale daemon → `disconnected`, a lost stored handle surfaces `disconnected` in the live block with the session file byte-identical, no-daemon live probe, malformed status document → `disconnected`, version-probe mismatch delegates nothing, credential-shaped runtime stderr redacted in transport and persisted record, supervisor `done`/`partial`/`unverified` verdicts over the real `loop --task-path ... --json` log-line contract, absent supervisor preserves the session byte-identically, ariadex session without a supervisor names the scheduler and supervisor paths, unknown supervisor refused before anything runs, sisyphusfy refused as a session provider, new-session supersession through stop-then-start, env-override precedence over PATH, and no-daemon `provider not started` honesty); 8 `tests/supervised_agent_cross_surface.rs` tests (MCP `run_agent` start delegates with the identical backing record and CLI live block, MCP run-spec with the supervisor journals the verdict, sisyphusfy refused as an MCP session provider, `tools/list` advertises no new surface, the portal counts the supervised session and spawns nothing, the doctor verdict is byte-identical across a start+pause round trip, `agent list` serializes the supervised provider, and the checker never sees sessions nor spawns either runtime).
- Live round trips (task 4.3, both providers on real installed binaries): real `ariadex` — `forge agent start --provider ariadex` against the real runtime delegated the documented verb, the real bounded `ariadex status` probe recorded the runtime's own 12-hex handle and the probed `adapter_version` (`ariadex 0.1.0+g<sha>-dirty`) with `ariadex-daemon: alive` → `active`; a real `ariadex pause` then surfaced `live: paused mode=PAUSE daemon=alive` through `forge agent status`; a real `forge agent resume` returned `active`; `ariadex stop` reconciled the daemon down and no tmux server or runtime process survived the evidence run. Real `sisyphusfy` — `forge agent run-spec --provider sisyphusfy` consumed the bound spec's tasks file through the real `loop --task-path ... --json --adapter opencode` verb under a PATH with no agent CLI installed: the real outcome document (`stop_reason: command_not_found` with the full run record) parsed to a `partial` verdict with the supervisor's named reason and the spec binding intact, without invoking any model. Live probes corrected the proposal's input-shape assumption: `run <positional>` only discovers `openspec/changes/<name>/tasks.md` or root-level task files and refuses a `.forge/specs` path with a real `{"error": "change not found"}` envelope (which the adapter classified honestly as `unverified`, session untouched), so the adapter uses the documented low-level `loop --task-path` verb — every shape recorded in `tests/fixtures/supervised/NOTES.md`.
- The native scaffold test was excluded from the aggregate command because it is an existing long-running native-toolchain integration test. Run that native test when its environment is available.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`fleet-registry-observation` implemented, verified and
archived on 2026-09-23 as
`2026-09-23-fleet-registry-observation`; canonical spec promoted to
[openspec/specs/fleet-registry-observation/spec.md](openspec/specs/fleet-registry-observation/spec.md).
The implementation adds the standalone `src/fleet` projection over
the Workspace Governance `projects.json` registry document
(`forge fleet list|status|inspect [ID] [--workspace-registry PATH]
[--max-age N]`, or `FORGE_WORKSPACE_REGISTRY`; the global
`--registry` names the local SQLite database, so the workspace
document takes its own flag): versioned `0.1.0`
`FleetReport`/`FleetEntry`/`FleetMalformedEntry` contract with a
`schema_version: 1` input contract that tolerates `workspace_root`,
the `discovery` block, absent or null `adoption`, and unknown
fields; the confinement root is the canonicalized directory of the
registry document — the untrusted document never relocates it — so
absolute-outside paths, `..` traversal and symlink or
dangling-symlink escapes refuse as malformed entries; per-entry
isolation excludes and names each malformed entry with its reason
while valid entries keep reporting; duplicate ids, unknown or
missing schema version, malformed JSON, oversized file (>1 MiB) or
entry count (>1024) and `--max-age` outside 1..=31536000 refuse at
report level with the typed `fleet-registry-invalid`; declared
profiles and lifecycles surface verbatim from the WG vocabulary and
are never coerced to Forge profiles; entries carry the
`forge.yaml` presence probe and the `managed|unmanaged` label
joined by id from the local registry only, never registering,
importing or mutating; freshness classifies `fresh|stale|unconfigured`
from the file mtime against the 86400s default window and a stale
source never renders healthy; the surface is strictly read-only
(no mirror table, no journal row, fixture trees byte-identical
across every call); the portal renders the block only for
`scope: fleet` projects views when a registry is configured —
stale rolls up to `warn`, a configured-but-unobservable source is
`unavailable`, an unconfigured one adds no entry at all; and
`forge list` appends the same normalized fleet block only when
configured, staying byte-identical otherwise.

## Verification evidence (fleet-registry-observation, 2026-09-23)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 507 lib tests plus all non-skipped contract and cross-surface suites passed (48 integration binaries, 0 failures).
- New fleet suites: 22 `src/fleet` unit tests (WG-shape parse with every tolerated field recorded, profiles verbatim never coerced, unknown/missing schema-version refusal, duplicate-id refusal naming the id, traversal and symlink-escape per-entry isolation with the rest rendering, internal `..` segments stay confined, absolute-inside-root acceptance, blank/invalid/oversized id and path malformed classes, freshness boundaries with a filesystem-backed backdating probe, unconfigured report, `--max-age` bound refusal, managed/unmanaged id join, 1 MiB and 1024-entry refusals, credential redaction with char bounding, determinism except `observed_at`/`age_seconds`, inspect found/malformed/missing paths, shared normalized entry fields, renderer required-field coverage, flag-then-env precedence); 16 `tests/fleet_contract.rs` tests (help surfaces, unconfigured exit 0 in human and JSON, clean projection with declared fields/source/timestamps, env-var selection, managed join without any registration write, stale registry never healthy, `--max-age` window flips fresh→stale, escaping entry named while alpha renders, duplicate/unknown-version/malformed-JSON refusals with typed code and empty stdout, bound validation before any read, inspect success plus malformed/unknown typed failures, credential-shaped declared fields redacted, whole fixture tree byte-identical across nine mixed fleet calls); 7 `tests/fleet_cross_surface.rs` tests (fleet reads never journal, `forge list` byte-identical unconfigured and only-appended configured with the local part unchanged, doctor/upgrade-dry-run/import identical with a fleet configured, portal fleet entries byte-equivalent to the CLI normalized projection while the unconfigured portal carries no `fleet:` entry, stale block rolls up to `warn` never `ok`, a malformed source renders `unavailable` rather than masking, and no MCP tool or API route exists for the fleet surface).
- Real sibling round trip (optional task 4.3, satisfied with live evidence): the real `workspace-governance/projects.json` on this host (66 entries, document mtime 2026-09-22) consumed the surface end to end — `forge fleet status` reports `freshness=stale` (age 129330s past the 86400s window), `entries=66`, `malformed=0`; `forge fleet list` renders every declared entry with its verbatim WG profile (`typescript-product`, `dotnet-library`, …), adoption `adopted`/`unknown`, `forge_yaml=missing` and `state=unmanaged` — honest for this document because Workspace Governance resolves entry paths against an invocation-time `--root ..` while Forge's confinement root is the registry document's directory; `forge fleet inspect actoria`, the `forge list` fleet block and `forge portal view projects` (67 fleet entries, section status `warn` while stale) all served the same Core query. Next action for consumers wanting resolved `forge.yaml` evidence from the real portfolio: keep or copy the registry document at the workspace root it inventories.
- The native scaffold test was excluded from the aggregate command because it is an existing long-running native-toolchain integration test. Run that native test when its environment is available.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`external-checker-emission` implemented, verified and
archived on 2026-09-23 as
`2026-09-23-external-checker-emission`; canonical spec promoted to
[openspec/specs/external-checker-emission/spec.md](openspec/specs/external-checker-emission/spec.md).
The implementation adds the standalone `src/checker` projection
(`forge check [TARGET] [--include-policy] [--max-alerts N]`) over the
Driftwatchdog external-checker protocol: versioned `0.1.0`
`CheckerDocument`/`CheckerAlert`/`AlertSeverity` contract with schema
discriminator `forge-checker/0.1.0`; `alerts` always present (the
sibling's forward-compatible parser refuses `{}`); severities restricted
to `error`/`warning` with `fail`→`error` and
`warn`/`unavailable`/`unverified`→`warning` naming the missing evidence
while pass/not-applicable findings are omitted; `source` project-relative
when file-scoped (absolute and traversal tokens rejected at construction,
else the plane name `doctor`/`governance`/`readiness`); `symbol` a stable
finding/policy id (`doctor/<id>`, raw `driftwatch-<RULE>` POLICY-IDs,
`governance/<provider>`, `readiness/<profile>`, `check/truncated`);
messages bounded and redacted through `policy::redact_credentials` with
the assessed project's absolute path replaced by `<project>`;
`--max-alerts` bounded (default 64, valid 1..=10000, typed
`check-invalid` outside) with explicit `check/truncated` summary naming
the dropped count; the DriftWatch policy plane is opt-in only, so a
default checker run never invokes the adapter (feedback-loop guard); a
strictly read-only surface (no journal row, no persisted observation,
manifest/registry/`.forge/`/HEAD byte-identical across runs, documents
differ only in `generated_at`); findings never alter the exit code and
only Forge-side failures write stderr with empty stdout; human and
`--format json` print byte-identical documents; and no MCP/API/portal
exposure.

## Verification evidence (external-checker-emission, 2026-09-23)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 479 lib tests plus all non-skipped contract and cross-surface suites passed (46 integration binaries, 0 failures).
- New checker suites: 15 `src/checker` unit tests (severity mapping for all four finding statuses plus applicability, all eight governance provider statuses, governance-evaluation-error degradation, planned/supported/unknown readiness projection, project-relative source confinement with absolute/traversal/URL rejection, truncation bound with dropped-count summary, message char bound, credential redaction, `<project>` path replacement, always-present `alerts` key, determinism except `generated_at`, four-required-non-empty-field invariant, policy-symbol exception for `driftwatch-config` detection); 14 `tests/checker_contract.rs` tests (help surfaces, clean project → `"alerts":[]` exit 0, format parity, mixed severities with stable symbols, missing-evidence warnings, governance plane projection, unregistered target → empty stdout + `error[unknown-project]`, truncation bound, `check-invalid` range refusal, policy plane opt-in marker proof, credential redaction through the policy plane, no-mutation byte equality with repeated-run stability, and a mirrored sibling-protocol parser accepting every emitted document and refusing `{}`); 5 `tests/checker_cross_surface.rs` tests (doctor verdict byte-equivalent across a check run, inspect record and governance observation unchanged, journal row count unchanged, MCP `tools/list` snapshot never advertises the checker, portal dashboard renders and `feature add` remains compatible).
- Real provider round trip (optional task 4.4, satisfied with live evidence): the sibling driftwatchdog binary on this host consumed the surface end to end — a `driftwatch.toml` `[[checkers]]` registration (`command = "forge"`, `args = ["check", "."]`, `env = { FORGE_REGISTRY = ... }`) run through `driftwatch check`: clean project → status `empty`, 0 alerts; after removing documentation evidence → status `success` with the persisted alert (`severity warning`, `source doctor`, `symbol doctor/docs-present`, message verbatim); with a planned profile and broken build evidence → 7 alerts, severities `error` for `doctor/features-compatible`, `doctor/build-config`, `doctor/maturity-requirements` and `warning` for `doctor/dependency-drift`, `doctor/docs-present`, `doctor/registry-observation`, `readiness/python-ai`. Alerts were read back through `driftwatch export json` with fields preserved, proving the emitted documents parse under the real sibling parser, not only the mirrored test parser. No live external governance provider was claimed.
- The native scaffold test was excluded from the aggregate command because it is an existing long-running native-toolchain integration test. Run that native test when its environment is available.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`governance-provider-contract-and-local-default` implemented, verified and
archived on 2026-09-22 as
`2026-09-22-governance-provider-contract-and-local-default`; canonical spec
promoted to
[openspec/specs/governance-provider-contract/spec.md](openspec/specs/governance-provider-contract/spec.md).
The implementation adds the standalone `local` governance provider, optional
`.forge/providers.yaml` selection, versioned `0.1.0` JSON executable-adapter
checks, bounded redacted observations, provider switching without manifest or
registry rewrites, and normalized CLI/MCP/API/portal read surfaces. No
Workspace Governance or sibling-project code is imported or required; a future
governance project can provide only the generic adapter boundary.

## Verification evidence (governance-provider-contract-and-local-default, 2026-09-22)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 464 unit tests plus all non-skipped contract and cross-surface suites passed.
- Governance contract/cross-surface tests: 11 passed; API contract/cross-surface tests: 17 passed; MCP contract/cross-surface tests: 20 passed; portal contract/cross-surface tests: 16 passed.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 29 passed, 0 failed; `git diff --check`: PASS.
- The native scaffold test was excluded from the aggregate command because it is an existing long-running native-toolchain integration test; no live sibling governance provider was claimed. Run that native test and any owning governance-project adapter fixture when those environments are available.
- No shared Gate Runtime is configured; no Gate pass is claimed.

`control-plane-portal` implemented, verified and archived on 2026-09-18
as `2026-09-18-control-plane-portal`; canonical specs promoted to
[openspec/specs/control-plane-portal/spec.md](openspec/specs/control-plane-portal/spec.md).
New in this cycle: `src/portal` (versioned
`PortalConfig`/`PortalScope`/`PortalSection` (twelve §36
sections: projects/features/components/policies/specs/
agents/deployments/repositories/documentation/analytics/
servers/settings)/`PortalEntry`/`PortalStatus`
(`ok`/`warn`/`fail`/`unknown`/`unavailable`/`partial`)/
`PortalSectionView`/`PortalDashboard`/`PortalOperation`
contract `0.1.0`; `PortalConfig::from_manifest` normalizes
the manifest's `portal:` block (default title `Forge
Control Plane`, default scope `project`), refuses empty
or oversized titles, unknown `default_scope` values and
a `title` on a disabled block with the typed
`portal-invalid` code; `parse_section` refuses unknown
section ids with `portal-invalid`; `build_dashboard`
renders all twelve sections plus the most recent
`operations` rows for one project (`scope: project`) or
the whole registry (`scope: fleet`); `build_section_view`
renders a single requested section; every section rolls
up to the worst entry status so a dashboard never masks
a `fail` / `unavailable` / `unknown` behind an `ok`
(R3 boundary); `unknown` / `unavailable` / `partial`
states are surfaced prominently with `source` and
`observed_at` on every entry; the portal is read-only —
every section carries a `controls_available` line naming
the CLI command that performs the matching mutation, and
the portal mutates nothing (R2 boundary: closing the
portal leaves every operation available through CLI and
mature MCP tools); portal calls journal one `portal`
row per `dashboard` / `view` with the real project id
(or the synthetic `__portal__` id for fleet views) so
the operations table stays project-agnostic; Core error
`portal-invalid` with stable code; CLI `forge portal
dashboard [TARGET] [--all]` and `forge portal view
<section> [TARGET]` (human/JSON); and the spec contract
from `core-http-api`, `external-planes-analytics` and
`semantic-ui-patterns` still holds after a portal round
trip on the same project (the registry journal stays
independent of the portal surface, the doctor verdict is
byte-equivalent before and after, the MCP `tools/list`
snapshot never advertises the portal surface, the
`forge feature add` workflow remains compatible, and a
fleet view invents no registered project).

`core-http-api` implemented, verified and archived on 2026-09-18
as `2026-09-18-core-http-api`; canonical specs promoted to
[openspec/specs/core-http-api/spec.md](openspec/specs/core-http-api/spec.md).
New in this cycle: `src/api` (versioned `ApiConfig`/`ApiRequest`/
`ApiResponse`/`Route`/`ApiError`/`ShutdownSignal` contract `0.1.0` over
HTTP/1.1 stdio; `ApiConfig::from_env` reads `FORGE_API_BIND` (default
`127.0.0.1`) and `FORGE_API_PORT` (default `8765`) so a non-loopback
listener is the operator's choice, never the default; the in-tree
HTTP/1.1 parser is bounded by `MAX_BODY_BYTES = 1 MiB`,
`READ_TIMEOUT = 10s` and `HANDLER_TIMEOUT = 60s` so a slow-loris
client or an unresponsive adapter cannot pin the listener; routes
mirror the brief's §35 surface
(`GET /healthz` anonymous + `GET /v1/projects` /
`POST /v1/projects` / `GET /v1/projects/{id}` /
`POST /v1/projects/{id}/doctor` /
`POST /v1/projects/{id}/features` /
`POST /v1/projects/{id}/upgrade` /
`POST /v1/projects/{id}/specs` /
`POST /v1/projects/{id}/agents` /
`POST /v1/projects/{id}/deployments` /
`GET /v1/operations/{id}`); every request (other than
`/healthz`) requires an `Authorization: Bearer <session-id>`
header where the token is a per-project OIDC admin session minted
through the `central-admin-identity` surface, the session is loaded
from `.forge/identity/<project>/sessions/<id>.json`, the
project-scoped session is checked through `validate_session`, and
a token minted for project A is refused for project B with
[`ForgeError::ApiProjectMismatch`] (R2 failure scenario); mutating
routes additionally require the session to carry `admin:access`
and `confirm: true` for the deploy / upgrade paths so an implicit
remote write is impossible; every mutating route reserves a
`pending` operation up front through
`Registry::reserve_idempotent_operation`, journals the final
`done` / `failed` state through `Registry::finalize_operation`,
and returns `202 Accepted` with the `operation_id` and a
`Location: /v1/operations/{id}` reference; the same
`Idempotency-Key` header replays the original operation id
without re-running the side effect (R2 boundary scenario) and
reusing a key with a different request body is refused with
[`ForgeError::IdempotencyKeyConflict`] (R2 failure scenario);
the API layer is a peer of the CLI and MCP journals — the
`operations` table now carries the `idempotency_key` and
`request_hash` columns plus a partial unique index, the migration
is in-place via `apply_migrations` so an existing registry with
the prior schema is upgraded automatically; the
`API_SYNTHETIC_PROJECT = "__api__"` project id keeps the
operations table project-agnostic for fleet-level routes; the
existing doctor / feature / upgrade / spec / agent / deploy / mcp
contracts still hold after an API round trip on the same project
(registry journal stays independent of the API surface, the doctor
verdict is byte-equivalent before and after, a feature added
through the API is visible to `forge feature list` / `forge
inspect`, the MCP `tools/list` snapshot does not advertise the
API surface, the API never modifies the manifest's other
sections, the typed `error[code]` envelope is rendered on
stdout for success and stderr for failure so a partial run is
always observable, a credential-shaped substring in evidence
never escapes through the API response, and stopping the API
server leaves the CLI surface fully usable); CLI `forge api
serve [--bind ADDR] [--port N] [--max-body-bytes B]` (human/JSON)
prints the loopback default + `contract: 0.1.0` banner and
returns the served connection count on graceful shutdown;
Core errors `api-invalid` / `api-unauthorized` /
`api-project-mismatch` / `idempotency-key-conflict` with stable
codes; and the spec contract from `mature-mcp-surface`,
`adapter-deployment` and `external-planes-analytics` still
holds after an API round trip on the same project.

`external-planes-analytics` implemented, verified and archived on 2026-09-18
as `2026-09-18-external-planes-analytics`; canonical specs promoted to
[openspec/specs/external-planes-analytics/spec.md](openspec/specs/external-planes-analytics/spec.md).
New in this cycle: `src/analytics` (versioned
`AnalyticsConfig`/`AnalyticsProviderConfig`/`AnalyticsProvider`
(`UnifiedContent`/`GithubAnalytics`/`Notion`/`Confluence`/
`GitlabAnalytics`/`CodebergAnalytics`)/
`ProviderSupportStatus` (`Supported`/`Planned`)/
`HealthObservation`/`ExternalPlaneReport`/`HealthObservation`/
`MetricSnapshot`/`MetricAggregate`/`ProjectMetricsReport`/
`MetricsSummary`/`DoctorSummary` contract `0.1.0`;
`AnalyticsConfig::from_manifest_meta` parses the manifest's
`analytics:` block, accepts only the bounded provider set
(`unified-content`/`github-analytics` supported;
`notion`/`confluence`/`gitlab-analytics`/`codeberg-analytics`
planned), refuses unknown providers, missing
`project_ref`, shell metacharacters in `project_ref`,
duplicate enabled providers, oversized provider lists
(`MAX_PROVIDERS_PER_PLANE = 8`), and out-of-range
`default_window_days` (`MIN_WINDOW_DAYS = 1`,
`MAX_WINDOW_DAYS = 90`) with the typed
`analytics-invalid` code; `inspect_external_planes` returns
the timestamped `available` / `disabled` / `unavailable`
(adapter) / `unavailable` (catalog, planned) /
`ambiguous-mapping` / `unconfigured` per-provider
observation; a disabled `analytics:` block or
`enabled: false` provider entry is reported as `disabled`
without invoking the adapter (R1 boundary scenario); a
planned provider is reported as `unavailable` with the
catalog source named; an adapter whose `project_ref` does
not match the manifest's is reported as
`ambiguous-mapping` so the registry never binds the
observation to another project's data (R1 failure
scenario); the adapter is invoked through the configured
`FORGE_ANALYTICS_BIN` (default
`forge-analytics-adapter`, overridable per entry) with
argument arrays and a bounded 15s wait so an unresponsive
tool cannot hang the registry; a missing binary, non-zero
exit, timeout, contract mismatch or unparseable output
surfaces as `unavailable`; credential-shaped evidence
is redacted by `redact_analytics_evidence` which delegates
to `policy::redact_credentials`; CLI `forge analytics
inspect [TARGET] [--dry-run]` (human/JSON, the
`available`/`disabled`/`unavailable`/`ambiguous-mapping`
status, the `source` discriminator, the timestamped
`observed_at` and the per-provider `evidence` are
rendered on stdout before the typed `error[...]` on
stderr so a partial run is observable); CLI `forge
analytics metrics [TARGET] [--window-days N] [--all]`
(human/JSON, the per-source `MetricSnapshot` carries the
`source`, `value`, `window` and `observed_at`); the
metrics aggregator never sums snapshots from different
windows into an authoritative total — when two snapshots
disagree on the same window or span different windows the
aggregate reports `state: mixed-windows` with
`current: null` and the note names the distinct windows
(R2 boundary scenario); a missing adapter reports the
affected metrics as `unavailable` without fabricating
zeros (R2 failure scenario); the optional
`MetricsSummary` is persisted under
`.forge/analytics/<project-id>/metrics.json` (atomic
`.tmp` + rename) so the per-project summary is
project-scoped; analytics operations journaled in the
registry's `operations` table under the `analytics` kind
with a `done` / `rejected` / `partial` verdict and the
project id (no synthetic project is invented on a
per-project call; the synthetic `__analytics__` project
id is used for `--all` so the operations table stays
project-agnostic); and the spec contract from
`agent-runtime-workflows`, `adapter-deployment` and
`repository-distribution` still holds after an
`analytics` round trip on the same project (the registry
journal stays independent of the analytics surface, the
doctor verdict is unchanged, the feature add workflow
remains compatible, the per-call `operations` row uses
the real project id, a credential-shaped substring in
analytics evidence is redacted by `redact_analytics_evidence`
which delegates to `policy::redact_credentials`, and a
disabled `analytics:` block or `enabled: false` provider
entry reports `disabled` without contacting the
provider).
New in this cycle: `src/identity` (versioned
`IdentityConfig`/`AuthChallenge`/`AuthCallback`/`ProviderClaims`/
`AdminSession`/`SessionState` (`active`/`expired`/`revoked`)/
`IdentityRejection`/`IdentityOutcome` (`Session`/`Rejected`)
contract `0.1.0`); `IdentityConfig::from_manifest` normalizes the
manifest's `identity:` block, accepts only the bounded provider
set (`okta`/`auth0`/`keycloak`/`azure-ad`/`google`/`github`/
`okta-fixture`), refuses unknown providers, cleartext issuers,
http-only redirect URIs (https / `http://localhost` /
`http://127.0.0.1` / `forge://` allowed), non-`openid` scope
lists, oversize scope/admin_values lists, out-of-range state and
session TTLs, shell metacharacters in URL fields, and a raw
secret in `client_secret_ref` (the manifest carries a
`scheme://` reference, never the secret) with the typed
`identity-invalid` code; `build_challenge` returns a fresh
PKCE `S256` authorization request with random state, nonce and
code_verifier; `validate_callback` refuses mismatched state,
expired challenge, provider-reported errors, missing codes and
shell metacharacter smuggling; `validate_claims` refuses wrong
issuer, wrong audience, expired token, mismatched nonce and
missing required scope; `mint_session` is refused with the
typed `identity-permission-denied` code when the provider
login is valid but the configured `admin_claim` value is not
in the allow list, so provider login never silently grants
admin (R1 boundary scenario); `validate_session` refuses
cross-project tokens with the typed
`identity-session-cross-project` code (R2 failure scenario),
revoked or expired sessions with `identity-session-expired`,
and missing permissions with `identity-permission-denied`;
`terminate_session` marks the session `Revoked` and the
persisted file is removed so a revoked session cannot be
re-presented; sessions are persisted under
`.forge/identity/<project>/sessions/<id>.json` (atomic `.tmp`
+ rename) so a project's session is project-scoped and never
shared with another project (R2 boundary scenario: revoking
one project's session does not implicitly revoke or validate
another project's session); the manifest's `identity:` block
is the source of truth and the validator is the only path
that produces the typed `IdentityConfig`; `redact_identity_evidence`
delegates to `policy::redact_credentials` so the identity
contract shares one definition of "secret" with the policy,
release, distribution, docs and deploy adapters; CLI `forge
identity validate-config|build-challenge|complete-auth|
session-list|session-inspect|session-validate|session-terminate`
(human/JSON, `complete-auth` accepts `--state`, `--code`,
optional `--error` / `--error-description`, `--subject`,
`--issuer`, `--audience`, `--nonce`, optional `--issued-at`
/ `--expires-at`, `--scope`, `--admin-claim-value`, and
`session-validate` accepts `--session` and `--permission`
defaulting to `admin:access`); identity operations journaled
in the registry's `operations` table under the `identity` kind
with a `done` / `rejected` verdict and the project id (no
synthetic project is invented; identity is always
project-scoped); the registry's `tools/list` snapshot stays
independent of the identity surface; the doctor verdict on the
same project is byte-equivalent after a successful
identity round trip; the `forge feature add` workflow remains
compatible after an identity round trip on the same project;
and a credential-shaped substring in evidence is redacted by
`redact_identity_evidence` which delegates to
`policy::redact_credentials`.

`ai-procedure-skills` implemented, verified and archived on 2026-09-18
as `2026-09-18-ai-procedure-skills`; canonical specs promoted to
[openspec/specs/ai-procedure-skills/spec.md](openspec/specs/ai-procedure-skills/spec.md).
New in this cycle: `src/procedure` (versioned
`ProcedureSpec`/`ProcedureStep`/`ProcedureListEntry`/
`CoreOperation` (`profile_inspect` / `profile_resolve` /
`profile_preflight` / `feature_resolve` / `feature_add` /
`feature_remove` / `feature_upgrade` / `component_resolve` /
`ui_pattern_resolve` / `ui_pattern_install` / `intent_validate`
/ `intent_resolve` / `intent_apply` / `doctor_run` / `test_run`
/ `commit` / `policy_run` / `spec_generate` / `spec_apply` /
`agent_start` / `upgrade_apply` / `upgrade_fleet` / `import_run`
/ `deploy_plan` / `deploy_apply` / `deploy_observe` /
`release_prepare` / `release_apply` / `docs_translate` /
`mirror_apply` / `report_findings`) contract v0.1.0; the catalog
ships eight named procedures (`create-project`, `upgrade-project`,
`prepare-release`, `fix-quality-findings`, `onboard-existing-project`,
`deploy-project`, `mirror-repository`, `translate-docs`) with
prerequisites, ordered Core operation steps and a verification
block; `validate_procedure` rejects unknown operations, empty
steps, non-monotonic ordinals, oversized known-issues equivalents
(bounded at `MAX_PROCEDURE_STEPS = 16` and `MAX_STEP_ARGS = 16`),
workflows without a final `report_findings` step, and any step
whose `args` carry a bypass marker (`--force`, `--skip-checks`,
`--no-validate`, `--bypass`, `--override`, `--no-doctor`,
`--ignore-failures`, matched case-insensitively and in
`--key=value` form) with the typed `procedure-bypass-refused` code
so a procedure or model that asks Core to skip a failed check is
refused (R2 failure scenario); every catalog procedure ends with a
single `report_findings` step so a workflow that ends with
unresolved gaps reports them rather than asserting completion (R2
boundary scenario); the `upgrade-project` procedure's sequence
contains a `spec_generate` step so a semantic conflict routes
through the existing remediation surface (R2 success scenario);
the catalog and validator carry no agent-provider, IDE or model
identifier, so a provider change is a no-op for the procedure
layer (R1 boundary scenario); CLI `forge procedure list|inspect|
validate` (human/JSON, `inspect` accepts a kebab-case id and
returns the immutable spec, `validate` accepts `--path` to a JSON
file and refuses with the typed `procedure-invalid` /
`procedure-bypass-refused` codes); procedure operations journaled
in the registry's `operations` table under the `procedure` kind
with a `done` / `rejected` verdict and a synthetic
`__procedure__` project id that keeps the operations table
project-agnostic; and the spec contract from `mature-mcp-surface`,
`validated-intent-planner`, `feature-lifecycle`,
`specification-remediation` and `doctor-maturity-assessment`
still holds after a procedure list / inspect / validate on the
same project (the registry journal stays independent of the
procedure surface, the doctor verdict is byte-equivalent before
and after a `procedure inspect` / `validate` refusal, feature
add keeps working on a project whose procedure journal has been
recorded, the upgrade procedure's `spec.generate` handoff is
discoverable, and a credential-shaped substring in evidence is
never constructed by the procedure layer).

`validated-intent-planner` implemented, verified and archived on 2026-09-18
as `2026-09-18-validated-intent-planner`; canonical specs promoted to
[openspec/specs/validated-intent-planner/spec.md](openspec/specs/validated-intent-planner/spec.md).
New in this cycle: `src/planner` (versioned
`Intent`/`IntentAction` (`create_project`/`extend_project`)/
`IntentConstraint`/`ValidatedIntent`/`AssemblyPlan`/`PlanStep`/
`PlanStepKind` (`doctor`/`test`/`quality_policy`/`install_feature`/
`install_component`/`install_ui_pattern`)/`UnresolvedWork`/
`IntentValidationOutcome`/`IntentResolveOutcome`/`IntentApplyOutcome`/
`AppliedStep` contract v0.1.0); `validate_intent` accepts only
`create_project` / `extend_project` actions, refuses empty profile,
capability counts above `MAX_CAPABILITIES_PER_INTENT = 32`,
required-and-forbidden intersection, unknown profile, unknown
constraint key, unknown capability, and a client-only profile
paired with a server-side capability (the rejection names the
recommended client/backend boundary, e.g. `flutter-app + rust-web
or python-service backend`, R1 failure scenario); the validated
intent retains the public constraint and the billing prohibition
in the normalized form (R1 success scenario); an ambiguous
required-architectural-choice request is surfaced as a typed
`IntentAmbiguous` rejection so the planner never silently
selects a profile or capability (R1 boundary scenario);
`resolve_plan` pins every step to the profile version captured
at validation time, the captured `intent_hash` and `catalog_hash`,
and a deterministic plan id `<profile>-<8hex(intent_hash)>-<8hex(catalog_hash)>`;
a re-resolve of the same intent produces the same plan id; a
compatible request schedules dependency-ordered
`install_feature` / `install_component` / `install_ui_pattern`
steps from the certified supported parts, then a profile-pinned
`test`, `quality_policy` and `doctor` gate (R2 success
scenario); the executor re-checks `profile_version`, `intent_hash`
and `catalog_hash` before any step so a drifted profile, drifted
intent, or drifted catalog surfaces as a typed `PlanStale`
error and writes no file (R2 failure scenario); requirements
with no deterministic descriptor become bounded `UnresolvedWork`
entries with a glue/business/spec hint and the executor records
them as `unresolved` so a missing deterministic part is
observable rather than masked by an AI substitution (R2
boundary scenario); `write_plan_receipt` persists
`.forge/planner/<plan-id>/plan.json` (atomic via `.tmp`+rename)
and `apply_plan` requires explicit `--confirm` so an implicit
project mutation is impossible; CLI `forge intent
validate|resolve|apply|list` (human/JSON, `resolve` accepts
`--action` `--profile` plus repeatable `--require`,
`--forbid`, `--constraint KEY=VALUE` and optional `--path`,
`apply` accepts the plan id plus `--confirm` and optional
`--path`, `list` accepts a project directory and a `--format`);
planner operations journaled in the registry's `operations`
table under the `planner` kind with a `done`/`rejected`
verdict and a synthetic `__planner__` project id that keeps
the operations table project-agnostic without inventing a
user-visible project; and the spec contract from
`semantic-component-registry`, `semantic-ui-patterns`,
`feature-lifecycle`, `doctor-maturity-assessment` and
`quality-policy-integration` still holds after a `validate` /
`resolve` / `apply` on the same project (the registry journal
stays independent of the planner surface, the doctor verdict
is unchanged byte-for-byte, feature add/remove/upgrade and
the ownership receipt contract still hold, and a
credential-shaped substring in planner evidence is never
constructed by the resolver).
as `2026-09-18-semantic-ui-patterns`; canonical specs promoted to
[openspec/specs/semantic-ui-patterns/spec.md](openspec/specs/semantic-ui-patterns/spec.md).
New in this cycle: `src/ui_pattern` (versioned
`UiPatternDescriptor`/`UiPatternState`/`UiPatternTypography`/
`UiPatternSpacing`/`UiPatternResponsive`/`UiPatternAccessibility`/
`UiPatternInteraction`/`UiPatternEvidence`/`UiPatternAdapter`/
`UiPatternQuality` (`experimental`/`verified`/`certified`/`deprecated`)/
`UiPatternRequest`/`UiPatternPlan`/`UiPatternStep`/`UiPatternRejection`/
`UiPatternEvidenceSummary`/`UiPatternResolveOutcome`/
`UiPatternInstallRequest`/`UiPatternInstallOutcome` contract v0.1.0;
`validate_descriptor` rejects programming primitives and generic
template placeholders (`if`, `loop`, `screenshot`, `html-fragment`,
`copy-paste`, `lorem-ipsum`, etc), unknown intents outside the
bounded vocabulary (`login`, `register`, `forgot-password`,
`dashboard`, `crud-table`, `filter-bar`, `form`, `settings`,
`profile`, `billing`, `empty-state`, `success-page`, `error-page`,
`modal`, `confirm-dialog`, `file-upload`, `navigation`), missing
state/typography/spacing/responsive/accessibility/interaction
contracts, copied-markup fragments (the catalog refuses
`<html>`/`<!doctype html>`/base64 PNG/JPEG headers so a
screenshot is never a verified pattern), and patterns without a
tested platform adapter; every catalog entry must declare all
required states (`loading`, `error`, `success`,
`form_validation`, `empty`, `keyboard_focus`) so a copied markup
fragment is refused at validation time; the catalog ships
seventeen tested entries (`login`, `register`, `forgot-password`,
`dashboard`, `crud-table`, `filter-bar`, `form`, `settings`,
`profile`, `billing`, `empty-state`, `success-page`, `error-page`,
`modal`, `confirm-dialog`, `file-upload` (experimental),
`navigation`) plus the deprecated test entry `webhook-receiver`;
`form` ships for `react-web`, `nextjs-web` and `flutter-app`
while `billing` only ships for the two web profiles, and
`empty-state`, `error-page`, `modal` and `navigation` ship for
all three; `validate_request` refuses empty profile, empty id
list, duplicate ids and programming primitives before any
catalog lookup with a typed `ui-pattern-invalid` error;
`resolve_patterns` is deterministic from the request, the
catalog and the profile, prefers the compatible `Certified`
candidate when multiple candidates satisfy the same id, and
reports the planner's evidence summary so the operator can
audit why a candidate was preferred (R1 success scenario); a
profile-incompatibility request surfaces a typed
`ui-pattern-unsupported-platform` rejection listing the tested
platforms (R1 boundary: a web-only pattern refuses `flutter-app`
rather than substituting copied web markup); an unknown id
surfaces a typed `ui-pattern-invalid` rejection that names
the missing catalog entry (R1 failure scenario); a request
whose only compatible candidate is `Deprecated` is refused with
a typed `ui-pattern-quality-conflict` rejection so the
planner never silently selects a deprecated pattern; the
resolver is the reviewable plan owner — it does not execute a
side effect, and the `UiPatternRejection.code` field carries
the typed error code so partial runs are observable on stdout
before the human output renders the summary; `install_pattern`
writes the adapter's ordinary source artifact (real React
JSX/Next.js TSX for web profiles, real Flutter Dart for
`flutter-app`) plus a separate `install.json` receipt under
`.forge/ui-patterns/<id>/` and refuses to overwrite a
customized file with a typed `ui-pattern-ownership-conflict`
(R2 failure scenario); an install on a profile with no
adapter surfaces a typed `ui-pattern-unsupported-platform`
rejection; the installed artifact is ordinary source with the
documented export (e.g. `function Form(...)`, `class Form extends
StatelessWidget`) so a `forge` removal after the install leaves
the project compiling through the project native toolchain
(`npm run build`, `next build`, `flutter build`) — the R2
boundary check verifies the source file is byte-identical to
the expected artifact after the receipt is dropped; CLI `forge
ui-pattern list|inspect|resolve|install` (human/JSON,
`resolve` accepts `--profile <p>` plus repeatable
`--pattern <id>` and renders the per-step evidence plus
rejection code, `install` accepts `--profile <p> --reason <r>
[--path <dir>]`); UI pattern operations journaled in the
registry's `operations` table under the `ui_pattern` kind with
a `done` / `rejected` (resolve) or `done` / `blocked` (install)
verdict and a synthetic `__ui_pattern__` project id that keeps
the operations table project-agnostic without inventing a
user-visible project; and the spec contract from
`semantic-component-registry`, `feature-lifecycle` and
`project-upgrade-orchestration` still holds after a
`ui-pattern` resolve and install on the same project (the
registry journal stays independent of the UI surface, the
doctor verdict is unchanged byte-for-byte, feature add/remove/
upgrade and the ownership receipt contract still hold, and a
credential-shaped substring in evidence is never constructed
by the resolver).

`semantic-component-registry` implemented, verified and archived on 2026-09-17
as `2026-09-17-semantic-component-registry`; canonical specs promoted to
[openspec/specs/semantic-component-registry/spec.md](openspec/specs/semantic-component-registry/spec.md).
New in this cycle: `src/component` (versioned
`ComponentDescriptor`/`ComponentContract`/`ComponentPort`/
`ComponentQuality` (`experimental`/`verified`/`certified`/`deprecated`)/
`ComponentEvidence`/`ComponentRequest`/`ComponentPlan`/`ComponentStep`/
`ComponentRejection`/`ComponentEvidenceSummary`/
`ComponentResolveOutcome`/`ComponentQualifyRequest`/
`ComponentQualifyEvidence`/`ComponentQualifyOutcome` contract v0.1.0;
`validate_descriptor` rejects programming primitives (`if`,
`loop`, `try-catch`, `string-concat`, `addition`, etc) and
incomplete shells (no semantic purpose, no inputs, no
outputs, no tested profile mapping, no install strategy, no
tests, no documentation, unnamed or undescribed ports, or
a known-issues list that exceeds `MAX_KNOWN_ISSUES = 16`)
with a typed `component-invalid` error and the missing
criterion named; the catalog ships fifteen tested entries
that map to the brief's §11 surface (`paginated-query`,
`idempotency-guard`, `validated-form`, `audit-action`,
`soft-delete`, `retry-external-call`, `require-permission`,
`api-mutation`, `loading-state`, `error-boundary`,
`confirm-dialog`, `empty-state`, `toast`, `file-picker`,
`webhook-receiver` (deprecated)) and each entry declares
its `depends_on` (linked to features), its
`profiles` (the supported stack implementations), a
deterministic install strategy, validators, documentation
and tests, plus a quality level and the underlying
`ComponentEvidence` (usage count, test coverage,
`last_verified`, `known_issues`, `security_review`); the
`paginated-query` and `idempotency-guard` candidates ship
for `rust-web` and `python-service` so two stacks
implement the same semantic capability while preserving
their own implementation and exposing the shared contract
(R1 boundary scenario); `validate_request` refuses
empty profile, empty id list, duplicate ids and
programming primitives before any catalog lookup with a
typed `component-invalid` error; `resolve_components` is
deterministic from the request, the catalog and the
profile, prefers the compatible `Certified` candidate
when multiple candidates satisfy the same id and reports
the planner's evidence summary so the operator can audit
why a candidate was preferred (R2 success scenario); a
profile-incompatible request surfaces a typed
`component-invalid` rejection listing the tested profiles
(R1 boundary scenario); an unknown id surfaces a typed
`component-invalid` rejection that names the missing
catalog entry (R1 failure scenario); a request whose only
compatible candidate is `Deprecated` is refused with a
typed `component-quality-conflict` rejection so the
planner never silently selects a deprecated descriptor
(R2 boundary scenario); the resolver is the
reviewable plan owner — it does not execute a side
effect, and the `ComponentRejection.code` field carries
the typed error code (`component-invalid` /
`component-quality-conflict`) so partial runs are
observable on stdout before the human output renders
the summary; `qualify_component` gates promotion to
`Certified` on `security_review == true`, `test_coverage
>= 0.85`, a fresh `last_verified` (within 180 days) and a
`known_issues` list at or under `MAX_KNOWN_ISSUES`; a
refused promotion preserves the prior quality level and
writes no receipt so the catalog's prior state stays
intact (R2 failure scenario); `record_qualification`
writes the accepted promotion to
`.forge/components/<id>/qualify.json` (parent directory
created on demand) and skips the write on refusal or
when the target quality is `Deprecated`; CLI `forge
component list|inspect|resolve|qualify` (human/JSON,
`resolve` accepts `--profile <p>` plus repeatable
`--component <id>` and renders the per-step evidence,
`qualify` accepts `--to <quality> --reason <r>` plus
optional `--coverage`, `--last-verified`, repeated
`--known-issue`, `--security-review` and `--path` so a
test fixture can target a temp project directory
without sharing the current working directory);
component operations journaled in the registry's
`operations` table under the `component` kind with a
`done` / `rejected` (resolve) or `done` / `blocked`
(qualify) verdict and a synthetic `__component__`
project id that keeps the operations table
project-agnostic without inventing a user-visible
project; and the spec contract from `feature-lifecycle`
and `project-upgrade-orchestration` still holds after
a component resolve or a qualify run on the same
project (the registry journal stays independent of the
component surface, the doctor verdict is unchanged,
feature add/remove/upgrade and the ownership receipt
contract still hold, and a credential-shaped substring
in evidence is never constructed by the resolver).

`adapter-deployment` implemented, verified and archived on 2026-09-17
as `2026-09-17-adapter-deployment`; canonical specs promoted to
[openspec/specs/adapter-deployment/spec.md](openspec/specs/adapter-deployment/spec.md).
New in this cycle: `src/deploy` (versioned
`DeployConfig`/`DeployTargetSpec`/`DeployHealthSpec`/`DeployRequest`/
`DeployPlan`/`DeployReport`/`DeployStageOutcome`/`HealthObservation`/
`DeployState`/`DeployIdentity`/`DeployListEntry` contract v0.1.0;
`DeployConfig::from_manifest_meta` parses the manifest's
`deployment` block, accepts the typed
`deployment.targets[].kind` (one of `local` / `docker-compose`
/ `ssh` planned), refuses duplicate target names,
artifact paths that lexically resolve outside the
project, an empty deployment block, and the mixing of
typed `targets[]` with the legacy `deployment.type` +
`deployment.target`; `prepare_deploy` captures the named
target, the working-tree revision, the artifact identity
(path + content hash + byte size) and the configured
health check into a `DeployPlan` whose `ready` verdict
is true only when the chosen target has a usable
artifact (or the adapter does not need one) and the
working tree carries a git HEAD; `apply_deploy` refuses
without `--confirm` (typed `deploy-invalid`), walks the
per-target adapter invocation through the configured
`FORGE_DEPLOYER_BIN` binary (default
`forge-deployer`, overridable via environment variable)
with argument arrays plus a bounded 60s wait so an
unresponsive target cannot hang the registry, captures
the timestamped health observation and persists a
`DeployState` under
`.forge/deploy/<project-id>/<deploy-id>/state.json`
(via atomic write, `.tmp` + rename) so a successful
run overwrites the prior state and a failed run leaves
the last good observation intact; a missing binary,
non-zero exit, timeout, contract mismatch or unparseable
output surfaces as `deploy-target-unavailable` with the
typed reason and the prior state is left untouched
(R1 + R2 boundary); the per-stage JSON envelope
carries the per-target outcome so a partial run is
observable on stdout before the typed
`error[deploy-health-failed]` is rendered on stderr;
`DeployIdentity` is derived from
`<project>-<target>-<12-hex-sha>` so a different
revision or target cannot silently reuse the previous
deploy record (R2 boundary: disconnected means unknown,
not offline proof); `observe_deploy` re-runs the health
check on a previously applied deploy and updates the
same `state.json` with the new observation timestamp
(the last successful observation is preserved as
`last_observation` and the new value is reported as
`current_state` so a transient unreachable target does
not overwrite a known running deployment); credential-
shaped evidence is redacted by
`redact_deploy_evidence` which delegates to
`policy::redact_credentials` (the same redaction the
policy / release / distribution / docs adapters
consume); CLI `forge deploy plan|apply|observe|list|
inspect [TARGET] [--target-name NAME] [--confirm]
[--dry-run]` (human/JSON); deploy operations journaled
in the registry's `operations` table under the
`deploy` kind with a `done`/`partial`/`blocked`
verdict; and the spec contract from `release-publishing`
still holds after a deploy run on the same project
(registry journal stays independent of the deploy
surface, doctor verdict is unchanged, project
isolation and redaction rules are preserved).
New in this cycle: `src/release` (versioned `Semver`/`ReleaseConfig`/
`ReleaseIdentity`/`ReleaseRequest`/`ReleaseReport`/`ReleaseState`/
`StageOutcome` contract v0.1.0; `Semver::parse` rejects empty,
non-triple, non-numeric, leading-zero and invalid-prerelease
versions and the manifest's `release.versioning` accepts only
`semver` so an unsupported versioning scheme can never reach
the apply path; `ReleaseConfig::from_manifest_meta` refuses
duplicate, empty or unknown `release.checks` kinds, package
paths and Dockerfile paths that lexically resolve outside the
project, and an empty changelog; the release id is derived
from `<project>-<semver>-<12-hex-sha>` so a different revision
or version cannot silently reuse the previous release record;
`prepare_release` captures the working-tree revision, the
changelog content hash and a bounded excerpt, the configured
doctor/test/DriftWatch checks (each bound to the captured
revision so a stale plan cannot be applied after the tree has
moved on) and the enabled `docs.translations.<locale>` locales
(R1 boundary: a project without any configured locale omits the
docs stage entirely) into a `PlanReport` whose `ready` verdict
is true only when every applicable check passed at the captured
revision; `apply_release` re-runs the captured checks and
refuses without `--confirm` (typed `release-invalid`), marks the
plan not ready with `release-check-failed` when any applicable
check fails, is unavailable or is stale, and walks every
selected stage — `commit`, `tag`, `push`, `mirror`, `package`,
`container`, `docs`, `notes` — with stable statuses
`delivered`/`skipped`/`disabled`/`failed`/`conflict`; the tag
stage is annotated at the captured revision and refuses to
replace an existing tag that points at a different commit
(R2 boundary: `release-identity-conflict` evidence named
verbatim, recovery points to `git tag -d` and to releasing at
a different semver); a previously-delivered stage on the same
revision is reported as `skipped` so retries do not redo work
the registry has already observed; the package, container and
notes adapters are external binaries (defaults
`forge-package-publisher` / `forge-container-publisher` /
`forge-notes-renderer`, overridable via `FORGE_PACKAGE_BIN` /
`FORGE_CONTAINER_BIN` / `FORGE_NOTES_BIN`) invoked with
argument arrays and a bounded per-run timeout so an
unresponsive provider cannot hang the registry; a missing
binary, non-zero exit, timeout or empty receipt surfaces as
`failed` for that stage while the prior `ReleaseState` and
prior stage outcomes stay intact; `ReleaseState` is persisted
under `.forge/release/<project-id>/<release-id>/state.json`
via atomic write (`.tmp` + rename) so a successful run
overwrites the prior state and a failed run leaves it
untouched; the mirror stage reuses the `distribution` contract
so a project with a `distribution` block performs a real
`forge mirror` and a project without one reports the mirror
stage as `disabled`; the docs stage reuses the
`documentation-translation` contract so a project with
enabled translation locales performs a real `forge docs
translate` per locale and a project without configured locales
reports the docs stage as `disabled`; credential-shaped
evidence is redacted by `redact_release_evidence` which
delegates to `policy::redact_credentials`; CLI `forge release
prepare|apply|list|inspect` (human/JSON, the per-stage
`evidence` is rendered on stdout before the typed exit-code
error so a partial run is observable); release operations
journaled in the registry's `operations` table under the
`release` kind with a `done`/`blocked`/`partial` verdict; and
the spec contract from `repository-distribution` and
`documentation-translation` still holds after a release run on
the same project (the mirror and docs stages reuse the same
Core contracts and the registry journal remains independent
of the release surface).

`documentation-translation` implemented, verified and archived on 2026-09-17
as `2026-09-17-documentation-translation`; canonical specs promoted to
[openspec/specs/documentation-translation/spec.md](openspec/specs/documentation-translation/spec.md).
New in this cycle: `src/docs` (versioned `DocsConfig`/
`LocaleConfigEntry`/`TranslateRequest`/`TranslateReport`/
`TranslateOutcome`/`LocaleFreshness`/`FreshnessStatus`/`ReviewStatus`
contract v0.1.0; manifest `docs.source` defaults to `README.md` and
`docs.translations.<locale>.enabled` defaults to `false` so
translation is never enabled by default; locales are validated
against a strict language-tag grammar that refuses separators,
`..`, and embedded paths so a locale can never smuggle a path
into the state layout; `run_translate` plans and applies one
explicit locale or `--all` enabled locales, reuses unchanged
segments by content hash, requests only changed segments from
the provider, merges translations in source order with code
blocks reinserted verbatim, revalidates the source hash after
the provider returns (a source edit during generation is
reported as `failed` without overwriting the prior derivative),
preserves link destinations and explicit non-translatable
terms (a violation marks the derivative `needs-review` instead
of claiming translation quality from provider success), and
writes the state to `.forge/docs/<locale>/state.json`); the
provider is an external binary (default `forge-docs-translator`,
overridable via `FORGE_DOCS_TRANSLATOR_BIN`) invoked with an
argument array — never a shell — and a per-run timeout, with
`spawn` + bounded wait so an unresponsive tool cannot hang the
registry; a missing binary, non-zero exit, timeout, contract
mismatch or unparseable output surfaces as `failed` while the
prior derivative and state stay intact; CLI `forge docs
translate [LOCALE] [--all] [--project PATH]` (human/JSON, the
typed `error[docs-invalid]` / `error[translation-failed]`
errors render on stderr and the per-locale outcome JSON
prints to stdout on partial failure so a partial run is
observable); a derivative path that resolves to the source
file or outside the project is refused before any write, a
disabled locale is refused on explicit request and skipped on
`--all` without invoking the provider, and a credential-shaped
substring in evidence is redacted by `redact_docs_evidence`
which delegates to `policy::redact_credentials`; doctor
exposes `docs-<locale>` and `docs-freshness` findings with
`pass`/`warn`/`fail` derived from `assess_freshness` (read-
only: stale when the recorded source hash no longer matches,
`never-translated` when the derivative is absent, `needs-review`
when the recorded review state names violations, and `misconfigured`
when the source or derivative path is broken), so the existing
doctor contract still holds after a successful run; and the spec
contract from `repository-distribution` still holds after a
docs-translate run on the same project (the registry journal
remains independent of the docs surface).
as `2026-09-17-repository-distribution`; canonical specs promoted to
[openspec/specs/repository-distribution/spec.md](openspec/specs/repository-distribution/spec.md).
New in this cycle: `src/distribution` (versioned
`DistributionConfig`/`MirrorConfigEntry`/`MirrorProvider`
`github`+`gitee` supported, `gitlab`+`codeberg` planned/
`MirrorSupportStatus`/contract v0.1.0 over JSON-RPC 2.0 stdio;
`DistributionConfig::from_manifest_meta` validates the
manifest's `distribution` block, refuses duplicate enabled
mirrors and unknown providers, and defaults the first
declared mirror to the remote name `mirror-<provider>`;
`plan_mirror` and `apply_mirror` drive `forge mirror` end
to end, with per-remote `MirrorRemoteOutcome` carrying role
`primary` or `mirror` and stable statuses `delivered`/
`skipped`/`disabled`/`diverged`/`unavailable`/`failed`;
`MirrorState` persisted under
`.forge/distribution/<project-id>/state.json` so a
`--retry-failed` re-push only fires when the local HEAD SHA
differs from the previously delivered SHA (already-delivered
refs at the same SHA are reported as `skipped`); diverging
mirror history surfaces as `diverged` with a recovery
guidance and refuses `--force`; a disabled mirror is reported
as `disabled` without contacting the remote (R1 boundary);
a partial primary+mirror run records each remote independently
so the primary's `delivered` is never misreported when a
mirror fails; credential-shaped evidence is redacted by
`redact_distribution_evidence` which delegates to
`policy::redact_credentials` (the same redaction the policy
adapter consumes); Core errors `distribution-invalid`/
`mirror-disabled`/`mirror-diverged`/`mirror-credentials` with
stable codes; CLI `forge mirror [TARGET] --ref REF --confirm
[--dry-run] [--retry-failed]` (human/JSON, the per-remote
evidence is printed to stdout before the typed exit-code
error so partial runs are observable); MCP `mirror_project`
tool classified as `external_write` and dispatched through
the same Core contracts the CLI uses; mirror operations
journaled in the registry's `operations` table under the
`mirror` kind with a `done`/`partial` verdict; and the spec
contract from `agent-runtime-workflows` still holds after a
mirror run on the same project (the push `confirm`-required
guard and the registry journal remain independent of the
distribution surface).
as `2026-09-17-mature-mcp-surface`; canonical specs promoted to
[openspec/specs/mature-mcp-surface/spec.md](openspec/specs/mature-mcp-surface/spec.md).
New in this cycle: `src/mcp` (versioned `McpToolDescriptor`/
`McpToolKind` (`ReadOnly` / `Mutating` / `ExternalWrite`) /
`McpRequest` / `McpResponse` / `McpRpcError` contract v0.1.0 over
JSON-RPC 2.0 stdio; tool registry of sixteen mature operations
(`list_projects`/`inspect_project`/`list_profiles`/
`inspect_profile`/`list_features`/`run_doctor` read-only,
`create_project`/`import_project`/`add_feature`/`remove_feature`/
`upgrade_feature`/`generate_spec`/`run_agent`/`run_tests`/
`commit` mutating, `push` external-write) with each tool's
declared JSON Schema and stable contract version; one
`tools/list` RPC and sixteen tool RPCs, each dispatched through
the same Core contracts the CLI uses, with the JSON-RPC
response carrying the full `data` envelope and the diagnostic
stream carrying a redaction-safe summary keyed on the tool
name so a credential embedded in a failed Core call does not
leak through stderr; `create_project` rejects shell
metacharacters in the project id before any file is created
or registry row is written (treated as literal data, never
executed as shell code); `push` is refused with the
`push-confirm-required` data code when `confirm` is missing
or `false`, so an implicit remote write is never accepted;
`deploy` / `publish` / `release` / `mirror` / `docs` are
intentionally absent from the registry because their Core
operations are not implemented yet (R1 + R2 boundary
scenarios); Core errors `mcp-invalid` / `mcp-unauthorized`
with stable codes; CLI `forge mcp serve` (human, the
JSON-RPC response stream is the output; `tools/list` and the
sixteen tool RPCs are the input/output contract); MCP
operations journaled in the registry's `operations` table
under the `mcp` kind; and the spec contract from
`agent-runtime-workflows` still holds after the new
transport serves an inspect request and the CLI surface
returns the equivalent domain record).

`agent-runtime-workflows` implemented, verified and archived on 2026-09-17
as `2026-09-17-agent-runtime-workflows`; canonical specs promoted to
[openspec/specs/agent-runtime-workflows/spec.md](openspec/specs/agent-runtime-workflows/spec.md).
New in this cycle: `src/agent` (versioned `AgentSession`/`AgentProvider`/
`SessionState`/`SessionTransition`/`TransitionRecord` contract v0.1.0
covering start/pause/takeover/resume/restart/new-session; bundled
OpenCode/Codex adapters return explicit `unsupported` for
pause/takeover because the existing PTY manager is not wired into
this build, with the recovery note naming the integration point;
session storage under `.forge/agents/<id>/` carrying
`session.json` and `transitions.log`; `run_spec` refuses with
`error[spec-invalid]` when the bound spec is missing, leaving
the session file preserved so the boundary scenario is
observable) and `src/gitops` (versioned `TestOutcome`/
`CommitOutcome`/`PushOutcome` contract v0.1.0; profile-aware
`test` runs the descriptor's `cargo test`/`npm test`/etc. and
surfaces a non-zero exit as `error[test-failed]` with the
captured stdout/stderr; `commit --path X --message M` stages
only the listed paths via `git add --` and refuses when the
working tree has tracked edits outside the requested paths,
preserving untracked files; `push --remote X --ref-name Y
--confirm` requires explicit confirmation and refuses
implicit remote writes with `error[push-confirm-required]`),
Core errors `agent-unavailable`/`agent-unsupported`/
`test-failed`/`git-dirty`/`push-confirm-required` with stable
codes, CLI `forge agent start|pause|takeover|resume|restart|
new-session|status|list|run-spec` and `forge test|commit|push`
(human/JSON), operations journaled in the registry's
`operations` table with the `agent` kind, and the spec
contract from `specification-remediation` still holds after a
session that ends without verification preserves its
`session.json` with the original spec id.

`specification-remediation` implemented, verified and archived on 2026-09-17
as `2026-09-17-specification-remediation`; canonical specs promoted to
[openspec/specs/specification-remediation/spec.md](openspec/specs/specification-remediation/spec.md).
New in this cycle: `src/spec` (versioned `SpecRequest`/`SpecDraft`/
New in this cycle: `src/spec` (versioned `SpecRequest`/`SpecDraft`/
`SpecProvenance`/`SpecGenerateOutcome`/`SpecStatus` contract v0.1.0 with
provenance covering project id, path, profile, source revision (manifest
mtime), finding ids/categories, DriftWatch policy ids and dependencies;
storage layout `.forge/specs/<project>-<short-hash>/` carrying
`proposal.md` / `design.md` / `tasks.md` / `manifest.json`; bounded
proposal size enforced at `MAX_FINDINGS_PER_SPEC = 32`; `generate_spec`
is idempotent on the same project + sorted finding set so an unchanged
finding re-run reports the existing spec without rewriting files
(boundary scenario); `SpecRoute` distinguishes `Deterministic`,
`Semantic` and `Manual` queues and `apply_routing` requires the
deterministic action's evidence (or generated spec, or recorded manual
state) before reporting completion; `route_finding` keeps the doctor
finding's `Manual` remediation class as a judgment call (no project
changes, no AI claim) and routes any finding whose id matches a
known lifecycle action to `Deterministic`; Core errors
`spec-invalid`/`spec-write-failed` with stable codes, CLI
`forge spec generate|list|inspect|route|apply` (human/JSON, structured
`error[spec-invalid]` for empty/oversized/ambiguous requests and the
existing `feature-ownership-conflict` handoff from `project-upgrade-orchestration`
now resolves through `forge spec apply semantic-<feature>` writing the
bounded proposal without changing files), the `SpecRequest` is
validated before any write so a refusal leaves the project untouched,
and existing doctor/upgrade/feature/import/generate/quality-policy
contracts still hold after the new commands run against a generated
project.

`quality-policy-integration` implemented, verified and archived on 2026-09-17
as `2026-09-17-quality-policy-integration`; canonical specs promoted to
[openspec/specs/quality-policy-integration/spec.md](openspec/specs/quality-policy-integration/spec.md).
New in this cycle: `src/policy` (versioned `DriftWatchConfig`/default
binary `driftwatch` overridable via `FORGE_DRIFTWATCH_BIN`, bounded
`Command::new` + argument-array invocation with per-run `wait_timeout`
so an unresponsive tool cannot hang the registry; `PolicyReport`/`PolicyFinding`/`PolicySeverity`
contract v0.1.0 with custom deserialization that accepts `info`/`ok` as
aliases for `pass`; project-scoped execution with `current_dir(dir)` and
`--project <dir>`; `PolicyOutcome::Reported` / `Unavailable` so a
missing binary, non-zero exit, timeout or unparseable JSON surfaces as
an `unavailable` finding instead of `pass`; `redact_credentials` and
`redact_report_in_place` covering AWS / GitHub / GitLab / Slack / JWT /
private-key / `key=value` shapes and run on every consumed report as
defense in depth; `observation_is_stale` keyed on `forge.yaml` and
configured driftwatch-file mtimes), Core `PolicyUnavailable`
(`policy-unavailable`), CLI `forge doctor` invokes the adapter and
threads the outcome through `run_doctor(..., Some(&outcome))` with
per-rule `driftwatch-<id>` findings, `driftwatch-policy` rollup and
not-applicable policies preserved with their reason and
`applicable: false`, and existing doctor contract still holds (no
registry/observation/dependency-drift regression on the unchanged
fixtures).

`extended-profile-catalog` implemented, verified and archived on 2026-09-17
as `2026-09-17-extended-profile-catalog`; canonical specs promoted to
[openspec/specs/extended-profile-catalog/spec.md](openspec/specs/extended-profile-catalog/spec.md).
New in this cycle: `ProfileSupportStatus` (`Supported` / `Planned`) on
`ProfileDescriptor`, the sixth supported profile `react-web`
(`adapter-react`, typescript, `npm@20` 0.1.0) with a tested native-buildable
template (`forge new --profile react-web` renders
`forge.yaml`/`README.md`/`index.html`/`src/main.js`/`src/app.test.mjs`/
`package.json`/`scripts/build.mjs`/`vite.config.js`/`Dockerfile`/
`.gitignore`; build `npm run build`, test `npm test`), and seven reserved
specialist candidates (`aspnet-saas`, `flutter-client`, `nextjs-content`,
`python-ai`, `python-data`, `rust-cli`, `rust-worker`) discoverable through
`inspect_profile` with `support_status: planned` but refused by
`resolve_profile` / `preflight_profile` / `generate` with a new
`ForgeError::UnsupportedProfile` (code `unsupported-profile`) before any
file change; the original five MVP ids and their semantics stay intact,
react-web refuses server-side capabilities with the existing backend
boundary hint, `flutter-client` description names the backend boundary
('rust-web'/'python-service') it needs, and CLI `forge profile inspect`
shows the support status plus description in human and JSON output.

`project-upgrade-orchestration` implemented, verified and archived on 2026-09-17
as `2026-09-17-project-upgrade-orchestration`; canonical specs promoted to
[openspec/specs/project-upgrade-orchestration/spec.md](openspec/specs/project-upgrade-orchestration/spec.md).
New in this cycle: `src/upgrade` (pinned
[`UpgradePlan`](src/upgrade/mod.rs) with old/new versions, kind-ordered
steps, asset list, validators, migration strategy and recovery
implications; `apply_upgrade` precondition sweep journals a `blocked`
`upgrade` row and emits a structured `SemanticConflict` naming the
owned file and the suggested `forge spec generate` follow-up;
already-satisfied upgrades are a no-op with no file, manifest or
registry write; missing requested features install at the tested
version; `run_fleet` snapshots the explicit registry selection,
journals each project with `done`/`failed`/`blocked`/`skipped` states,
isolates per-project failures and reports a healthy verdict only when
nothing failed or blocked; retry re-plans from the current manifest so
completed steps are not blindly repeated; `postgres` steps are marked
irreversible with declared strategy
`manifest-repin+manual-schema-review`), Core `record_operation`
append-only journal, CLI `forge upgrade [TARGET] [--feature FEATURE]
[--all] [--dry-run]` (human/JSON, structured `error[unknown-feature]`
and `error[feature-ownership-conflict]`, fleet owns its exit code
without the generic error path), and existing feature-lifecycle
contracts still hold after fleet upgrades (admin depends on auth, both
reach 0.1.0 in dependency order with all manifest sections preserved).

`feature-lifecycle` implemented, verified and archived on 2026-09-16
as `2026-09-16-feature-lifecycle`; canonical specs promoted to
[openspec/specs/feature-lifecycle/spec.md](openspec/specs/feature-lifecycle/spec.md).
New in this cycle: `src/feature` (18-descriptor versioned catalog at
tested `0.1.0` with compatibility derived from the MVP profile
descriptors, dependencies, conflicts, install/upgrade strategies,
validation policies, docs and tests; dependency-ordered deterministic
plans with exact versions; add/remove/upgrade through manifest-only
edits preserving all other sections plus deterministic `.forge/features`
ownership receipts; reverse-dependency and user-edit preflight blocks
with full preservation; atomic writes with restore-on-validation-failure;
registry refresh so source/manifest/registry agree), Core errors
`unknown-feature`/`incompatible-feature`/`feature-ownership-conflict`,
CLI `forge feature list|inspect|resolve|add|remove|upgrade`
(human/JSON, stable `error[code]` diagnostics), and `forge new`
dependency-closure selection (`--feature admin` records `auth`+`admin`)
while preserving the existing `incompatible-profile` contract.

`doctor-maturity-assessment` implemented, verified and archived on 2026-09-16
as `2026-09-16-doctor-maturity-assessment`; canonical specs promoted to
[openspec/specs/doctor-maturity-assessment/spec.md](openspec/specs/doctor-maturity-assessment/spec.md).
New in this cycle: `src/doctor` (read-only finding inventory with stable
rule IDs, PASS/WARN/FAIL/UNAVAILABLE statuses, evidence, applicability and
automatic/AI/manual remediation classes; versioned L0-L4 maturity policy
descriptors with target-gated applicability so L0 prototypes are respected;
registry-observation staleness via manifest-mtime comparison; unknown profile
reported as unavailable findings, never a hard refusal), Core
`--target L0..L4` parsing (`parse_target_level`), CLI `forge doctor [<path>]
[--target]` (human/JSON, exit 0 with `healthy:false` on FAIL/UNAVAILABLE/
unmet/stale; hard errors only for path/manifest IO via existing codes), and
`healthy` defined as no FAIL/UNAVAILABLE, no unmet applicable controls and
no stale observation.

`core-manifest-registry` implemented, verified and archived on 2026-09-16
as `2026-09-16-core-manifest-registry`; canonical specs promoted to
[openspec/specs/core-manifest-registry/spec.md](openspec/specs/core-manifest-registry/spec.md).
Rust workspace `forge` 0.1.0: `src/core` (schema-v1 `forge.yaml`
validation, legacy `platform.yaml` only via explicit `--manifest`),
`src/registry` (SQLite via bundled rusqlite, unique id/canonical-path,
nullable observations, pending→done/failed operation journal with
open-time reconciliation), `src/main.rs` (thin CLI: `list`, `inspect`,
`register`, human/JSON output, stable `error[code]` diagnostics, exits
0/1/2). Foundation decisions recorded in
[ADR 0001](docs/adr/0001-foundation-toolchain.md); build commands recorded
in [README.md](README.md).

`profile-registry` implemented, verified and archived on 2026-09-16 as
`2026-09-16-profile-registry`; canonical specs promoted to
[openspec/specs/profile-registry/spec.md](openspec/specs/profile-registry/spec.md).
New in this cycle: `src/profile` (five versioned MVP descriptors with
capabilities, packages, layout, conventions, build/test commands,
deployment defaults and quality policies; `list`/`inspect`/`resolve`/`preflight`;
descriptor validation naming the missing field), Core errors
`unknown-profile`/`invalid-profile`/`incompatible-profile`/`toolchain-missing`,
CLI `forge profile list|inspect|resolve|preflight` (human/JSON, stable
`error[code]` diagnostics), and `register` gating on profile resolution
plus feature compatibility before any row mutation.

`project-import` implemented, verified and archived on 2026-09-16 as
`2026-09-16-project-import`; canonical specs promoted to
[openspec/specs/project-import/spec.md](openspec/specs/project-import/spec.md).
New in this cycle: `src/import` (read-only detection of language,
framework, package manager, database, Docker, CI, auth, features,
DriftWatch, Git remote and deployment with unknown-vs-missing evidence
and suggested profile/maturity; `ambiguous-import` on profile or
monorepo-root disagreement until `--profile` selects; minimal validated
manifest written only on `--accept` after a registry identity
pre-check, with rollback on registration failure and no legacy
 doubling), Core errors `ambiguous-import`/`import-conflict`, CLI
`forge import [<path>] [--profile] [--accept] [--id]` (human/JSON,
stable `error[code]` diagnostics), and `Registry::check_identity_available`
for mutation-free collision checks.

## Verification evidence (core-http-api, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  11 new `src/api` lib unit tests for the HTTP/1.1 request
  parser (method, path, query, headers, body with
  `MAX_BODY_BYTES = 1 MiB` cap, `Authorization: Bearer`
  extraction, `Idempotency-Key` extraction, body too large
  refusal), routing for every published path
  (`/healthz`, `/v1/projects`, `/v1/projects/{id}`,
  `/v1/projects/{id}/doctor`, `/v1/projects/{id}/features`,
  `/v1/projects/{id}/upgrade`, `/v1/projects/{id}/specs`,
  `/v1/projects/{id}/agents`,
  `/v1/projects/{id}/deployments`,
  `/v1/operations/{id}` plus the `405 method-not-allowed`
  alt-method fallthrough), the `RequiredPermission` table
  (`admin:access` for every mutating route, `None` for
  read routes), `ApiConfig::from_env` binding loopback by
  default and honoring `FORGE_API_BIND` / `FORGE_API_PORT`,
  the `err_status` mapping (401 for `api-unauthorized` and
  session-expired, 403 for `api-project-mismatch` and
  `identity-permission-denied`, 409 for
  `idempotency-key-conflict` and the deploy / push
  confirm-required codes, 400 for the validation codes,
  500 otherwise), the `request_hash` determinism
  (`sha256(method \n path \n body)` is stable per
  request and changes when the method changes), and the
  `api/healthz` route never demanding a bearer token;
  11 new `tests/api_contract.rs` contract tests for the
  CLI subcommand help (top-level `api` mention, the
  `forge api serve` help mentioning the loopback
  default), the `forge api serve` lifecycle (port
  binding on `127.0.0.1`, graceful stop, no extra
  process drift), `GET /healthz` returning 200 without
  authorization, `POST /v1/projects/{id}/doctor` returning
  the same findings the CLI surfaces, missing-token /
  invalid-token requests refused with the typed
  `api-unauthorized` code, a session minted for project A
  presented to project B refused with the typed
  `api-project-mismatch` code (R2 failure scenario), the
  `mutating` route recording an `operation_id` that is
  retrievable through `GET /v1/operations/{id}`, the
  `Idempotency-Key` replay reusing the same `operation_id`
  with `replay: true` (R2 boundary scenario), the
  `Idempotency-Key` conflict refusing a key reused with a
  different body via the typed `idempotency-key-conflict`
  code (R2 failure scenario), an unknown route
  returning 404, the wrong method on a known path
  returning 405, and the doctor command still working
  after the API server is stopped (R1 boundary
  scenario); 6 new `tests/api_cross_surface.rs`
  regression tests for the API journal row carrying the
  real project id and a `done` / `failed` state without
  ever inventing a synthetic project on a per-project
  call, the doctor verdict being byte-equivalent before
  and after an API request on the same project, a
  feature added through the API being visible to
  `forge inspect` and to the manifest on disk so the
  feature ownership receipt contract is preserved, two
  consecutive doctor calls through the API returning
  the same findings on the same input, the MCP
  `tools/list` snapshot staying unchanged after an API
  round trip (the API does not advertise itself through
  MCP), and a credential-shaped bearer token never
  appearing verbatim in the response body; plus the
  unchanged 33 test binaries (401 lib tests incl. 11 new
  api unit tests, 11 api contract, 6 api
  cross-surface, 17 analytics contract, 6 analytics
  cross-surface, 12 procedure contract, 8 procedure
  cross-surface, 12 planner contract, 7 planner
  cross-surface, 33 ui_pattern contract, 5 ui_pattern
  cross-surface, 15 component contract, 5 component
  cross-surface, 14 release contract, 4 release
  cross-surface, 12 documentation contract, 4
  documentation cross-surface, 12 distribution contract,
  4 distribution cross-surface, 4 agent-runtime-
  workflows cross-surface, 8 agent contract, 8 gitops
  contract, 13 mcp contract, 9 mcp cross-surface, 8 doctor
  contract, 10 feature contract, 12 generate contract,
  10 import contract, 9 profile contract, 7 quality
  policy contract, 12 upgrade contract, 12 spec
  contract, 5 CLI contract, 4 cross-surface regression,
  19 identity contract, 6 identity cross-surface, 10
  deploy contract, 4 deploy cross-surface).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate core-http-api --strict
  --no-interactive`: valid pre-archive; `openspec
  archive core-http-api --yes`: archived as
  `2026-09-18-core-http-api` with the canonical
  `spec/core-http-api` promoted; `openspec validate
  --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive, includes the promoted
  `spec/core-http-api`).
- `git diff --check`: PASS; staged set reviewed
  (3 files modified: `src/core/mod.rs` for the new
  `ApiInvalid` / `ApiUnauthorized` / `ApiProjectMismatch`
  / `IdempotencyKeyConflict` typed errors, `src/lib.rs`
  to register the new module, `src/main.rs` for the
  `forge api` subcommand, the `ApiCommands` enum and the
  `cmd_api_serve` helper, `src/registry/mod.rs` for the
  new `idempotency_key` / `request_hash` columns plus
  the `OperationEntry::idempotency_key` /
  `OperationEntry::request_hash` fields and the
  `reserve_idempotent_operation` /
  `finalize_operation` / `operation` /
  `operation_by_idempotency` helpers and the in-place
  `apply_migrations` upgrader; 3 files added:
  `src/api/mod.rs` with 11 unit tests,
  `tests/api_contract.rs` with 11 contract tests,
  `tests/api_cross_surface.rs` with 6 cross-surface
  regression tests; plus the promoted spec and the
  change archive — 7 files; archive under
  `openspec/changes/archive/2026-09-18-core-http-api/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- A live HTTP/1.1 server round trip is exercised
  end-to-end through the contract tests; the in-process
  parser is bounded by `MAX_BODY_BYTES`, `READ_TIMEOUT`
  and `HANDLER_TIMEOUT` so a malicious or slow caller
  cannot pin the listener. The `Authorization` token
  flows through the same `crate::identity` validators
  the CLI consumes, and the registry's existing
  `operations` table is the only place the API
  publishes operation state. A real external client
  (e.g. a portal deployment) is the downstream
  integration step and is not claimed here.

## Verification evidence (external-planes-analytics, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  26 new analytics lib unit tests for `parse_provider` (supported
  set and unknown-provider refusal), `provider_support_status`
  (Supported vs Planned), `AnalyticsConfig::from_manifest_meta`
  (well-formed block, unknown provider refusal, missing
  `project_ref`, shell metacharacter `project_ref`,
  duplicate provider, oversized provider list, out-of-range
  window, absent block), `inspect_external_planes` (disabled
  master block, disabled per-entry boundary, planned provider
  catalog source, dry-run without adapter invocation, missing
  adapter, failing adapter, ambiguous-mapping project_ref
  refusal), `aggregate_project_metrics` (counter metrics
  with windows, missing repository evidence, observations
  from the same window agree, mixed-windows refusal,
  out-of-range window refusal), `MetricsSummary` round-trip
  through disk, `redact_analytics_evidence` delegating to
  the policy redactor, and the human renderers
  (`render_report_human` / `render_metrics_human`) carrying
  the required fields; 17 new analytics CLI contract tests
  for the help output, the top-level help including the
  `analytics` subcommand, `inspect` accepting a well-formed
  block under `--dry-run`, `inspect` refusing a missing
  block, an unknown provider, a missing `project_ref`, a
  shell-metacharacter `project_ref`, a duplicate provider,
  a disabled master block (boundary, no adapter contact),
  a planned provider (catalog source, `unavailable`),
  a missing adapter (`unavailable` with evidence), a
  failing adapter (`unavailable` with evidence), an
  adapter with a different `project_ref`
  (`ambiguous-mapping` refusal), a consistent adapter
  (`available` with `evidence: stars=42`), and a leaky
  adapter whose evidence carries a credential-shaped
  substring (the secret is redacted to `[REDACTED]` in
  stdout); 6 new analytics cross-surface regression tests
  for the registry's `analytics` journal row keeping the
  operations table independent of the analytics surface,
  the doctor verdict on the same project being byte-
  identical after a successful analytics round trip, a
  credential-shaped substring in evidence being redacted
  through `redact_analytics_evidence` (which delegates to
  `policy::redact_credentials`), the `forge feature add`
  workflow remaining compatible after an analytics round
  trip on the same project, the R1 boundary (a disabled
  `analytics:` block or `enabled: false` provider entry is
  reported as `disabled` without contacting the provider
  and renders no evidence line), and the R2 boundary
  (snapshots from different windows or disagreeing
  values surface as `mixed-windows` with `current: null`
  rather than silently summing across windows); plus the
  unchanged existing test binaries (390 lib tests incl. 26
  new analytics unit tests, 17 analytics contract, 6
  analytics cross-surface, plus the unchanged 33
  pre-existing test binaries).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  smoke-analytics` then appending an `analytics:` block to
  the manifest; `forge analytics inspect <proj> --dry-run`
  returns the per-provider `available` observation with
  the `dry-run` source named and the would-invoke evidence
  line; `forge analytics inspect <proj>` without an
  adapter on PATH reports both providers as `unavailable`
  with the adapter error in the evidence and the journal
  row recorded as `rejected`; `FORGE_ANALYTICS_BIN=…/good.sh
  forge analytics inspect <proj>` against a fixture
  adapter that prints
  `{"project_ref":"owner/repo","evidence":["stars=42","growth=3"],...}`
  reports both providers as `available` with the
  evidence lines and the journal row recorded as `done`;
  `forge analytics metrics <proj>` against the same
  project aggregates the local counters
  (`projects=1`, `quality-healthy=0`, …) plus the
  `repository-stars=42` and `repository-stars-growth=3`
  with the per-source `observed_at` and the
  `window=7-day` boundary; `forge analytics metrics --all`
  walks the registered projects and records the journal
  under the synthetic `__analytics__` project id; `forge
  doctor <proj>` before and after the analytics round trip
  produces the byte-identical verdict so the existing
  doctor contract still holds; `forge analytics inspect`
  on a project without an `analytics:` block exits 1 with
  `error[analytics-invalid]: analytics invalid: project
  \`no-analytics\` has no \`analytics:\` block; declare one
  in forge.yaml to enable the external content / analytics
  plane`; and a string scan over the `analytics` module
  returns no agent-provider / IDE / model identifier, so
  an agent provider change is a no-op for the analytics
  layer (R1 boundary).
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate external-planes-analytics --strict
  --no-interactive`: valid pre-archive; `openspec
  archive external-planes-analytics --yes`: archived as
  `2026-09-18-external-planes-analytics` with the
  canonical `spec/external-planes-analytics` promoted;
  `openspec validate --all --strict --no-interactive`: 24
  passed, 0 failed (post-archive, includes the promoted
  `spec/external-planes-analytics`).
- `git diff --check`: PASS; staged set reviewed (3 files
  modified: `src/core/manifest.rs` for the new
  `AnalyticsMeta` typed fields, `src/core/mod.rs` for the
  `analytics-invalid` / `analytics-adapter-unavailable` /
  `analytics-mapping-ambiguous` typed errors, `src/lib.rs`
  to register the new module, `src/main.rs` for the `forge
  analytics` subcommand, the `AnalyticsCommands` enum and
  the `cmd_analytics_*` helpers; 3 files added:
  `src/analytics/mod.rs` with 26 unit tests,
  `tests/analytics_contract.rs` with 17 contract tests,
  `tests/analytics_cross_surface.rs` with 6 cross-surface
  regression tests; plus the promoted spec and the change
  archive — 7 files; archive under
  `openspec/changes/archive/2026-09-18-external-planes-analytics/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  `forge-analytics-adapter` binary is not present in the
  local sandbox, so the contract is validated through
  `FORGE_ANALYTICS_BIN` fixture shell scripts that stand
  in for a real `unified-content` / `github-analytics`
  round trip. The credential redaction rule set is the
  same as `policy::redact_credentials`, which is itself
  verified through the existing quality policy contract
  tests. A real provider round trip is a downstream
  integration step and is not claimed here.

## Verification evidence (central-admin-identity, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  50 new identity lib unit tests for the `IdentityConfig`
  validator (well-formed block, audience defaults to
  `client_id`, empty / unknown / cleartext provider,
  cleartext issuer, missing `openid` scope, empty admin
  values, state / session TTL range, raw secret in
  `client_secret_ref`, shell metacharacters in
  `redirect_uri`, bad project id, oversized scope list);
  `build_challenge` shape stability and randomness across
  repeated calls; `validate_callback` accepting the matching
  state within TTL, rejecting mismatched state, expired
  challenge, provider error, cross-project state and shell
  metacharacters in the code; `validate_claims` accepting a
  matching token and rejecting wrong issuer, wrong audience,
  expired token, mismatched nonce and missing required scope;
  `admin_claim_grants` mapping to the allow list;
  `mint_session` refusing a non-admin claim with
  `identity-permission-denied` and accepting an admin claim;
  `validate_session` accepting an active session, rejecting
  cross-project tokens with `identity-session-cross-project`,
  expired sessions, revoked sessions and missing permissions;
  `terminate_session` flipping state to `Revoked`; the path
  helper rejecting non-hex session ids; the save/load
  round-trip and the project-mismatch save refusal;
  `load_session` returning `None` for a missing file;
  `list_sessions` returning sessions in stable id order and
  an empty list for a missing directory; the R2 boundary
  (revoking project A's session does not affect project B's
  session); `redact_identity_evidence` delegating to the
  policy redactor; the human renderers
  (`render_challenge_human` / `render_session_human` /
  `render_outcome_human`) carrying the required fields; and
  the stability of the bounded provider list and the
  supported code-challenge method set
  (`SUPPORTED_PROVIDERS` and `SUPPORTED_CODE_CHALLENGE_METHODS`);
  19 new identity CLI contract tests for the help output,
  the top-level help including the `identity` subcommand,
  `validate-config` accepting a well-formed block and
  rejecting an unknown provider, a cleartext issuer, a
  missing `openid` scope, a raw secret in
  `client_secret_ref` and a missing `identity:` block;
  `build-challenge` returning random state / nonce /
  code_verifier with the `S256` method; `complete-auth`
  minting a session for the configured admin-claim value,
  refusing a value outside the allow list with
  `identity-permission-denied` (R1 boundary) and refusing a
  state mismatch with `identity-invalid` (R1 failure);
  `session-list` returning the minted sessions in stable
  order; `session-inspect` returning the full session
  payload; `session-validate` granting `admin:access` for
  an active session, refusing a cross-project token with
  `identity-session-cross-project` (R2 failure) and refusing
  a missing session id with `identity-session-not-found`;
  `session-terminate` revoking the session and removing the
  file (so a terminated session cannot be re-presented);
  and `complete-auth` redacting a credential-shaped
  substring in the evidence; 6 new identity cross-surface
  regression tests for the registry's `identity` journal
  row keeping the operations table independent of the
  identity surface (the identity surface never invents a
  registered project), the doctor verdict being unchanged
  after a successful identity round trip, a
  credential-shaped substring in evidence being redacted
  through `redact_identity_evidence` (which delegates to
  `policy::redact_credentials`), the `forge feature add`
  workflow remaining compatible after an identity round
  trip on the same project, the R2 boundary (revoking one
  project's session does not affect another project's
  session) and the R1 boundary (a user authenticated at
  the provider but lacking the configured `admin_claim`
  value is refused with `identity-permission-denied` so
  provider login never silently grants admin); plus the
  unchanged 33 test binaries (414 lib tests incl. 50 new
  identity unit tests, 19 identity contract, 6 identity
  cross-surface, 12 procedure contract, 8 procedure
  cross-surface, 12 planner contract, 7 planner
  cross-surface, 33 ui_pattern contract, 5 ui_pattern
  cross-surface, 15 component contract, 5 component
  cross-surface, 14 release contract, 4 release
  cross-surface, 12 documentation contract, 4
  documentation cross-surface, 12 distribution contract,
  4 distribution cross-surface, 4 agent-runtime-workflows
  cross-surface, 8 agent contract, 8 gitops contract, 13
  mcp contract, 9 mcp cross-surface, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality policy
  contract, 12 upgrade contract, 12 spec contract, 5 CLI
  contract, 4 cross-surface regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  identity-smoke` then appending an `identity:` block to
  the manifest; `forge identity validate-config
  <proj>` returns `contract: 0.1.0` with the
  `provider=okta`, `client_id=forge-admin`,
  `audience=forge-admin`, `admin_claim=groups`,
  `state_ttl_seconds=120` and `session_ttl_seconds=3600`
  fields; `forge identity build-challenge <proj>` returns
  the random 32-byte hex state / nonce / code_verifier plus
  the S256 code challenge; `forge identity complete-auth
  <proj> --state <s> --code abcd1234 --subject user-1
  --nonce <n> --admin-claim-value forge-admins` mints a
  session carrying `permissions: ["admin:access"]`,
  `state: "active"` and the
  `.forge/identity/identity-smoke/sessions/<id>.json`
  file; `forge identity complete-auth <proj> --state <s>
  --code abcd1234 --subject intern --nonce <n>
  --admin-claim-value interns` exits 1 with
  `error[identity-permission-denied]: identity permission
  denied: ... provider login for subject `intern` does not
  include the project admin permission; admin_claim
  `groups` value `interns` is not in the configured allow
  list; provider login does not imply admin authorization`
  (R1 boundary); `forge identity complete-auth <proj>
  --state deadbeef --code abcd1234 --error access_denied
  --error-description "token
  ghp_abcdefghijklmnopqrstuvwxyz0123456789 was used"
  --subject user-1 --nonce deadbeef --admin-claim-value
  forge-admins` exits 1 with
  `error[identity-invalid]` and the credential-shaped
  secret is redacted to `[REDACTED]` in stderr; `forge
  identity session-list <proj>` renders the persisted
  sessions in stable id order; `forge identity
  session-validate <other-proj> --session <a-id>` for a
  session minted under project A exits 1 with
  `error[identity-session-cross-project]: identity
  session cross-project: session `<id>` was minted for
  project `identity-smoke`; presenting it to project
  `other-proj` is refused; sessions are project-scoped
  and may not be shared across unrelated applications`
  (R2 failure); `forge identity session-terminate
  <proj> --session <id>` marks the session `revoked`,
  writes `terminated: true` to stdout, removes the
  persisted file (so a terminated session cannot be
  re-presented), and re-validating the same id exits 1
  with `error[identity-session-not-found]`; `forge
  doctor <proj>` before and after the identity round
  trip produces the byte-identical doctor verdict so
  the existing doctor contract still holds; the
  registry's `operations` table records the `identity`
  journal rows with the real project id (no synthetic
  project is invented; identity is always
  project-scoped); and a string scan over the `identity`
  module returns no agent-provider / IDE / model
  identifier, so an agent provider change is a no-op
  for the identity layer.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate central-admin-identity --strict
  --no-interactive`: valid pre-archive; `openspec
  archive central-admin-identity --yes`: archived as
  `2026-09-18-central-admin-identity` with the canonical
  `spec/central-admin-identity` promoted; `openspec
  validate --all --strict --no-interactive`: 24 passed,
  0 failed (post-archive, includes the promoted
  `spec/central-admin-identity`).
- `git diff --check`: PASS; staged set reviewed (3 files
  modified: `src/core/manifest.rs` for the new
  `IdentityMeta` typed fields, `src/core/mod.rs` for the
  `identity-invalid` / `identity-auth-failed` /
  `identity-session-expired` / `identity-session-not-found`
  / `identity-session-cross-project` / `identity-permission-denied`
  typed errors, `src/lib.rs` to register the new module,
  `src/main.rs` for the `forge identity` subcommand, the
  `IdentityCommands` enum and the `cmd_identity`
  helpers; `Cargo.toml` for the new `rand` dependency
  used by the PKCE / state / nonce generator; 3 files
  added: `src/identity/mod.rs` with 50 unit tests,
  `tests/identity_contract.rs` with 19 contract tests,
  `tests/identity_cross_surface.rs` with 6 cross-surface
  regression tests; plus the promoted spec and the
  change archive — 9 files; archive under
  `openspec/changes/archive/2026-09-18-central-admin-identity/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  OIDC provider round trip is not present in the local
  sandbox, so the contract is validated through fixture
  claims, fixture callbacks and the typed rejection
  codes. The redaction rule set is the same
  `policy::redact_credentials` consumed by every other
  adapter, which is itself verified through the existing
  quality policy contract tests. A real provider round
  trip is a downstream integration step and is not
  claimed here.

## Verification evidence (semantic-ui-patterns, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS; 22 new
  ui_pattern unit tests for catalog id stability and shape
  (17 patterns + 1 deprecated test entry, every entry declares
  the bounded intent and every required state, every adapter
  ships ordinary source at the documented path), primitive
  and placeholder rejection (programming primitives plus
  `screenshot`/`html-fragment`/`copy-paste`/`lorem-ipsum`),
  descriptor validation (missing state contract, missing
  required state, unknown intent, primitive id, HTML fragment
  artifact, empty adapters, oversized known issues), the typed
  `UiPatternRejection` codes (ui-pattern-invalid for unknown
  ids, ui-pattern-unsupported-platform for the Flutter
  boundary on a web-only pattern, ui-pattern-quality-conflict
  for the deprecated-only request), quality ranking
  (certified preferred over verified/experimental, deprecated
  excluded from the ranking), the install path (writes
  `src/ui/<id>.tsx` for `react-web`/`nextjs-web` and
  `lib/ui/<id>.dart` for `flutter-app`, plus
  `.forge/ui-patterns/<id>/install.json` with the per-pattern
  evidence summary), the ownership conflict (install refuses
  to overwrite a customized file, leaves the file and the
  receipt untouched), the unsupported-platform refusal on
  install, the unknown-id refusal on install, the
  byte-identical idempotent re-install, the Flutter-app
  install, and the human renderers
  (`render_plan_human` / `render_outcome_human` /
  `render_install_human`); 33 new ui_pattern CLI contract
  tests for the help output, the catalog list in human and
  JSON, the `inspect` contract (typed states, typography,
  spacing, responsive, accessibility, interaction, adapters,
  evidence, certified quality), `resolve` with certified
  candidates for `react-web` and the typed
  `ui-pattern-unsupported-platform` rejection for `flutter-app`
  on a web-only pattern (R1 boundary), the typed
  `ui-pattern-invalid` rejection for an unknown id, the typed
  exit-1 refusal for the programming primitive `if` and the
  placeholder `screenshot`, the per-state and per-adapter
  presence checks for every catalog id, the deprecated
  `webhook-receiver` quality marker, the `ui-pattern-quality-conflict`
  for a deprecated-only request, the `install` write of a
  real React `Form` component plus receipt, the `install`
  write of a real Flutter `Form` widget, the
  ownership-conflict refusal preserving the customized file
  and skipping the receipt write, the
  unsupported-platform refusal on install with the typed
  code, the unknown-id refusal on install with the typed
  code, and the R2 boundary (dropping the receipt keeps the
  ordinary source on disk so the project continues to build
  through the native toolchain); 5 new ui_pattern
  cross-surface tests for the registry `ui_pattern` journal
  row keeping the operations table independent of the UI
  surface, the doctor verdict staying byte-identical after a
  `ui-pattern resolve` plus `install` on the same project,
  the `forge feature add` workflow remaining compatible
  after a `ui-pattern install` on the same project, the R1
  boundary (a web-only pattern refuses `flutter-app` with the
  typed `ui-pattern-unsupported-platform` rejection and never
  substitutes copied web markup into the Flutter project),
  and the R2 boundary (the installed source artifact survives
  receipt removal so Forge is not on the build path); plus
  the unchanged existing 25 test binaries (273 lib tests, 33
  ui_pattern contract, 5 ui_pattern cross-surface, 15
  component contract, 5 component cross-surface, 10 release
  contract, 4 release cross-surface, 12 documentation
  contract, 4 documentation cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4
  agent-runtime-workflows cross-surface, 8 agent contract, 8
  gitops contract, 13 mcp contract, 9 mcp cross-surface, 8
  doctor contract, 10 feature contract, 12 generate
  contract, 10 import contract, 9 profile contract, 7
  quality policy contract, 12 upgrade contract, 12 spec
  contract, 5 CLI contract, 4 cross-surface regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile react-web --id ui-smoke`
  then `forge ui-pattern list` shows the catalog with the 17
  ids in stable order plus the deprecated `webhook-receiver`;
  `forge ui-pattern inspect form` renders the contract
  (eight states including the six required ones, the
  typography/spacing/responsive/accessibility envelopes, the
  three tested adapters, the certified evidence with
  coverage 0.92 and `security_review: true`); `forge
  ui-pattern resolve --profile react-web --pattern login
  --pattern form` returns the certified plan with the
  per-step evidence summary; `forge ui-pattern resolve
  --profile flutter-app --pattern billing` returns the typed
  `ui-pattern-unsupported-platform` rejection listing the
  tested `react-web, nextjs-web` platforms so the operator
  sees the boundary instead of a silently substituted copy;
  `forge ui-pattern resolve --profile react-web --pattern if`
  exits 1 with
  `error[ui-pattern-invalid]: ui pattern invalid: ui pattern
  'if' is a programming primitive or a generic template
  placeholder; the registry refuses to model language
  constructs or copied markup fragments`; `forge ui-pattern
  install form --profile react-web --reason "studio needs
  the standard form" --path <proj>` writes
  `src/ui/form.tsx` carrying the real `function Form(...)` /
  `data-state=` contract plus
  `.forge/ui-patterns/form/install.json` with
  `pattern_id: form`, `profile: react-web`, `quality:
  certified`, and the eight named states; `forge ui-pattern
  install form --profile flutter-app --reason ... --path
  <proj>` writes `lib/ui/form.dart` carrying `class Form
  extends StatelessWidget` plus the `Semantics(label:
  'form', ...)` accessibility surface; `forge ui-pattern
  install form --profile react-web --reason "should refuse"
  --path <proj>` on a project with a pre-existing
  `src/ui/form.tsx` carrying a user edit exits 1 with
  `error[ui-pattern-ownership-conflict]` and writes neither
  the artifact nor the receipt; `forge ui-pattern install
  billing --profile flutter-app --reason ... --path <proj>`
  exits 1 with
  `error[ui-pattern-unsupported-platform]` and writes no
  state file; the byte-identical R2 boundary check drops the
  receipt and confirms `function Form(...)` and `import {
  useState } from 'react';` remain on disk; and `forge
  doctor <proj>` before and after the install run produces
  the byte-identical verdict so the existing doctor contract
  still holds.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate semantic-ui-patterns --strict
  --no-interactive`: valid pre-archive; `openspec archive
  semantic-ui-patterns --yes`: archived as
  `2026-09-18-semantic-ui-patterns` with the canonical
  `spec/semantic-ui-patterns` promoted; `openspec validate
  --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive, includes the promoted
  `spec/semantic-ui-patterns`).
- `git diff --check`: PASS; staged set reviewed (3 files
  modified: `src/core/mod.rs` for the new
  `UiPatternInvalid` / `UiPatternUnsupportedPlatform` /
  `UiPatternQualityConflict` / `UiPatternDeprecatedDep` /
  `UiPatternOwnershipConflict` typed errors, `src/lib.rs` to
  register the new module, `src/main.rs` for the `forge
  ui-pattern` subcommand, the `UiPatternCommands` enum and
  the `cmd_ui_pattern` helpers; 3 files added:
  `src/ui_pattern/mod.rs` with 22 unit tests,
  `tests/ui_pattern_contract.rs` with 33 contract tests,
  `tests/ui_pattern_cross_surface.rs` with 5 cross-surface
  regression tests; plus the promoted spec and the change
  archive — 7 files; archive under
  `openspec/changes/archive/2026-09-18-semantic-ui-patterns/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  `npm run build`, `next build` or `flutter build` round
  trip is not present in the local sandbox, so the R2
  boundary is validated through the contract
  `install_pattern`/`inspect_ui_pattern` round trip and the
  `forge doctor` byte-equality check; the installed source
  is ordinary React JSX/Next.js TSX/Flutter Dart with the
  documented export, so the project continues to compile
  through its native toolchain after Forge is removed. A
  real `npm run build` / `next build` / `flutter build`
  round trip is a downstream integration step and is not
  claimed here.

## Verification evidence (semantic-component-registry, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  23 new component unit tests for catalog id stability and
  shape, programming-primitive rejection, descriptor
  validation (missing inputs / outputs / profiles / install
  strategy / tests / documentation / port name or
  description, oversized known-issues), the typed
  `ComponentRejection` codes (component-invalid for
  unknown ids, profile-incompatibility, primitive ids;
  component-quality-conflict for deprecated-only), quality
  ranking (certified preferred over experimental/verified,
  deprecated excluded from the ranking), the `qualify`
  evidence gate (security review required, test coverage
  >= 0.85, fresh `last_verified` within 180 days),
  receipt-write idempotence (no receipt on refusal, no
  receipt for `Deprecated` target, no receipt for
  already-at-target), and the human renderers
  (`render_plan_human` / `render_outcome_human` /
  `render_qualify_human`); 15 new component CLI contract
  tests for the help output, the catalog list in human
  and JSON, the `inspect` contract (typed inputs, typed
  outputs, evidence, certified quality), `resolve` with
  certified candidates for `rust-web` and the typed
  `component-invalid` rejection for `flutter-app` /
  `nextjs-web`, the typed `component-quality-conflict`
  rejection for a deprecated-only request, the typed
  `error[component-invalid]` exit-1 refusal for a
  programming primitive id, the UI-component-on-server
  and server-component-on-UI profile boundaries, the
  `qualify` refusal without a security review (no
  receipt written, prior quality preserved), the
  `qualify` refusal on stale verification, the
  `qualify` acceptance with complete evidence
  (receipt written with the correct path and the
  certified target), the `qualify` refusal on an
  unknown target quality, and the no-side-effect
  contract on refusal (no `.forge` directory created);
  5 new component cross-surface tests for the registry
  `component` journal row keeping the operations table
  independent of the component surface, the doctor
  verdict staying unchanged after a successful resolve,
  the `forge feature add` workflow remaining
  compatible after a `component resolve` on the same
  project, the `paginated-query` per-stack boundary
  (rust-web / python-service installable, nextjs-web /
  flutter-app refused with `component-invalid`), and
  the typed rejection codes rendering on stdout so a
  partial run is observable; plus the unchanged 25 test
  binaries (251 lib tests, 15 component contract, 5
  component cross-surface, 10 release contract, 4
  release cross-surface, 12 documentation contract, 4
  documentation cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4
  agent-runtime-workflows cross-surface, 8 agent
  contract, 8 gitops contract, 13 mcp contract, 9 mcp
  cross-surface, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract,
  9 profile contract, 7 quality policy contract, 12
  upgrade contract, 12 spec contract, 5 CLI contract,
  4 cross-surface regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  smoke-comp` then `forge component list` shows the
  catalog with the 15 ids in stable order; `forge
  component inspect paginated-query` renders the
  contract (typed source/cursor inputs, typed page
  output), the evidence (usage 11, coverage 0.92,
  security_review true, last_verified 2024-08-30) and
  the profile set (`rust-web, python-service`); `forge
  component resolve --profile rust-web --component
  paginated-query --component idempotency-guard`
  returns the certified plan with the per-step evidence
  summary; `forge component resolve --profile rust-web
  --component webhook-receiver` returns the typed
  `component-quality-conflict` rejection so the planner
  does not silently select the deprecated candidate;
  `forge component resolve --profile rust-web --component
  if` exits 1 with
  `error[component-invalid]: component invalid: component
  'if' is a programming primitive; the registry refuses
  to model language constructs`; `forge component resolve
  --profile flutter-app --component paginated-query`
  reports the typed `component-invalid` rejection naming
  the tested profiles; `forge component resolve --profile
  nextjs-web --component paginated-query` reports the
  same boundary outcome; `forge component resolve
  --profile rust-web --component nosuch` reports the
  typed unknown-id rejection; `forge component qualify
  toast --to certified --reason "production ready"
  --coverage 0.95 --path .` writes
  `.forge/components/toast/qualify.json` with
  `target_quality: certified` and `security_review:
  true`; `forge component qualify toast --to certified
  --reason "no security review" --coverage 0.95
  --security-review false --path .` exits 0 with
  `promoted: false`, `prior_quality: verified`, and
  `note: promotion refused: missing security review;
  component 'toast' remains at quality 'verified'`,
  writing no receipt; `forge component qualify toast
  --to certified --reason "stale" --coverage 0.95
  --last-verified 2000-01-01T00:00:00Z` exits 0 with
  `note: promotion refused: last_verified ... is older
  than the certified freshness window of 180 days`,
  writing no receipt; and `forge doctor <proj>` before
  and after the resolve run produces the byte-identical
  verdict so the existing doctor contract still holds.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate semantic-component-registry --strict
  --no-interactive`: valid pre-archive; `openspec archive
  semantic-component-registry --yes`: archived as
  `2026-09-17-semantic-component-registry` with the
  canonical `spec/semantic-component-registry`
  promoted; `openspec validate --all --strict
  --no-interactive`: 24 passed, 0 failed (post-archive,
  includes the promoted
  `spec/semantic-component-registry`).
- `git diff --check`: PASS; staged set reviewed (3
  files modified: `src/core/mod.rs` for the new
  `ComponentInvalid` / `ComponentQualityConflict`
  typed errors, `src/lib.rs` to register the new
  module, `src/main.rs` for the `forge component`
  subcommand, the `ComponentCommands` enum and the
  `cmd_component` helpers; 2 files added:
  `src/component/mod.rs` with 23 unit tests,
  `tests/component_contract.rs` with 15 contract
  tests, `tests/component_cross_surface.rs` with 5
  cross-surface regression tests; plus the promoted
  spec and the change archive — 7 files; archive
  under
  `openspec/changes/archive/2026-09-17-semantic-component-registry/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  certified/verified/experimental descriptor stream
  is not present in the local sandbox, so the
  contract is validated through the built-in
  catalog. The redaction rule set is the same
  `policy::redact_credentials` consumed by the
  policy / release / distribution / docs adapters,
  which is itself verified through the existing
  quality policy contract tests. A real catalog
  provider round trip is a downstream integration
  step and is not claimed here.

## Verification evidence (validated-intent-planner, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  18 new planner unit tests for action parsing,
  client-only profile + server-side capability refusal,
  unknown profile / unknown capability / unknown
  constraint key / required-and-forbidden intersection /
  too-many capabilities refusals, plan id stability on
  the same intent, catalog hash determinism,
  `revalidate_plan` acceptance of a fresh plan and
  refusal of a stale plan, `apply_plan` refusal
  without `--confirm`, `write_plan_receipt` /
  `read_plan_receipt` round trip, plan steps
  presence, the `MAX_UNRESOLVED_PER_PLAN` constant,
  and the executor's `unresolved` reporting; 12 new
  planner CLI contract tests for the help output, the
  contract version in artefacts, the human renderers
  carrying the required and forbidden lists, the JSON
  envelope from `forge intent validate` (profile id,
  required capabilities, forbidden capabilities,
  contract version), the typed `error[intent-invalid]`
  exit-1 refusal for a Flutter + postgres request
  with the `flutter-app` / `postgres` / `backend`
  substrings, the typed `error[intent-invalid]`
  exit-1 refusal for an unknown capability, the typed
  `error[intent-invalid]` exit-1 refusal for an
  unknown action, the receipt persisted by `forge
  intent resolve` and round-tripped through the
  public API, the typed `error[plan-apply-failed]`
  refusal without `--confirm` with the manifest
  features map preserved, the typed
  `error[plan-stale]` refusal for a tampered receipt
  with the manifest features map preserved, the human
  render of an applied plan listing the steps and
  unresolved entries, and the `forge intent list`
  JSON envelope reporting the persisted plan; 7 new
  planner cross-surface regression tests for the
  registry `planner` journal row keeping the
  operations table independent of the planner
  surface, the doctor verdict staying byte-equivalent
  after a `forge intent resolve`, dropping the
  planner receipt leaving the project state intact,
  a stale-plan refusal writing nothing, the planner
  apply path not corrupting an existing feature
  ownership receipt, the receipt round-tripping
  through the public API, and the `forge feature add`
  workflow remaining compatible after a `forge intent
  resolve` on the same project; plus the unchanged 25
  test binaries (289 lib tests, 12 planner contract,
  7 planner cross-surface, 33 ui_pattern contract, 5
  ui_pattern cross-surface, 15 component contract, 5
  component cross-surface, 14 release contract, 4
  release cross-surface, 12 documentation contract,
  4 documentation cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4
  agent-runtime-workflows cross-surface, 8 agent
  contract, 8 gitops contract, 13 mcp contract, 9 mcp
  cross-surface, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality policy
  contract, 12 upgrade contract, 12 spec contract,
  5 CLI contract, 4 cross-surface regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  planner-smoke <path>` registers a rust-web project;
  `forge intent validate --action create_project
  --profile rust-web --require auth --require admin
  --forbid billing --constraint public=true` reports
  `intent validated` with the normalized
  `required: [admin, auth]`, `forbidden: [billing]`
  and the `public=true` constraint preserved in the
  `ValidatedIntent` (R1 success); `forge intent
  validate --action create_project --profile
  flutter-app --require auth --require postgres`
  exits 1 with
  `error[intent-invalid]: ... 'flutter-app' is a
  client-only stack and cannot serve the server-side
  capability 'postgres'; recommended boundary:
  flutter-app + rust-web or python-service backend`
  (R1 failure); `forge intent validate --action
  create_project --profile rust-web --require nosuch`
  exits 1 with
  `error[intent-invalid]: ... does not support
  required capability 'nosuch'` (R1 boundary); `forge
  intent resolve ... --path <proj>` writes `plan
  rust-web-<8hex>-<8hex>` with the five pinned steps
  (`install_feature auth@0.1.0`, `install_feature
  admin@0.1.0`, `test cargo test`, `quality_policy
  driftwatch --project . --policies
  AUTH-001,PRIVACY-003,DEPLOY-001`, `doctor forge
  doctor --target L2`) and the receipt at
  `.forge/planner/<plan-id>/plan.json`; `forge intent
  apply <plan-id> --path <proj>` exits 1 with
  `error[plan-apply-failed]: refusing to apply a
  planner plan without --confirm` (R2 boundary);
  `forge intent apply <plan-id> --confirm --path
  <proj>` reports
  `plan ... applied: 5 step(s), 4 file(s) written,
  stale=false` and updates `forge.yaml` with the
  `auth: 0.1.0` and `admin: 0.1.0` features; `forge
  doctor <proj>` before and after the apply run
  reports the same doctor findings (the planner
  surface stays independent of the doctor surface);
  `forge feature add notifications <proj>` still
  succeeds after a planner resolve so the feature
  ownership surface is unaffected; and `forge intent
  list <proj>` reports the persisted plan id under
  `.forge/planner/`.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate validated-intent-planner --strict
  --no-interactive`: valid pre-archive; `openspec
  archive validated-intent-planner --yes`: archived
  as `2026-09-18-validated-intent-planner` with the
  canonical `spec/validated-intent-planner` promoted;
  `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the
  promoted `spec/validated-intent-planner`).
- `git diff --check`: PASS; staged set reviewed (3
  files modified: `src/core/mod.rs` for the new
  `IntentInvalid` / `IntentAmbiguous` / `PlanStale` /
  `PlanConflict` / `PlanApplyFailed` typed errors and
  the matching `intent-invalid` / `intent-ambiguous` /
  `plan-stale` / `plan-conflict` / `plan-apply-failed`
  stable codes, `src/lib.rs` to register the new
  module, `src/main.rs` for the `forge intent`
  subcommand, the `IntentCommands` enum and the
  `cmd_intent` helpers plus the aliased feature
  renderer to avoid a name clash; 3 files added:
  `src/planner/mod.rs` with 18 unit tests,
  `tests/planner_contract.rs` with 12 contract
  tests, `tests/planner_cross_surface.rs` with 7
  cross-surface regression tests; plus the promoted
  spec and the change archive — 7 files; archive
  under
  `openspec/changes/archive/2026-09-18-validated-intent-planner/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  natural-language model is not present in the local
  sandbox, so the contract is validated through the
  `Intent` / `ValidatedIntent` / `AssemblyPlan` round
  trip and the `forge intent` CLI surface; the
  `IntentAction` enumeration and the `--require` /
  `--forbid` / `--constraint` flags are the
  model-agnostic contract a provider adapter would
  emit. A real provider round trip is a downstream
   integration step and is not claimed here.

## Next change

`runtime-hardening-and-test-isolation` implemented, verified and archived on 2026-09-21
as `2026-09-21-runtime-hardening-and-test-isolation`; canonical specs promoted to
[openspec/specs/runtime-hardening-and-test-isolation/spec.md](openspec/specs/runtime-hardening-and-test-isolation/spec.md).
New in this cycle: `src/release/engine.rs` `run_with_timeout` now kills and reaps
the direct adapter child before returning a timeout (and on wait failure), matching
policy/docs/analytics; `src/deploy/engine.rs` `wait_with_timeout` now waits after
kill so the timed-out child is reaped; `src/mcp` round-trip test drives
`run_session` with an explicit temporary registry instead of the host default, plus
a sequential-independence unit test and `tests/mcp_contract.rs` regression tests
for read-only-HOME isolation and repeated-run stability; release timeout keeps the
typed unavailable/timeout outcome and records no successful stage. The next
eligible package is `profile-and-release-readiness`; later packages depend on its
evidence: `provider-integration-evidence` → `specification-governance-refresh`.

`profile-and-release-readiness` implemented, verified and archived on 2026-09-21
as `2026-09-21-profile-and-release-readiness`; canonical specs promoted to
[openspec/specs/profile-and-release-readiness/spec.md](openspec/specs/profile-and-release-readiness/spec.md).
New in this cycle: `src/readiness` (versioned `MatrixRow`/`MatrixReport`/
`ArtifactEvidence`/`GateReport`/`ReadinessStatus`
(`passed`/`failed`/`unverified`) contract `0.1.0`;
`run_profile_row` renders the supported profile fixture into a disposable
directory (registry never contacted, caller tree untouched), strips the
Forge binary's directory from `PATH` and proves no `forge` resolves on the
native `PATH`, captures toolchain version, build/test exit statuses,
source SHA-256 and timestamp, and classifies missing toolchains as
`unverified` (never passing) and failed native commands as `failed` with
the profile and command named; unknown ids refused with `unknown-profile`,
planned ids with `unsupported-profile`, both before any fixture is
generated; `artifact_evidence` reports the platform-native binary path,
SHA-256 and `forge --version` smoke; `evaluate_gate` passes only when
every selected row passes; new Core errors `readiness-invalid` /
`readiness-not-ready` with stable codes; CLI `forge readiness
matrix|artifact|check` (human/JSON, `matrix` exits 0 with the evidence
rows, `check` prints the gate report on stdout and exits 1 with the typed
code on stderr when blocked); `.github/workflows/ci.yml` plus
`scripts/release-check.sh` as the CI/local parity gate (fmt, build, full
test suite, clippy, strict OpenSpec validation, full matrix evidence,
subset gate); `docs/release-readiness.md` checklist with the runner /
profile contract and the remediation pointers). Matrix evidence on this
host: `aspnet-web` passed (`dotnet` 10.0.400), `nextjs-web` passed (`node`
v24.18.0), `rust-web` passed (`cargo` 1.98.1), `flutter-app` failed
(`flutter build appbundle` finds no `android/app/build.gradle` in the
generated fixture), `python-service` failed (`python3 -m build` passes,
`python3 -m pytest` reports `No module named pytest`), `react-web` failed
(`npm test` under `node --test` reports `document is not defined`); the
qualified gate (`rust-web` + `nextjs-web` + `aspnet-web`) exits 0 with
`gate ready=true`. The three failures are recorded as template/host gaps
for the owning specs, not as verified rows. The next eligible package is
`provider-integration-evidence`; `specification-governance-refresh` follows
it.

`provider-integration-evidence` implemented, verified and archived on 2026-09-21
as `2026-09-21-provider-integration-evidence`; canonical specs promoted to
[openspec/specs/provider-integration-evidence/spec.md](openspec/specs/provider-integration-evidence/spec.md).
New in this cycle: `src/provider` (versioned `ProviderStatus`
(`supported`/`unavailable`/`not-run`/`disabled`)/`SandboxKind`
(`live`/`fixture`)/`EvidenceProvenance`/`ProviderRow`/`ProviderMatrix`/
`ProviderDescriptor`/`RunOptions` contract `0.1.0` over five stable
providers `driftwatch-policy`/`oidc-identity`/`analytics`/`deploy`/
`release`; `matrix` reports every row as `not-run` unless `--live`
with `FORGE_PROVIDER_LIVE=1`, so an unattempted sandbox is never
`supported`; `run_controlled` drives one controlled round trip with a
10s bounded wait and argument arrays (policy `check --project <dir>
--format json`, analytics `health --provider/--project/--project-ref/
--plane`, deploy `apply ... --dry-run` with a JSON stdin envelope,
release `publish --stage package|container ... --dry-run`, identity
in-memory challenge/callback/claims/mint/validate/terminate through
`crate::identity` with the probe session terminated); every captured
string — receipts, evidence, `--version` probes, diagnostics — passes
through `redact_provider_evidence` delegating to
`policy::redact_credentials`; provenance records provider, sandbox,
source, project id, VCS revision (`unversioned` for temp dirs, never
invented), timestamp, tool version, redacted receipt and teardown;
temp probe dirs are removed and targeted runs attribute the real
project id; analytics project-ref mismatch surfaces as structured
`ambiguous-mapping` (never another project's data); a split release
stays `partial` naming the delivered stage so retry skips rather than
replays it; probes emit only `supported`/`unavailable`/`not-run`
(`disabled` stays owned by the manifest adapters, classified by the
schema); Core error `provider-invalid` with stable code; CLI `forge
provider matrix [--live]` / `forge provider run <id> [TARGET]
[--live] [--fixture PATH] [--project-ref REF]` / `forge provider
inspect <id>` (human/JSON, unknown ids exit 1 with the typed code,
`not-run` exits 0 without contacting anything); provider operations
journaled under the `provider` kind with the synthetic `__provider__`
id (or the real project id for targeted runs, never inventing a
project); `docs/provider-evidence.md` records the matrix, opt-in and
secret/teardown rules and every live provider not run on this host
and why (no driftwatch binary, no OIDC issuer, no analytics adapter
or project ref, no deployer binary, no publisher binaries); and the
spec contracts from `mature-mcp-surface`, `core-http-api`,
`control-plane-portal`, `doctor-maturity-assessment` and
`feature-lifecycle` still hold after a provider round trip (MCP
`tools/list` unchanged, API/portal/doctor/feature suites green, doctor
verdict byte-equivalent, feature add compatible, portal dashboard
renders). The next eligible package is
`specification-governance-refresh`.

`specification-governance-refresh` implemented, verified and archived on 2026-09-21
as `2026-09-21-specification-governance-refresh`; canonical spec promoted to
[openspec/specs/specification-governance-refresh/spec.md](openspec/specs/specification-governance-refresh/spec.md).
New in this cycle: accurate one-sentence purposes for all 25 canonical specs that
carried the generated `Purpose: TBD` placeholder (the two specs with authored
purposes, `adapter-deployment` and `repository-distribution`, untouched);
`scripts/check-spec-governance.mjs` placeholder/link/pointer/status traceability
(non-zero exit with file:line rule failures, PASS otherwise, negative-tested with
probe files); `scripts/release-check.sh` runs both node checkers before strict
validation; `docs/adr/0001-foundation-toolchain.md` status labeled as historical
planning context over the implemented toolchain; the implementation-cycle example
pointer generalized to `<active-change>`; `docs/requirements-coverage.md` intro
qualified with the audit queue and the local/fixture/native/provider/release
evidence distinction; the queue-setup `README.md` / `ROADMAP.md` /
`openspec/config.yaml` status refresh committed as part of this change (the new
checker requires it). No active changes remain.

## Verification evidence (specification-governance-refresh, 2026-09-21)

- `node scripts/check-openspec-change-names.mjs`: PASS;
  `node scripts/check-spec-governance.mjs`: PASS (probe placeholder/broken-link
  files fail the check as designed and were removed after).
- `openspec validate specification-governance-refresh --strict --no-interactive`:
  valid pre-archive; `openspec archive specification-governance-refresh --yes`:
  archived as `2026-09-21-specification-governance-refresh` with the canonical
  `spec/specification-governance-refresh` promoted; `openspec validate --all
  --strict --no-interactive`: 28 passed, 0 failed (post-archive). Post-archive,
  the promoted spec's regenerated `Purpose: TBD` line was replaced with the
  change's purpose sentence and the `current_spec` pointer removed (no active
  changes remain); checker re-run PASS.
- `git diff --check`: PASS; staged set reviewed (25 canonical purpose lines,
  governance docs, release-check hook, new checker, promoted spec, change
  archive).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Docs-only change: no Rust source touched, so there is no cargo build/test
  delta; the change's own verification strategy (checker, strict validation,
  link/whitespace checks, doc review) is fully executed.

## Verification evidence (provider-integration-evidence, 2026-09-21)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1);
  `cargo clippy --all-targets -- -D warnings`: PASS (one
  `too_many_arguments` site refactored into a `RowParams` builder plus
  two `redundant_closure` cleanups during the cycle).
- `cargo test`: full suite PASS — 460 lib tests incl. 24 new provider
  unit tests at full-run time (plus 3 added after: repeat attribution,
  git-revision binding, disabled schema round trip; final 27 lib
  provider tests green in isolation), 12 new
  `tests/provider_contract.rs` tests (help surface, default all-`not-run`
  matrix, unknown-provider `provider-invalid`, no-flag `not-run`,
  policy/identity fixture success with provenance, analytics
  `ambiguous-mapping`, deploy `unavailable`, release `partial`,
  credential redaction on stdout, inspect descriptor, no invented
  project), 6 new `tests/provider_cross_surface.rs` tests (synthetic
  `__provider__` journal without inventing a project, doctor verdict
  byte-equivalent, feature add compatible, MCP `tools/list` unchanged,
  portal dashboard renders, secret never verbatim); all 45 binaries ok.
  One `api_contract` test (`missing_token_returns_401`,
  `Connection refused`) flaked once under full-suite parallel load,
  untouched by this change, and passes in isolation (11/11) and in the
  final full run.
- Manual smoke: `forge provider matrix` reports 5 `not-run` rows;
  `forge provider run analytics <proj> --live` without
  `FORGE_PROVIDER_LIVE=1` is `not-run` with the opt-in reason; the same
  run with a fixture adapter reporting the manifest's `project_ref` is
  `supported` with `sandbox: fixture`, the real project id and
  `teardown: true`.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate provider-integration-evidence --strict
  --no-interactive`: valid pre-archive; `openspec archive
  provider-integration-evidence --yes`: archived as
  `2026-09-21-provider-integration-evidence` with the canonical
  `spec/provider-integration-evidence` promoted (9/10 tasks; 4.3
  completes with this handoff); `openspec validate --all --strict
  --no-interactive`: 28 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (3 files modified:
  `src/core/mod.rs` for the `provider-invalid` typed error,
  `src/lib.rs` to register the new module, `src/main.rs` for the
  `forge provider` subcommand, the `ProviderCommands` enum and the
  `cmd_provider` helpers; 4 files added: `src/provider/mod.rs` with 27
  unit tests, `tests/provider_contract.rs` with 12 contract tests,
  `tests/provider_cross_surface.rs` with 6 cross-surface regression
  tests, `docs/provider-evidence.md`; plus the promoted spec and the
  change archive). Committed as `57220c8`; no push performed.
  Pre-existing queue-setup modifications (`README.md`, `ROADMAP.md`,
  `openspec/config.yaml`) and the `specification-governance-refresh`
  change dir left untouched in the worktree.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- No live sandbox is configured here, so no `sandbox: live` row is
  claimed: live DriftWatch/OIDC/analytics/deploy/release evidence
  remains a downstream step run with `FORGE_PROVIDER_LIVE=1` and
  runner-supplied secrets per `docs/provider-evidence.md`. Fixture
  rows are labeled `sandbox: fixture` and are supplemental, never
  provider support.

## Verification evidence (runtime-hardening-and-test-isolation, 2026-09-21)

- `cargo fmt --check`: PASS; `cargo build`: PASS; `cargo clippy --all-targets
  -- -D warnings`: PASS.
- `cargo test`: full suite PASS (427 lib tests incl. 3 new release
  `run_with_timeout` unit tests for adapter completion, timeout kill-and-reap
  via `kill -0` probe, and non-zero exit without success; 1 new MCP
  sequential-independence unit test; 15 `mcp_contract` tests incl. 2 new for
  read-only-HOME isolation and repeated-run stability; all 41 test binaries ok).
- Reproduced the reported failure first: with `HOME` pointed at a read-only
  directory, `mcp::tests::run_session_round_trip_known_request_and_unknown_tool`
  FAILED before the fix (registry-backed `list_projects` through the host
  default) and PASSES after (explicit temporary registry).
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate runtime-hardening-and-test-isolation --strict
  --no-interactive`: valid pre-archive; `openspec archive
  runtime-hardening-and-test-isolation --yes`: archived as
  `2026-09-21-runtime-hardening-and-test-isolation` with the canonical
  `spec/runtime-hardening-and-test-isolation` promoted; `openspec validate
  --all --strict --no-interactive`: 28 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (3 files modified:
  `src/release/engine.rs` for kill-and-wait on timeout/wait-failure,
  `src/deploy/engine.rs` for wait-after-kill, `src/mcp/mod.rs` for isolated
  round-trip plus independence test, `tests/mcp_contract.rs` for read-only-HOME
  and repeat-stability tests; plus the promoted spec and the change archive).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Descendant process-group cleanup remains platform-dependent and is recorded
  as a direct-child guarantee only, per the change design; real provider round
  trips remain downstream integration steps and are not claimed here.

## Verification evidence (profile-and-release-readiness, 2026-09-21)

- `cargo fmt --check`: PASS; `cargo build`: PASS; `cargo clippy
  --all-targets -- -D warnings`: PASS (two findings fixed during the
  cycle: a needless borrow in `artifact_evidence_for` and an unused
  import in `tests/readiness_contract.rs`).
- `cargo test`: full suite PASS — 436 lib tests incl. 9 new readiness
  unit tests (supported-profile catalog order, PATH strip keeping only
  the Forge directory out, source-hash stability/sensitivity, `forge`
  resolution probe, unknown/planned refusal codes, missing-toolchain
  `unverified` classification with no pass, filter dedupe/refusal,
  artifact version+checksum); 10 new `tests/readiness_contract.rs`
  tests (help surface, unknown/planned refusal before any fixture,
  `rust-web` passed row with toolchain version, source SHA-256 and
  `forge_absent_from_path: true`, dedupe, artifact path/checksum/smoke
  matching `forge --version`, `check` pass on a passing row, `check`
  block keeping the gate report on stdout with `readiness-not-ready`
  on stderr, human rendering); 5 new
  `tests/readiness_cross_surface.rs` tests (matrix invents no project
  and writes no journal row, artifact leaves the registry file
  byte-identical, doctor verdict byte-equivalent across a readiness
  run, `forge feature add` compatible after a readiness run,
  `rust-web` row proving the fixture builds without Forge); all 43
  binaries ok. One `api_contract` healthz test flaked once under
  full-suite parallel load (`Connection reset by peer`, untouched by
  this change) and passes in isolation (11/11) and in the final full
  run.
- Native matrix (final, current binary): `passed=3 failed=3
  unverified=0 ready=false` — `aspnet-web` passed, `nextjs-web`
  passed, `rust-web` passed; `flutter-app` failed (no
  `android/app/build.gradle` in the fixture), `python-service` failed
  (`No module named pytest`), `react-web` failed (`document is not
  defined` under `node --test`). Qualified gate (`--profile rust-web
  --profile nextjs-web --profile aspnet-web`): exit 0, `gate
  ready=true`. Full-matrix gate correctly blocks with
  `error[readiness-not-ready]` naming each failed row.
- `scripts/release-check.sh` runs the CI-parity sequence (fmt, build,
  full `cargo test`, clippy, strict OpenSpec validation, full matrix
  evidence, subset gate); `.github/workflows/ci.yml` provisions
  Rust/Node/.NET and gates the same three qualified profiles.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate profile-and-release-readiness --strict`:
  valid pre-archive; `openspec archive profile-and-release-readiness
  --yes`: archived as `2026-09-21-profile-and-release-readiness` with
  the canonical `spec/profile-and-release-readiness` promoted;
  `openspec validate --all --strict --no-interactive`: 28 passed, 0
  failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (4 files modified:
  `src/core/mod.rs` for the `readiness-invalid` /
  `readiness-not-ready` typed errors, `src/generate/mod.rs` for
  `pub(crate)` visibility of `split_command` / `toolchain_present`,
  `src/lib.rs` to register the new module, `src/main.rs` for the
  `forge readiness` subcommand, the `ReadinessCommands` enum and the
  `cmd_readiness` helpers; 6 files added: `src/readiness/mod.rs` with
  9 unit tests, `tests/readiness_contract.rs` with 10 contract tests,
  `tests/readiness_cross_surface.rs` with 5 cross-surface regression
  tests, `.github/workflows/ci.yml`, `scripts/release-check.sh`,
  `docs/release-readiness.md`; plus the promoted spec and the change
  archive). Committed as `d37a9e8`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- CI itself is not executed here (no runner in the sandbox); the
  workflow calls the same script verified locally. The flutter/react
  template gaps and the pytest prerequisite are recorded in
  `docs/release-readiness.md` for the owning specs; a real
  full-matrix pass and any remote publication remain downstream steps
  and are not claimed here.

## Verification evidence (control-plane-portal, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS.
- `cargo test`: full suite (lib + integration tests)
  PASS; 423 lib tests incl. 22 new portal unit tests
  for section id stability (twelve §36 sections in
  stable order), unknown-section refusal with
  `portal-invalid`, whitespace-tolerant parsing,
  config defaults, typed `portal:` block fields,
  empty/oversized-title refusal, unknown-scope
  refusal, title-on-disabled-block refusal,
  oversized entry list/id refusal, empty-id refusal,
  worst-status section rollup, worst-section
  dashboard rollup, fleet-scope project-id drop,
  unavailable-for-missing-project, unknown-prominent
  rollup, human renderers and section-label/id
  consistency; 10 new portal CLI contract tests for
  the help output (top-level `portal` mention,
  `dashboard` + `view` subcommands), the per-project
  dashboard rendering all twelve sections in human
  and JSON (`contract: 0.1.0`, `scope: project`),
  the fleet dashboard (`scope: fleet`, null
  project id, synthetic `__portal__` journal row),
  the single-section view (`section_id`,
  per-entry evidence, `controls_available`),
  the typed `error[portal-invalid]` exit-1 refusal
  for an unknown section, and the `unknown`-prominent
  rollup on a project with no observable state;
  6 new portal cross-surface regression tests for
  the fleet view inventing no registered project,
  the doctor verdict staying byte-equivalent after
  a portal round trip, the MCP `tools/list`
  snapshot staying unchanged (no `portal_*` tool
  advertised), the `forge feature add` workflow
  remaining compatible after a portal round trip,
  the per-call `portal` journal row carrying the
  real project id, and an unknown-section refusal
  mutating nothing.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  portal-smoke` then `forge portal dashboard
  portal-smoke` renders the twelve sections
  (`projects=ok`, `features=ok`,
  `components=unknown`, `policies=ok`,
  `specs=unknown`, `agents=unknown`,
  `deployments=unknown`, `repositories=unknown`,
  `documentation=unknown`, `analytics=unknown`,
  `servers=unknown`, `settings=ok`) with rollup
  `unknown`; `forge portal view servers
  portal-smoke` renders the single section with
  `evidence: sessions=0` and
  `controls_available: forge api serve`; `forge
  portal view nope portal-smoke` exits 1 with
  `error[portal-invalid]` naming the twelve
  supported sections.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate control-plane-portal --strict
  --no-interactive`: valid pre-archive; `openspec
  archive control-plane-portal --yes`: archived as
  `2026-09-18-control-plane-portal` with the
  canonical `spec/control-plane-portal` promoted;
  `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the
  promoted `spec/control-plane-portal`).
- `git diff --check`: PASS on the working tree;
  staged set reviewed (5 files modified:
  `src/core/manifest.rs` for the new `PortalMeta`
  typed fields, `src/core/mod.rs` for the
  `portal-invalid` typed error, `src/lib.rs` to
  register the new module, `src/main.rs` for the
  `forge portal` subcommand, the `PortalCommands`
  enum and the `cmd_portal` helpers,
  `src/registry/mod.rs` for the new
  `recent_operations` / `operations_for_project`
  readers; 3 files added: `src/portal/mod.rs` with
  22 unit tests, `tests/portal_contract.rs` with 10
  contract tests, `tests/portal_cross_surface.rs`
  with 6 cross-surface regression tests; plus the
  promoted spec and the change archive — 14 files;
  archive under
  `openspec/changes/archive/2026-09-18-control-plane-portal/`).
  Committed as `966bd48`; no push performed.
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real portal framework integration is not exercised:
  no ASP.NET Core / Next.js renderer exists in the
  local sandbox, so the contract is validated
  through the `forge portal dashboard` / `forge
  portal view` CLI surface and the versioned JSON
  envelope a future renderer would consume. The
  portal is read-only by construction; a real
  graphical portal deployment is a downstream
  integration step and is not claimed here.

## Verification evidence (ai-procedure-skills, 2026-09-18)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  28 new procedure lib unit tests for catalog id stability
  (eight procedures in stable id order), the
  `every_catalog_entry_passes_validate_procedure` invariant,
  `inspect_procedure` acceptance / empty-id / unknown-id /
  non-kebab-case refusals, validator rejections for empty
  steps, too many steps (`MAX_PROCEDURE_STEPS = 16`), non-
  monotonic ordinals, zero ordinal, missing `report_findings`
  step, `report_findings` not in the final slot, multiple
  `report_findings` steps, every bypass marker in
  `BYPASS_MARKERS` (`--force`, `--skip-checks`, `--no-validate`,
  `--bypass`, `--override`, `--no-doctor`, `--ignore-failures`)
  refused with the typed `procedure-bypass-refused` code, the
  `--key=value` form (e.g. `--skip-checks=doctor`), mixed-case
  force flags (`--FORCE`), empty step descriptions, too many
  step args, empty arg values, the `CoreOperation` label round
  trip, the `from_label` rejection of unstable tokens (`push`,
  `planner.dispatch`, `agent.pause`, `agent.takeover` etc.),
  the `report_findings` only-synthetic-step invariant, the
  bypass-marker scanner, the human renderers
  (`render_list_human` / `render_inspect_human`), the platform-
  neutral scan (no `opencode` / `codex` / `claude` / `vscode` /
  etc.), the upgrade procedure's `spec_generate` handoff (R2
  success), and the catalog-wide `report_findings` final-step
  invariant; 16 new procedure CLI contract tests for the help
  output, top-level help inclusion, the catalog list carrying
  `contract: 0.1.0` plus the eight ids in stable order, the
  `create-project` SOP (nine ordered steps, eight
  prerequisites, verification block, the nine expected Core
  operations), the `upgrade-project` SOP carrying a
  `spec.generate` handoff, the typed `procedure-invalid`
  rejection for an unknown id / non-kebab-case id / empty id,
  the human and JSON acceptance of a minimal valid procedure
  via `forge procedure validate --path <json>`, the typed
  `procedure-bypass-refused` rejection carrying the marker name,
  the `procedure-invalid` rejection for a workflow without a
  `report_findings` step (R2 boundary), the
  `procedure-invalid` rejection for an unknown Core operation
  (R1 failure), the `procedure-invalid` rejection for malformed
  JSON, the catalog-wide `report_findings` final-step invariant
  through `forge procedure inspect <id>`, and the
  catalog-wide "every step references a supported Core operation"
  scan across the eight procedures; 8 new procedure
  cross-surface regression tests for the registry `procedure`
  journal row keeping the operations table independent of the
  procedure surface (list / inspect / validate all leave the
  per-project list empty so no user-visible project is
  invented), the doctor verdict staying byte-equivalent before
  and after a `procedure inspect`, the doctor verdict staying
  byte-equivalent before and after a `procedure list`, the
  doctor verdict staying byte-equivalent before and after a
  `procedure validate --force` refusal (R2 failure carries no
  project state), the `forge feature add` workflow remaining
  compatible after a `procedure inspect` on the same project,
  the `forge spec generate` workflow remaining compatible after
  a `procedure inspect` on the same project (R2 success), and
  the `forge mcp serve` `tools/list` snapshot remaining
  unchanged by the procedure surface (the procedure layer is
  CLI-only in v0.1.0); plus the unchanged existing 30 test
  binaries (317 lib tests incl. 28 new procedure unit tests,
  16 procedure contract, 8 procedure cross-surface, 12
  planner contract, 7 planner cross-surface, 33 ui_pattern
  contract, 5 ui_pattern cross-surface, 15 component
  contract, 5 component cross-surface, 14 release contract,
  4 release cross-surface, 12 documentation contract, 4
  documentation cross-surface, 12 distribution contract, 4
  distribution cross-surface, 4 agent-runtime-workflows
  cross-surface, 8 agent contract, 8 gitops contract, 13 mcp
  contract, 9 mcp cross-surface, 8 doctor contract, 10
  feature contract, 12 generate contract, 10 import contract,
  9 profile contract, 7 quality policy contract, 12 upgrade
  contract, 12 spec contract, 5 CLI contract, 4 cross-surface
  regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id
  proc-smoke <path>` registers a rust-web project; `forge
  procedure list` returns the eight named procedures in stable
  id order (`create-project`, `deploy-project`,
  `fix-quality-findings`, `mirror-repository`,
  `onboard-existing-project`, `prepare-release`,
  `translate-docs`, `upgrade-project`) carrying `version:
  0.1.0` and the contract `0.1.0`; `forge procedure inspect
  create-project` renders the nine-step SOP (profile.inspect,
  profile.preflight, component.resolve, ui_pattern.resolve,
  feature.add, doctor.run, test.run, policy.run,
  report_findings) with the prerequisites and the verification
  block; `forge procedure inspect upgrade-project` confirms
  the `spec.generate` handoff (R2 success) plus the final
  `report_findings` step (R2 boundary); `forge procedure
  inspect nope` exits 1 with
  `error[procedure-invalid]: procedure invalid: procedure
  'nope' is not in the catalog; known procedures:
  create-project, upgrade-project, prepare-release,
  fix-quality-findings, onboard-existing-project,
  deploy-project, mirror-repository, translate-docs`; `forge
  procedure validate --path <bypass.json>` exits 1 with
  `error[procedure-bypass-refused]: procedure bypass refused:
  procedure 'smoke-bypass' step 1 (op doctor.run) carries
  the bypass marker '--no-validate'; Core still validates
  every operation and the procedure layer refuses to forward
  the request; Core still validates every operation;
  procedures do not override Core outcomes`; `forge procedure
  validate --path <valid.json>` exits 0 and renders the
  validated spec via the human renderer; `forge doctor
  <proc-smoke>` before and after a `forge procedure list` /
  `inspect` run produces the byte-identical verdict so the
  existing doctor contract still holds; the registry's
  `operations` table records the `procedure` journal rows
  with the synthetic `__procedure__` project id so a future
  portal or API surface can read the procedure history
  through the same Core contract the CLI uses; and a string
  scan over the catalog (8 procedures, 41 total steps)
  returns no agent-provider / IDE / model identifier, so an
  agent provider change is a no-op for the procedure layer
  (R1 boundary).
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate ai-procedure-skills --strict
  --no-interactive`: valid pre-archive; `openspec archive
  ai-procedure-skills --yes`: archived as
  `2026-09-18-ai-procedure-skills` with the canonical
  `spec/ai-procedure-skills` promoted; `openspec validate
  --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive, includes the promoted
  `spec/ai-procedure-skills`).
- `git diff --check`: PASS; staged set reviewed (3 files
  modified: `src/core/mod.rs` for the new `ProcedureInvalid`
  / `ProcedureUnsupportedOperation` / `ProcedureBypassRefused`
  / `ProcedureRunFailed` typed errors and the matching
  `procedure-invalid` / `procedure-unsupported-operation` /
  `procedure-bypass-refused` / `procedure-run-failed` stable
  codes, `src/lib.rs` to register the new `procedure` module,
  `src/main.rs` for the `forge procedure` subcommand, the
  `ProcedureCommands` enum and the `cmd_procedure` helpers;
  3 files added: `src/procedure/mod.rs` with 28 unit tests,
  `tests/procedure_contract.rs` with 16 contract tests,
  `tests/procedure_cross_surface.rs` with 8 cross-surface
  regression tests; plus the promoted spec and the change
  archive — 7 files; archive under
  `openspec/changes/archive/2026-09-18-ai-procedure-skills/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  `opencode` / `codex` / `claude` agent provider or model
  is not present in the local sandbox, so the contract is
  validated through the static `procedure_catalog` /
  `inspect_procedure` / `validate_procedure` round trip
  and the `forge procedure` CLI surface; the catalog and
  validator carry no agent-provider identifier so the
  R1 boundary (provider change is a no-op) is enforced
  by construction. A real agent-provider round trip is a
  downstream integration step and is not claimed here.

## Verification evidence (semantic-component-registry, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: full suite (lib + integration tests) PASS;
  23 new deploy unit tests for target validation,
  config defaults, default target resolution, unknown
  target kind refusal, duplicate target name refusal,
  artifact outside project refusal, legacy single
  target acceptance, legacy/typed mixing refusal,
  empty block refusal, identity stability on the same
  inputs, identity divergence on different
  target/revision, artifact read success and missing/
  outside-project refusal, target lookup rejection of
  unknown names, health spec `kind` requirement,
  unknown health kind refusal, ssh target unavailable
  boundary, prepare ready plan, prepare unknown target
  refusal, apply refusal without `--confirm`, list empty
  for a fresh project, and observe refusal without
  prior state; 14 new deploy CLI contract tests for
  help listing the `plan|apply|observe|list|inspect`
  subcommands, plan capturing target / artifact /
  revision, plan refusing unknown target with
  `error[deploy-invalid]`, plan refusing `ssh` target
  with `error[deploy-target-unavailable]`, plan
  requiring `default` or explicit `--target-name` for
  multi-target projects, apply refusing without
  `--confirm` with `error[deploy-invalid]`, apply
  recording `delivered` apply + `running` observation
  through the `FORGE_DEPLOYER_BIN` fixture, apply
  recording `failed` apply + recovery through the
  failing fixture, apply preserving the unreachable
  boundary (apply `delivered` + observe `failed` +
  observation `unknown`), apply refusing a missing
  adapter binary without writing state, list reporting
  the persisted deploy with the expected project /
  target / current_state, inspect returning the deploy
  state with the per-stage outcomes + current_state,
  observe updating the state on a subsequent run, and
  observe refusing without prior state with
  `error[deploy-target-stale]`; 5 new deploy
  cross-surface tests for the registry `deploy` journal
  row keeping the operations table independent of the
  deploy surface, the doctor verdict being unchanged
  after a successful deploy, redeploy on the same
  revision deriving the same deploy id (idempotency),
  redeploy after a new commit deriving a different
  deploy id (R2 boundary), and a credential-shaped
  substring in evidence being redacted through
  `redact_deploy_evidence` (which delegates to
  `policy::redact_credentials`); plus the unchanged
  existing 25 test binaries (228 lib tests, 13 mcp
  contract, 12 release contract, 4 release
  cross-surface, 12 documentation contract, 4
  documentation cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4
  agent-runtime-workflows cross-surface, 8 agent
  contract, 8 gitops contract, 13 mcp contract, 9
  mcp cross-surface, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract,
  9 profile contract, 7 quality policy contract, 12
  upgrade contract, 12 spec contract, 5 CLI contract,
  4 cross-surface regression).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id smoke-dep`
  then appending a `deployment` block to the manifest
  with a `default: home` target of `kind: local`, a
  `docker-compose.yml` artifact, and a `docker` health
  check for service `app`; `FORGE_DEPLOYER_BIN=…/smoke-
  deployer.sh forge deploy plan smoke-dep` reports
  `ready: yes` with the captured target / artifact /
  artifact hash / health check and the deploy id
  derived from the working-tree revision; `forge deploy
  apply smoke-dep --confirm` runs the fixture deployer
  end to end and records apply `delivered` and observe
  `running` plus a registry `deploy` journal row;
  `.forge/deploy/smoke-dep/smoke-dep-home-<12hex>/state.json`
  carries the per-stage outcomes and the running
  observation; `forge deploy list smoke-dep` renders
  the persisted deploy id with its project / target /
  current_state (`running`) / last_run_at; `forge
  deploy inspect <id> smoke-dep` returns the same
  per-stage outcomes + current_state the apply
  produced; a re-observation on a fresh project (no
  prior state) exits 1 with
  `error[deploy-target-stale]: deploy target stale:
  no prior deploy state at ...; run 'forge deploy
  apply' first` and writes nothing; `forge deploy
  apply` on a project with two targets and no
  `default` and no `--target-name` exits 1 with
  `error[deploy-invalid]: deployment invalid:
  deployment block declares 2 targets but no
  'default'; pass '--target' or set
  'deployment.default'`; `forge deploy plan` on a
  project with `kind: ssh` exits 1 with
  `error[deploy-target-unavailable]: target 'vps'
  uses kind 'ssh' which is planned for a later
  release`; `forge deploy apply` on a project with
  `FORGE_DEPLOYER_BIN=/nonexistent/forge-deployer`
  exits 1 with `error[deploy-target-unavailable]` and
  writes no state file; and a credential-shaped
  substring in evidence is redacted as `[REDACTED]`
  in both the per-stage note and the journal detail.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate adapter-deployment --strict
  --no-interactive`: valid pre-archive; `openspec
  archive adapter-deployment --yes`: archived as
  `2026-09-17-adapter-deployment` with the canonical
  `spec/adapter-deployment` promoted; `openspec
  validate --all --strict --no-interactive`: 24
  passed, 0 failed (post-archive, includes the
  promoted `spec/adapter-deployment`).
- `git diff --check`: PASS; staged set reviewed (4
  files modified: `src/core/manifest.rs` for the new
  `deployment.targets[]` + `deployment.artifact` +
  `deployment.default` + `deployment.health` typed
  fields, `src/core/mod.rs` for the
  `deploy-invalid` / `deploy-target-unavailable` /
  `deploy-target-stale` / `deploy-health-failed`
  typed errors, `src/lib.rs` to register the new
  module, `src/main.rs` for the `forge deploy`
  subcommand and the `cmd_deploy_*` helpers; 2 files
  added: `src/deploy/mod.rs` with 16 unit tests,
  `src/deploy/engine.rs` with 7 unit tests; 2
  integration files added: `tests/deploy_contract.rs`
  with 14 contract tests, `tests/deploy_cross_surface.rs`
  with 5 cross-surface regression tests; plus the
  promoted spec and the change archive — 9 files;
  archive under
  `openspec/changes/archive/2026-09-17-adapter-deployment/`).
- No shared Gate Runtime is configured; no Gate pass
  is claimed.
- Real provider integration is not exercised: a real
  `forge-deployer` binary is not present in the local
  sandbox, so the contract is validated through
  `FORGE_DEPLOYER_BIN` fixture shell scripts that
  stand in for real `docker compose` or `scp` round
  trips. The credential redaction rule set is the
  same as `policy::redact_credentials`, which is
  itself verified through the existing quality
  policy contract tests. A real Docker Compose or
  SSH provider round trip is a downstream integration
  step and is not claimed here.

## Verification evidence (release-publishing, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 374 passed, 0 failed (206 lib incl. 14 new release
  unit tests for semver parsing, version rejection, config defaults
  and check/package/dockerfile validation, identity derivation
  stability, changelog presence/outside-project refusal, manifest
  maturity preservation, prepare success, prepare failure when
  doctor reports blocking findings, prepare boundary when no
  translation locale is configured, apply refusing without
  `--confirm`, apply recording tag conflict when the same semver
  is requested at a different commit, apply reporting `skipped`
  when the tag already points at the working-tree revision, apply
  persisting a release state for retry, and list reporting zero
  entries for a fresh project; 10 new release CLI contract tests
  for help listing the `prepare|apply|list|inspect` subcommands,
  prepare capturing a `ready` plan with the changelog path and
  the three check kinds, prepare refusing an unknown semver,
  prepare refusing a manifest without a `release` section, apply
  refusing without `--confirm`, apply recording per-stage
  `delivered` outcomes for commit/tag/package/notes through
  `FORGE_PACKAGE_BIN` and `FORGE_NOTES_BIN` shell fixture
  adapters, apply recording package `failed` on a second pass
  while the prior tag stays `skipped` (per-stage independence),
  apply reporting tag `conflict` when HEAD moves past a tagged
  commit, list reporting the persisted release with the
  expected project and version, and inspect returning the
  release state with the per-stage outcomes; 4 new release
  cross-surface tests for the doctor verdict staying unchanged
  across a successful release prepare, the registry
  `release` journal row carrying the per-stage summary, the
  release surface reading the new manifest verbatim after a
  `feature add`, and a prior tag reporting `conflict` after a
  new commit; plus the unchanged 5 CLI contract, 4
  cross-surface regression, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract, 7
  profile contract, 7 quality_policy_contract, 12 upgrade
  contract, 12 spec contract, 8 agent contract, 8 gitops
  contract, 13 mcp contract, 9 mcp cross-surface, 4
  agent-runtime-workflows cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4 quality policy
  cross-surface, 7 quality policy contract, 12 documentation
  contract, 4 documentation cross-surface; the slow
  `cargo build+test` evidence path is exercised through the
  `rust_scaffold_builds_and_tests_with_native_toolchain`
  test that finishes in ~195s when the host toolchain is
  on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id rel-smoke` then
  appending a `release` block to the manifest and creating
  `CHANGELOG.md`; `forge release prepare rel-smoke --version 1.0.0`
  returns `ready: yes` with the captured changelog, the captured
  source revision, the three check kinds (`doctor` and `test`
  passing, `driftwatch` disabled), and a registry `release`
  journal row recorded as `done`; `forge release apply rel-smoke
  --version 1.0.0 --confirm --stage tag --stage package
  --stage notes` with `FORGE_PACKAGE_BIN` and `FORGE_NOTES_BIN`
  pointing at fixture shell scripts (`fake-pkg.sh` echoes
  `npm:rel-fixture@0.1.0 receipt-ok`, `fake-notes.sh` echoes
  `notes:rendered:…`) records the tag as `delivered`, the
  package stage as `delivered` and the notes stage as
  `delivered`, writes `.forge/release/rel-smoke/rel-smoke-1.0.0-12e19a9efefc/state.json`
  with the per-stage outcomes, and the registry records a
  second `release` journal row; `forge release list rel-smoke`
  renders the persisted release ids with their project id,
  version, stage count and last-run timestamp; `forge release
  inspect <id> rel-smoke` returns the same per-stage outcomes
  the apply produced; `forge release apply rel-smoke --version
  1.0.0 --confirm --stage tag` after a new commit is made reports
  the tag as `conflict` with both `existing:` and `requested:`
  evidence lines and the recovery notes naming `git tag -d` and
  a different semver; `forge release prepare rel-smoke-fail
  --version 1.0.0` on a project whose doctor reports blocking
  findings returns `ready: no` with the failing checks named
  in the summary; `forge release apply rel-smoke-fail
  --version 1.0.0 --confirm --stage tag` on the same project
  exits 1 with `error[release-check-failed]` and writes no
  release state.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate release-publishing --strict
  --no-interactive`: valid pre-archive; `openspec archive
  release-publishing --yes`: archived as
  `2026-09-17-release-publishing` with the canonical
  `spec/release-publishing` promoted; `openspec validate
  --all --strict --no-interactive`: 25 passed, 0 failed
  (post-archive, includes the promoted
  `spec/release-publishing`).
- `git diff --check`: PASS; staged set reviewed (4 files
  modified: `src/core/manifest.rs` for the new
  `release.*` typed fields, `src/core/mod.rs` for the
  `release-invalid` / `release-check-failed` /
  `release-identity-conflict` typed errors, `src/lib.rs` to
  register the new module, `src/main.rs` for the
  `forge release` subcommand and the `cmd_release_*`
  helpers; 2 files added: `src/release/mod.rs` with 14 unit
  tests, `src/release/engine.rs` with the prepare/apply/list
  logic and 8 unit tests; 2 integration files added:
  `tests/release_contract.rs` with 10 contract tests,
  `tests/release_cross_surface.rs` with 4 cross-surface
  regression tests; plus the promoted spec and the change
  archive — 9 files; archive under
  `openspec/changes/archive/2026-09-17-release-publishing/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: a real
  `forge-package-publisher` / `forge-container-publisher` /
  `forge-notes-renderer` binary is not present in the local
  sandbox, so the contract is validated through
  `FORGE_PACKAGE_BIN` / `FORGE_NOTES_BIN` fixture shell
  scripts that stand in for real provider round trips. The
  credential redaction rule set is the same as
  `policy::redact_credentials`, which is itself verified
  through the existing quality policy contract tests. A real
  package/container/notes provider round trip is a
  downstream integration step and is not claimed here.

## Verification evidence (documentation-translation, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 337 passed, 0 failed (183 lib incl. 16 new docs
  contract tests for locale validation, config defaults and
  opt-in, explicit source and paths, empty terms and bad
  locales refusal, default derivative path matching the brief
  model, segmentation and verbatim code preservation, output
  validation flagging dropped links and non-translatable
  terms, derivative-equal-to-source refusal, derivative
  outside project refusal, unknown and disabled locales
  refusal, missing binary without touching prior state, state
  round-trip preserving hashes and review, request validation
  rejecting empty and ambiguous, freshness tracking current /
  stale / never / misconfigured, and report health requiring
  all locales ok; 12 new docs CLI contract tests for help
  listing, success with hash + review + verbatim code +
  preserved link destination and state, unchanged source
  reporting `current` without a second provider request,
  incremental retranslation of only changed segments
  (provider sees one segment, three reused, code block and
  link destination preserved), derivative-equal-to-source
  refusal with `error[docs-invalid]`, derivative-outside-
  project refusal with `error[docs-invalid]`, disabled locale
  refusal + `--all` skip without provider contact, unknown
  locale refusal with `error[docs-invalid]`, provider failure
  keeping the prior derivative and state intact while the
  leaked credential-shaped secret is redacted in stdout and
  stderr, missing translator binary failing without writing,
  unparseable translator output failing cleanly, and altered
  links / non-translatable terms marking the derivative
  `needs-review` with both violation reasons named; plus the
  2 new doctor `docs-zh-CN` and `docs-freshness` contract
  tests for never-translated warn with the recovery note and
  misconfigured fail; the unchanged 5 CLI contract, 3
  cross-surface regression, 8 doctor contract, 10 feature
  contract, 12 generate contract, 10 import contract, 7
  profile contract, 7 quality_policy_contract, 12 upgrade
  contract, 12 spec contract, 8 agent contract, 8 gitops
  contract, 13 mcp contract, 9 mcp cross-surface, 4
  agent-runtime-workflows cross-surface, 12 distribution
  contract, 4 distribution cross-surface, 4 quality policy
  cross-surface, 7 quality policy contract, and 12
  distribution cross-surface; the slow `cargo build+test`
  evidence path is exercised through the unchanged fixture
  tests that finish in ~200s when the host toolchain is on
  PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id smoke-app`
  then appending a `docs` block to the manifest and creating
  `README.md`, `FORGE_DOCS_TRANSLATOR_BIN=…/fake-translator.sh
  forge docs translate zh-CN --project <proj>` on the
  registered rust-web project runs the fake `sed` translator
  end to end and writes `docs/README.zh-CN.md` carrying the
  translated text, the verbatim code block and the preserved
  link destination, plus
  `.forge/docs/zh-CN/state.json` carrying the source hash,
  review `ok`, and segments keyed by content hash; `forge
  doctor` on the same project reports `[PASS] docs-zh-CN`
  and `[PASS] docs-freshness` (the existing doctor contract
  still holds); seeding a `state.json` with a stale hash and
  a derivative file flips both findings to `[WARN]` with the
  recovery note `re-run `forge docs translate zh-CN``; `forge
  docs translate zh-CN --project <proj>` on a manifest with
  `path: ../evil.md` exits 1 with
  `error[docs-invalid]: docs invalid: derivative path
  `../evil.md` resolves outside the project; keep derivatives
  inside the project directory` and writes nothing; `forge
  docs translate fr --project <proj>` on a manifest with
  `fr.enabled: false` exits 1 with
  `error[docs-invalid]: docs invalid: locale `fr` is disabled;
  set `docs.translations.fr.enabled: true` to translate it`
  and writes nothing; `forge docs translate zh-CN --project
  <proj>` with a deliberately failing provider that emits
  `token ghp_abcdefghijklmnopqrstuvwxyz0123456789` on stderr
  keeps the prior `docs/README.zh-CN.md` and
  `.forge/docs/zh-CN/state.json` byte-identical and reports
  `error[translation-failed]` on stderr plus a typed `failed`
  per-locale outcome on stdout with the credential replaced
  by `[REDACTED]` everywhere it surfaces.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate documentation-translation --type change
  --strict --no-interactive`: valid pre-archive; `openspec
  validate documentation-translation --type spec --strict
  --no-interactive`: valid post-archive; `openspec validate
  --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive, includes the promoted
  `spec/documentation-translation`).
- `git diff --check`: PASS; staged set reviewed (6 files
  modified, 2 files added: `src/core/manifest.rs` for the
  new `docs.source` / `docs.non_translatable` manifest
  fields, `src/core/mod.rs` for the `docs-invalid` /
  `translation-failed` typed errors, `src/lib.rs` to register
  the new module, `src/doctor/mod.rs` for the
  `docs-<locale>` / `docs-freshness` findings plus 2
  contract tests, `src/main.rs` for the `forge docs
  translate` subcommand, `tests/agent_runtime_workflows_
  cross_surface.rs` for a formatting-only adjustment, plus
  `src/docs/mod.rs` with 16 unit tests, `tests/docs_
  contract.rs` with 12 contract tests, and the promoted
  spec — 9 files; archive under
  `openspec/changes/archive/2026-09-17-documentation-translation/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: a real
  `forge-docs-translator` binary is not present in the local
  sandbox, so the contract is validated through
  `FORGE_DOCS_TRANSLATOR_BIN` fixture scripts that stand in
  for the real provider. The credential redaction rule set is
  the same as `policy::redact_credentials`, which is itself
  verified through the existing quality policy contract
  tests. A real translation provider round trip is a
  downstream integration step and is not claimed here.

## Verification evidence (repository-distribution, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 300 passed, 0 failed (165 lib incl. 18 new
  distribution contract tests for provider validation,
  supported-vs-planned classification, manifest config
  parsing with primary+mirror separation, duplicate-mirror
  refusal, empty distribution refusal, request validation
  (empty refs, dash-prefixed refs, whitespace, implicit
  remote write refusal), plan/apply independence per
  remote, disabled-mirror boundary outcome, planned-provider
  unavailable outcome, credential redaction round-trip,
  state file round-trip, divergent mirror recovery,
  partial-failure independence and retry-skip-on-match);
  12 new distribution CLI/MCP contract tests for help
  listing, full primary+mirror delivery against bare-repo
  remotes, partial failure when the mirror remote is
  unconfigured, `--confirm` refusal without the flag,
  disabled mirror dry-run outcome, retry that records
  `skipped` for already-delivered refs, credential-shaped
  evidence round-trip, missing-distribution-section refusal,
  MCP `tools/list` advertising `mirror_project` as
  `external_write` with `confirm` required, MCP
  `mirror_project` typed refusal without `confirm`, MCP
  envelope matching the CLI JSON shape, and MCP dry-run not
  contacting any remote; 4 new distribution cross-surface
  tests for doctor verdict preservation after a successful
  mirror, registry `mirror` journal row carrying the
  per-remote summary, `commit`+`mirror` sequencing with a
  second commit re-pushing on retry (the state file tracks
  the SHA, not just the ref name), and agent session
  independence from the distribution surface; plus the
  unchanged 5 CLI contract, 4 cross-surface regression, 8
  doctor contract, 10 feature contract, 12 generate
  contract, 10 import contract, 7 profile contract, 7
  quality policy contract, 12 upgrade contract, 12 spec
  contract, 8 agent contract, 8 gitops contract, 13 mcp
  contract (incl. the new `mirror_project` registration in
  the tool list and the consistent `external_write` kind),
  9 mcp cross-surface; rust_scaffold and react_web scaffold
  tests are exercised in the long `cargo build+test` run
  that finishes in ~220s when the host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge mirror <proj> --ref main --confirm
  --dry-run` on a registered rust-web project with a
  `distribution: { primary: github, mirrors: [gitee] }`
  section lists the per-remote `would-push` plan; `forge
  mirror <proj> --ref main --confirm` against a project
  whose `origin` is a local bare repository and whose
  `mirror-gitee` remote is intentionally not configured
  reports `primary github delivered commit=<sha>` and
  `mirror gitee failed: ... fatal: 'mirror-gitee' does not
  appear to be a git repository ...` and exits 1 with
  `error[distribution-invalid]` so the per-remote evidence
  on stdout names the failing remote while the typed exit
  code is preserved; a `forge mirror <proj> --ref main
  --confirm --retry-failed` run after the partial failure
  records `primary github skipped commit=<sha>` (the SHA
  matches the state file) and re-attempts the failed
  mirror; divergent history on the mirror (a push to the
  mirror from a side clone) is detected as
  `mirror gitee diverged` with the recovery note
  `investigate the mirror's diverging history before
  re-pushing / remove the diverging commits from
  mirror-gitee or align with origin`; `forge mirror` on a
  project without a `distribution` section exits 1 with
  `error[distribution-invalid]: distribution invalid: project
  ... has no `distribution` section; declare a primary or at
  least one mirror`; `forge mcp serve` advertises
  `mirror_project` with `kind: external_write` and a schema
  that requires `path` and `confirm`; a JSON-RPC request to
  `mirror_project` with `confirm: false` is refused with
  `TOOL_REFUSED` (`-32012`) and the data code references
  the confirm boundary; a successful `mirror_project` MCP
  request returns the same `{contract, mirror: {...}}`
  envelope the CLI JSON output carries, and the
  `tools/list` snapshot does not include `deploy`,
  `publish`, `release`, `mirror` (as a top-level tool) or
  `docs`; a credential-shaped substring in evidence is
  redacted through `redact_distribution_evidence` which
  delegates to the same `policy::redact_credentials` helper
  the DriftWatch adapter uses; and a second commit on the
  same branch after the first mirror run re-records the
  local SHA in the state file so a subsequent
  `mirror --retry-failed` re-pushes the new commit instead
  of reporting `skipped` (the state file tracks the SHA,
  not just the ref name, so a stale approval cannot
  silently skip a new commit).
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate repository-distribution --strict
  --no-interactive`: valid pre-archive; `openspec validate
  --all --strict --no-interactive`: 25 passed, 0 failed
  (post-archive, includes the promoted
  `spec/repository-distribution`).
- `git diff --check`: PASS; staged set reviewed (4 files
  modified: `src/core/mod.rs` for the new typed errors,
  `src/lib.rs` to register the new module, `src/main.rs`
  for the `forge mirror` subcommand and the `cmd_mirror`
  helper, `src/mcp/mod.rs` for the `mirror_project` tool
  and dispatcher; 2 files added: `src/distribution/mod.rs`
  with 18 unit tests, `tests/distribution_contract.rs` with
  12 CLI/MCP contract tests, and
  `tests/distribution_cross_surface.rs` with 4 cross-surface
  regression tests, plus the promoted spec — 8 files;
  archive under
  `openspec/changes/archive/2026-09-17-repository-distribution/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real provider integration is not exercised: the local
  bare repository stands in for a GitHub primary and a
  Gitee mirror, so the contract is verified through
  `git push` against in-process bare repos. Real Gitee or
  GitHub HTTPS endpoints are not contacted from the
  sandbox, so a real mirror round trip (a push that hits a
  provider API) is a downstream integration step and is
  not claimed here. The credential redaction rule set is
  the same as `policy::redact_credentials`, which is itself
  verified through the existing quality policy contract
  tests.

## Verification evidence (mature-mcp-surface, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 273 passed, 0 failed (147 lib incl. 22 new mcp
  contract tests for tool registry / kind classification /
  unknown-tool / schema validation / shell-metacharacter id
  refusal / push confirm-required / project id kebab-case
  rejection / JSON-RPC envelope parse errors / diagnostic
  redaction / id round-trip on the wire / CLI/MCP domain
  equivalence on inspect / journal entry from a mutating tool;
  13 new mcp_contract tests for the full stdio loop through
  the built binary covering the help listing, the mature
  registry snapshot, the kind boundary (read-only/mutating/
  external_write), the unknown-tool structured error, the
  inspect/CLI domain equivalence, the create-project end-to-end
  with manifest and CLI list observability, the
  shell-metacharacter literal-data refusal, the
  push-confirm-required refusal with the data code, the
  parse-error envelope shape, the diagnostic redaction, the
  list/CLI domain equivalence, and the multi-request round
  trip; 5 new mcp_cross_surface tests for the doctor/CLI
  finding-count equivalence, the create + CLI feature-add
  invariant preservation, the generate-spec idempotency
  through the wire, the agent session recording through MCP
  observable from the CLI surface, and the commit
  paths-only contract against a tracked edit outside scope;
  plus the unchanged 5 CLI contract incl. the new `mcp`
  subcommand in the help output, 4 cross-surface regression,
  8 doctor contract, 10 feature contract, 12 generate
  contract, 10 import contract, 9 profile contract, 7
  quality_policy_contract, 12 upgrade contract, 12 spec
  contract, 8 agent contract, 8 gitops contract; rust_scaffold
  and react-web scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  existing scaffold tests that finish in ~210s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge mcp serve` on the in-repo binary
  answers `tools/list` with the sixteen mature tools and the
  right `kind` for each (read-only/mutating/external_write),
  serves `create_project` for `rust-web` and reports the
  generated manifest + files + journal entry, serves
  `list_projects` afterwards and reports the registered
  record, and serves an `inspect_project` request that the
  CLI surface re-renders to the equivalent domain record;
  a request to a `deploy` tool is refused with the
  `TOOL_MISSING` code and a `tools/list` snapshot does not
  advertise `deploy` / `publish` / `release` / `mirror` /
  `docs`; a request with `confirm: false` for `push` is
  refused with the `push-confirm-required` data code; a
  request to `create_project` with `id: "evil; rm -rf /"`
  is refused as literal data and the destination directory
  is left empty; a request to `create_project` with an
  id that contains a credential-shaped substring is refused
  on the JSON-RPC response (the model can see it) but the
  diagnostic stream does not echo the secret.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate mature-mcp-surface --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the promoted
  `spec/mature-mcp-surface`).
- `git diff --check`: PASS; staged set reviewed (6 files
  modified, 3 files added: `src/mcp/mod.rs`,
  `tests/mcp_contract.rs`, `tests/mcp_cross_surface.rs`, plus
  the promoted spec — 9 files; archive under
  `openspec/changes/archive/2026-09-17-mature-mcp-surface/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real model-issued MCP traffic stays untested: the contract
  is verified end to end through synthetic JSON-RPC requests
  that match the v0.4 shape; the tool list advertises only
  operations whose Core contract was already verified in
  earlier cycles (read-only surfaces from `core-manifest-
  registry` / `profile-registry` / `doctor-maturity-assessment`,
  mutating surfaces from `deterministic-project-generation` /
  `feature-lifecycle` / `specification-remediation` /
  `agent-runtime-workflows` / `quality-policy-integration` /
  `project-upgrade-orchestration`, and the external-write
  `push` from `agent-runtime-workflows`). A real model
  consumer wiring `forge mcp serve` into an MCP-capable
  agent remains a downstream integration step and is not
  claimed here.

## Verification evidence (agent-runtime-workflows, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 233 passed, 0 failed (125 lib incl. 12 new agent
  contract tests for session-id validation, present-or-missing
  provider handling, pause/takeover unsupported state, restart
  preservation, run-spec refusal on missing bound spec,
  write/read roundtrip, and provider binary probe; 7 new gitops
  contract tests for the profile-aware test command, the
  requested-paths-only commit, the tracked-edit refusal, the
  untracked-file preservation, the empty-message refusal, the
  not-a-git-repository refusal, the push-confirm-required guard,
  the no-confirm and with-confirm push attempts, and the
  unrelated-tracked-changes helper for porcelain rename
  handling; 4 new cross-surface tests for the agent × spec
  interaction through pause, the commit × feature-add
  interaction, the test × upgrade dry-run interaction, and the
  push journal × confirm-flag interaction; 10 agent contract
  through the built binary, 8 gitops contract, 4
  agent-runtime-workflows cross-surface, 5 CLI contract incl.
  the new `agent`/`test`/`commit`/`push` subcommands in the
  help output, 3 cross-surface regression, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality_policy_contract, 12
  upgrade contract, 12 spec contract; rust_scaffold and
  react-web scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  existing scaffold tests that finish in ~210s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id smoke-app`
  renders a `rust-web` project; `forge commit . --path
  README.md --message "bump readme"` produces a commit whose
  `files_changed` is `["README.md"]`; `forge push . --remote
  origin --ref-name main` exits 1 with
  `error[push-confirm-required]`; `forge push . --remote origin
  --ref-name main --confirm` reaches the underlying `git push`
  and reports the typed `git-dirty` failure with the captured
  stderr; `forge agent start . --session sess-1 --provider
  opencode` records an `active` state with provider
  `opencode`, spec binding, and writes
  `.forge/agents/sess-1/session.json` plus
  `transitions.log`; `forge agent pause . --session sess-1`
  returns `state: unsupported` with the explicit
  `pause primitive` evidence and a recovery note naming the
  existing PTY-based manager as the integration point;
  `forge agent takeover` returns the same `unsupported`
  state with `takeover primitive` evidence; `forge agent
  status . --session sess-1` renders the recorded session
  plus the full transition timeline; `forge agent list .`
  reports the session inventory with provider, state and
  transition count.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate agent-runtime-workflows --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/agent-runtime-workflows`).
- `git diff --check`: PASS; staged set reviewed (4 files modified,
  6 files added: `src/agent/mod.rs`, `src/gitops/mod.rs`,
  `tests/agent_contract.rs`, `tests/gitops_contract.rs`,
  `tests/agent_runtime_workflows_cross_surface.rs`, the
  promoted spec — 10 files; archive under
  `openspec/changes/archive/2026-09-17-agent-runtime-workflows/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real OpenCode/Codex PTY-manager integration stays untested:
  the existing PTY-based agent manager is not wired into this
  build, so the bundled adapters report `unsupported` for
  pause/takeover with explicit evidence rather than
  simulating success; the contract is verified through the
  test fixtures that confirm the unsupported state and the
  recovery note. Real PTY manager integration remains future
  work, matching the design decision that contract fixtures
  supplement but do not replace a real integration run.
- Real `git push` to a remote is also untested: the sandbox
  has no remote configured, so the with-confirm push attempt
  surfaces the typed `git-dirty` error with the captured
  `git push` stderr (`fatal: 'origin' does not appear to be a
  git repository`); the contract is verified end to end for
  the confirm-required guard and the underlying `git push`
  invocation.

## Verification evidence (specification-remediation, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 199 passed, 0 failed (113 lib incl. 7 new spec
  contract tests for spec-id stability, finding-set hash,
  validate-request refusal, build-draft provenance, idempotent
  generation, list, manual/deterministic/semantic routing and
  apply_routing outcomes, 9 spec contract incl. traceable proposal
  with provenance, idempotent re-run with mtime preservation,
  empty/oversized refusal, route classification, deterministic
  apply with no files, manual apply with no AI claim, semantic apply
  producing a bounded proposal, list/inspect roundtrip, and
  doctor-after-spec regression, 4 cross-surface incl. 1 new
  upgrade × spec semantic-conflict handoff, 5 CLI contract, 8 doctor
  contract, 10 feature contract, 12 generate contract, 10 import
  contract, 9 profile contract, 7 quality_policy_contract, 12 upgrade
  contract; rust_scaffold and react-web scaffold skipped in the
  regular run; the slow `cargo build+test` evidence path is exercised
  through the existing scaffold tests that finish in ~210s when the
  host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge spec generate` on a rust-web project writes
  `proposal.md` / `design.md` / `tasks.md` / `manifest.json` under
  `.forge/specs/<project>-<hash>/`; `forge spec list` reports the
  entry; `forge spec inspect <id>` shows the traceable provenance
  (project, profile, path, source revision, contract, generated_at,
  findings, dependencies, acceptance scenarios); a second
  `forge spec generate` with the same finding set reports
  `spec existing: ...` and writes nothing (boundary scenario);
  `forge spec generate` with no findings exits 1 with
  `error[spec-invalid]: spec invalid: spec generate requires at
  least one finding id`; `forge spec route dependency-drift` returns
  `route: deterministic` with `action: forge upgrade`; `forge spec
  route manifest-valid` returns `route: manual` with no action and
  no suggested spec; `forge spec apply driftwatch-DEPLOY-002` returns
  `route: semantic`, `status: spec-generated`, the bounded proposal
  is written, and the doctor verdict is unchanged after the spec
  operations; `forge spec apply manifest-valid` records the manual
  status with `note: manual boundary: no project changes and no AI
  fix claimed`; `forge upgrade` on a project with a drifted receipt
  still exits 1 with `error[feature-ownership-conflict]` and the
  stderr names `forge spec generate`; the cross-surface
  `upgrade_semantic_conflict_handoff_resolves_through_spec_apply`
  test confirms the receipt and manifest are preserved while the
  bounded proposal is written under `.forge/specs/`.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate specification-remediation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/specification-remediation`).
- `git diff --check`: PASS; staged set reviewed (8 files modified,
  3 files added: `src/spec/mod.rs`, `tests/spec_contract.rs`, the
  promoted spec — 11 files; archive under
  `openspec/changes/archive/2026-09-17-specification-remediation/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real AI/agent implementation stays untested: the bounded spec is
  the handoff and the agent runtime is the next change; this cycle
  proves the spec storage, provenance, idempotency and routing
  contracts end to end, not the agent that consumes the spec. The
  spec's `tasks.md` enumerates the agent-side follow-up and the
  finding ids, dependencies and source revision are preserved for
  the agent adapter.

## Verification evidence (quality-policy-integration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 180 passed, 0 failed (104 unit incl. 18 new policy
  contract tests for redaction shapes, JSON parsing, missing binary,
  invalid output, non-zero exit, stale observation, plus 5 new doctor
  policy integration tests for unavailable rollup, rule-id/severity
  preservation, not-applicable applicability, redaction-on-consume and
  stale-source demotion, 5 CLI contract, 3 cross-surface regression,
  8 doctor contract incl. the new `driftwatch-policy` finding, 10
  feature contract, 12 generate contract, 10 import contract, 9
  profile contract, 7 quality_policy_contract incl. missing-binary,
  parseable-report normalization with not-applicable preservation,
  non-zero exit, invalid output, credential redaction, per-project
  isolation and human-output redaction, 12 upgrade contract;
  rust_scaffold skipped in the regular run; the slow
  `cargo build+test` evidence path is exercised through the
  `rust_scaffold_builds_and_tests_with_native_toolchain` test that
  finishes in ~210s when the host toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor` with
  no `FORGE_DRIFTWATCH_BIN` reports a `driftwatch-policy` finding
  with status `unavailable` and evidence naming the missing binary
  (`driftwatch invocation failed: binary not found on PATH`) so the
  project is not labeled healthy; `FORGE_DRIFTWATCH_BIN=…fake.sh forge
  doctor` against the same project reports `driftwatch-AUTH-001`
  (warn), `driftwatch-DEPLOY-002` (fail) and `driftwatch-FLUTTER-AUTH-001`
  (applicable:false, reason preserved) plus a `driftwatch-policy`
  rollup at `fail`; the same fake script reporting a credential-laden
  payload surfaces every secret as `[REDACTED]` in both JSON and
  human output; two projects running `forge doctor` in sequence each
  reference only their own evidence strings; aging `forge.yaml` after
  a successful run flips `driftwatch-AUTH-001` from pass to warn with
  a stale observation line.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate quality-policy-integration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/quality-policy-integration`).
- `git diff --check`: PASS; staged set reviewed (5 implementation +
  test files modified, 3 files added: `src/policy/mod.rs`,
  `tests/quality_policy_contract.rs`, the promoted spec — 8 files;
  archive under `openspec/changes/archive/2026-09-17-quality-policy-integration/`).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Real DriftWatch adapter execution stays untested: a real
  `driftwatch` binary is not present in the local sandbox, so the
  contract is validated through `FORGE_DRIFTWATCH_BIN` fixture
  scripts that stand in for the real tool. The brief's "real
  DriftWatch integration evidence" claim is deferred until a real
  binary is available, matching the design decision that contract
  fixtures supplement but do not replace a real integration run.

## Verification evidence (extended-profile-catalog, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 148 passed, 0 failed (80 unit incl. 11 new profile
  support_status/react-web/planned tests after dropping the stale
  `react-web` unknown-profile probe, 5 CLI contract, 3 cross-surface
  regression, 8 doctor contract incl. 1 new planned-profile
  doctor finding, 10 feature contract, 12 generate contract incl.
  react-web render + planned-profile generation refusal, 9 profile
  contract incl. 3 new react-web and planned-profile contract
  cases, 10 import contract, 11 upgrade contract; rust_scaffold
  skipped in the regular run; the slow `cargo build+test` evidence
  path is exercised through the `rust_scaffold_builds_and_tests_
  with_native_toolchain` test that finishes in ~218s when the host
  toolchain is on PATH).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all six supported ids
  (aspnet-web, flutter-app, nextjs-web, python-service, react-web,
  rust-web) at 0.1.0 and omits the planned candidates; `forge
  profile inspect react-web` (human + JSON) reports
  `support_status: supported` with `adapter-react`, toolchain
  `npm@20`, build `npm run build`, test `npm test`, and the
  client-only capability set; `forge profile inspect flutter-client`
  reports `support_status: planned` with a description that names
  the backend boundary; `forge profile resolve react-web --feature
  i18n` resolves `react-web@0.1.0 via adapter-react`; `forge
  profile resolve react-web --feature postgres` exits 1 with
  `error[incompatible-profile]` and the backend hint; `forge
  profile resolve aspnet-saas` exits 1 with
  `error[unsupported-profile]`; `forge new --profile react-web`
  creates a registered project with the expected 10 files;
  `forge new --profile rust-cli` exits 1 with
  `error[unsupported-profile]` and writes nothing.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate extended-profile-catalog --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  25 passed, 0 failed (post-archive, includes the promoted
  `spec/extended-profile-catalog`).
- `git diff --check`: PASS; staged set reviewed (8 implementation +
  test files, 5 archive files, 1 promoted spec — 14 files; 1033
  insertions, 36 deletions).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native react-web validation was not exercised in this
  cycle (no node toolchain integration in the local sandbox);
  rendering alone is verified and the
  `react_web_scaffold_builds_and_tests_with_native_toolchain` test
  remains available as the `npm run build` / `npm test` evidence
  path when the host toolchain is on PATH. Planned candidates stay
  discoverable through `inspect_profile` and `planned_profiles` but
  refuse generation before any file change; their promotion to
  `Supported` remains future work, not an implementation claim here.

## Verification evidence (project-upgrade-orchestration, 2026-09-17)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 138 passed, 0 failed (71 unit incl. 11 new upgrade
  plan/apply/fleet tests after the drifted-receipt test fix, 5 CLI
  contract, 3 cross-surface regression incl. 1 new
  upgrade × feature-lifecycle interaction, 8 doctor contract,
  10 feature contract, 12 generate contract, 10 import contract,
  7 profile contract, 12 new upgrade contract incl. dry-run plan with
  old/new versions/assets/recovery, apply advancing versions and
  changing files, semantic-conflict handoff on drifted receipt with
  preserved files, already-satisfied no-op, unknown feature failure
  before edits, missing requested feature install, postgres schema
  irreversible with declared strategy, fleet completion with
  per-project journals, fleet isolation of a blocked project without
  wholesale success, fleet retry skipping satisfied projects and
  re-planning on changed preconditions, fleet dry-run skipping
  writes).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --id up-app` registers
  L1; `forge upgrade --feature auth` installs auth and writes receipt
  + manifest; `forge upgrade --all` reports `0 success, 0 failure,
  0 blocked, 1 skipped` for the now-satisfied project; `forge upgrade
  --all --dry-run` reports `plan only` without changes; `forge upgrade
  --feature nosuch` exits 1 with `error[unknown-feature]`;
  `forge doctor` still reports the same PASS manifest/profile/
  features/drift/build/deployment after fleet upgrades; postgres
  --dry-run after aging `postgres: 0.0.9` shows `upgrade postgres:
  0.0.9 -> 0.1.0 [package+configuration+codemod+schema]` with the
  irreversible `manifest-repin+manual-schema-review` recovery note.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-upgrade-orchestration --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive, includes the promoted
  `spec/project-upgrade-orchestration`).
- `git diff --check`: PASS; staged set reviewed (4 files modified,
  2 files added: implementation, tests, archive and promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Per-service native generation validation stays out of scope here;
  upgrade only repins manifest versions, receipts and runs declared
  policy validators; DriftWatch execution evidence stays deferred to
  v0.3 (`quality-policy-integration` and later).

## Verification evidence (feature-lifecycle, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 115 passed, 0 failed (61 unit incl. 12 new feature
  catalog/resolver/add/remove/upgrade/ownership/section-preservation
  tests, 5 CLI contract, 2 cross-surface regression, 8 doctor contract,
  10 new feature contract incl. catalog discovery, dep-ordered plans,
  conflict/missing/unsupported failures before edits, add-then-upgrade
  agreement, reverse-dep and ownership blocks with preservation,
  exact-reinstall no-op, closure selection in `new`, repeatability,
  12 generate contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web --feature admin` records
  auth+admin 0.1.0; `feature resolve rust-web --feature billing` plans
  auth,billing in order; `feature add billing` updates manifest+receipt+
  registry; `feature remove auth` exits 1 with
  `error[incompatible-feature]` naming admin,billing dependents and
  preserving files; `forge doctor` still PASSes manifest/profile/
  features-compatible on the feature-modified project.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate feature-lifecycle --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Package installation stays per-service native resolution (scaffolds are
  dependency-free by design); declared policy validators are reported
  per plan while DriftWatch execution evidence stays deferred to v0.3
  (`quality-policy-integration` and later).

## Verification evidence (doctor-maturity-assessment, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 93 passed, 0 failed (49 unit incl. 9 new doctor
  inventory/maturity/stale/unavailable tests, 5 CLI contract, 2
  cross-surface regression, 8 new doctor contract incl. stable
  finding IDs with evidence/remediation classes, unavailable-inspector
  reporting, repeatability without file changes, L2 missing-control
  reporting, L4 recovery denial, L0 nonapplicability, stale-observation
  reporting and generated-project registry freshness, 12 generate
  contract, 10 import contract, 7 profile contract).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new --profile rust-web` then `forge doctor`
  reports PASS manifest/profile/features/drift/build/deployment with
  `[UNAVAILABLE] repository` outside a git repo and verdict `not
  healthy`; `--target L2` JSON reports unmet
  L2-auth/admin/ci/driftwatch (deployment met via Dockerfile) and
  L1-structure met after the forge.yaml-presence fix.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate doctor-maturity-assessment --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (10 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Doctor is local inspection in v0.1; DriftWatch execution evidence stays
  deferred to v0.3 (`quality-policy-integration` and later).

## Verification evidence (deterministic-project-generation, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 76 passed, 0 failed (40 unit incl. 11 new generate
  normalization/rendering/failure/cancel/collision/preflight tests, 5 CLI
  contract, 2 cross-surface regression, 10 import contract, 7 profile
  contract, 12 new generate contract incl. explicit-vs-interactive
  equivalence, nonempty/cancel/unknown/incompatible/id-collision failures,
  missing-toolchain unverified reporting, native rust/node/dotnet/flutter
  builds and python compile check).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge new` for all five MVP profiles renders, registers
  and lists with L1 maturity; repeat `new` on a nonempty destination exits
  1 with `error[generation-conflict]`; piped interactive answers create the
  same request as flags; `--verify-native` on rust-web reports native
  `cargo build` + `cargo test` success; empty-PATH `--verify-native`
  exits 1 with `error[toolchain-missing]` containing "not tested".
- Native evidence (Forge unavailable): generated rust `cargo build` +
  `cargo test` PASS; `npm run build` + `npm test` PASS; `dotnet build`
  PASS (0 warnings, 0 errors); `flutter test` PASS (1 test); python
  `compileall` PASS while `pytest`/`build` modules are absent, so only
  rendering (not a pytest run) is claimed for python-service.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate deterministic-project-generation --strict --no-interactive`:
  valid pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Generated scaffolds are dependency-free by design (offline-portable);
  per-service framework packages remain per-service resolution, not part
  of the scaffold claim.

## Verification evidence (project-import, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 53 passed, 0 failed (29 unit incl. 8 new import
  detection/adoption tests, 5 CLI contract, 2 cross-surface regression,
  7 profile contract, 10 new import contract incl. read-only proposal,
  ambiguity-before-writes, missing-remote inspection, accept-writes-only-
  manifest, id-collision/unwritable/legacy failures, repeatability,
  unknown-profile refusal, incompatible-manifest gating).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge import` on a Rust fixture reports Language rust,
  Framework axum, suggested rust-web/L1 with high confidence;
  `--accept` writes only `forge.yaml` and `inspect` returns profile
  rust-web maturity L1; mixed rust+flutter exits 1 with
  `error[ambiguous-import]` in human and JSON.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate project-import --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Detection fixtures are synthetic; no new native-toolchain profile
  support is advertised. Native generation validation stays deferred to
  `deterministic-project-generation`.

## Verification evidence (profile-registry, 2026-09-16)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 35 passed, 0 failed (21 unit incl. 8 new profile
  descriptor/resolver/preflight tests, 5 CLI contract, 2 cross-surface
  regression, 7 new profile contract incl. list/inspect/resolve/preflight,
  flutter+postgres rejection, register gating and repeatability).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge profile list` shows all five MVP IDs at 0.1.0;
  `profile inspect rust-web` JSON carries adapter/build/test metadata;
  `profile resolve flutter-app --feature postgres` exits 1 with
  `error[incompatible-profile]` suggesting a backend boundary; empty-PATH
  `profile preflight rust-web` exits 1 with `error[toolchain-missing]`
  without claiming the profile was tested.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate profile-registry --strict --no-interactive`: valid
  pre-archive; `openspec validate --all --strict --no-interactive`:
  24 passed, 0 failed (post-archive).
- `git diff --check`: PASS; staged set reviewed (12 files, implementation
  + tests + archive + promoted specs only).
- Committed as `9a3b263`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.
- Profile metadata alone does not establish working templates; native
  toolchain generation validation is deferred to
  `deterministic-project-generation`.

## Prior verification evidence (core-manifest-registry)

- `cargo fmt --check`: PASS; `cargo build`: PASS (Rust 1.98.1).
- `cargo test`: 20 passed, 0 failed (13 unit, 5 CLI contract, 2
  cross-surface regression incl. register→restart→inspect roundtrip,
  id/path collisions, unavailable/unknown reporting, dual-manifest and
  unsupported-schema failures leaving files unchanged, stale-pending
  journal reconciliation).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- Manual smoke: `forge --version`/`list`/`register`/`inspect` roundtrip
  against `tests/fixtures/valid-full`; unknown id exits 1 with
  `error[unknown-project]`; unknown subcommand exits 2 without creating a
  registry; empty registry lists an empty collection.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 24 passed, 0 failed
  (post-archive); `openspec validate core-manifest-registry --strict`:
  valid pre-archive.
- `git diff --check`: PASS; staged set reviewed (23 files, implementation
  + tests + fixtures + ADR + README + archive + promoted specs only).
- Committed as `15c225f`; no push performed.
- No shared Gate Runtime is configured; no Gate pass is claimed.

## Implementation cycle

1. Run `node scripts/check-openspec-change-names.mjs` before selection; failure blocks status/instructions and implementation.
2. Run `openspec list`, reconcile roadmap dependencies, and update the single pointer before work.
3. Run `openspec status --change <active-change>` and `openspec instructions apply --change <active-change>`; read all selected artifacts and applicable local rules.
4. Follow BFS analysis, structural pass, DFS requirement implementation, then BFS regression/completeness. Check tasks only against evidence.
5. Run the actual local build/test/integration commands and applicable Gate before archive; record exact failures and next actions. Gate FAIL or unresolved REVIEW_REQUIRED blocks completion when a Gate is configured.
6. Run the name checker and `openspec validate --all --strict --no-interactive`; review diffs and original impact surfaces.
7. Archive verified work without `--skip-specs`, inspect promoted canonical specs, and commit only related implementation/tests/archive/specs.
8. Advance `current_spec` to the next active eligible change, or remove the line when no active changes remain; update this evidence, commit HANDOFF separately and stop without push.

Planning-only documentation does not implement, archive or commit active changes. Future blockers must identify the exact failed command and next action; they must not be recorded as completion.

`platform-contract-consumption` implemented, verified and archived on 2026-09-26 as `2026-09-26-platform-contract-consumption`; canonical spec promoted to [openspec/specs/platform-contract-consumption/spec.md](openspec/specs/platform-contract-consumption/spec.md). Forge vendors `platform-contracts@d31495d` under `contracts/` (envelope plus 9 family schemas, `registry.json`, `vocabulary/secret-field-substrings.json` behind `contracts/manifest.json` with revision and per-file sha256), and `src/contract` owns the envelope, family ids, refusing status maps and the `CONTRACTS` inventory (29 rows covering every `0.1.0` constant). CLI `forge contract list` renders the inventory, `inspect <family>` renders the vendored schema, `emit <family> [TARGET]` projects `gate-result`/`readiness`/`release-evidence`/`capability`/`audit-event` from existing Core records (read-only, journals nothing), and `validate <file|->` checks envelope pattern, family, major, required fields and secret-field refusal. Status maps refuse rather than default: `unverified` readiness, `pending`/`partial` journal states and unknown values are typed `contract-invalid`. `policy::redact_credentials` unions the consumed `secret_field_substrings` (credential, private_key, bearer, session_ke, etc.) into the kv-secret redactor. MCP/API/portal gain no contract surface; every existing document stays byte-identical.

## Verification evidence (platform-contract-consumption, 2026-09-26)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --lib -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS; 590 lib tests, 0 failures (1 ignored: native scaffold). New suites: 11 `src/contract` unit tests (digest agreement, inventory completeness/agreement, each mapping row plus refusal, envelope pattern, secret-field refusal, validate accept/refuse for known/unknown/major/secret).
- CLI: `forge contract list` renders 29-row inventory with source revision; `inspect platform.gate-result` renders schema required fields; `validate -` accepts a valid gate-result envelope and refuses an invalid enum, an unknown family, a major mismatch and a secret-field payload with typed `contract-invalid` and non-zero exit.
- Cross-surface: `forge mcp serve` `tools/list` unchanged (18 tools, no contract tool); `forge portal view contract` refuses `portal-invalid`; `forge gate status`/`forge check` unchanged.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 37 passed, 0 failed; `git diff --check`: PASS.
- Parity: `scripts/contract-parity.sh` walks vendored families against `PLATFORM_CONTRACTS_DIR` or `../platform-contracts` and reports per-family fixture counts; ignored `parity_walk` test fails loudly when asked to run without a source.

`profile-matrix-evidence` implemented, verified and archived on 2026-09-26 as `2026-09-26-profile-matrix-evidence`; its three requirements (declared commands match the generated tree, command changes keep metadata and ownership truthful, full-matrix gate claim is evidence-gated) were promoted into [openspec/specs/profile-and-release-readiness/spec.md](openspec/specs/profile-and-release-readiness/spec.md). react-web's template now guards DOM access (`src/main.js` exports `greeting` and a guarded `mount` that is a no-op without `document`; `src/app.test.mjs` asserts both under bare `node --test` without touching the global), python-service's descriptor test uses the stdlib runner (`python3 -m unittest discover -s tests -v`; pyproject `pytest` config removed, `tests/test_main.py` ships as `unittest.TestCase`), and flutter-app's build is `flutter analyze` against a self-contained `analysis_options.yaml` (no `flutter_lints` fetch) while `flutter build appbundle` stays documented as the operator release command requiring `android/` and the Android SDK. The per-profile tree digests and the `MATRIX` expectations in `tests/workspace_metadata_contract.rs` were refreshed profile-by-profile (flutter `37af95…`, python `5f34cb…`, react `250f7f…`). `forge readiness matrix` now reports six `passed` rows (aspnet 8.0.424, flutter 3.47.1, nextjs v24.18.0/npm 11.16.0, python 3.12.3/build+unittest, react 11.16.0, rust cargo 1.98.1) with Forge absent from PATH and a disposable fixture, so the full-matrix gate is available on a runner that supplies the six toolchains; missing toolchains remain `unverified`. `docs/release-readiness.md` known-gaps narrowed to runner prerequisites.

## Verification evidence (profile-matrix-evidence, 2026-09-26)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`: PASS (all suites; 12 workspace_metadata_contract passed after refreshing python/flutter/react digests and the python `verification.command`).
- Readiness: `forge readiness matrix` `passed=6 failed=0 unverified=0 ready=true` with Forge absent (aspnet `dotnet build/test` 8.0.424, flutter `analyze`+`test` 3.47.1, nextjs `npm run build/test` v24.18.0, python `python3 -m build` + `python3 -m unittest discover -s tests -v` 3.12.3, react guarded DOM-free `npm test` 11.16.0, rust `cargo build/test` 1.98.1). Before: 4 passed, flutter `build appbundle` failed missing `android/app/build.gradle`, react `document is not defined`.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 36 passed, 0 failed; `git diff --check`: PASS.

`artifact-and-ci-baseline` implemented, verified and archived on 2026-09-27 as `2026-09-27-artifact-and-ci-baseline`; its seven requirements (truthful verification entry point, licence and package metadata consistency, declared toolchain floor enforcement, dependency and licence policy enforcement, reproducible installable artifact with digest, CI execution of Forge's own verification surfaces, honest labelling and explicit non-publication) were promoted into [openspec/specs/artifact-and-ci-baseline/spec.md](openspec/specs/artifact-and-ci-baseline/spec.md). `Cargo.toml` now carries `[workspace]` (so the recorded `.project.json` `verification.command` `cargo test --workspace` describes the real package graph), `[workspace.dependencies]` with inherited floors and no restated dev-duplicates, full `[package]` metadata (`readme`, `repository`, `homepage`, `keywords`, `categories`) and `rust-version = "1.87"`; `Cargo.lock` stays at `version = 3` so the floor toolchain can parse it. The 1.87 floor is derived, not asserted: live verification disproved the design's first guess (`clap 4` at 1.74, mirroring the sibling) — `cargo +1.74 check` fails on edition-2024 transitive manifests (`clap_lex 1.1.1`), `cargo +1.85.0 check` fails with `E0658 os_str_display` in `src/agent/mod.rs:1723,1748`, and `cargo +1.87 check --workspace --all-targets` passes (as does stable 1.98.1); derivation, previous floor (none) and raise procedure recorded in `docs/adr/0002-msrv-and-toolchain-floor.md`. New assets: `LICENSE` (MIT, matching the declaration), `CHANGELOG.md` (at the release plane's default path, newest entry `0.1.0` agreeing with the manifest and `forge --version` under contract test), `deny.toml` (advisories/licences/bans/sources; `LGPL-2.1-or-later` allow-listed explicitly for the transitive `r-efi` UEFI shim with its reason, no wildcard), `scripts/{package,checksum,install,smoke,bump}.sh` (stdlib POSIX shell; target triple from `rustc -vV`; archive holds binary + LICENSE + README + CHANGELOG with relative paths only; `dist/` gitignored), and `tests/artifact_baseline_contract.rs` (6 tests: licence, changelog, version agreement, workspace/inheritance, script presence, CI honesty). CI is now the 12-job graph (`fmt`, `clippy`, `test`, `native-scaffold`, `msrv`, `deny`, `governance`, `readiness`, `surfaces`, `contract-parity`, `gate`, `artifact`), every job with `permissions: contents: read` and an explicit timeout; the `readiness` job installs `dotnet 10.0.x`, correcting the `8.0.x` mismatch against the `dotnet 10.0.400` qualification. `scripts/release-check.sh` runs `cargo test --workspace`, the workspace clippy form, `cargo deny check` + `cargo audit` (missing tool blocks) and the parity walk when the source resolves, and its header now states that "gate" there means the readiness gate, never the shared runtime. No workflow pushes, tags, publishes or deploys; `auto-tag.yml` deliberately absent.

## Verification evidence (artifact-and-ci-baseline, 2026-09-27)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent`: PASS; 62 binaries, 1195 tests, 0 failures. One flake observed on the first full run (`api_contract::missing_token_returns_401`, `ConnectionReset` under parallel load, no `src/` change in this cycle to implicate); it passed in isolation and the clean re-run passed all 62 binaries.
- Native toolchain test (excluded from the aggregate as the known long-running scaffold build): `cargo test --lib -- --exact generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain` — PASS (464s, real cargo build+test of the generated tree, unaffected: this change touches no `src/` files).
- Floor proofs: 1.74 FAIL (edition-2024 manifest), 1.85.0 FAIL (`E0658`), 1.87 PASS, stable 1.98.1 PASS (`cargo check --workspace --all-targets` in each).
- `cargo deny check`: advisories/bans/licenses/sources ok; `cargo audit`: 0 warnings/errors (103 crates).
- End-to-end: `scripts/package.sh` → `dist/forge-0.1.0-x86_64-unknown-linux-gnu.tar.gz` (`e43a23ea…f37`) → `scripts/checksum.sh --verify` OK → `scripts/install.sh --prefix <scratch>` → `scripts/smoke.sh --bin <installed>` OK (`--version`/`--help`/`list`/`new`/`doctor`/`readiness artifact`/`check` parseable). Refusals proven: missing archive, unknown prefix, digest mismatch (exit 2, nothing written). Archive carries no absolute host paths; installed binary matches `./target/debug/forge` on version/list surfaces.
- Surfaces byte-deterministic across repeated runs (`gate status`, `check`, `fleet list`, `provider matrix`); no `src/` change, so all pre-change contract/cross-surface suites pass unchanged.
- Local `forge gate .`: exit 1 `gate-runtime-unavailable` (stale `~/.cargo/bin/driftwatchdog`, no `.driftwatch` store in this checkout) — no gate pass claimed; the sibling-owned `driftwatch init` prerequisite stands. CI's `gate` job is the environment where the real verdict executes.
- `shellcheck` is not installed on this host (recorded gap); all six scripts pass `sh -n` syntax checks and are exercised end to end instead.
- Pre-existing, out of scope: `node scripts/check-spec-governance.mjs` fails on ROADMAP's stale "no active changes" wording and a `TBD` placeholder in the archived `platform-contract-consumption` spec — both files untouched by this change and owned by other cycles; `openspec validate --all --strict --no-interactive`: 37 passed, 0 failed pre- and post-archive; `git diff --check`: PASS.
- Pointer state: `artifact-and-ci-baseline` archived (31/32 file tasks evidenced; the 32nd is this HANDOFF record); `openspec list` shows three remaining proposals.

`post-mvp-readiness` implemented, verified and archived on 2026-09-27 as `2026-09-27-post-mvp-readiness`; its five requirements were promoted into the new canonical [openspec/specs/readiness/spec.md](openspec/specs/readiness/spec.md). `README.md` gained a task-oriented `v0.1 command map` (`forge.yaml`/registries plus `import`/`list`/`inspect`/`new`/`doctor`, each with real behavior, typed refusal codes and honest limits, all traceable to the built binary), a `docs/quickstart.md` real-run transcript (import-proposal → `--accept` → list → inspect → new → doctor → governance `local` pass → `--version`, demo-root prefix redacted to `<demo>` under a stated rule, timestamps real), an install section that names the delivered digest-verified packaging as owned by the archived `artifact-and-ci-baseline` (never a plan), and the governance keep-local boundary including the sibling-owned adapter execute-bit prerequisite (mode `100644` re-observed this cycle). No `src/`, registry, CLI, contract or `.project.json` change. Two proposal-vs-reality reconciliations were required because `artifact-and-ci-baseline` archived after this proposal was authored: the install requirement now presents the delivered path (the original "no packaging yet" wording would have been false), and `LICENSE` was verified single-writer (created by `artifact-and-ci-baseline`) rather than duplicated; the canonical spec Purpose was set from the change (the archiver's TBD placeholder is not promoted).

## Verification evidence (post-mvp-readiness, 2026-09-27)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS (`forge 0.1.0`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS.
- `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent`: PASS; 1195 passed, 0 failed (no `src/` change; count matches the pre-change baseline).
- Quickstart oracle (scratch registry, real binary): proposal-only `import` changes nothing; `--accept --id` registers; `list` renders both projects; `inspect` renders the stored observation; `new --profile rust-web` stages the pinned assets with receipt; `doctor` reports `not healthy` with `UNAVAILABLE` (no git repo) rather than guessing while applicable L1 controls read `met`; `governance status` passes on the `local` default; incompatible `new --profile flutter-app --feature postgres` refused with `error[incompatible-profile]` and no files written. Transcript committed at `docs/quickstart.md` contains no host paths (3 demo-root occurrences redacted, verified by grep) and no credentials.
- `LICENSE` exists (MIT, matching `Cargo.toml:8`); single writer confirmed, no duplicate.
- Sibling-owned blockers re-observed, untouched: workspace-governance adapter mode `100644`; no `.driftwatch` store in this checkout, so no gate pass claimed.
- `node scripts/check-openspec-change-names.mjs`: PASS; `openspec validate --all --strict --no-interactive`: 37 passed, 0 failed pre- and post-archive; `git diff --check`: PASS (one archiver-generated blank-line-at-EOF in the canonical spec repaired before commit).
- Pointer state: `post-mvp-readiness` archived (22/22 tasks evidenced); `openspec list` shows two remaining proposals (`governance-vocabulary-consumption`, `gate-evidence-export-consumption`), both explicitly blocked awaiting sibling implementation evidence (`vocabulary.json` absent on this host; driftwatchdog `gate-evidence-export` authored but unselected) — no active eligible change remains, so the `current_spec` line is removed.


`governance-vocabulary-consumption` implemented, verified and
archived on 2026-09-27 as
`2026-09-27-governance-vocabulary-consumption`; its nine
requirements were promoted into the consumed
[openspec/specs/governance-provider-contract/spec.md](openspec/specs/governance-provider-contract/spec.md)
(governance vocabulary consumption, not to be confused with the
earlier workspace-governance adapter consumption).  Forge now consumes
the canonical governance vocabulary through a thread-safe loader with
documented resolution order and typed refusal boundaries: explicit path,
then `FORGE_GOVERNANCE_VOCABULARY`, then the vendored digest-pinned
copy — the same explicit-beats-env shape the gate plane already uses for
`FORGE_GATE_BIN`.  An explicitly named file that is missing or does not
parse refuses the request naming that file (never a silent fall-through);
the vendored tier degrades to `vocabulary-unavailable` and every
consulting surface behaves exactly as it did before the vocabulary
existed, so absence never blocks generation, import, doctor, check,
fleet or gate and nothing is fetched from the network or guessed
locally.  The consumed document carries `schema_version: 1`,
`profiles` (15 entries including `rust-product`, `typescript-product`),
`kinds` (12 entries including `platform`, `product`), `placeholder_markers`
(9 entries including `todo`, `fixme`, `xxx`, `hack`, `unimplemented!`,
`todo!`), and `secret_field_substrings` (15 entries); the four sets
are split by role — declaration values from governance, contract field
names from platform-contracts, captured-output redaction staying in
`policy::redact_credentials`, and repository-check words from governance.
The vocabulary is vendored as
`contracts/vocabulary/governance-vocabulary.json` with its own entry
in `contracts/manifest.json` carrying `source: workspace-governance` and
the pinned sibling revision.  `scripts/sync-contracts.mjs` copies the
sibling's `vocabulary.json` through the same digest-pinned mechanism,
and the offline manifest test verifies the vendored copy against its
recorded sha256.  External profile descriptors are validated at load:
a non-canonical `governance_profile` or `kind` refuses with
`invalid-profile` naming the field and the offending value, and no
project tree is staged; built-in descriptors are grandfathered (the
`flutter-product` divergence is surfaced by the doctor finding rather
than refused at load).  Runtime-name duplication is reduced to one shared
`policy::DRIFTWATCH_BINARY_CANDIDATES` used by gate, policy and
provider; the ordered defaults are rendered byte-compatible to the prior
pipe-separated literals.  The declaration-vocabulary doctor finding
compares the project's `.project.json` against the consumed vocabulary:
`applicable: false` when no declaration exists (the checker plane
stays byte-identical for such projects); `Pass` with `applicable: false`
for canonical declarations (informational, never gating health or
maturity); `Warn` for non-canonical kind or profile; and `Fail` only
for a Forge-authored capability claim whose `evidence_ref` no longer
resolves.  The finding is projected through the checker plane without
changing the alert document schema.  Forge's own kind is normalized
(`control-plane` → `platform` per the WG remap table) and
`evidence_status` left at `planned` pending WG canonicalization.  The
`quality` block declares the `placeholder_markers` override (Forge domain
words `placeholder` and `stub` removed, all eight debt markers retained)
with `placeholder_threshold: 0`; the decision is recorded in
`.ai-rules/completion.md` alongside the `tenancy`/`billing` blocked
declarations.  The `capabilities` block carries the five configured
capabilities with real `evidence_ref` paths and the two blocked
capabilities with the non-goal recorded.  Generated declarations emit
capabilities only when the profile descriptor declares them; all six
supported profiles currently declare none, so generated trees carry no
`capabilities` key.  The vocabulary is also consumed by the gate plane
(no surface change) and by the provider matrix.  Three residual
companion asks remain open in Workspace Governance:
`evidence_status` canonical normalization (field bounds and vocabulary
set), `rust-product` profile spec authorship, and the
`driftwatch init` action for this checkout's gate store initialization.
The `gate-evidence-export-consumption` proposal remains blocked awaiting
the sibling's real export document (live evidence disproved the last
two design guesses about sibling behaviour).

## Verification evidence (governance-vocabulary-consumption, 2026-09-27)

- `cargo fmt --all -- --check`: PASS; `cargo build`: PASS.
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo test --lib vocabulary`: PASS (18 tests including resolution order,
  explicit/env/vendored tiers, size bounds, traversal rejection, wrong
  schema version, directory candidate, vendored copy with pinned revision).
- `cargo test --lib doctor` (26 tests including 8 declaration-vocabulary
  finding tests): PASS.
- `cargo test --lib gate` (37 tests including 23 gate tests): PASS.
- `cargo test --lib profile` (43 tests including external descriptor
  validation with canonical/non-canonical mapping): PASS.
- `cargo test --lib generate::workspace` (12 tests including capabilities
  emission and determinism): PASS.
- `cargo test --test gate_contract` (16 CLI tests): PASS.
- `cargo test --test gate_provider_contract` (8 tests): PASS.
- `cargo test --test governance_contract` (20 tests): PASS.
- `cargo test --test governance_cross_surface` (filtered 0; no new surface): PASS.
- `cargo test --test workspace_metadata_contract` (12 tests): PASS.
- `cargo test --test workspace_metadata_cross_surface` (4 tests): PASS.
- `cargo test --test gate_cross_surface` (filtered 0; no new surface): PASS.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 37 passed, 0 failed.
- `git diff --check`: PASS.
- `git diff --stat`: 16 modified files, 931 insertions, 41 deletions;
  3 new tracked files (`src/vocabulary.rs`, `contracts/vocabulary/`).
- Vocabulary fixture: `contracts/vocabulary/governance-vocabulary.json`
  (schema_version 1, 15 profiles, 12 kinds, 9 placeholder_markers, 15
  secret_field_substrings, consumed at revision
  `0301ee88864d37957f5a6688005d5758b201cbe7`).
- Pointer state: `governance-vocabulary-consumption` archived (this record);
  `gate-evidence-export-consumption` remains blocked (sibling export not
  yet confirmed); no other eligible change; `current_spec` line removed.
- No gate pass claimed for this repository; no shared Gate Runtime
  configured; `driftwatch init` remains sibling-owned next action.

## Verification evidence (gate-evidence-export-consumption, 2026-09-27)

- `cargo build`: PASS.
- `cargo test --lib gate` (49 tests including 12 evidence module tests): PASS.
- `cargo test --test gate_contract` (28 CLI contract tests): PASS.
- `cargo test --test gate_cross_surface` (12 tests including 4 new): PASS.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `node scripts/check-spec-governance.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 36 passed, 0 failed.
- `git diff --check`: PASS.
- `git diff --stat`: 15 files, 2122 insertions, 16 deletions; 3 new files
  (`src/gate/evidence.rs`, `tests/fixtures/gate-evidence/NOTES.md`,
  `tests/fixtures/gate-evidence/forge-all-unverified.json`).
- Evidence fixture: `tests/fixtures/gate-evidence/forge-all-unverified.json`
  (schema_version 1, project `forge`, all 9 fields `unverified`, consumed
  at sibling revision `221faeca`).
- No gate pass claimed for this repository (`.driftwatch` uninitialized);
  honest state: all 9 fields `unverified`, no `verified` claim.
- Companion sibling next actions: none open (sibling's `driftwatch init`
  for this checkout remains open in that sibling's HANDOFF).
- Archive: `2026-09-27-gate-evidence-export-consumption`; spec promoted to
  `openspec/specs/gate-evidence-export-consumption/spec.md`; no active
  changes remain; `current_spec` line removed.

## decoupled-remote-publish archived (2026-09-28)

`decoupled-remote-publish` implemented, verified and archived on 2026-09-28 as
`2026-09-28-decoupled-remote-publish`; 8 requirements promoted to
[openspec/specs/decoupled-remote-publish/spec.md](openspec/specs/decoupled-remote-publish/spec.md).
New: `src/publish/{remote_compose,port_allocator,db_overlay,caddy}.rs`
(`RemoteComposeAdapter` default lane, no target scripts), `JenkinsAdapter`
retained behind `FORGE_PUBLISH_ADAPTER=jenkins`, per-command timeouts,
redacted failure evidence, `tests/decoupled_remote_publish_contract.rs`
(5 CLI tests). Live canaries green: alethefy, mortalect, crossify,
dharmatlas, cvunify. Full 20/20 rollout split to `fleet-live-rollout`
(deferred to a later cycle, after liveness + portal). Next active:
`fleet-liveness-status`, then `portal-web-ui`.
Validation at archive: 46 passed, 0 failed; suite green; `git diff --check` pass.

## Verification evidence (decoupled-remote-publish, 2026-09-28)

- `cargo build`: PASS.
- Full workspace suite (`--skip rust_scaffold_builds_and_tests_with_native_toolchain`): PASS, 0 failures.
- `cargo clippy --all-targets`: no new-file warnings (remaining `-D warnings` errors are pre-existing baseline drift, verified via stash).
- `rustfmt --check` on all touched files: PASS. `git diff --check`: PASS.
- `node scripts/check-openspec-change-names.mjs`: PASS.
- `openspec validate --all --strict --no-interactive`: 46 passed, 0 failed.
- Live: `forge publish all` green with healthy containers for 5 canary projects; registry/Caddyfile/overlay docs verified on target.
- No gate pass claimed (no Gate configured); no push performed.

# Proposal: Maintainer plugin platform

## Why

Forge already owns almost every capability an operator needs to
run a workspace of sibling projects from `vibecoding` through to
maintained-and-published, but almost none of it is reachable from
the browser, and the pieces that maintain *metadata* are
disconnected from the pieces that *push* metadata.

The concrete state, measured rather than assumed:

- `~/code` holds **96** sibling projects; **7** are registered.
- `src/import/mod.rs:430` detects `docker-compose.yml`,
  `compose.yaml`, `Containerfile` and friends and classifies them
  as `docker+compose`, but `src/catalog/source.rs:513` hard-codes
  `compose: None` for registry-registered projects. So
  `forge project list --compose docker+compose` returns nothing
  even though openpanel, hermora, rust-ecommerce and openlearning
  all ship a compose file. The detection exists and is discarded.
- `src/github/adapter.rs` already reads and proposes writes for
  `topic`, `description`, `homepage` and `language`, through a
  reviewable pull request by default — but it is reachable only
  from `forge project github`, never from the dashboard.
- `forge classify` / `forge describe` already carry domain,
  portfolio-tags, profile and **lifecycle**, through a
  suggest → review → approve/reject flow. But `classify suggest`
  *requires* `--suggested-value`: the operator supplies the answer,
  nothing is derived from the repository. And `semantic::approve`
  writes nothing anywhere, by design — "the remediation and
  adapter packages own the actual write" — so an approved
  classification never reaches GitHub topics or the registry.
- GitHub and OpenPanel are already two instances of one plugin
  contract (`{id, command, enabled}` in `providers.yaml`, JSON
  over stdin/stdout, bounded argv, per-run timeout, secret-leak
  rejection). There is no registry that names them as a set, and
  no way to ask "what can this workspace reach through its
  configured plugins".

The result is an operator who has to leave the browser, remember
CLI verbs, and hand-assemble metadata, for a workspace where
almost nothing is registered yet.

## What Changes

This change is the **first slice** of the maintainer platform. It
is deliberately one coherent outcome: *Forge observes, reviews and
publishes each project's own metadata outward through its
configured plugins, and the browser can search the result.*

- **Metadata evidence for registered projects.** `local_record`
  populates `compose` and `ci` from the project's own directory
  using the detection that already exists in
  `src/import/mod.rs`, instead of `None`. A project with
  `compose.yaml` reports `docker+compose`; one with only a
  `Dockerfile` reports `docker`; one with neither reports
  `none`. The same directory walk already proves the registry
  read, so this adds no new filesystem boundary.
- **Derived classification.** `forge classify derive` proposes
  domain, language and lifecycle from evidence already in the
  repository — the manifest's declared `stack`/`profile`/
  `maturity`, the GitHub `topics` and `language` the adapter
  already observes, and the README's first heading. It is
  deterministic, runs no model, and never writes: it produces a
  proposal through the existing `forge-semantic-proposal/0.1.0`
  contract so the operator still reviews before anything moves.
- **Approved metadata reaches the remote.** `forge classify apply`
  takes proposals that are already `Approved` and hands the
  resulting field set to the configured plugin as a propose
  request. The plugin answers with a reviewable change set and a
  pull-request reference; Forge never mutates a remote itself, so
  GitHub stays behind its adapter and its PR flow.
- **A plugin registry.** `forge plugins` lists every configured
  plugin with its id, kind, enabled state and the capabilities it
  advertises, and refuses to report a capability a plugin has not
  declared. GitHub and OpenPanel become named entries in this
  registry rather than two hard-coded special cases. Adding a
  third remote is a `providers.yaml` entry and a plugin
  descriptor, not a code change.
- **The browser gains the maintainer surface.** A **Maintain**
  card per project, above the read-only cards: current GitHub
  metadata, the derived classification with per-field approve /
  reject, and one action to publish approved metadata through the
  plugins. Plus a fleet-level filter row carrying the catalog's
  existing predicates — language, lifecycle, profile, compose, CI,
  tag — so "show me the Python projects that ship compose and are
  not yet published" is one control rather than a CLI incantation.

No change to the plugin transport itself: `providers.yaml`,
JSON-over-stdin/stdout, the bounded argv, the timeout and the
secret-leak rejection are reused exactly as they are. This change
adds a registry that *names* plugins and a metadata request kind,
not a new invocation path.

## Package Boundary and Split Assessment

| Package | Single outcome | Boundary | Independent oracle |
|---|---|---|---|
| `maintainer-plugin-platform` (**this**) | Compose/CI evidence reaches the catalog; classification is derivable; approved metadata is publishable through a named plugin registry; the browser can search and maintain | `src/catalog/source.rs`, `src/semantic/`, `src/plugins/` (new), `src/api/`, `frontend/`, `src/github/` | `forge project list --compose docker+compose` returns the four known compose projects; `forge plugins` lists github + openpanel; the new contract tests; the surviving suites |
| `forge-workspace-bulk-onboard` (follow-on) | Take `~/code` from 7 registered to all 96 in one reviewed batch | registry only | `forge list` count |
| `forge-project-retirement` (follow-on) | The "death" half of the lifecycle | see the recorded defect | its own ruling first |

One package, one outcome: *an operator can see what Forge knows
about a project, correct it, and push it outward — without leaving
the browser*. Splitting it further would ship a catalog that still
reports `compose: None` while the browser claims to search it.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Reusable contract | Gap | Decision |
|---|---|---|---|
| `src/catalog/source.rs` `local_record` | Every `CatalogRecord` field, `predicate_values`, the `--compose/--ci` query predicates | Sets `compose: None` unconditionally | **populate** from the existing detection |
| `src/import/mod.rs` `detect_compose` / `detect_ci` | Already walks `Dockerfile`, `compose.{yaml,yml}`, `Containerfile`, `.github/workflows` and returns a `Detection` | Only reached at adopt time; the catalog's per-read path never calls it | **reuse** as the single detector |
| `src/semantic/` | `forge-semantic-proposal/0.1.0`, five proposal kinds, suggest/list/show/approve/reject, `--confirm` | `classify suggest` requires the operator's value; `approve` writes nothing | **reuse** the contract; add `derive` (no write) and `apply` (write via plugin) |
| `src/github/` | `forge-github-metadata/0.1.0`, `ALLOWED_PROPOSED_FIELDS = [topic, description, homepage, language]`, PR-by-default mutation, `normalize_observation` keeping topics/release-tags/portfolio-tags as three namespaces | Reachable only from the CLI | **reuse** as the GitHub plugin's implementation |
| `src/publish/providers.rs` | `{id, command, enabled}`, `select_provider`, `invoke_provider` with bounded argv, timeout, secret-leak rejection | Publish-shaped request only; no registry that names plugins as a set | **reuse** the transport; add the registry + a metadata request kind |
| `src/delivery/handlers.rs` | OpenPanel as a first-class provider, `preflight → stage → promote` | Hard-coded `PROVIDER_ID = "openpanel"` | **reuse**; become a registry entry rather than a constant |
| `frontend/` | Fleet table, workbench lifecycle card, action rows | No filter row, no maintainer card | **reuse** the shell; add both |

## User Experience and Interface Impact

`UI/UX: additive, with one behaviour change operators will feel`.

- The fleet gains a filter row (language, lifecycle, profile,
  compose, CI, tag) above the table. The existing free-text
  search keeps working; the filters compose with it.
- Each project gains a **Maintain** card between the action rows
  and the upgrade workflow: what GitHub currently says, what Forge
  derived, per-field approve/reject, and one publish action.
- A project whose compose file Forge cannot see reports `none`
  rather than a blank. A GitHub remote Forge cannot reach reports
  `unavailable` with the reason, never a fabricated empty field.
- Behaviour change: `--compose` and `--ci` predicates start
  returning rows where they previously returned none. Any script
  that treated "no rows" as "no projects use compose" was already
  wrong, but it will see different output.

## BFS Impact Map

- **Capabilities added:** `forge classify derive`,
  `forge classify apply`, `forge plugins`, catalog compose/CI
  evidence for registered projects, browser Maintain card,
  browser fleet filters.
- **Capabilities unchanged:** every CLI command and its flags;
  the plugin transport; the GitHub adapter's PR-by-default
  mutation; the semantic proposal contract; the registry and
  journal schemas.
- **Contracts:** the plugin contract gains a metadata request
  kind. It is additive: an existing plugin that does not
  recognise it answers `unsupported`, which the registry reports
  as a capability gap rather than a failure.
- **Callers:** `local_record` gains two fields; the browser gains
  two surfaces; `semantic` gains two entry points.
- **Tests:** new contract tests for the detectors, the derive
  pass, the plugin registry and the request kind. Every existing
  suite stays green.
- **Privacy/security:** metadata pushed outward is limited to the
  four already-allowed fields plus derived classification; no path,
  no credential. The plugin transport's secret-leak rejection
  applies unchanged.

## Capabilities

- `forge-project-metadata-evidence`: compose/CI detection reaches
  the catalog for registered projects.
- `forge-project-classification-derivation`: classification is
  derivable from repository evidence, reviewable, and appliable.
- `forge-plugin-registry`: GitHub, OpenPanel and future remotes
  are named, enumerable, capability-declared plugins.
- `forge-web-maintainer-surface`: the browser can search the
  fleet by the catalog's predicates and maintain one project's
  metadata.

## Non-goals

- No change to the plugin transport: `providers.yaml`, the JSON
  request/response shape, the bounded argv, the timeout and the
  secret-leak rejection are reused as they are.
- No model, provider or network call in the derive pass. It is
  deterministic over local evidence.
- No bulk onboarding of the other 89 projects; that is a
  follow-on.
- No project retirement; see the recorded defect in HANDOFF.
- No new GitHub field beyond the four already in
  `ALLOWED_PROPOSED_FIELDS`.
- No rewrite of `providers.yaml`. Adding a plugin is an entry in
  the existing file plus a capability declaration.
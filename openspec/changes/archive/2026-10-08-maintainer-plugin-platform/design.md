# Design: Maintainer plugin platform

## 1. Implementation boundary

- **Repository / project:** the `forge` crate. No sibling touched.
- **Modules changed:**
  - `src/import/mod.rs`: expose `detect_compose` / `detect_ci` beyond
    adopt time (visibility only; no logic change).
  - `src/catalog/source.rs`: `local_record` populates `compose`
    and `ci` from those detectors instead of `None`.
  - `src/semantic/`: new `derive` (evidence → proposal, no write)
    and `apply` (approved proposal → plugin request).
  - `src/plugins/mod.rs` (new): the plugin registry — enumerate
    configured plugins, read their declared capabilities, and
    route a metadata request to the plugin that advertises it.
  - `src/api/`: admin routes for plugins, derive, apply; the
    catalog query already accepts the predicates.
  - `src/main.rs`: `forge plugins`, `forge classify derive`,
    `forge classify apply`.
  - `src/api/command_catalog.rs`: the new CLI surfaces as
    `web_exec` rows so the browser's action renderer picks them
    up from the same contract as everything else.
  - `frontend/`: fleet filter row; per-project Maintain card.
- **Modules reused unchanged:** `src/publish/providers.rs`
  (`ProviderEntry`, `select_provider`, `invoke_provider`),
  `src/github/` (adapter + normalize + PR-by-default mutation),
  `src/semantic/proposal.rs` (`forge-semantic-proposal/0.1.0`),
  the registry and journal schemas.

## 2. The plugin boundary

An existing plugin is `{id, command, enabled}` in
`providers.yaml` or `$FORGE_PUBLISH_PROVIDER_CONFIG`, invoked by
`invoke_provider` with a JSON request on stdin, bounded argv, a
per-run timeout and secret-leak rejection. That transport is
reused verbatim.

What is added is the **registry**: a named view over the
configured set, plus one new request kind.

```
forge plugins list          # every configured plugin
  id        openpanel                       enabled
  id        github                         enabled
  kind      delivery                       # preflight stage promote
  kind      metadata                       # topic description homepage language
```

A plugin declares what it can do in a descriptor next to its
command, so the registry can answer "which plugin maintains
metadata" without hard-coding ids:

```yaml
plugins:
  - id: github
    kind: metadata
    command: gh-forge-adapter
    capabilities: [topic, description, homepage, language]
    enabled: true
```

`src/plugins/` owns: reading the descriptor, validating the
capability list against a closed set, and reporting a plugin that
declares an unknown capability as `invalid` rather than silently
ignoring it. A plugin with no descriptor is a `delivery` plugin,
which is every plugin that exists today — so no existing
`providers.yaml` changes and no existing plugin breaks.

The new request kind is metadata propose:

```json
{ "contract": "forge-metadata-propose/0.1.0",
  "operation_id": "…", "project_id": "…",
  "fields": { "description": "…", "topic": ["…"] },
  "mode": "pr" }
```

A plugin that does not recognise the contract answers
`unsupported`, and the registry reports that as a capability gap
on that plugin, not as a failed run. That keeps the contract
additive: OpenPanel keeps working exactly as it does today
without learning about metadata.

## 3. Metadata evidence for registered projects

`src/import/mod.rs` already has the detectors, walking the
project's own directory:

- compose: `Dockerfile`, `Containerfile`,
  `docker-compose.yml`, `docker-compose.yaml`, `compose.yaml`,
  `compose.yml` → `docker+compose` when a compose file is
  present, else `docker`, else `none`; the evidence lists the
  filenames actually found.
- CI: `.github/workflows/` → `ci_present`, else `ci_missing`.

`local_record` in `src/catalog/source.rs` receives the resolved
project directory already (it reads the record's path today), so
the detection adds no new filesystem boundary and no new
capability. A project whose directory is unreadable reports
`none` with `evidence: unavailable`, never a false `none`.

The `--compose` and `--ci` predicates in
`src/catalog/query.rs` then start matching. This is the honest
outcome of code that already existed: the detection was written,
and discarded.

## 4. Deriving classification

`forge classify derive [TARGET]` reads only what is already in
the repo or already observed, and proposes through the existing
`forge-semantic-proposal/0.1.0` contract:

| Proposal kind | Derived from | Confidence rule |
|---|---|---|
| `profile` | `forge.yaml` `project.profile` | `high` — it is declared |
| `lifecycle` | `forge.yaml` `project.maturity` / `target_maturity` | `high` when equal, `medium` when a gap exists |
| `portfolio-tags` | GitHub `topics` via `normalize_topics`, plus the catalog's `languages` | `medium` — GitHub topics are human-set and may be stale |
| `domain` | README's first `#` heading, plus the manifest `name`/`runtime` | `low` — a heading is a weak signal |

No model, no provider, no network. `--confidence` is bounded and
every derivation records its evidence, so the operator reviews a
claim with its source rather than a bare assertion.

The five proposal kinds and the `Suggested → Approved/Rejected`
state machine are unchanged. `derive` writes proposals only;
`approve` still writes nothing, by design.

## 5. Applying approved metadata outward

`forge classify apply [TARGET] --confirm` is the only new write
path, and it is deliberately narrow:

1. Read proposals for the target; refuse unless every selected one
   is already `Approved`. A `Suggested` proposal is refused by
   name — the operator must approve it first.
2. Project the approved values onto the four fields
   `ALLOWED_PROPOSED_FIELDS` permits (`topic`, `description`,
   `homepage`, `language`). Anything else is refused rather than
   silently dropped, because a silently dropped approval is a lie
   about what was published.
3. Build a `forge-metadata-propose/0.1.0` request with
   `mode: "pr"` — the GitHub adapter's default and its safe path.
   Direct mutation is not reachable from `apply`.
4. `invoke_provider` hands it to the plugin that advertises
   `kind: metadata`. The response carries the reviewed change set
   and a PR reference.
5. Record a journal row naming the plugin, the fields, and the PR
   reference. No path, no credential.

Forge never mutates a remote directly. GitHub stays behind
`src/github/adapter.rs` and its PR flow; OpenPanel stays behind
`src/delivery/`. The only new thing is that Forge knows which
plugin to ask.

## 6. Browser surface

- **Fleet filters.** A row above the project table carrying the
  catalog's existing predicates — language, lifecycle, profile,
  compose, CI, tag — wired to `GET /v1/projects/catalog`, which
  already accepts all of them and already 401s without a session.
  The filters compose with the existing free-text search. No new
  endpoint.
- **Maintain card.** Per project, above the read-only cards: the
  GitHub observation (description, topics, homepage, language) as
  Forge last saw it with its freshness, the derived proposals with
  per-field approve/reject, and one action that applies the
  approved set through the plugins. A GitHub remote Forge cannot
  reach renders `unavailable` with the reason, because a blank
  field and an unreachable field are different facts.

Both reuse `buildActionControl`, so the Maintain actions inherit
the preview → confirm → apply discipline and the typed-field
guarantees already pinned by the contract tests.

## 7. Contract and compatibility

- Plugin contract: additive. An unrecognised request kind answers
  `unsupported`.
- `providers.yaml`: no existing file needs editing; a plugin with
  no descriptor is a `delivery` plugin.
- Catalog JSON: `compose` and `ci` become populated where they
  were `null`. Additive to consumers; a filter that matched
  nothing before may match now.
- CLI: every existing command and flag is unchanged.
- Registry and journal schemas: unchanged.

## 8. Failure and boundary policy

| Case | Behaviour |
|---|---|
| Project directory unreadable | `compose: none`, `evidence: unavailable` — never a false `none` |
| Plugin command missing | Registry reports the plugin `unavailable` with the reason; the other plugins still list |
| Plugin declares an unknown capability | `invalid`; the registry refuses to advertise it |
| Plugin does not recognise `forge-metadata-propose/0.1.0` | `unsupported`, reported as a capability gap, not a failed run |
| No plugin advertises `kind: metadata` | `apply` refuses naming the capability no plugin has |
| Proposal not `Approved` | Refused by name |
| Approved value outside the four allowed fields | Refused, not dropped |
| GitHub adapter absent or token missing | Observation reports `unavailable`; `derive` falls back to local evidence and says so |

## 9. Verification oracle

- `forge project list --compose docker+compose` returns
  openpanel, hermora, rust-ecommerce, openlearning once
  registered — the four with a compose file on disk.
- `forge project list --ci present` returns projects with
  `.github/workflows`.
- `forge plugins` lists github (metadata) and openpanel
  (delivery) with capabilities; a provider-only install lists
  delivery plugins only.
- `forge classify derive` on a project with a `forge.yaml`
  proposes profile + lifecycle with `high` confidence and
  records evidence; it writes no field.
- `forge classify apply` refuses a `Suggested` proposal by name
  and, on an approved set with no metadata plugin, refuses
  naming the missing capability.
- The browser's filter row returns the same rows as the
  equivalent `forge project list` invocation for the same
  predicate.
- Every surviving test target stays green at its previous count.

## 10. Decision ledger

- **Resolved:** reuse the existing provider transport verbatim;
  add a registry that names plugins, not a second invocation path.
- **Resolved:** a plugin without a descriptor is a `delivery`
  plugin, so no existing configuration changes.
- **Resolved:** `derive` is deterministic over local evidence —
  no model, no network.
- **Resolved:** `apply` is PR-mode only. Direct mutation stays
  unreachable from the new path.
- **Resolved:** the catalog's `compose`/`ci` are populated rather
  than left `null`; the code that detects them already exists.
- **Blockers:** none. Bulk onboarding of the other 89 projects
  and project retirement are follow-ons, not this package.
# Design: Generated project metadata and read-only search

## 1. Implementation boundary

Repository `/home/paul/code/forge`; Rust 2024 workspace, Forge Core plus thin CLI and stdio MCP transports. Add Core module `src/project_metadata/` (model, build, receipt, search), export from `src/lib.rs`; extend nested `ProjectCommands` in `src/cli/commands_ops.rs` and dispatch in `src/cli/project.rs`; extend MCP registry/dispatch in `src/mcp/tools.rs` and `src/mcp/methods.rs`; add focused unit and cross-surface contract tests. Update `docs/architecture.md`, `docs/quickstart.md` or the relevant metadata guidance, README command map, and the canonical metadata spec through archive. Do not alter `CatalogRecord`, its serialization, workspace schema, CodeGraph, sibling repositories, or project registration lifecycle.

## 2. Language and runtime

Rust 2024, the workspace's locked Rust toolchain and existing serde/serde_json/serde_yaml, sha2 and filesystem dependencies. No new crate or runtime. Run `cargo fmt --check`, `cargo test --lib project_metadata`, targeted integration suites, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive`, and the bounded local Gate per `.ai-rules/workflow.md`. CLI and MCP share Core functions; do not implement separate search behavior in transports.

## 3. Ownership and shared code

Forge owns `.forge/project-metadata.json` and `.forge/project-metadata.receipt.json` as generated, project-local artifacts. `.project.json` remains workspace-governance-owned input; its `discoverability`, `interfaces`, `capabilities`, `integrations`, and taxonomy values are read-only. `forge.yaml` is optional Forge-owned identity/profile input. Existing catalog Core supplies registry project identity and explicit project paths; it is not extended. No sibling code, shared crate, or CodeGraph runtime dependency. CodeGraph's CLI/own MCP remains a separately owned option and is not invoked.

## 4. User experience and interface

A maintainer selects one project path with `forge project metadata build PATH`; the first run reports `created`, a later unchanged run `unchanged`, and an authorized refresh reports `updated`. A foreign artifact or modified receipt-owned file refuses with its path and a `project-metadata-conflict` error; malformed source names the source file and field. For agents, `search_project_metadata` is listed as read-only in MCP and accepts a required non-empty `query`, optional `project_id`, and optional `limit` (default 10, maximum 50). `forge project metadata search QUERY [--project ID] [--limit N]` returns the same Core result. Output includes contract/version, matches and per-project issues; an empty match is explicit. The search scans only projects in the local Forge SQLite registry, never discovers parent directories or workspace roots. It reads existing bundle artifacts and performs no writes. CLI human and JSON formats use current `Format` convention; MCP uses JSON. Keyboard operation and screen-reader output are not applicable to CLI/stdio; help text and MCP tool descriptions/JSON schemas are the accessible entry points. No browser interface is changed.

## 5. Behavioral model

### Bundle

`forge-project-metadata/1.0.0` document fields: `contract`, `project_id`, `name`, `description`, `keywords`, `topics`, `audience`, `use_cases`, `search_phrases`, `role`, `stack`, `profile`, `lifecycle`, `capabilities`, `interfaces`, `integration_ids`, `documentation`, `specifications`, `source_revision`. Arrays are normalized, case-insensitively de-duplicated and sorted; optional fields are omitted, never guessed. `name` comes from `forge.yaml` when valid or the explicit project id; description, keywords, topics, audience, use cases, search phrases and homepage come only from `.project.json.discoverability`; role, stack, profile, lifecycle, capability ids, owners and evidence, interface ids, provide/consume side, provider, mechanism, contract, digest, entry, required flag and evidence_ref come only from `.project.json`, with `forge.yaml` profile used only when declaration profile is absent. `integration_ids` contains only declared integration ids. Documentation also includes `README.md` when present; specifications are canonical `openspec/specs/*/spec.md` directory identifiers read under the selected root, sorted by id. README prose, spec prose and source code are not indexed. Project metadata fields with malformed declared types or missing required edge identifiers are rejected rather than omitted. Input values pass the existing credential redactor and byte/count bounds before serialization. `source_revision` is a SHA-256 over the bounded `.project.json`, optional `forge.yaml`, optional `README.md`, and sorted specification identifiers, not Git state.

### Build transition

| Current target state | Operation | Result |
|---|---|---|
| No metadata and no receipt | build | stage and promote both; `created` |
| Valid receipt + exact artifact digest; regenerated bytes equal | build | write nothing; `unchanged` |
| Valid receipt + exact current artifact digest; regenerated bytes differ | build | stage and promote both; `updated` |
| Artifact exists without receipt, receipt invalid, owned bytes differ from receipt, or path is unsafe | build | typed refusal; no filesystem change |
| Malformed required JSON, invalid declared fields, or unreadable bounded source | build | typed refusal naming file/field; no filesystem change |

Receipt stores schema version, generator id, project id, artifact relative path and SHA-256. Its source revision is the SHA-256 of the bounded `.project.json`, optional `forge.yaml`, optional README bytes, and sorted specification identifiers; generation starts no subprocess. Temporary files are staged beneath `.forge/` and promoted without replacing a target that appeared concurrently. Refresh first preserves the prior owned pair; promotion failures attempt rollback and report rollback failures. A process interruption between promotions is diagnosed as an invalid pair and is never silently overwritten. The selected root is canonicalized; all reads are beneath it; symlinks escaping it are refused. Build is the only writer, and writes only these two paths.

### Search

Search opens the existing local registry read-only and enumerates registered project records. Optional `project_id` filters before file reads. For each path: canonicalize and verify root; require a valid receipt/artifact pair; parse the exact metadata contract; match query tokens case-insensitively against id/name/description/keywords/topics/audience/use-cases/search phrases/capability/interface identifiers and specification identifiers. Every token must match; relevance is the number of matched metadata fields, descending; results include the matched field names and tie-sort by project id then path. Query terms are bounded to 128 Unicode scalar values and at most 8 non-empty whitespace-separated tokens; limit is 1..=50. Missing, malformed, stale-source-revision, or unreadable artifacts are included as typed per-project issues and excluded from matches. No issue is reported as a successful empty match. The reader never opens or migrates a missing registry and does not write registry or project bytes.

## 6. Contract and compatibility

The bundle has a separate stable contract id `forge-project-metadata/1.0.0`; a changed or unknown contract is an incompatibility issue, not a guessed parse. CLI shape: `forge project metadata build <PATH>` and `forge project metadata search <QUERY> [--project <ID>] [--limit <1..50>]`. MCP tool `search_project_metadata` has `{query: string, project_id?: string, limit?: integer}`; no caller-supplied path or arbitrary source. MCP result is `{"metadata_search": {"contract": ..., "matches": [...], "issues": [...]}}`. `tools/list` advertises the read-only tool only after implementation and contract tests exist. Existing MCP tools and catalog contract do not change except registry count +1. No migration or environment variable.

## 7. Failure and boundary policy

- Missing `.project.json`: build refuses with a source-file error; search reports `metadata-missing` for a registered project without an artifact.
- Invalid `.project.json` shape, malformed `forge.yaml`, malformed receipt/artifact, unknown bundle contract: typed refusal for build or per-project issue for search; never silently discard malformed values.
- Foreign target, tampered receipt, or path escape: refuse before writes; no overwrite, delete, or fallback.
- Missing or invalid declared inputs: build fails with a source-file error; the deterministic source revision is derived from bytes and file names, not Git HEAD.
- Missing/unreadable registry: CLI/MCP returns a typed error; no database creation, migration or journal repair.
- Missing/corrupt metadata per registry entry: search returns remaining matches plus a named issue; issue counts and source paths are redacted and bounded.
- Empty query, >8 tokens, >128 characters, or limit outside 1..=50: typed invalid-argument error before reads.
- No matching metadata: empty matches plus issues and a stable message; not an unavailable source or pass.
- No retry, subprocess, network, background indexing, or cross-repository write.

## 8. Verification oracle

Permanent tests exercise consumer-visible behavior: deterministic bytes across repeated builds; each declared discovery/interface field survives generation and remains searchable; derived project ids and sorted specification labels; empty/missing declaration errors; malformed source and unknown contract errors; create/update/unchanged receipt states; modified/foreign artifact refusal without changed bytes; path traversal/symlink escape refusal; read-only registry and project bytes during search; token AND behavior; relevance and deterministic ties; query/limit boundaries; missing/corrupt/stale bundle issue per project while valid peers still match; no-match versus missing-artifact distinction. CLI and MCP integration tests call the same Core behavior and assert equivalent matches/issues; MCP registry marks tool `read_only`; no CodeGraph binary is needed. Run the repository-wide suite and Gate after targeted tests.

## 9. Decision ledger

- **Metadata owner:** Forge owns generated artifact and receipt; workspace-governance owns input schema. Evidence: declaration schema and Forge catalog boundary above.
- **Storage:** project-local `.forge/` to keep portability and eliminate a second central database; digest receipt protects user edits, patterned after standard snapshots.
- **Search scope:** local registry only; explicit registration prevents hidden workspace discovery. CodeGraph integration is deferred because its source repository and stable interface contract were not found, and its indexing lifecycle is independent.
- **Search semantics:** deterministic token AND matching, field-count relevance, stable id/path tie-break; no vector/AI ranking, to preserve reproducibility and privacy.
- **Generation timing:** explicit command only; no implicit write during import/new because it would broaden unrelated workflows and make artifact freshness depend on hidden sources.
- **Refresh authorization:** a valid receipt is the authority to replace the exact owned artifact; any user edit blocks refresh.
- **UX:** CLI and agent MCP only; no browser flow. N/A for browser, responsive, and screen-reader requirements because neither surface renders a browser UI; typed CLI help and JSON tool schema are used instead.
- **No unresolved implementation decision.**
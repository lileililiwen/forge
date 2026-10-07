# Tasks: Show published projects in the web fleet

## 1. BFS — Baseline and impact coverage

- [x] Confirm the proposal, design and the delta spec agree and that a
      separate implementer could proceed without inventing architecture or an
      owner.
- [x] Read `src/api/fleet.rs` (`gather`, `CandidateRow`, `SourceDescriptor`,
      `build_envelope`, `render_row`), `src/registry/mod.rs` (`operations`
      schema, `OperationEntry`, `recent_operations`), `src/api/admin.rs`
      `redact_local_paths`, and `src/api/ui/data.rs` `pick_last_publish` as the
      reuse contract.
- [x] Map the modified requirement and each scenario to its source, row field,
      summary key, frontend render path and contract-test assertion.
- [x] Add `tests/forge_web_publish_fleet_contract.rs` with the seven scenarios
      from design §7.
- [x] Record the exact toolchain (Rust 2021, `rustc 1.87` floor) and the
      project-local ownership (extend `src/api/fleet.rs` and
      `src/registry/mod.rs`; reuse everything else unchanged).

## 2. DFS — Requirement-by-requirement implementation

- [x] `src/registry/mod.rs`: add `PublishedOperation` and
      `Registry::latest_publishes(limit) -> Result<Vec<PublishedOperation>>`,
      a bounded `GROUP BY project_id` over
      `kind IN ('publish','publish.github')` selecting the newest `op_id` per
      project, ordered newest-first with a `LIMIT`.
- [x] `src/api/fleet.rs`: add `RowSource::Published` (`"published"`), a
      `PublishProjection` struct, a `publish: Option<PublishProjection>` field
      on `CandidateRow`, and a local `redact_local_paths` copy.
- [x] Implement `read_published(db_path, max_age, local_ids, &mut candidates,
      now) -> SourceDescriptor`: honour `FORGE_PUBLISH_HISTORY` and
      `FORGE_PUBLISH_HISTORY_LIMIT`; merge into an existing `registry`
      candidate when the id is locally registered, else push an observed row;
      derive `healthy`/`stages` from `detail`.
- [x] Wire the source into `gather` (after the registry rows, before/with the
      external sources), emit the `published` descriptor, and add
      `published` to `summary.by_source`.
- [x] `render_row`: emit the optional `"publish"` object; bump
      `WEB_FLEET_CONTRACT_VERSION` to `forge-web-fleet/0.2.0`.
- [x] `frontend/app.js`: add `published` to `SOURCE_LABELS`/`SOURCE_BADGE`,
      render a publish chip (state + health + redacted target/revision) in the
      evidence cell; `frontend/index.html`: add the `published` source-filter
      option.
- [x] Implement the delta spec scenarios alongside the code (each scenario has
      a matching contract-test assertion).

## 3. BFS — Cross-surface regression and completeness

- [x] Verify `forge_web_fleet_contract.rs` stays green (its fixtures write no
      publish operations).
- [x] Verify the inventory and workspace-registry sources are unchanged and
      the conflict semantics still hold.
- [x] Verify no response can carry an absolute path, credential or adapter
      binary from either a merged or a standalone published row.
- [x] Verify the projection is read-only: no `operations`/`projects` write and
      no project-file change.
- [x] Confirm no Core publish/delivery/identity source file was modified
      beyond `src/api/fleet.rs` and `src/registry/mod.rs`.

## 4. Verification

- [x] `cargo fmt` then `cargo fmt --check` clean.
- [x] `cargo build` 0 errors.
- [x] `cargo test` green for `forge_web_publish_fleet_contract` (new),
      `forge_web_fleet_contract`, the `forge` bin catalog tests, and
      `cargo test --lib api::`; record the actual pass counts.
- [x] `node scripts/check-openspec-change-names.mjs` PASS and
      `openspec validate forge-web-publish-fleet --strict --no-interactive`
      PASS, then `openspec validate --all --strict --no-interactive`.
- [x] `git diff --check` clean; review newly added files.
- [x] Browser/API smoke against a **copy** of the operator's registry
      (`cp ~/.local/share/forge/registry.db /tmp/forge-smoke.db`), never the
      real file: sign in, fetch `/v1/admin/projects`, assert the published-only
      projects appear as observed rows and no absolute path appears. Recorded
      as evidence, not assumed.

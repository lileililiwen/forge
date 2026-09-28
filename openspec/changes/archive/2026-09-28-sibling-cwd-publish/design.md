# Design: sibling cwd publish via Forge

## Context

Forge already publishes a single project through `forge publish --project <id>` (registry lookup) and `forge publish --folder <path>` (explicit folder), and a fleet through `forge publish fleet --inventory <path>` (versioned `forge-project-inventory/0.1.0` with `compose_ready`/`compose_missing`/`invalid`/`source_unavailable` classification). Both modes converge on the same `forge-publish-provider/0.1.0` external-provider contract (`src/publish/providers.rs`, `src/main.rs::cmd_publish_provider`), and `jenkins-local/adapters/forge-publish-provider.py` is the Mac runtime plugin behind it. Sibling checkouts already carry `.project.json` (`schema_version: 1`, `id`, `kind`, `profile`, `verification`, `deployment.deployable/compose_file`) and — when deployable — `docker-compose.yml`. The gap is ergonomic and ownership-related: a sibling cannot `forge publish` from its own directory without naming itself. The sibling should depend only on `forge` on `PATH`; Forge should discover identity from cwd, capture HEAD, and delegate to the provider already configured for that checkout.

## Goals / Non-Goals

**Goals:**
- A sibling publishes itself via `forge publish` from its own checkout with zero central registration, using cwd-discovered identity.
- Explicit overrides keep working: `--project`/`--folder` beat cwd; `--provider`/`--revision`/`--dry-run` keep semantics.
- Same provider engine, journal, and status projection for bare and explicit modes (no second code path).
- No sibling ever calls `jenkins-local` scripts directly; Forge remains the only user-facing publish entry point.
- Failures typed and explicit; no silent fallback scans.

**Non-Goals:**
- Spec-first/OpenSpec gating around publish (workflow policy, not CI/CD precondition).
- Auto-scaffolding `docker-compose.yml`/`Dockerfile` for non-deployable siblings (report `compose_missing` so a later change can scaffold).
- Fleet parallelism, `forge api` publish routes, local Docker watch loop.
- Changes to `workspace-governance` registry or `jenkins-local` Mac routing/Caddy/Cloudflare.

## Decisions

### D1: Cwd identity source order — `.project.json` > `forge.yaml` > directory name
- **Chosen**: Try cwd `.project.json` (`id` field) first — present in 70+ siblings and written by `forge new`/`init_project.py`. Fall back to `forge.yaml:project.id` (Forge-native). Fall back to directory basename (existing `--folder` derivation) only if neither file exists.
- **Rationale**: Matches what the sibling actually has today (most siblings lack `forge.yaml`). Keeps Forge-native projects first-class.
- **Alternatives**: Directory name only (too brittle across clones), `forge.yaml` first (breaks most siblings), reading `packages.json` (wrong source of truth).
- **Validation**: Shared `validate_project_id` (kebab/snake, `1..=128`, `^[A-Za-z0-9][A-Za-z0-9_-]*$`). All three sources must pass it; non-conforming basename fails closed with `publish-invalid`.

### D2: Bare `forge publish` is syntactic sugar over `cmd_publish_provider`
- **Chosen**: `Publish { project: Option, folder: Option, cwd: Option<PathBuf>, ... }` where `cwd` defaults to `.`. When neither `project` nor `folder` is supplied, resolve `cwd` to an absolute directory, discover `project_id` via D1, and run the existing `cmd_publish_provider` body with that `project_dir`/`project_id`. No duplicated provider logic.
- **Rationale**: Leverages existing config lookup (`FORGE_PUBLISH_PROVIDER_CONFIG` or `<cwd>/.forge/providers.yaml`), revision capture, `validate_revision`, operation-id `publish-{id}-{sha12}`, dry-run/journal/status. Fleet path unchanged.
- **Alternatives**: New `publish cwd` subcommand (unnecessary verb), registry lookup by path (requires prior `forge import`).

### D3: Revision is always the discovered directory's `HEAD`
- **Chosen**: Same `git_revision(&project_dir)` helper used by `--folder`, i.e. `git -C <dir> rev-parse HEAD`. Must pass `validate_revision` (exactly 40 hex). Non-repo or non-hex revision → `publish-invalid` before provider spawn, same message as explicit modes.
- **Alternative considered**: Allow `revision: null` for non-git checkouts (rejected — would break `compose_project_name` identity and mac runtime).

### D4: No inventory content validation at single-publish time
- **Chosen**: Bare publish does not load or validate any `forge-project-inventory/0.1.0` document. Inventory remains strictly the fleet path (`forge inventory show`, `forge publish fleet --inventory`). A missing Compose file is therefore not checked synchronously on bare publish — it surfaces as a provider-stage failure (or later as `compose_missing` in `forge inventory show` when the sibling is inventoried). Auto-increase later.
- **Rationale**: Keeps single-publish minimal and reversible; inventory classification stays the right place to report `compose_missing` without coupling bare publish to fleet code.

### D5: No new persistence
- No new registry columns, tables, or `.forge/` files. Bare publish reuses `publish` journal kind with `queue_id=None` (same as `--folder`/`--project`), additive `revision`/`build_status`/`run_status`/`container_identity`. Fleet watch (`deploy status --queue`) behavior unchanged.

## Risks / Trade-offs

- **Ambiguous cwd (monorepo or nested checkouts)** → Mitigation: resolve `cwd` argument with `canonicalize`; do not walk parents; if cwd discovery fails, error names the cwd and suggests `--folder`.
- **Directory-name aliasing** → Mitigation: never synthesize identity from directory when `.project.json` is malformed — malformed `.project.json` is a typed `publish-invalid` naming the file, not a silent fallback.
- **Provider config per-checkout drift** → Mitigation: document `FORGE_PUBLISH_PROVIDER`/`FORGE_PUBLISH_PROVIDER_CONFIG` and per-checkout `.forge/providers.yaml`; no global registry invented.
- **Previously-failing bare invocation now succeeds** → Mitigation: caller that relied on `forge publish` failing without flags must now pass explicit flags; document in `publish-commands` delta and CHANGELOG note; no silent behavior change when flags are present.

## Migration Plan

1. Ship bare mode additive; keep `forge publish --help` advertising bare invocation.
2. Update sibling-facing docs to prefer `forge publish` from checkout; keep `--folder` example as override.
3. No data migration; existing registries load unchanged (`serde(default)` paths).
4. Rollback: reverting removes bare discovery; `forge publish` without flags resumes prior `publish-invalid` error — fleet and explicit modes unaffected.

## Open Questions

- None blocking. Later scaffolding change can decide the `compose_missing` remediation contract (template + `.project.json:deployment` update).

# Design: fleet-liveness-status

## Context

`forge deploy status` answers "what did Forge publish" from the
`operations` journal; nothing answers "is the service online". The
`decoupled-remote-publish` rollout showed the gap is load-bearing:
journal-healthy projects served fallback 404s (no Caddy rule),
Cloudflare 502s (rule without upstream), and crash loops (missing
target secrets) — all invisible to the journal. This design makes the
join explicit, read-only, and typed.

## Goals / Non-Goals

**Goals:**
- `forge fleet online` prints one verdict per roster project that an
  operator can trust without a follow-up `ssh`/`curl`.
- Ground truth only: served Caddyfile for routes, target `docker ps`
  for containers, real HTTPS for reachability. No local projections.
- Read-only end to end: no journal rows, no registry writes, no
  target writes, no secret reads.

**Non-Goals:**
- Recovery, redeploy, alerting, history, MCP/API projection (see
  proposal non-goals; `portal-web-ui` consumes the report).

## Decisions

### D1 — New `fleet online` subcommand reusing fleet roster flags

**Chosen:** `forge fleet online [--inventory <path>] [--fleet-registry <path>] [--workspace-root <path>] [--lifecycle <name>] [--timeout-secs N] [--format human|json]`, mirroring `publish fleet` roster resolution exactly (explicit inventory → legacy registry compatibility adapter → same `compose_ready` filter; skipped entries reported with their classification, never probed).

**Alternatives:** `--live` flag on `deploy status` (rejected: mixes journal reads with network probes and complicates the `--watch` contract); top-level `forge online` (rejected: liveness is a fleet property, sits with `fleet list|status|inspect`).

### D2 — New `src/fleet/online.rs` module, Core-owned, transport-injected

**Chosen:** Rust, same crate/toolchain (workspace Rust per `Cargo.toml`, `cargo test`/`cargo clippy` conventions). `src/fleet/online.rs` owns the join: roster in → probes out → `LivenessReport`. SSH/`docker` go through the existing `publish::SubprocessTransport` + `CommandSpec` vocabulary (bounded, redacted); HTTP goes through a small bounded `curl -s -m <timeout> --max-filesize` runner in the same module. No new dependencies (`cargo deny` closure unchanged).

**Alternatives:** `ureq` dependency (rejected: new dep + policy review for a single GET); reusing `run_subprocess` directly (rejected: it is private to `publish`; the module exposes a `pub(crate)` bounded runner or the transport gains a probe method — implementation detail, decided at build).

### D3 — Served Caddyfile is the route ground truth

**Chosen:** Parse `^http://<host> \{$` rules from the served file
(`RUNTIME`… no — `PLATFORM_ROOT/Caddyfile` via `ssh cat`). The nav
host (`apps.<domain>`) and the `:80` fallback are excluded from
probing. A project with no rule is `NO-ROUTE`, whatever the adapter
subdomain projection claims.

**Alternatives:** Reusing `caddy::choose_route` on the registry
(rejected: projection, not ground truth; diverges exactly when it
matters — stale reloads).

### D4 — Verdict state machine

**Chosen:**

| Container | Route | HTTP | Verdict |
|---|---|---|---|
| any | absent | skipped | `NO-ROUTE` |
| absent | present | skipped | `NOT-DEPLOYED` |
| present | present | 000/timeout/502/503/504 | `DOWN` |
| present | present | fallback body | `DOWN` (router rule stale or app unmounted — still not serving) |
| present | present | any other 2xx/3xx/4xx | `ONLINE` |

Fallback-body detection: bounded body read (64 KiB cap), match
`Unknown application hostname`, redacted. Empty-body 404 from the
app itself reads `ONLINE` (the app answered; path semantics belong
to the project, e.g. API-only services with no `/`).

**Alternatives:** Requiring 2xx (rejected: API-only services like
`alethefy` legitimately 404 `/` while healthy).

### D5 — Contract and compatibility

**Chosen:** `forge-fleet-liveness/0.1.0` JSON: `{contract, generated_at, inventory_source, target, domain, entries: [{id, classification, container, route, http_status, verdict, detail}], summary: {online, down, no_route, not_deployed}}`. Exit 0 iff every probed host is `ONLINE` (skipped/inventory-invalid entries follow the existing fleet convention: reported, never silent; an empty `compose_ready` set is a typed `publish-invalid` like `publish fleet`). Human table mirrors the JSON fields. No registry/journal schema change.

### D6 — Failure and boundary policy

| Case | Behavior |
|---|---|
| `curl`/`ssh` binary absent | Entry (or whole run) `unavailable` detail naming the binary; exit non-zero; never online |
| Probe timeout (`--timeout-secs` 1..=120, default 12) | `DOWN` with `timeout after Ns` detail |
| Unparseable served Caddyfile | Run aborts `publish-invalid` naming the path (routes are unknowable) |
| Secret-shaped bytes in any capture | `policy::redact_credentials` before report/journal/human output |
| Inventory invalid entry | Reported with classification; not probed |
| `--watch` | Not in scope (fleet publishes are minutes-long; liveness is a point sample; `--watch` stays on `deploy status`) |

### D7 — Verification oracle

- Unit (`src/fleet/online.rs`): verdict classifier matrix (all table
  rows), Caddyfile host parser (nav/fallback exclusion, escaping),
  redaction of secret-shaped probe output, timeout shaping.
- CLI contract (`tests/fleet_online_contract.rs`): fixture roster →
  stubbed transport/probes assert JSON contract fields, human table
  columns, exit codes (all-online 0, any-down 1, empty roster
  `publish-invalid`); skipped entries never probed.
- Live: real `forge fleet online` against the Mac post-fleet with
  per-host verdicts recorded (evidence, not a pass claim for this
  repository's own hosting).

### D8 — Risks / trade-offs

- **Probes look like monitoring** → Mitigation: point-sample only,
  no history/retention/scheduling; documented non-goal.
- **HTTPS PKI in dev** → Mitigation: default full verification;
  no `--insecure` in v1 (tunnel serves valid certs); self-signed
  origins report `DOWN` with the TLS error as detail, honestly.
- **Concurrent fleet mutating routes mid-probe** → Mitigation:
  report carries `generated_at`; mixed-generation reads are
  possible and accepted (same as `deploy status` polling).

## Migration Plan

Additive only: new subcommand, new module, new contract, new
tests. No migration (no persistence), no flag changes, no sibling
impact.

## Open Questions

None — all material decisions resolved above. (`portal-web-ui`
consumes the report; its tracking column is that package's scope.)

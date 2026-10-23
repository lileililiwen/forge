# Change: publish-reachability-truth

## Why

`forge publish` runs the publish stages on the target and then states
what the run achieved. The canonical capability says what a deploy
publishes:

> #### Scenario: Caddy renders after fleet
>
> - **WHEN** `forge publish fleet` finishes with 20/20 healthy
> - **THEN** `PLATFORM_ROOT/Caddyfile` contains one host rule per
>   published project at `<project>.tooosall.uk` and the `apps` nav
>   host

and how the public address reaches the target:

> route public HTTP hosts through the existing wildcard Cloudflare →
> Caddy boundary

So the address is real: a Cloudflare Tunnel fronts the Caddy router on
the Mac, and every published project gets its own host rule there. The
deploy stage ships the router documents and reloads the router, so the
run establishes that the routing for `<project>.<domain>` is in place.

The summary then claims more than the run established:

```rust
// src/publish/pipeline.rs, before
let note = if request.dry_run {
    format!("dry-run: {} stage(s) planned, no side effects", stages.len())
} else if healthy {
    "publish succeeded; project reachable at subdomain".to_string()
} else {
    "publish did not complete; see stages for failure detail".to_string()
};
```

**Nothing in the run observed a reachability.** The deploy stage runs
`docker compose up -d --build` for the project, ships the router
documents, and reloads the router. No stage probes the Cloudflare
tunnel, resolves the wildcard DNS, or sends a request to the host. The
summary said "reachable" anyway.

And for three of the four actions the claim names an address the
report does not even carry. The subdomain is set only for a deploy:

```rust
// src/publish/pipeline.rs, before
if matches!(action, PublishAction::Deploy) && is_ok {
    subdomain = adapter.subdomain(&request.project_id);
}
```

so a healthy `forge publish sync` printed a summary claiming the
project is "reachable at subdomain" while the report carried no
subdomain and the human renderer printed no `subdomain:` line at all.

**One string, read by every surface.** The note is the summary the
human renderer prints, the `note` the JSON document carries, and the
`reason` the CLI puts on a publish failure — in `src/cli/publish.rs`,
`src/cli/release.rs`, `src/cli/gitops.rs` and `src/cli/fleet_exec.rs`.
One claim, many surfaces, and the claim was not earned.

## What Changes

- **The summary states what the run established.** A healthy publish
  names the address the deploy published, and only when the report
  carries one. It does not claim a reachability the run never
  observed.
- **A publish that published no address says so.** An action that
  publishes no subdomain — every action but `Deploy` — says that it
  published none, instead of naming an address it does not have.
- **The claim is pinned.** A unit test drives the publish orchestrator
  with the stub adapter and the recording transport, for both a deploy
  and a non-deploy action.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The publish summary | `forge` | Yes | `src/publish/pipeline.rs`: one note, built once, read by every surface. |
| The publish report | `forge` | Read only | `src/publish/model.rs`: `PublishReport` keeps its fields, including `subdomain`. |
| The human renderer | `forge` | Read only | It prints `summary: {note}`; the note changes. |
| The JSON document | `forge` | Read only | It carries `note`; the note changes. |
| The CLI error reasons | `forge` | Read only | They copy `report.note`; the note changes. |
| The deploy stage | `forge` | Read only | `src/publish/remote_compose/`: the commands, the router documents and the reload are unchanged. |
| The Caddy rendering | `forge` | Read only | `src/publish/remote_compose/caddy.rs` is unchanged. |
| The journal | `forge` | Read only | The journal detail carries the per-stage note, never the summary. |
| The publish summary test | `forge` | Yes | Unit tests in `src/publish/pipeline.rs`. |
| The `publish-reachability-truth` capability | `forge` | Added | this package |
| The `decoupled-remote-publish` capability | `forge` | Modified | The Caddy requirement names the rule. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §27 (distribution) owns the publish lane; the `decoupled-remote-publish` capability owns the deploy and the host rules. |
| Contracts | No field is added, removed or renamed. The `note` string changes. The contract version is unchanged. |
| Callers | `forge publish` (human and JSON), its exit reasons, the release lane's publish stage and the gitops lane. |
| Persistence | The journal rows are unchanged. |
| Integrations | None. No probe is added. |
| Failures | No new error code. The failure summary is unchanged. |
| Tests | Unit tests in `src/publish/pipeline.rs`. |
| Dependencies | None added. |
| Concerns | Quality: the summary now says what the run did. Compatibility: a reader matching the phrase "reachable at subdomain" must read the new wording; the address itself is still in the report's `subdomain` field and in the renderer's `subdomain:` line. Security: no new surface; no secret is involved. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `publish-reachability-truth` | forge | Rust | added | this package |
| `decoupled-remote-publish` | forge | Rust | modified | the Caddy requirement names the rule. |

## Non-Goals

- **No probe is added.** Forge does not ask the Cloudflare tunnel, the
  DNS or the host whether the address answers. A probe would add a
  network dependency to the deploy lane and would turn a tunnel or DNS
  outage into a publish failure — a worse outcome than a summary that
  does not overclaim.
- **The subdomain is unchanged.** The address is still derived from
  the configured domain and the project id, still carried by the
  report, and still printed by the renderer.
- **The deploy stage is unchanged.** The commands, the router
  documents and the router reload are untouched.
- **The failure and dry-run summaries are unchanged.**
- **The journal is unchanged.**

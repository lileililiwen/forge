## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The publish summary | `forge` | Yes. `src/publish/pipeline.rs`. |
| The publish summary test | `forge` | Yes. Unit tests in `src/publish/pipeline.rs`. |
| The `publish-reachability-truth` capability | `forge` | Added. |
| The `decoupled-remote-publish` capability | `forge` | Modified. |
| The publish report | `forge` | Read only. |
| The human renderer | `forge` | Read only. |
| The JSON document | `forge` | Read only. |
| The CLI error reasons | `forge` | Read only. |
| The deploy stage and the Caddy rendering | `forge` | Read only. |
| The journal | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/publish/pipeline.rs, before
} else if healthy {
    "publish succeeded; project reachable at subdomain".to_string()
} else {
```

"reachable" is a claim about a live HTTP endpoint. The run observed:
the project's containers started on the target, the router documents
were shipped, and the router was reloaded. It did not observe the
Cloudflare tunnel, the wildcard DNS, or a request to the host.

### The address the report does not carry

```rust
// src/publish/pipeline.rs, before
if matches!(action, PublishAction::Deploy) && is_ok {
    subdomain = adapter.subdomain(&request.project_id);
}
```

`Sync`, `Db` and `Prepare` publish no address, so their report carries
no subdomain, and the human renderer prints no `subdomain:` line. The
summary still named "subdomain".

### One string, many surfaces

The note is read by:

| Surface | Where |
|---|---|
| The human renderer's `summary:` line | `src/publish/pipeline.rs::render_report_human` |
| The JSON document's `note` | `src/publish/model.rs::PublishReport` |
| The CLI's publish failure reason | `src/cli/publish.rs` (three call sites) |
| The release lane's publish reason | `src/cli/release.rs` |
| The gitops lane's publish reason | `src/cli/gitops.rs` |
| The fleet lane's publish reason | `src/cli/fleet_exec.rs` |

## The Rule

One rule, in the one place the note is built:

> The publish summary states what the run established. A healthy
> publish names the address the deploy published, and only when the
> report carries one; it does not claim a reachability the run did not
> observe.

Two notes, chosen by whether the report carries an address:

| Report | Note |
|---|---|
| healthy, with a subdomain | `publish succeeded; the project is published at https://<subdomain>` |
| healthy, without a subdomain | `publish succeeded; no subdomain was published for this action` |
| dry run | unchanged: `dry-run: N stage(s) planned, no side effects` |
| not healthy | unchanged: `publish did not complete; see stages for failure detail` |

**"published at" is what the run established.** The deploy stage
shipped the router documents that carry the host rule for
`<project>.<domain>` and reloaded the router, so the routing for that
address is in place. "reachable" said more: that the Cloudflare
tunnel, the wildcard DNS and the host itself answer right now. Nothing
observed that.

**The address is still the same address.** The subdomain is derived
from the configured domain and the project id, exactly as before. The
summary now names it, and the report still carries it in `subdomain`
and the renderer still prints the `subdomain:` line.

**No probe.** Forge does not ask the tunnel, the DNS or the host
whether the address answers. A probe would add a network dependency
to the deploy lane and would turn a tunnel or DNS outage into a
publish failure — a worse outcome than a summary that does not
overclaim.

## Failure Boundaries

| Boundary | Behaviour |
|---|---|
| A healthy deploy | The note names the published address; the report carries the subdomain; the renderer prints the `subdomain:` line. |
| A healthy sync, db or prepare | The note says no subdomain was published; the report carries no subdomain; the renderer prints no `subdomain:` line. |
| A healthy `all` run | The stages include the deploy, so the report carries the subdomain and the note names the address. |
| A failed stage | The failure summary is unchanged; the report's `healthy` is false. |
| A dry run | The dry-run summary is unchanged; it claims nothing. |
| An adapter that returns no subdomain for a deploy | The note takes the no-address form; the report carries no subdomain. |

## Risks

- **The wording changes.** A reader matching the phrase "reachable at
  subdomain" must read the new wording. The address is still in the
  report's `subdomain` field and in the renderer's `subdomain:` line.
- **The summary is shorter and less reassuring by design.** It says
  what the run did, not what the world looks like afterwards.
- **The deploy stage is unchanged.** Nothing about how a project is
  published moves.
- **The contract version is unchanged.** No field is added, removed or
  renamed.

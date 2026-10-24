# Change: portal-policy-verdict-truth

## Why

The control-plane portal reports the **Policies** section as `ok` for
every registered project, with no evidence, although no policy was ever
evaluated.

Reproduced on the current tree, with a project that has no DriftWatch
configuration at all:

```
$ forge register /tmp/portal-probe/proj
$ forge portal dashboard portal-probe
...
[Policies] section=`policies` status=`ok` source=`registry` generated_at 2026-10-24T13:00:53Z
  spa: /workbench
  web: partial — reads in the workbench; policy writes stay in the terminal
  project_id: portal-probe
  entries: 1
  - id=`portal-probe:policies` status=`ok` label=`portal-probe driftwatch policies` source=`policy` observed_at 2026-10-24T13:00:15Z
```

`status='ok'`, and **no evidence line at all**. The project has no
policy file, and no policy was ever evaluated.

**Every sibling section is honest about having nothing.** The same
dashboard says `receipts=0`, `proposals=0`, `sessions=0`, `states=0`
and `Gate has not run for this project; unverified is not a pass.` —
each with `status=unknown`. The Policies section is the only one that
reports a pass it never observed.

**The column it reads is never written.** The section's status comes
from the registry's `quality_status` column:

```rust
// src/portal/sections.rs, before
let status = match r.quality_status.as_deref() {
    Some("ok") | Some("healthy") | None if r.runtime.is_some() => PortalStatus::Ok,
    Some("warn") => PortalStatus::Warn,
    Some("fail") => PortalStatus::Fail,
    _ => PortalStatus::Unknown,
};
```

`quality_status` is inserted as `NULL` by the registry
(`src/registry/model.rs:644`) and no statement in the product ever
updates it. So `None` is not an edge case — it is the only case a real
project reaches, and it is wired to `Ok`.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. The canonical
`control-plane-portal` capability already requires the honest
behaviour — *"WHEN a status observation is stale or unknown THEN the
view labels that state and retains its observation timestamp"* — and the
section breaks it.

## What Changes

- **One rule judges a project's policies in the portal.** The status
  comes from a verdict the portal actually observed. No observed
  verdict is `unknown`, never `ok`.
- **The section states what it observed.** The entry names the policy
  artifacts the portal found in the project directory and, when no
  verdict was observed, names the command that produces one.
- **The artifact list is shared with the doctor.** The portal and the
  doctor agree on what a DriftWatch policy artifact is, so there is one
  list and not two.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The portal's policy rule | `forge` | Yes | `src/portal/sections.rs`: one rule over what the portal observed. |
| The doctor's artifact detector | `forge` | Yes | `src/doctor/probes.rs`: `detect_driftwatch` becomes `pub(crate)` so the portal reads the same list. |
| The portal's Policies section | `forge` | Yes | The status follows the rule; the entry carries evidence. |
| The portal's contract test | `forge` | Added | `tests/portal_policy_verdict_truth_contract.rs`. |
| The `portal-policy-verdict-truth` capability | `forge` | Added | this package |
| The `control-plane-portal` capability | `forge` | Modified | the views requirement states the general rule. |
| The other eleven portal sections | `forge` | Read only | They already say `unknown` with evidence. |
| The portal's projects section | `forge` | Read only | Its `ok` stays a liveness signal: the project directory is available and a runtime is recorded. |
| The portal contract version | `forge` | Read only | Unchanged. |
| The doctor's policy findings | `forge` | Read only | The doctor's own verdicts are unchanged; only the detector's visibility changes. |
| The registry schema | `forge` | Read only | `quality_status` keeps its column and its meaning; nothing is written to it. |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §36 owns the twelve control-plane sections; the `control-plane-portal` capability owns the portal's status vocabulary. |
| Contracts | No field is removed or renamed. `PORTAL_CONTRACT_VERSION` is unchanged, the entry's `id`, `label`, `status`, `source` and `observed_at` are unchanged, and the entry gains the `evidence` lines it never had. |
| Callers | `forge portal dashboard`, `forge portal view policies`, and the SPA's read of the same document. |
| Persistence | Nothing is written. The portal stays a read-only view; the registry schema is unchanged. |
| Integrations | None. No adapter, provider or repository is touched, and the portal never runs the policy adapter. |
| Failures | No new error code. A project whose directory cannot be read reports `unavailable`, a project with no observed verdict reports `unknown`, and the dashboard keeps its worst-status rollup. |
| Tests | A new contract test `tests/portal_policy_verdict_truth_contract.rs`; the existing portal tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the Policies section can no longer be read as "the policy plane is healthy" when no policy was evaluated, and it now says what it looked at. Compatibility: a reader of the Policies section now sees `unknown` instead of `ok` for a project with no recorded verdict, and the entry gains evidence lines. Security: no new surface; the evidence names configuration file names inside the project and never a host path. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `portal-policy-verdict-truth` | forge | Rust | added | this package |
| `control-plane-portal` | forge | Rust | modified | the views requirement states the general rule. |

## Non-goals

- **The doctor is unchanged.** Its policy findings, its verdicts and its
  staleness rule are exactly as they are; only the artifact detector
  becomes visible to the portal.
- **The portal never runs the policy adapter.** The portal is a
  read-only view; it observes the project directory and the registry,
  and nothing else.
- **The registry schema is unchanged.** `quality_status` keeps its
  column. Nothing starts writing it in this change.
- **The other eleven portal sections are unchanged.**
- **The portal's projects section is unchanged.** Its `ok` stays a
  liveness signal.
- **The portal contract version is unchanged.**

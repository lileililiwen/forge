# Design: portal-policy-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The portal's policy rule | `forge` | Yes. `src/portal/sections.rs`. |
| The doctor's artifact detector | `forge` | Yes. `src/doctor/probes.rs`. |
| The portal's Policies section | `forge` | Yes. `src/portal/sections.rs`. |
| The portal's contract test | `forge` | Added. `tests/portal_policy_verdict_truth_contract.rs`. |
| The `portal-policy-verdict-truth` capability | `forge` | Added. |
| The `control-plane-portal` capability | `forge` | Modified. |
| The other eleven portal sections | `forge` | Read only. |
| The portal's projects section | `forge` | Read only. |
| The portal contract version | `forge` | Read only. |
| The doctor's policy findings | `forge` | Read only. |
| The registry schema | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/portal/sections.rs, before
pub(super) fn build_policies_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let status = match r.quality_status.as_deref() {
                Some("ok") | Some("healthy") | None if r.runtime.is_some() => PortalStatus::Ok,
                Some("warn") => PortalStatus::Warn,
                Some("fail") => PortalStatus::Fail,
                _ => PortalStatus::Unknown,
            };
            PortalEntry {
                id: format!("{r_id}:policies", r_id = r.id),
                label: format!("{} driftwatch policies", r.name),
                status,
                source: "policy".to_string(),
                observed_at: r.observed_at.clone(),
                evidence: Vec::new(),
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}
```

### What it produced

```
[Policies] section=`policies` status=`ok` source=`registry` ...
  - id=`portal-probe:policies` status=`ok` label=`portal-probe driftwatch policies` source=`policy` observed_at ...
```

`ok`, no evidence, a project with no policy file.

### Why the `None` arm is the only arm a real project reaches

`quality_status` is a column nothing writes:

- `src/registry/model.rs:644` inserts `NULL` for `quality_status`,
  `agent_status` and `docs_status`;
- the `ON CONFLICT DO UPDATE` clause (`src/registry/model.rs:645-657`)
  does not touch them;
- no statement anywhere in the product updates them — the only reads are
  `src/registry/store.rs:237`, `src/portal/sections.rs` and
  `src/cli/feature.rs:787`.

So the `None` arm is not an edge case. It is every project. And it is
wired to `Ok` whenever a runtime is recorded.

### The sibling sections

Every other section derives its status from something it looked at, and
says so:

| Section | Its status comes from | Its evidence |
|---|---|---|
| components | the receipts it counted | `receipts=0` |
| specs | the proposals it counted | `proposals=0` |
| agents | the sessions it counted | `sessions=0` |
| deployments | the states it counted | `states=0` |
| analytics | the summaries it counted | `summaries=0` |
| servers | the sessions it counted | `sessions=0` |
| documentation | the recorded documentation verdict | (none) |
| settings (Quality) | the Gate evidence and its freshness | `Gate has not run for this project; unverified is not a pass.` |
| policies | a column nothing writes | (none) |

The Policies section is the only one whose evidence is empty and whose
status is a pass.

### The canonical rule it breaks

`control-plane-portal`, requirement *Shared project and lifecycle views*:

> #### Scenario: Shared project and lifecycle views boundary
> - **WHEN** a status observation is stale or unknown
> - **THEN** the view labels that state and retains its observation
>   timestamp

A policy verdict that was never produced is an unknown observation. The
section labelled it `ok`.

## The Rule

One rule judges a project's policies in the portal, in the one place the
portal's sections live:

```rust
/// The one rule that judges a project's policies in the portal.
///
/// A policy verdict is a judgement Forge has to observe. The portal
/// reads the project directory and the registry and never runs the
/// policy adapter, so it states a verdict only when one was recorded.
/// The absence of a verdict is `unknown`, never a pass: an unevaluated
/// policy is not a healthy policy.
fn policy_verdict(
    available: bool,
    recorded: Option<&str>,
) -> (PortalStatus, Vec<String>) {
    if !available {
        return (
            PortalStatus::Unavailable,
            vec![
                "the project directory could not be read; the policy plane was not observed"
                    .to_string(),
            ],
        );
    }
    match recorded {
        Some("ok") | Some("healthy") => (
            PortalStatus::Ok,
            vec!["policy verdict=ok".to_string()],
        ),
        Some("warn") => (
            PortalStatus::Warn,
            vec!["policy verdict=warn".to_string()],
        ),
        Some("fail") => (
            PortalStatus::Fail,
            vec!["policy verdict=fail".to_string()],
        ),
        _ => (
            PortalStatus::Unknown,
            vec![
                "no policy verdict was observed; unverified is not a pass".to_string(),
                "run `forge doctor` to evaluate the policy plane".to_string(),
            ],
        ),
    }
}
```

### The writer reads it, and says what it observed

```rust
pub(super) fn build_policies_section(records: &[ProjectRecord]) -> Vec<PortalEntry> {
    records
        .iter()
        .map(|r| {
            let (status, mut evidence) = policy_verdict(r.available, r.quality_status.as_deref());
            if r.available {
                // The artifacts are the doctor's own list, so the portal
                // and the doctor agree on what a policy artifact is.
                let (_, artifacts) =
                    crate::doctor::probes::detect_driftwatch(Path::new(&r.path));
                evidence.extend(artifacts);
            }
            PortalEntry {
                id: format!("{r_id}:policies", r_id = r.id),
                label: format!("{} driftwatch policies", r.name),
                status,
                source: "policy".to_string(),
                observed_at: r.observed_at.clone(),
                evidence,
                attributes: BTreeMap::new(),
            }
        })
        .collect()
}
```

`detect_driftwatch` returns `(false, vec!["no driftwatch configuration"])`
when the project has no policy, and the names of the files it found
otherwise. The names are configuration file names inside the project; no
host path reaches the evidence, the same way the Quality section's
evidence never names the project directory.

The artifacts are only looked for when the directory was readable: an
unreadable directory is `unavailable`, and claiming "no driftwatch
configuration" about a directory nobody read would be a second false
claim.

### Why the artifact list is shared with the doctor

The doctor already answers "does this project have a DriftWatch
configuration?" with one list:

```rust
// src/doctor/probes.rs
pub(super) fn detect_driftwatch(dir: &Path) -> (bool, Vec<String>) {
    // driftwatch.yaml, driftwatch.yml, driftwatch.json, .driftwatch.yaml,
    // .driftwatch.yml, driftwatch.toml, gate.toml, .ai-gate/gate.yaml,
    // .driftwatch/
}
```

The portal would otherwise have to keep a second list of the same
names, and the two would drift. The change makes the detector
`pub(crate)` and lets the portal read it. The doctor's own behaviour is
unchanged: it still calls the same function with the same arguments.

### The options, decided

| Option for the Policies section's status | Verdict |
|---|---|
| Keep `Ok` when `quality_status` is absent | Rejected. `quality_status` is never written by anything, so this arm is every project, and it turns "never evaluated" into "healthy". |
| `Unknown` when no policy verdict was observed, with the observed artifacts and the command that produces a verdict as evidence | **Chosen.** The absence of an observation is `unknown`, never a pass — the rule the Quality section already follows ("unverified is not a pass"), and the rule the canonical boundary scenario requires. |
| Run the policy adapter from the portal | Rejected. The portal is a read-only view over the same Core contracts; spawning the DriftWatch binary from a dashboard read is a side effect the read-only plane must not have. |
| Use the Gate evidence's aggregate as the policy verdict | Rejected. The Gate is the project gate, not the policy plane; naming it a policy verdict would be a different false claim. |
| Use the doctor's own policy finding | Rejected. The doctor judges a report it obtained by running the adapter, and it does not persist one, so there is no stored finding for the portal to read. |
| Delete the Policies section | Rejected. §36 names twelve sections; removing one breaks the section contract and the SPA deep link. |
| Start writing `quality_status` from the doctor | Rejected here. It is the right long-term shape, but it adds a registry write to a read-only assessment and a schema migration; it is a separate change with its own contract. This change keeps the column as the verdict source and makes its absence honest. |

### Why the projects section is not in this change

`build_projects_section` also reads `quality_status`, and its `else`
arm is also `Ok`. That `ok` is a **liveness** signal — the project
directory is available and a runtime is recorded — and the programme
already decided that it stays one (the `forge list` registry "Health"
column is the same signal). The Policies section is different: its
status is a *quality verdict* about the policy plane, and it produced
one that was never observed.

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A project with a recorded policy verdict | the section reports that verdict: `ok`, `warn` or `fail`, with `policy verdict=<value>` in the evidence. |
| A project with no recorded policy verdict | the section reports `unknown`, with the observed artifacts and the note that unverified is not a pass. |
| A project whose directory cannot be read | the section reports `unavailable`, with the note that the policy plane was not observed. |
| A project with no DriftWatch configuration | the section reports `unknown`, and the evidence says `no driftwatch configuration`. |
| The doctor's own findings | unchanged: the doctor still runs the adapter and judges the report with `PolicyVerdict`. |
| The portal's other eleven sections | unchanged. |
| The portal's projects section | unchanged: its `ok` stays a liveness signal. |
| The dashboard's rollup | unchanged: it still reports the worst section, so a project whose policy plane was never evaluated rolls up as `unknown` rather than `ok`. |
| The registry | unchanged: no write, no schema change, no migration. |
| The portal contract version | unchanged. |
| The entry's evidence | the section now carries evidence lines; the count stays within the portal's bound of 16 lines per entry (at most 9 artifact names plus 2 notes). |

## Risks

| Risk | Mitigation |
|---|---|
| The Policies section reads `unknown` for every project today. | That is the truth: no policy verdict has been recorded for any project, because nothing writes the column. The section now says so and names the command that produces one. |
| A reader counts `ok` sections to judge a workspace. | That count was wrong before this change: it counted unevaluated policies as healthy. |
| The dashboard's rollup changes from `ok` to `unknown` for a project whose only `ok` section was Policies. | The rollup reports the worst section, which is the documented behaviour; a workspace with no observed policy verdict is not `ok`. |
| The doctor's behaviour changes because its detector became `pub(crate)`. | The detector's body, its arguments and its callers are unchanged; only its visibility widened. |
| The evidence names the project's configuration files. | The names are relative file names inside the project (`driftwatch.yaml`), never a host path, and the Quality section already publishes file-level evidence the same way. |
| The portal contract changes because the entry gained evidence. | The `evidence` field already exists in the contract and is `skip_serializing_if` empty; the other fields are unchanged and the contract version is unchanged. |

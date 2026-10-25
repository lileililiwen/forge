# Change: resolve-plan-verdict-truth

## Why

Two resolver lanes journal a pure plan as a completed action.

`forge component resolve` and `forge ui-pattern resolve` do not install
anything. They read a catalog, pick the strongest compatible candidates
for a profile and return a **plan** whose steps say `action: "install"`.
Nothing is installed, no file is written, no source changes. The journal
row both lanes write says the resolution is `done`.

Reproduced on the current tree:

```
$ forge --registry /tmp/res-probe/registry.db component resolve \
    --profile rust-web --component paginated-query
...
  "note": "rust-web resolved=1 rejected=0; resolved 1 component(s) for profile 'rust-web'",
  "plan": { "steps": [ { "action": "install", "id": "paginated-query", ... } ] }
```

and the journal row it wrote:

```
(1, 'component', '__component__', 'done', ..., "rust-web resolved=1 rejected=0; resolved 1 component(s) for profile 'rust-web'")
```

```
$ forge --registry /tmp/uip-probe/registry.db ui-pattern resolve \
    --profile react-web --pattern billing
(1, 'ui_pattern', '__ui_pattern__', 'done', ..., "react-web resolved=1 rejected=0; resolved 1 ui pattern(s) for profile 'react-web'")
```

`done` is the state the registry uses for a completed action. A reader
of the journal — the portal's activity view, an operator asking what
Forge did to a project — reads these rows as two completed resolutions.
Nothing was resolved into the project; two plans were printed.

The state was chosen by a private copy of the rule in each lane:

```rust
// src/cli/component.rs, before
let journal_state = if outcome.plan.steps.is_empty() {
    "rejected"
} else {
    "done"
};
```

```rust
// src/cli/ui_pattern.rs, before
let journal_state = if outcome.plan.steps.is_empty() {
    "rejected"
} else {
    "done"
};
```

Two copies, two lanes, one wrong word. The canonical capabilities
already require the honest behaviour for the lanes that own them:
`semantic-component-registry` — *"WHEN certification lacks required
verification or security evidence THEN promotion fails and the prior
quality level remains"* — and `semantic-ui-patterns` — *"Forge SHALL
install pinned compatible pattern implementations"*. A resolution is
not an installation, and the journal said it was.

This is the rule the repair programme is built on: Forge states
something its own gathered evidence does not support. The release lane
was repaired this way in change 37 and the deploy lane in change 38:
a plan is a plan, and the journal names it as one.

## What Changes

- **One rule names what a resolution did.** A resolution that produced
  a plan is a **planned** run; a request the resolver could not satisfy
  is a **rejected** run. The rule lives in the shared Core, so both
  lanes read it and the two copies cannot drift.
- **A resolution is never journalled as `done`.** The journal state
  comes from the rule, not from a lane-local word.
- **The row names the run.** The journal detail carries
  `verdict=planned` or `verdict=rejected`, so a reader never has to
  guess what the state means.
- **The output carries the verdict too.** The command's own note — in
  the human output and in the JSON document — names the run.

## BFS Impact Map

| Surface | Owner | In this package | Notes |
|---|---|---|---|
| The resolution rule | `forge` | Yes | `src/core/resolve.rs`: one shared rule, `ResolveRun`, that names the run a resolution describes. |
| The shared Core module | `forge` | Yes | `src/core/mod.rs`: the module publishes the rule. |
| The component resolver lane | `forge` | Yes | `src/cli/component.rs`: the lane reads the one rule. |
| The UI pattern resolver lane | `forge` | Yes | `src/cli/ui_pattern.rs`: the lane reads the same rule. |
| The resolution contract test | `forge` | Added | `tests/resolve_plan_verdict_truth_contract.rs`. |
| The `resolve-plan-verdict-truth` capability | `forge` | Added | this package |
| The `semantic-component-registry` capability | `forge` | Modified | the unit contract requirement states the general rule. |
| The `semantic-ui-patterns` capability | `forge` | Modified | the installation requirement states the general rule. |
| The component and UI pattern catalogs | `forge` | Read only | The catalogs and their evidence are unchanged. |
| The resolvers themselves | `forge` | Read only | The plan, the steps, the rejections and the evidence summary are unchanged. |
| The component install path | `forge` | Read only | `forge component` has no install command; the plan names an action, the plan does not perform it. |
| The UI pattern install path | `forge` | Read only | `forge ui-pattern install` still installs and still journals `done`; it is a different command. |
| The qualify lanes | `forge` | Read only | Unchanged by this change (change 41 owns them). |
| The registry journal | `forge` | Read only | The row's `state` column now carries `planned` or `rejected`; the schema is unchanged. |
| The portal's activity view | `forge` | Read only | It prints the journal state verbatim; `planned` reads as a plan. |
| The release and deploy lanes | `forge` | Read only | They read their own lane's verdict type (changes 37 and 38). |

| Concern | Result |
|---|---|
| Requirements | `requirement.md` §11 and §12 own the component registry and its quality levels; the semantic UI pattern catalog and its installation are owned by `semantic-ui-patterns`. |
| Contracts | No field is removed or renamed. The journal row's `state` column changes its value for two kinds; the plan, the note and the JSON document keep their fields, and the note gains the `verdict=` token. |
| Callers | `forge component resolve` and `forge ui-pattern resolve`. The resolver library callers are unchanged. |
| Persistence | The journal rows for the `component` and `ui_pattern` kinds now read `planned` or `rejected` instead of `done` or `rejected`. |
| Integrations | None. No adapter, provider or repository is touched. |
| Failures | No new error code. A request the resolver cannot satisfy is still journalled as `rejected` with its reasons. |
| Tests | A new contract test `tests/resolve_plan_verdict_truth_contract.rs`; the existing component and UI pattern contract tests stay green. |
| Dependencies | None added. |
| Concerns | Quality: the journal no longer claims two completed resolutions that never happened. Compatibility: a reader that counted `done` rows for these kinds now counts `planned` rows; the release and deploy lanes already use `planned` for a plan, so the vocabulary is consistent. Security: no new surface. |

## Capabilities

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `resolve-plan-verdict-truth` | forge | Rust | added | this package |
| `semantic-component-registry` | forge | Rust | modified | the unit contract requirement states the general rule. |
| `semantic-ui-patterns` | forge | Rust | modified | the installation requirement states the general rule. |

## Non-goals

- **The resolvers are unchanged.** The plan, the selected candidates,
  the rejections and the evidence summary are exactly as they are.
- **The UI pattern install path is unchanged.** `forge ui-pattern
  install` installs the pattern and still journals `done`: an install
  is a completed action.
- **The release and deploy lanes are unchanged.** They read their own
  lane's verdict type.
- **The qualify lanes are unchanged.** Change 41 owns them.
- **The registry schema is unchanged.** The `state` column keeps its
  meaning for every other kind.
- **No journal row is rewritten.** Rows written before this change
  stay as they are.

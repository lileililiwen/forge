# Design: resolve-plan-verdict-truth

## Implementation Boundary

| Surface | Owner | In this package |
|---|---|---|
| The resolution rule | `forge` | Yes. `src/core/resolve.rs`. |
| The shared Core module | `forge` | Yes. `src/core/mod.rs`. |
| The component resolver lane | `forge` | Yes. `src/cli/component.rs`. |
| The UI pattern resolver lane | `forge` | Yes. `src/cli/ui_pattern.rs`. |
| The resolution contract test | `forge` | Added. `tests/resolve_plan_verdict_truth_contract.rs`. |
| The `resolve-plan-verdict-truth` capability | `forge` | Added. |
| The `semantic-component-registry` capability | `forge` | Modified. |
| The `semantic-ui-patterns` capability | `forge` | Modified. |
| The component and UI pattern catalogs | `forge` | Read only. |
| The resolvers themselves | `forge` | Read only. |
| The UI pattern install path | `forge` | Read only. |
| The qualify lanes | `forge` | Read only. |
| The registry journal | `forge` | Read only. |
| The portal's activity view | `forge` | Read only. |
| The release and deploy lanes | `forge` | Read only. |

## The Gap

### The claim

```rust
// src/cli/component.rs, before
let outcome = resolve_outcome(&request)?;
let journal_state = if outcome.plan.steps.is_empty() {
    "rejected"
} else {
    "done"
};
```

```rust
// src/cli/ui_pattern.rs, before
let outcome = resolve_ui_pattern_outcome(&request)?;
let journal_state = if outcome.plan.steps.is_empty() {
    "rejected"
} else {
    "done"
};
```

### What it produced

```
(1, 'component', '__component__', 'done', ..., "rust-web resolved=1 rejected=0; resolved 1 component(s) for profile 'rust-web'")
(1, 'ui_pattern', '__ui_pattern__', 'done', ..., "react-web resolved=1 rejected=0; resolved 1 ui pattern(s) for profile 'react-web'")
```

### What a resolution actually does

`resolve_components` and `resolve_ui_patterns` are pure functions of
the catalog, the request and the profile:

```rust
// src/component/engine.rs
pub fn resolve_components(request: &ComponentRequest) -> Result<ComponentPlan, ForgeError> {
    validate_request(request)?;
    let catalog = component_catalog();
    ...
}
```

They read the catalog, filter the candidates by profile, pick the
strongest compatible one and return a plan. The plan's steps carry an
`action` — `install` — because the step describes what an installation
*would* do. The resolver performs no installation: it writes no file,
creates no directory and changes no source.

The component lane has no install command at all:

```rust
// src/cli/commands_ops.rs
pub(crate) enum ComponentCommands {
    List,
    Inspect { id: String },
    Resolve { profile: String, components: Vec<String> },
    Qualify { ... },
}
```

So the journal row claims a completed action that has no command.

### `done` is the state for a completed action

The registry's journal states are the vocabulary the portal's activity
view and every operator read:

| State | Meaning |
|---|---|
| `done` | The action completed. |
| `planned` | The action was planned and nothing was carried out. |
| `blocked` | A rule refused the action. |
| `failed` | The action was attempted and did not complete. |
| `rejected` | The request was refused before anything was attempted. |

The release lane was repaired in change 37 and the deploy lane in
change 38: a plan is `planned`, never `done`. The two resolver lanes
were left with the old word.

### Two copies of the rule

The two lanes each carry their own copy of the same two-arm choice.
A copy is where the two lanes drift: one of them said `done` and the
other might have said something else. The programme's rule is one
shared rule in one place, read by every surface.

## The Rule

**One rule names what a resolution did: a resolution that produced a
plan is a planned run, and a request the resolver could not satisfy is
a rejected run. A resolution is never a completed resolution.**

### The shared rule, in the shared Core

```rust
// src/core/resolve.rs
/// What a resolution run did.
///
/// A resolution produces a plan; it installs nothing. The journal
/// state names this and nothing else: a plan is `planned`, never
/// `done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveRun {
    /// A plan the resolver produced.
    Planned,
    /// A request the resolver could not satisfy.
    Rejected,
}

impl ResolveRun {
    /// The run a resolution describes. A resolution with at least one
    /// step is a planned run; a request the resolver could not
    /// satisfy is a rejected run.
    pub fn for_steps(step_count: usize) -> Self {
        if step_count == 0 {
            ResolveRun::Rejected
        } else {
            ResolveRun::Planned
        }
    }

    /// The registry journal state this run records.
    pub fn journal_state(self) -> &'static str {
        match self {
            ResolveRun::Planned => "planned",
            ResolveRun::Rejected => "rejected",
        }
    }

    /// The label the journal detail publishes, so a reader never has
    /// to guess what the state means.
    pub fn label(self) -> &'static str {
        match self {
            ResolveRun::Planned => "planned",
            ResolveRun::Rejected => "rejected",
        }
    }
}
```

The Core is where a rule shared by two domains belongs: the component
registry and the UI pattern catalog are separate domains with separate
types, but "a plan is a plan" is one rule about both. The type carries
the vocabulary (`label`) and the journal state (`journal_state`), and
the rule function (`for_steps`) carries the decision, so a lane cannot
invent a word of its own.

### The lanes read the rule

```rust
// src/cli/component.rs, after
let outcome = resolve_outcome(&request)?;
// One rule names the run: a resolution installs nothing, so it is a
// planned run and never a completed resolution.
let run = ResolveRun::for_steps(outcome.plan.steps.len());
let summary = format!(
    "{} resolved={} rejected={} verdict={}",
    request.profile,
    outcome.plan.steps.len(),
    outcome.plan.rejections.len(),
    run.label()
);
let note = format!("{}; {}", summary, outcome.note);
if let Ok(registry) = open_registry(db_path) {
    let _ = registry.record_operation("component", "__component__", run.journal_state(), &note);
}
```

The UI pattern lane reads the same rule with the same shape. The note
is shared by the journal detail, the human output and the JSON
document, so all three name the run:

```
rust-web resolved=1 rejected=0 verdict=planned; resolved 1 component(s) for profile 'rust-web'
```

### The journal row after the change

```
(1, 'component', '__component__', 'planned', ..., "rust-web resolved=1 rejected=0 verdict=planned; resolved 1 component(s) for profile 'rust-web'")
(1, 'ui_pattern', '__ui_pattern__', 'planned', ..., "react-web resolved=1 rejected=0 verdict=planned; resolved 1 ui pattern(s) for profile 'react-web'")
```

## The options, decided

| Option for a resolution that produced a plan | Verdict |
|---|---|
| Keep `done` | Rejected. A resolution installs nothing; `done` is the state for a completed action. The plan's steps name an action the resolver never performed. |
| Keep `done` and add a note saying it was only a plan | Rejected. The state is what a reader counts and filters on; a plan behind a `done` state is still a completed resolution on the record. |
| Journal `planned` and name the run in the detail | **Chosen.** The same vocabulary the release and deploy lanes use since changes 37 and 38, and the detail names the run so a reader never has to guess. |
| Give the two lanes their own verdict types | Rejected. Two copies of one rule are where the lanes drift; the two lanes already had two copies and both were wrong in the same way. |
| Put the rule in the registry | Rejected. The registry stores the journal; it does not judge what a resolution did. The rule belongs to the domain, and the two domains share the Core. |
| Put the rule in the component module and import it into the UI pattern lane | Rejected. The UI pattern catalog would depend on the component registry for a word; the two are peers, and the shared Core is where a shared rule belongs. |
| Add a verdict field to the plan structs | Rejected. The plan is the resolver's answer, and the verdict is the transport's reading of it. The release and deploy plans carry no verdict field either; the verdict lives in the journal detail and the note. |
| Rewrite the journal rows written before this change | Rejected. Nothing can know what an old row described. The rule applies to new rows. |

## Failure Boundaries

| Condition | The rule's answer |
|---|---|
| A resolution with at least one step | The run is `planned`; the row says `planned` and the detail names `verdict=planned`. |
| A resolution with no step | The run is `rejected`; the row says `rejected` and the detail names `verdict=rejected`. The rejections still carry the reasons. |
| A request for an unknown component or pattern | Unchanged: a typed rejection in the plan, and a `rejected` journal row. |
| A request for a profile the candidate does not support | Unchanged: a typed rejection, and a `rejected` journal row. |
| A request for a primitive id | Unchanged: a typed `component-invalid` error before any row is written. |
| `forge ui-pattern install` | Unchanged: an install is a completed action and still journals `done`. |
| `forge component qualify` | Unchanged: change 41 owns that lane. |
| The release and deploy lanes | Unchanged: they read their own lane's verdict type. |
| The registry schema | Unchanged: the `state` column keeps its meaning for every other kind. |
| The portal's activity view | Unchanged: it prints the state verbatim; `planned` reads as a plan. |

## Risks

| Risk | Mitigation |
|---|---|
| A reader that counted `done` rows for the `component` and `ui_pattern` kinds now counts `planned` rows. | That count was wrong: it counted plans as completed resolutions. The release and deploy lanes already use `planned` for a plan, so the vocabulary is now consistent across the journal. |
| The rule is shared by two domains that could drift apart later. | The rule is one function in the shared Core, not a word copied into two lanes. A domain that needs a different vocabulary names a different type in its own module. |
| The step count decides the verdict. | A resolution with no step satisfied nothing, so it is a rejection; the rejections carry the reason. The empty-request case is refused by the request validator before the resolver runs. |
| The Core module grows. | The rule is one small type in its own file, `src/core/resolve.rs`; `src/core/mod.rs` only publishes it. |

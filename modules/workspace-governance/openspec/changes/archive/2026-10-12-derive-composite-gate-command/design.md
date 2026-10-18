# Design: Derive a composite command for a polyglot repository

## 1. The gap

`resolve()` has two answers: bind one candidate, or refuse. The refusal is
built on a sound rule — *a wrong command is worse than a listed gap* — but the
rule has no third answer for a repository that has more than one thing to build.

`crossify` holds `backend/CourseBridge.slnx` and `frontend/package.json`. Both
answer `build`. The binder refuses because neither dominates. Binding either
one alone would be the false green this workspace keeps finding: a green
`build` on a repository whose frontend was never compiled.

The correct command is both, joined with `&&`.

## 2. Why the tool did not already do this

Refusing was the right default while the tool could only pick one. It is the
wrong default once *joining* is available, because the ambiguity it is
protecting against — asserting a shape the repository does not have — does not
arise when the command names every shape it has.

So the change is not a relaxation. It is a third option, and it is narrower
than it looks, because the question is not "are there two candidates" but
**"do these candidates cover disjoint product surfaces"**.

## 3. The test for composability

Two candidates compose when both must run for the repository to be sound. Both
must run ⇒ neither implies the other ⇒ they are not two answers to one
question.

The test is available without guessing: **the candidates come from different
stacks**. A .NET solution and a Node `package.json` are different products in
one repository. Two Flutter packages are the same product described twice, and
joining their tests would run the same suite twice.

So: composite **iff** candidates span two or more distinct stacks. Within one
stack, nothing changes — the dominance rule still decides, and still refuses
when neither candidate dominates.

The stack is already known at every generator: `dotnet_entry`, `rust_commands`,
`node_commands`, `python_candidates`, and `flutter_candidates` each know what
they are. `Candidate` gains a `stack` field, defaulting to the generator's own
name, so the composite decision is made from declared information and never
from the shape label or the command string.

`Shape.label()` returning one name for a polyglot repository stays as it is. It
is a report label, and the fix is not to make it lie less by making it return a
list — the composite decision is made per concern from the candidates, which is
where the information actually is.

## 4. Ordering, and why the output is stable

A composite is ` && `-joined. Order is the order the generators emit, which is
already deterministic — each iterates a sorted manifest list. Two runs on an
unchanged tree therefore produce byte-identical commands, so the second run
classifies the repository `ALREADY_BOUND` rather than `DIRTY`. A composite
whose order varied would report every polyglot repository as dirty on every
run, which would be a new false positive introduced by this change.

## 5. Evidence

The evidence for a composite names every candidate it joins, with that
candidate's own evidence. A reader must be able to see which surfaces the
command covers without reading the command, and `FleetTest` asserts that every
path a bound command names exists — so a composite's evidence is checked by the
same invariant as a single candidate's.

## 6. What stays refused

The other 20 refusals are unchanged and are not this change's business:

| Reason | Checks | Repos |
|---|---|---|
| no checker on disk to run | 11 | `actoria`, `citylens`, `mewo`, `trailCrew`, `platform-contracts` |
| pure-Python package, no build step | 4 | `alethefy`, `ariadex`, `jenkins-local`, `sisyphusfy` |
| several `.csproj`, no solution | 6 | `aegify`, `hestiaLab`, `smotoox` |
| no Python security tool declared | 3 | `jenkins-local`, `platform-contracts`, `workspace-governance` |
| same-stack ambiguity | 0 after this change | — |

Each of those is real work or an owner decision. The debt record keeps them
visible with their reason, which is what the record exists for.

## 7. Verification

- The 24 checks in the 11 repositories bind, and the debt record falls to 20.
- `GateOracleTest` still holds: the record stays set-equal to the gate's own
  unbound set. That test is the one that catches a composite binding a command
  the gate cannot run.
- `FleetTest`'s path invariant runs over composite commands too, so a joined
  command naming a path that does not exist turns the suite red.
- A negative control: forcing composability for same-stack candidates must turn
  tests red, proving the guard is load-bearing rather than decorative.

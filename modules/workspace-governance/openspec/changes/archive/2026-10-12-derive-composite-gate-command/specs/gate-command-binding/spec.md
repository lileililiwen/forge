# gate-command-binding Specification (delta: derive-composite-gate-command)

## ADDED Requirements

### Requirement: Disjoint product surfaces compose into one command

The binder SHALL return a single `&&`-joined command when a concern has two or
more candidates that cover **disjoint product surfaces**, and SHALL do so only
when the candidates span two or more distinct stacks. A composite command
SHALL name every candidate it joins.

#### Scenario: A .NET backend and a Node frontend both need building

- **WHEN** a repository holds one `.slnx` and one `package.json` declaring
  `scripts.build`, and both are selected for `build`
- **THEN** the derived command builds both, joined with ` && `
- **AND** the outcome is `BOUND`, not an unresolved refusal

#### Scenario: The same for tests and security

- **WHEN** a repository holds a solution and a frontend, and both answer
  `tests` or `security`
- **THEN** the derived command runs both halves

#### Scenario: Two candidates of one stack still refuse

- **WHEN** two candidates come from the **same** stack — two Flutter packages,
  or two `package.json` files both declaring `scripts.test`
- **THEN** the binder does NOT compose them
- **AND** the existing dominance rule decides, and refuses when neither
  dominates

#### Scenario: One candidate still binds alone

- **WHEN** a concern has exactly one candidate
- **THEN** the behaviour is unchanged: that candidate is bound, with no
  composite wrapper

#### Scenario: The composite names its parts

- **WHEN** a composite command is written
- **THEN** its evidence names every candidate it joins, so a reader can see
  which surfaces the command covers

#### Scenario: The order is stable

- **WHEN** the binder derives a composite command twice from an unchanged tree
- **THEN** the two commands are byte-identical, so a re-run reports `ALREADY_BOUND`
  rather than `DIRTY`

### Requirement: An ambiguous entry point is refused rather than guessed

The binder SHALL bind exactly one command per concern, or refuse. It SHALL
refuse when two candidates of equal standing remain after the dominance rule,
and SHALL name every remaining candidate and its evidence in the refusal. A
composite command satisfies this requirement: it names every surface it covers
rather than asserting one shape the repository does not have.

#### Scenario: Two candidates in different stacks

- **WHEN** two candidates remain and they cover different stacks
- **THEN** the binder joins them into one command naming both, rather than
  refusing, because building or testing one half of a polyglot repository
  reports a green result the repository does not deserve

#### Scenario: Two candidates in the same stack

- **WHEN** two candidates remain and they cover the same stack
- **THEN** the binder refuses, naming both

## MODIFIED Requirements

### Requirement: Scoped refinement

Forge SHALL validate and journal a refinement request against the
current `app_revision` and the project's registered file scope. The
`studio.refine` row SHALL record that the request was recorded and
SHALL NOT claim that the refinement was performed, because this path
runs none.

#### Scenario: Refinement accepted

- **WHEN** the request matches the expected revision, every
  `selected_files` entry resolves inside the registered project root,
  and no shell metacharacter or secret-shaped string is present
- **THEN** Forge records a `studio.refine` journal row carrying the
  new `app_revision`, the bounded and redacted detail, and the
  bounded `selected_files` list

#### Scenario: Stale refinement

- **WHEN** the request's `expected_revision` does not match the
  current `spec_revision` or `app_revision`
- **THEN** Forge refuses with `studio-revision-conflict` and writes
  nothing

#### Scenario: Refinement path escape

- **WHEN** any `selected_files` entry resolves outside the
  registered project root or contains a secret-shaped string
- **THEN** Forge refuses with `studio-project-scope` or
  `studio-invalid-spec` and writes nothing

#### Scenario: A recorded refinement is not a performed refinement

- **WHEN** a refinement request is accepted
- **THEN** the `studio.refine` row's state names the recorded request
  and never claims that the refinement was performed

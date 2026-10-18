## MODIFIED Requirements

### Requirement: Reject a hand-rolled auth screen

The checker MUST report `UI_AUTH_HANDROLLED` as `ERROR` for a UI file whose file
name matches, case-insensitively, `login`, `logout`, `register`, `signup`,
`sign-in`, `signin`, `forgot.?password`, `reset.?password` or `auth`, when that
file is not inside a declared `ui.shared_layer.consume_paths` entry and does not
demonstrably render the shared layer. The match MUST be against the file name
only. The file MUST be reported when it cannot be read, and a declared adoption
of the shared layer MUST NOT suppress this finding for a file that renders a
screen of its own.

A file demonstrates that it renders the shared layer when it carries one or more
`platform-*` class names, or when a stylesheet reference in it names the shared
layer. It MUST NOT be treated as demonstrating the shared layer merely because
the repository declared adoption.

A `.cshtml` or `.razor` page that renders no markup of its own MAY be judged by
the surface it composes — its `@layout`, or the `Layout` its nearest
`_ViewStart.cshtml` assigns — and MAY go unreported only when the repository
declares `ui.shared_layer.adopted: true`, the composed surface resolves inside
the project, and that surface itself demonstrates the shared layer by the same
two tests. A page of any other suffix MUST NOT be exempt by composition, and a
composition that cannot be resolved MUST be reported. Every file that goes
unreported for either reason MUST be named in the report notes together with the
signal that was read.

#### Scenario: Hand-rolled login with no adoption

- **WHEN** a repository ships `pages/LoginPage.tsx` and declares no shared-layer
  adoption
- **THEN** the checker reports `UI_AUTH_HANDROLLED` as `ERROR`

#### Scenario: Adoption plus a new hand-rolled login

- **WHEN** a repository declares `ui.shared_layer.adopted: true` and still ships
  a login screen outside `consume_paths`
- **THEN** the checker reports `UI_AUTH_HANDROLLED` as `ERROR` while the codes
  the adoption does suppress stay silent

#### Scenario: Login page that renders the shared layer

- **WHEN** a login-named UI file carries one or more `platform-*` class names, or
  references a shared-layer stylesheet
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted for it, and a note names the
  file and the signal that was read

#### Scenario: Handler page rendered by the shared surface

- **WHEN** a repository declares adoption, a `.cshtml` page whose name matches the
  pattern renders no markup of its own, and the layout it composes links a
  shared-layer stylesheet
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted for it, and the note names
  the composed layout

#### Scenario: Handler page in a repository that declares no adoption

- **WHEN** a repository that declares no adoption ships a `.cshtml` page that
  renders no markup of its own
- **THEN** `UI_AUTH_HANDROLLED` is reported, because the composition signal is
  gated on a declared adoption

#### Scenario: Handler page whose layout resolves to nothing shared

- **WHEN** the composed layout cannot be resolved, or resolves to a surface with
  no shared-layer signal
- **THEN** `UI_AUTH_HANDROLLED` is reported

#### Scenario: Unreadable auth-named file

- **WHEN** an auth-named UI file cannot be read
- **THEN** `UI_AUTH_HANDROLLED` is reported, because the content test could not
  run

#### Scenario: Shared package's own screen

- **WHEN** a login-named UI file lies inside a declared
  `ui.shared_layer.consume_paths` directory
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted for it

#### Scenario: Directory named auth

- **WHEN** a file `Article.tsx` lives under `src/authors/`
- **THEN** no `UI_AUTH_HANDROLLED` finding is emitted, because the pattern is
  matched against the file name

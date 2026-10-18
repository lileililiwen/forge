## ADDED Requirements

### Requirement: The governance front door routes UI work to a named reference

`skills/workspace-governance/SKILL.md` MUST carry exactly one routing row for
the situation "about to write or change a user interface, an auth page, or a
theme". That row MUST point at `skills/workspace-governance/references/ui-convention.md`,
and that reference MUST exist. The reference MUST answer, without the reader
having to search the workspace: whether this repository is supposed to consume
the shared UI layer, the single command that answers it, and what to do next.

#### Scenario: An agent is about to write a login page

- **WHEN** a task in any repository involves a login, register, logout,
  forgot-password, theme, design token, or other user-interface surface
- **THEN** the skill's routing table names the UI convention reference as the
  route for that situation

#### Scenario: The route resolves to a real file

- **WHEN** the routing table is walked
- **THEN** each row names an existing reference and
  `python3 scripts/check_skill_refs.py` exits zero

### Requirement: The reference states the finding codes and what each one means

`skills/workspace-governance/references/ui-convention.md` MUST name every
finding code the governance UI checker publishes for a repository, and for each
one state what it means for the change in hand. The codes are owned by
`scripts/ui_conformance.py`; the reference MUST cite that checker as the source
of the vocabulary rather than inventing a parallel one, and it MUST state the
same set the checker actually defines.

That set is seven codes:

| code | what it means for the change in hand |
|---|---|
| `UI_AUTH_HANDROLLED` | A named auth screen is still hand-rolled and not under a declared `consume_paths`. Delete it and consume the shared package. |
| `UI_TOKEN_NOT_CONSUMED` | Adoption is not declared. Declare it, or record why the layer is declined. |
| `UI_COLOR_LITERAL` | A colour literal does not resolve to a token. Replace it with the token reference. |
| `UI_THEME_ABSENT` | No theme mechanism is present. Adopt `ThemeProvider` or declare a theme mechanism. |
| `UI_BRANDING_ABSENT` | No branding document is resolvable. Ship one or declare where it is. |
| `UI_CONTRAST_UNMEASURED` | No measured-contrast artefact exists. Record a measured run; build success is not contrast evidence. |
| `UI_SCOPE_UNDECLARED` | The repository ships UI and declares no scope, so the content rules are silent by declaration rather than by exemption. |

The reference MUST state that the frozen baseline is separate from the codes,
that a baselined finding is reported as baselined rather than as a pass, and
that the baseline exists to shrink. It MUST NOT describe a code the checker
does not define: a documented code that cannot be produced is a reader chasing
a finding that will never arrive.

#### Scenario: A finding is read correctly

- **WHEN** a reader opens the reference after a checker run named
  `UI_COLOR_LITERAL`
- **THEN** the reference tells them the fix is to replace the literal with a
  token reference, and does not tell them to open a baseline exemption or
  re-pin a digest

#### Scenario: A baselined finding is not read as a pass

- **WHEN** a reader's finding is covered by the frozen baseline
- **THEN** the reference states it is reported as baselined, that the baseline
  is a data file that may only shrink, and that adding an entry to silence a
  finding is not adoption

#### Scenario: No code is documented that cannot fire

- **WHEN** the reference's code table is compared against the checker's
  published vocabulary
- **THEN** every code named in the reference is one the checker can emit, and
  every code the checker can emit is named in the reference

### Requirement: The reference carries a worked adoption path per stack, with real paths

The reference MUST give a concrete adoption path for every stack present in
this workspace, and MUST cite real, existing paths rather than describing a
procedure in the abstract. It MUST name `somedanote` as the worked example and
cite its `vendor/platform-ui/`, `scripts/sync-platform-ui.mjs`,
`ui-conformance.json`, `branding.json`, `src/SomadaNote.Web/Branding/`, and
`tests/SomadaNote.Tests.Unit/SharedUi/`, because that repository is the only
real consumer and an abstract procedure is not copyable.

#### Scenario: A reader adopts on their own stack

- **WHEN** a reader on a stack named in the reference starts adoption
- **THEN** the reference names the files they will write and the check that
  proves the adoption, and every path it names exists

#### Scenario: The worked example is real

- **WHEN** the reference cites `somedanote` as the worked example
- **THEN** the cited paths are the ones that repository actually uses, not a
  sketch of how it might

### Requirement: The reference states the decided policies and does not re-open them

The reference MUST state the four decided policies as decisions, with the ruling
they came from: the feed is vendored and never published to npm; a local
username/password form serves site users while forge's OIDC+PKCE serves the
operator portal; Flutter/Dart is a permanent exemption; `branding.json` is
owned by each repository and schema-validated at build time. It MUST NOT present
any of them as an open question, and MUST NOT ask the reader to choose.

#### Scenario: A reader asks whether to publish the feed

- **WHEN** a reader considers publishing the vendored feed to a public npm scope
- **THEN** the reference says the feed is vendored by decision (D1) and that
  publishing is an account-level action that is not this path

#### Scenario: A reader asks about the Dart repositories

- **WHEN** a reader wonders why the Dart repositories are not failing the check
- **THEN** the reference says Flutter/Dart is a permanent exemption by decision
  (D3) and that the check does not claim coverage it does not have

### Requirement: The reference states what is out of scope for it

The reference MUST state that migrating a repository is that repository's own
change with its own spec, and that this repository does not edit another
repository's source. An agent that reads the reference must be able to tell the
difference between "guide me" and "do it for me".

#### Scenario: An agent is asked to migrate a sibling

- **WHEN** a reader asks this governance repository to migrate another
  repository onto the shared layer
- **THEN** the reference routes that to the adopting repository's own change,
  and states the cross-repo source-edit boundary

### Requirement: A per-repository binder adds the ui-conformance check to that repository's gate

`scripts/bind_ui_gate.py` MUST add a `ui-conformance` check and a matching
`commands.ui-conformance` entry to exactly one repository's
`.ai-gate/gate.yaml`, and MUST add them to no other repository. It MUST be
idempotent: a second run over an already-bound repository MUST write nothing
and report `no change`. It MUST refuse a repository whose manifest is not the
governance `rule_pack` shape, naming the reason, rather than writing an entry
into a document whose schema it does not recognise. It MUST support `--check`
as a dry run that exits non-zero when it WOULD change something, and `--json`
for machine consumption.

#### Scenario: An unbound repository is bound

- **WHEN** the binder is run with `--apply` on a repository whose
  `rule_pack`-shaped manifest selects `ui-conformance` nowhere
- **THEN** `checks.ui-conformance: true` and a matching
  `commands.ui-conformance` entry are written, and the tool reports what it
  changed

#### Scenario: The second run changes nothing

- **WHEN** the binder is run a second time on the repository it just bound
- **THEN** the manifest is byte-for-byte unchanged and the tool reports
  `no change`

#### Scenario: A dry run reports without writing

- **WHEN** the binder is run with `--check` on a repository that is not yet
  bound
- **THEN** it exits non-zero, names the change it would make, and writes
  nothing

#### Scenario: A manifest of an unrecognised shape is refused

- **WHEN** the binder is run on a repository whose `.ai-gate/gate.yaml` is not
  a `rule_pack`-shaped governance manifest
- **THEN** it refuses, names why, and writes nothing

#### Scenario: Only the named repository is touched

- **WHEN** the binder is run on one repository
- **THEN** no other repository's manifest is read for writing or modified

### Requirement: The binder never resolves a runtime or environment-dependent path

The command the binder writes MUST be written relative to the repository root
and MUST NOT be an absolute path, and MUST NOT depend on an environment
variable. A CI runner that lacks the variable fails exactly as a hard-coded path
fails, so the same refusal the shared feed already applies to a feed path
applies here. A `..` segment is NOT itself a defect: the house form of a
governance command is sibling-relative from the repository root, so the command
written here MUST match the shape of the `governance-quality` command this
workspace's own manifests already carry.

#### Scenario: The bound command runs from the repository root

- **WHEN** the bound command runs with the repository root as its working
  directory
- **THEN** it resolves without any environment variable being set when the
  workspace-governance checkout is present at the documented sibling path

#### Scenario: An absolute command is refused

- **WHEN** a caller passes an absolute path as the command
- **THEN** the binder refuses it by name, writes nothing, and exits non-zero

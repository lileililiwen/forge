# Workspace Governance `.project.json` schema note (task 1.1 extraction)

Extracted from the sibling repository `workspace-governance` at HEAD `6b8981d`
(2026-09-24). This note is the fixture contract Forge emits against; the
sibling owns the schema and any drift in it is a docs/fixture fix for Forge,
never a Forge schema migration.

## Published schema (`schemas/project.schema.json`)

Required top-level keys: `schema_version` (const `1`), `id`
(`^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$`), `kind` (non-empty string), `profile`
(non-empty string), `lifecycle` (enum `active|planning|reference|inventory|archived`),
`verification` (object), `deployment` (object).

- `verification` documents `command`, `gate_runtime`, `evidence_status`
  (all non-empty strings).
- `deployment` documents `deployable` (boolean), `jenkins_job`
  (string or null), `compose_file` (string or null).

## Checker semantics (`scripts/workspace_check.py`)

- `validate_project` (the only runtime reader of a declaration) consumes:
  `schema_version == 1`, `id` matching the entry id, non-empty
  `verification.command` for product-tier profiles, and
  `deployment.deployable` as a boolean. It never reads `gate_runtime` or
  `evidence_status` at runtime; the JSON schema file is documentation.
- Discovered unregistered projects are added with `adoption: pending`;
  their declared `profile` and `lifecycle` become the effective registry
  shape. Local metadata is the source of truth (`DISCOVERED_UNREGISTERED`
  message).
- Profile vocabulary (`validate_registry`): the constant set
  `{governance, product, dotnet-library, dotnet-template, deployment,
  inventory}` plus any `*-product` suffix plus
  `{platform, tooling, python-runtime, rust-platform, typescript-monorepo}`.
  Observed real usage in `projects.json`: `product` (34), `rust-product`
  (10), `typescript-product` (6), `python-product` (3), `dotnet-product`
  (1), plus the light tiers. `node-product` appears nowhere; Forge maps
  its TypeScript profiles to the observed `typescript-product` value.
- `jenkins_manifest.py` skips any project whose declaration has
  `deployment.deployable` falsy, so Forge's non-deployable defaults are
  inert for the deploy plane, and it only reads
  `verification.command`/`jenkins_job`/`compose_file` for deployable
  projects.

## Honesty mapping (Forge side)

| Forge profile | governance profile | kind | verification command |
| --- | --- | --- | --- |
| aspnet-web | dotnet-product | product | descriptor `test_command` |
| flutter-app | flutter-product | product | descriptor `test_command` |
| nextjs-web | typescript-product | product | descriptor `test_command` |
| python-service | python-product | product | descriptor `test_command` |
| react-web | typescript-product | product | descriptor `test_command` |
| rust-web | rust-product | product | descriptor `test_command` |

- `evidence_status` is always `planned` at generation; Forge never writes
  `passed`.
- `gate_runtime` is emitted only when the profile descriptor declares one.
  No Forge profile declares a gate runtime today (no shared Gate Runtime
  is configured), so generated documents omit the key; the sibling
  checker never reads it. A future gate-enabled profile adds the value to
  its descriptor mapping, never to the generator.
- `deployment.deployable: false` with `jenkins_job: null` and
  `compose_file: null` at creation; a real target flips these through the
  project's own edits, not Forge generation.
- Profiles whose descriptor declares no mapping (the `workspace: null`
  sentinel, including every planned candidate) emit no file and print a
  note; a guessed governance profile string is refused.
- The emitted document carries no host paths, no machine-specific roots
  and no absolute timestamps: it is a pure function of (project id,
  profile descriptor).

## Ownership receipt

Generation also stages
`.forge/workspace/project.json.receipt`, a deterministic Forge-managed
record of the exact declared bytes (`sha256:` line). As with feature
receipts, a manual edit to the owned file makes the bytes diverge from
the recorded hash and blocks upgrades with the standard ownership-conflict
refusal; the user's edited file is preserved, never overwritten. A
`.project.json` without a receipt (e.g. written by the sibling's
`init_project.py` or by hand) is foreign content: import and upgrade
never write or reject it.

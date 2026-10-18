# Declarations and the registry audit

## Read first

- `projects.json` — the single source of project identity.
- `schemas/project.schema.json` — the declaration shape (`kind`, `profile`,
  `verification`, `deployment`, optional `capabilities`, `release_evidence`,
  `quality`).
- `schemas/registry.schema.json` — the registry shape.
- `openspec/specs/registry-integrity/spec.md` — the canonical contract. It
  owns the failure vocabulary (`DECLARATION_ID`, `PROFILE_UNKNOWN`,
  `DISCOVERED_UNREGISTERED`, and the rest); this reference does not repeat it.

## Run

- `python3 scripts/workspace_check.py --root .. --audit` — the authoritative
  portfolio audit. It fails on governance or adoption errors.
- `python3 scripts/workspace_check.py --self-check` — the governance
  repository only; the fast health probe.
- `python3 scripts/workspace_check.py --root .. --project <id>` — one
  registered project.
- `python3 scripts/init_project.py /path/to/project --profile <profile> --kind <kind>` —
  adopt a project without copying shared runtimes or credentials into it.

## Rules of the road

- Registry presence is not implementation evidence; the checker records
  declarations and policy status only.
- A discovered folder is never adopted or deployable until its metadata and
  evidence pass.
- Kind and profile values come from `vocabulary.json`; see
  `skills/workspace-governance/references/vocabulary.md`.

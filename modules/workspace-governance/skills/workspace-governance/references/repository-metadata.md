# Repository metadata and GitHub publication

This capability owns local license/discoverability checks and authenticated
GitHub repository metadata. The audit is read-only; repair and publication are
explicit operations.

Owning modules are `python3 scripts/repository_metadata.py` and
`python3 scripts/github_metadata.py`; local repair is owned by
`python3 scripts/init_project.py`.

## Audit the workspace

```bash
python3 scripts/workspace_check.py --root <workspace-root> --audit --json
```

Standard adopted projects must contain `LICENSE`, a complete
`.project.json.discoverability` object, and verified publication evidence.
Inventory projects remain declaration-only. Findings such as
`DISCOVERABILITY_MISSING`, `LICENSE_MISSING`, and `PUBLICATION_UNVERIFIED`
identify the repair boundary.

## Repair local metadata

```bash
python3 scripts/init_project.py <project> \
  --profile <profile> --kind <kind> --apply-metadata \
  --description "Source-backed description" \
  --keywords repository governance \
  --topics repository-governance github-metadata \
  --audience developers \
  --use-case "audit project repositories" \
  --search-phrase "repository governance tool" \
  --homepage https://github.com/<owner>/<repo> \
  --license-spdx MIT \
  --visibility public
```

The command refuses incomplete semantic metadata and never overwrites an
existing license or discoverability block.

## Check or publish GitHub metadata

```bash
python3 scripts/github_metadata.py <project> --check --json
python3 scripts/github_metadata.py <project> --apply --json
python3 scripts/github_metadata.py <project> --publish \
  --owner <owner> --repository <repo> --json
```

All modes require `gh auth status` to succeed. If authentication fails, stop
and ask the operator to run `gh auth login` manually. `--check` does not mutate
files, remotes, or GitHub. `--apply` uses additive `gh repo edit` topic updates.
`--publish` may create a missing GitHub repository or push an existing origin,
but never force-pushes or replaces a conflicting remote.

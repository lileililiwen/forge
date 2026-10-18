# Design: Repository Discoverability Governance

## Implementation boundary

The implementation lives in `/home/paul/code/workspace-governance` using
Python 3.11+ standard library modules:

- `scripts/repository_metadata.py`: pure validation, normalization, and local
  metadata helpers.
- `scripts/workspace_check.py`: read-only orchestration and findings.
- `scripts/init_project.py`: additive local repair and license provisioning.
- `scripts/github_metadata.py`: subprocess adapter for `git` and `gh`; no
  token handling and no dependency on Driftwatchdog or Forge.
- `schemas/project.schema.json`: declaration contract.

Do not modify product source, sibling repositories, deployment execution, or
the existing Forge adapter contract.

## Language and runtime

Use Python 3.11+ and the standard library only. Run tests with
`python3 -m unittest discover -s tests -v`. CLI subprocesses must be injectable
or replaceable in tests. JSON output remains deterministic with sorted keys.

## Ownership and shared code

`workspace-governance` is the policy and audit owner. `driftwatchdog` may
consume resulting findings as evidence, and Forge may consume the existing
read-only adapter, but neither owns GitHub metadata or local repair. No new
common/manager extraction is authorized by this change.

## Behavioral model

| State | Meaning | Allowed transition |
|---|---|---|
| `DISCOVERED_UNREGISTERED` | Workspace discovery found a project not in registry | Audit reports; local repair may add missing project files but does not register it automatically |
| `LOCAL_METADATA_REQUIRED` | License or discoverability declaration is absent/invalid | `init_project.py --apply` adds only explicit, valid missing values |
| `LOCAL_STANDARDIZED` | Required local files and declaration metadata validate | GitHub check may compare remote state |
| `PUBLICATION_UNVERIFIED` | GitHub auth/remote/metadata evidence is absent or mismatched | `github_metadata.py --check` reports; `--apply` may edit metadata only with valid auth |
| `METADATA_VERIFIED` | GitHub description/homepage/topics and remote evidence match declaration | Audit records pass; no further mutation occurs |

Local declarations are intent. Publication evidence is a separate object with
provider, owner, repository, branch, state, and repository-relative evidence
reference. A declaration must never claim `metadata_verified` without a current
verification reference.

## Contract and compatibility

The optional schema additions are:

```json
{
  "discoverability": {
    "description": "string",
    "keywords": ["string"],
    "topics": ["lowercase-topic"],
    "audience": ["string"],
    "use_cases": ["string"],
    "search_phrases": ["string"],
    "homepage": "https://example.invalid",
    "license_spdx": "MIT",
    "visibility": "public"
  },
  "publication": {
    "provider": "github",
    "owner": "owner",
    "repository": "repo",
    "default_branch": "main",
    "state": "metadata_verified",
    "evidence_ref": "HANDOFF.md"
  }
}
```

Existing declarations remain schema-valid because the blocks are optional at
the JSON Schema layer; profile validation makes them mandatory for standard
adopted profiles. Inventory projects remain declaration-only.

Stable findings are `LICENSE_MISSING`, `DISCOVERABILITY_MISSING`,
`DISCOVERABILITY_INCOMPLETE`, `DISCOVERABILITY_INVALID`,
`PUBLICATION_UNVERIFIED`, `GITHUB_AUTH_REQUIRED`, `GITHUB_REMOTE_MISSING`,
`GITHUB_METADATA_DRIFT`, and `GITHUB_IDENTITY_AMBIGUOUS`.

## Failure and boundary policy

- Missing/invalid `gh` auth: blocking error; print manual `gh auth login`, do
  not continue or inspect credentials.
- Missing origin: `--check` reports `GITHUB_REMOTE_MISSING`; `--publish` may
  create one only when owner, repository, and visibility are explicit.
- Existing remote: preserve it; never replace or force-push.
- Existing topics: additive updates only; never delete without an explicit
  operator flag that is outside this change.
- Missing semantic metadata: repair refuses to invent it and reports the exact
  required arguments.
- GitHub unavailable after local repair: local changes remain, state is
  `PUBLICATION_UNVERIFIED`, and retry command is recorded.
- Secret-like values: reject before write and never include them in output.

## Verification oracle

- Unit tests cover every stable finding, valid metadata, preservation, auth
  failure, identity ambiguity, command construction, and no-token output.
- `python3 scripts/workspace_check.py --self-check` proves the governance
  repository contract.
- `python3 scripts/check_skill_refs.py` proves routing references.
- `python3 -m unittest discover -s tests -v` must pass.
- `openspec validate --all --strict --no-interactive` must pass before archive.
- Live GitHub publication is environment evidence, not required for unit tests;
  when unavailable it is recorded as blocked rather than passed.

## Decision ledger

- Use `gh` as the GitHub API boundary because the user explicitly requires it
  and it already handles configured HTTPS/keyring authentication.
- Keep audit read-only and put mutations behind explicit CLI modes.
- Use MIT only as the explicit default for ordinary source projects when no
  license exists; never replace an existing license or invent a copyright
  holder.
- Defer organization/team policy, visibility changes, topic deletion, and
  remote force operations.

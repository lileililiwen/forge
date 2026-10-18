# readiness Specification

## Purpose

`workspace-governance` is the workspace's shared governance owner: it owns the project registry, governance profiles, bootstrap templates, evidence vocabulary, and deployment declarations, and the read-only Forge governance adapter. Two changes shipped in sequence (`registry-and-declaration-integrity`, `shared-rules-single-source`), the workspace audit is at **0 errors / 18 warnings**, `python3 -m unittest discover -s tests` reports **122 tests, 0 failures**, and six canonical specs validate under `--strict` (`HANDOFF.md`). The README is 182 lines and accurately states ownership, the mandatory command, onboarding, promotion, and Jenkins handoff. It is a **shared/infrastructure** project, not a product, so readiness means a new maintainer or consumer can *use* the governance surface, not that a UI exists. Three documentation gaps remain: 1. **No consolidated CLI usage.** The commands are spread across the "Mandatory command", "Onboard a project", "Deployment promotion", and "Jenkins" sections, so there is no single copy-paste reference for the common tasks (self-check, audit, auto-gate, quality check, maturity, drift, init, promote, manifest, Forge adapter). 2. **No terminal evidence.** The repository is CLI-only, so screenshots are **not applicable**, but there is no checked-in capture of a real `workspace_check.py --self-check` or `--audit --json` run to show what a healthy and an unhealthy audit actually look like. 3. **Registry/evidence vocabulary is scattered.** "Declaration", `evidence_status` (`declared`/`configured`/`verified`/`unverified`), `release_evidence`, `deployable`, the promotion ledger, and the finding codes are explained across `README.md`, `docs/capabilities.md`, `docs/product-code-quality.md`, and the canonical specs, but nowhere is the distinction — **declaration is intent, evidence is verification, `release_evidence` backs a deployment, `deployable` requires both** — stated in one place. This is exactly the confusion packages 1 and 2 removed from the tooling; the docs still carry it. Readiness here means a consumer can copy a working command, see a real audit capture, and look up the vocabulary without reading the scripts.
## Requirements
### Requirement: CLI usage is consolidated and accurate

The README SHALL provide one copy-paste example for each governance command, and
every example MUST use a flag and produce output the shipped scripts support.

#### Scenario: Consumer runs a command

- **WHEN** a consumer needs to run an audit, gate, quality check, maturity
  report, drift report, onboarding, promotion, manifest, or the Forge adapter
- **THEN** the README provides a copy-paste example for that command

#### Scenario: Example invents a flag (negative)

- **WHEN** a usage example names a flag or output the scripts do not produce
- **THEN** the readiness claim is FAIL and the example MUST be corrected to real behavior

### Requirement: Terminal capture is present and screenshots are not applicable

The README SHALL embed at least one checked-in terminal capture of a real
`workspace_check.py --self-check` and `--audit --json` run, stored under
`docs/assets/`, and SHALL state that GUI screenshots are not applicable because
the project is CLI-only. Every committed capture SHALL be redacted of tokens,
credentials, host usernames, absolute private paths, and secret-shaped values.

#### Scenario: Reader sees a real audit

- **WHEN** a reader opens the README
- **THEN** a checked-in capture shows the self-check result and an audit run

#### Scenario: Capture contains sensitive data (negative)

- **WHEN** a capture contains a token, credential, host username, absolute private path, or secret value
- **THEN** it MUST NOT be committed and MUST be regenerated redacted

#### Scenario: External host introduced (negative)

- **WHEN** a capture is hotlinked from an external URL
- **THEN** the change is rejected, because the README must render offline

### Requirement: Registry and evidence vocabulary has one entry point

The repository SHALL define the registry and evidence vocabulary in one place,
distinguishing a declaration (intent) from evidence (`evidence_status`),
`capabilities`, `release_evidence`, `deployable`, the promotion ledger, and
finding codes, and each definition SHALL link its canonical source rather than
restate policy.

#### Scenario: Consumer resolves a term

- **WHEN** a consumer looks up `release_evidence` or `deployable`
- **THEN** the README names the term's meaning and links `docs/capabilities.md` or the owning spec

#### Scenario: Declaration read as evidence (negative)

- **WHEN** the documentation implies that a declaration or registry presence is implementation evidence
- **THEN** the readiness claim is FAIL and the text MUST be corrected

#### Scenario: Policy restated and drifting (negative)

- **WHEN** the vocabulary copies a policy out of its canonical source
- **THEN** it MUST be replaced with a link, because a second source of truth will drift

### Requirement: Readiness stays documentation-only

The change SHALL NOT modify any script, checker, schema, profile, template,
`projects.json`, promotion ledger, `.project.json`, or canonical spec, and SHALL
NOT change governance behavior.

#### Scenario: Boundary review

- **WHEN** the change diff is reviewed
- **THEN** only `README.md`, `docs/**`, and the change folder are touched, and `workspace_check.py --self-check` is unchanged at 0 errors / 0 warnings

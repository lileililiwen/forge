# Forge Publish Plugin Orchestration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Forge the single publish control plane with switchable external providers and one converged manual/GitHub-push publish request.

**Architecture:** Forge owns provider lifecycle, publish request validation, idempotency, and evidence. Providers are external JSON-line processes selected by provider configuration; OpenPanel and Jenkins remain standalone repositories that implement the contract in dependent changes. No provider implementation is embedded in Forge core.

**Tech Stack:** Rust 1.87+, existing Forge CLI, SQLite registry, serde JSON, existing release/deploy evidence model, stdio subprocess adapters.

**Spec:** `openspec/changes/forge-publish-plugin-orchestration/`

## Global Constraints

- Provider wire contract is `forge-publish-provider/0.1.0`.
- Disabled or unavailable providers must not be invoked.
- Push and manual publishing must use one `PublishRequest` path.
- Provider output is redacted evidence; secrets and source contents never enter Forge persistence.
- Existing release/deploy contracts and unrelated dirty worktree changes must remain compatible.
- Forge must not add OpenPanel or jenkins-local as a workspace dependency.

---

### Task 1: Freeze provider contract fixtures

**Files:**
- Create: `tests/fixtures/publish-provider/capabilities-request.json`
- Create: `tests/fixtures/publish-provider/publish-request.json`
- Create: `tests/fixtures/publish-provider/success-response.json`
- Create: `tests/fixtures/publish-provider/failure-response.json`
- Modify: `docs/adapter-contracts/publish-provider.md`
- Test: `src/publish/providers.rs` unit tests

**Interfaces:**
- Consumes: existing `forge-deploy-executor/0.1.0` envelope conventions.
- Produces: exact request/response JSON shapes and redaction rules used by all later provider tasks.

- [ ] **Step 1: Write failing fixture/parser tests** for contract, operation, provider, operation id, project, revision, status, health, evidence, and recovery fields.
- [ ] **Step 2: Run** `rtk cargo test publish::providers --lib`; expected failure because the provider module does not exist.
- [ ] **Step 3: Add** the frozen contract document and JSON fixtures.
- [ ] **Step 4: Run** `rtk openspec validate forge-publish-plugin-orchestration --strict --no-interactive` and the focused test.
- [ ] **Step 5: Commit** `docs: freeze Forge publish provider contract`.

### Task 2: Add provider registry and lifecycle

**Files:**
- Create: `src/publish/providers.rs`
- Modify: `src/publish/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/publish/providers.rs`

**Interfaces:**
- Consumes: Task 1 contract types.
- Produces: `ProviderId`, `ProviderState`, `ProviderManifest`, `ProviderRegistry`, `enable`, `disable`, `inspect`, and `select_enabled`.

- [ ] **Step 1: Write failing tests** for install/list, enable, disable, unavailable provider, and disabled-provider refusal.
- [ ] **Step 2: Run** `rtk cargo test publish::providers --lib`; expected lifecycle failures.
- [ ] **Step 3: Implement** bounded provider records persisted through the existing registry operation store; no provider process invocation belongs in the registry.
- [ ] **Step 4: Run** focused tests and `rtk cargo test --lib publish::`.
- [ ] **Step 5: Commit** `feat: add Forge publish provider lifecycle`.

### Task 3: Converge manual publish requests

**Files:**
- Modify: `src/main.rs`
- Modify: `src/publish/mod.rs`
- Test: `src/publish/providers.rs`
- Test: `tests/publish_contract.rs`

**Interfaces:**
- Consumes: `ProviderRegistry` and existing project/revision resolution.
- Produces: `PublishRequest { project_id, folder, repository, revision, provider, trigger, operation_id, dry_run }` and CLI forms `forge publish --project <id>` and `forge publish --folder <path>`.

- [ ] **Step 1: Write failing CLI tests** for project and folder resolution, missing project, invalid folder, and explicit provider selection.
- [ ] **Step 2: Run** `rtk cargo test --test publish_contract`; expected failure for new flags.
- [ ] **Step 3: Implement** both forms through one request constructor and reject ambiguous project/folder input before provider execution.
- [ ] **Step 4: Run** focused CLI tests and `rtk cargo test --lib publish::`.
- [ ] **Step 5: Commit** `feat: converge Forge manual publish inputs`.

### Task 4: Add provider process invocation and evidence

**Files:**
- Modify: `src/publish/providers.rs`
- Modify: `src/core/mod.rs`
- Modify: `docs/adapter-contracts/publish-provider.md`
- Test: `src/publish/providers.rs`

**Interfaces:**
- Consumes: `PublishRequest` and provider manifest command.
- Produces: bounded stdio invocation with one JSON response, timeout, contract validation, redaction, and terminal `PublishReport`.

- [ ] **Step 1: Write failing tests** for success, dry-run, timeout, malformed JSON, contract mismatch, nonzero exit, and secret redaction.
- [ ] **Step 2: Run** focused provider tests; expected failures.
- [ ] **Step 3: Implement** argument-array subprocess invocation with project-root working directory and bounded output; never invoke a shell.
- [ ] **Step 4: Run** focused tests and `rtk git diff --check`.
- [ ] **Step 5: Commit** `feat: invoke Forge publish providers safely`.

### Task 5: Add verified GitHub push convergence

**Files:**
- Modify: `src/publish/providers.rs`
- Modify: `src/main.rs`
- Test: `tests/publish_contract.rs`
- Test: `src/publish/providers.rs`

**Interfaces:**
- Consumes: signed GitHub push payload, repository allowlist, branch policy, and Task 3 request constructor.
- Produces: `PublishTrigger::GitHubPush`, signature verification, full SHA validation, and duplicate-delivery idempotency.

- [ ] **Step 1: Write failing tests** for valid signature, invalid signature, wrong repository/ref, short SHA, duplicate delivery, and new delivery.
- [ ] **Step 2: Run** focused tests; expected failures.
- [ ] **Step 3: Implement** push verification and route accepted events into the same `PublishRequest` constructor.
- [ ] **Step 4: Run** `rtk cargo test --test publish_contract` and `rtk cargo test --lib publish::`.
- [ ] **Step 5: Commit** `feat: route verified GitHub pushes through Forge publish`.

### Task 6: Provider toggle and compatibility matrix

**Files:**
- Modify: `src/main.rs`
- Modify: `docs/quickstart.md`
- Test: `tests/publish_contract.rs`

**Interfaces:**
- Consumes: provider lifecycle and publish engine.
- Produces: `forge provider list|enable|disable|inspect` and a dry-run matrix proving OpenPanel/Jenkins can be switched independently.

- [ ] **Step 1: Write failing CLI tests** for provider commands and disabled-provider behavior.
- [ ] **Step 2: Implement** provider lifecycle CLI and provider selection diagnostics.
- [ ] **Step 3: Run** `rtk cargo test --test publish_contract` and the provider unit suite.
- [ ] **Step 4: Run** strict OpenSpec validation, full Forge library tests, and `rtk cargo build`.
- [ ] **Step 5: Inspect** staged/uncommitted diffs and archive only after the dependent provider conformance work is ready.

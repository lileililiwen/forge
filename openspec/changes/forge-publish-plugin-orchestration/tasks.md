# Tasks: forge-publish-plugin-orchestration

## 1. BFS — Baseline and impact coverage

- [x] Confirm existing Forge release/deploy state and registry persistence.
- [x] Confirm OpenPanel deployment-adapter, application-delivery, git-webhook,
  and JSON-RPC plugin surfaces.
- [x] Confirm Jenkins compatibility behavior and remove Mac-local runtime
  assumptions from the provider boundary.
- [x] Freeze `forge-publish-provider/0.1.0` request, response, lifecycle, and
  redaction fixtures.

## 2. DFS — Requirement-by-requirement implementation

- [x] Add Forge tests for provider list/enable/disable/inspect.
- [x] Implement provider configuration and disabled-provider refusal.
- [x] Add Forge coverage for project and folder publish forms.
- [x] Implement the unified manual publish request path.
- [x] Add signed GitHub push intake and idempotent delivery handling.
- [x] Implement provider process invocation and response validation.
- [x] Freeze external provider registration examples for standalone OpenPanel
  and jenkins-local executables.
- [x] Make provider configuration independently switchable in Forge.

## 3. BFS — Cross-surface regression and completeness

- [x] Prove disabled providers are never invoked.
- [ ] Prove duplicate push deliveries invoke a provider at most once with an end-to-end provider fixture.
- [ ] Prove provider secrets and source contents never enter Forge evidence.
- [ ] Prove OpenPanel provider failure does not prevent Jenkins provider use.
- [ ] Prove Jenkins provider remains optional and no Mac script bundle is
  required.
- [ ] Prove manual and push-triggered publish produce equivalent requests.

## 4. Verification

- [x] Run Forge unit, CLI, contract, and integration tests.
- [ ] Run OpenPanel provider conformance tests and relevant workspace gates.
- [ ] Run Jenkins compatibility provider tests.
- [x] Run strict OpenSpec validation in all affected repositories.
- [x] Perform a dry-run provider toggle matrix before any runtime canary.

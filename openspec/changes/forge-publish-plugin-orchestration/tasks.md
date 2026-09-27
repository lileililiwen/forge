# Tasks: forge-publish-plugin-orchestration

## 1. BFS — Baseline and impact coverage

- [ ] Confirm existing Forge release/deploy state and registry persistence.
- [ ] Confirm OpenPanel deployment-adapter, application-delivery, git-webhook,
  and JSON-RPC plugin surfaces.
- [ ] Confirm Jenkins compatibility behavior and remove Mac-local runtime
  assumptions from the provider boundary.
- [ ] Freeze `forge-publish-provider/0.1.0` request, response, lifecycle, and
  redaction fixtures.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Add failing Forge tests for provider list/enable/disable/inspect.
- [ ] Implement provider registry and disabled-provider refusal.
- [ ] Add failing Forge tests for project and folder publish forms.
- [ ] Implement the unified `PublishRequest` path.
- [ ] Add signed GitHub push intake and idempotent delivery handling.
- [ ] Implement provider process invocation and response validation.
- [ ] Freeze external provider registration examples for standalone OpenPanel
  and jenkins-local executables.
- [ ] Make OpenPanel and Jenkins providers independently switchable.

## 3. BFS — Cross-surface regression and completeness

- [ ] Prove disabled providers are never invoked.
- [ ] Prove duplicate push deliveries invoke a provider at most once.
- [ ] Prove provider secrets and source contents never enter Forge evidence.
- [ ] Prove OpenPanel provider failure does not prevent Jenkins provider use.
- [ ] Prove Jenkins provider remains optional and no Mac script bundle is
  required.
- [ ] Prove manual and push-triggered publish produce equivalent requests.

## 4. Verification

- [ ] Run Forge unit, CLI, contract, and integration tests.
- [ ] Run OpenPanel provider conformance tests and relevant workspace gates.
- [ ] Run Jenkins compatibility provider tests.
- [ ] Run strict OpenSpec validation in all affected repositories.
- [ ] Perform a dry-run provider toggle matrix before any runtime canary.

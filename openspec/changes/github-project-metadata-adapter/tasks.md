# Tasks: github-project-metadata-adapter

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read the provider/governance selection and redaction code plus `src/release/`; record the bounded-process, credential-reference and confirmation patterns to reuse.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/github/*`, external adapter executable contract, PR-mode default, no new dependency, token from environment only.
- [ ] 1.3 Map each requirement and scenario to its adapter call, normalization field, provider state, caller, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement the `forge-github-metadata/0.1.0` request/response contract and bounded argument-array invocation with timeout.
- [ ] 2.2 Implement normalization into catalog records with source, revision and freshness, and the closed provider-state mapping.
- [ ] 2.3 Implement PR-mode default and explicit-confirmation direct mode; refuse implicit settings changes.
- [ ] 2.4 Keep tokens and response bodies out of reports; keep GitHub topics, release tags and portfolio tags separate.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove no request is sent without configuration and local-only Forge is unaffected.
- [ ] 3.2 Exercise unauthorized, forbidden, not found, rate limited, unavailable, stale, partial and malformed-adapter boundaries against a local stub.
- [ ] 3.3 Prove a token never reaches any output and namespaces never merge.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no live GitHub host was contacted and no repository was mutated; all fixtures are local executable stubs.

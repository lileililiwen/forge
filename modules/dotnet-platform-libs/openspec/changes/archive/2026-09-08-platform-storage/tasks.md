## 1. Contracts

- [x] 1.1 Create `Platform.Storage` with validated keys, metadata, upload/download requests, presign results, outcomes, and provider status.
- [x] 1.2 Add unit tests for traversal/control-character rejection, content limits, expiry bounds, and safe failure messages.

## 2. Adapters

- [x] 2.1 Implement a local filesystem adapter with atomic writes, bounded paths, safe disposal, and deterministic tests.
- [x] 2.2 Implement an S3-compatible adapter with application-provided client/configuration, presigning, timeout, and failure classification.
- [x] 2.3 Add adapter architecture tests proving provider dependencies do not leak into `Platform.Storage`.

## 3. Adoption documentation

- [x] 3.1 Document authorization, tenant key prefixes, metadata ownership, retention, and bucket/filesystem migration boundaries.
- [x] 3.2 Port only the starter storage behavior and tests that fit the provider-neutral contract.
- [x] 3.3 Add package-folder guidance using `Contracts`, `Keys`, `Results`, and `DependencyInjection`.

## 4. Verification

- [x] 4.1 Run targeted tests, full solution build/test, package packing, strict OpenSpec validation, and architecture tests.
- [x] 4.2 Run `git diff --check`; record cloud-provider tests as blocked if credentials or external services are unavailable.

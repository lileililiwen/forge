## 1. BFS baseline

- [x] 1.1 Freeze current release, consumer configuration, package-version and skill behavior in focused tests.
- [x] 1.2 Add the canonical endpoint, Sleet tool pin and public-feed contract fixtures.
- [x] 1.3 Record the external GitHub Pages enablement prerequisite without claiming deployment.

## 2. DFS requirement implementation

- [x] 2.1 Implement deterministic static-feed generation and immutable-version collision checks.
- [x] 2.2 Replace GitHub Packages publication with persistent `gh-pages` plus GitHub Pages deployment.
- [x] 2.3 Implement the canonical portable consumer `NuGet.config` and cold restore/build proof.
- [x] 2.4 Implement idempotent user-level local-source add/remove configuration.
- [x] 2.5 Implement check/apply legacy-consumer migration with atomic refusal on ambiguous versions.
- [x] 2.6 Update consumer, release and troubleshooting documentation.
- [x] 2.7 Update `dotnet-platform-adoption` so agents never commit a local source or credential.

## 3. BFS regression and completeness

- [x] 3.1 Verify all packable manifest packages retain `Platform.*` IDs and exact versions.
- [x] 3.2 Verify migration fixtures for central, inline, already-conforming and ambiguous-version consumers.
- [x] 3.3 Verify local-source setup changes only the isolated user config fixture.
- [x] 3.4 Re-audit Docker, GitHub Actions and clean-machine restore paths for credentials and absolute paths.

## 4. Verification

- [x] 4.1 Run focused migration, feed and conformance tests.
- [x] 4.2 Run repository quality, API, audit and package-conformance gates.
- [x] 4.3 Run OpenSpec name preflight, strict validation and `git diff --check`.
- [x] 4.4 Archive the change, commit implementation/spec promotion, update HANDOFF in a second commit, and stop without pushing.

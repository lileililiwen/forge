# Push Log — forge

Reviewable record of commits pushed to `origin/main` for CI-alignment /
standalone-CI work, so they can be reviewed later without re-reading git
history. Newest first within each dated section.

## 2026-10-15 — provision driftwatchdog (gate runtime) in CI via release install script

**Decision (user):** install `driftwatchdog` in CI so the `test`/`gate`
(and `surfaces`/`readiness`, which also invoke `forge check`/`forge gate`)
jobs stop failing for a missing gate runtime. Use the project's own
installer from the GitHub release:
`curl -fsSL https://github.com/lileililiwen/driftwatchdog/releases/latest/download/install.sh | sh`
(adds `$HOME/.local/bin` to `GITHUB_PATH`).

**driftwatchdog release prep (sibling repo `lileililiwen/driftwatchdog`):**
The install one-liner only works if a *release* exists AND `install.sh`
is a release asset. Before this, the repo had tags `v0.1.0`–`v0.1.10` but
**no published releases**, and `release.yml` never uploaded `install.sh`.
Three release.yml bugs were fixed and `Cargo.toml` bumped to `0.1.11`
(matches `auto-tag`'s computed next version and the `tag == Cargo` gate):
- artifact suffix `driftwatchdog-linux-x86_64` → `linux-x86_64` (so the
  published archive name matches what `install.sh` resolves); same for
  arm64/darwin.
- `Verify stage contents` SIGPIPE (`tar | head` under `pipefail`) → `tar | sed -n '1p'`.
- `install.sh` added to the release `files:` so the one-liner resolves.
- `npm-publish` `if` reverted to the parser-safe `startsWith(github.ref,'refs/tags/v')`
  (`secrets` context is illegal in a job-level `if`).

Tag `v0.1.11` is being published by `release.yml` (run pending); once
`latest` resolves there, forge CI will install from it.

## 2026-10-15 — codex decoupling + local-standalone CI (this session)

| Commit | Title | Local verification | Why / what changed |
|--------|-------|--------------------|--------------------|
| `6663389` | Decouple forge agent from the codex dev tool at runtime | `verify-local.sh` exit 0: fmt ✔, clippy `-D warnings` ✔, 1232 tests ✔, gate ✔ | `codex` is a dev tool, not a runtime requirement. `start_or_resume` (`src/agent/transitions.rs`) used to return `agent-unavailable` when the binary was absent from PATH, so `forge agent start --provider codex` (and `new-session`/`resume`) failed on any machine without the dev tool — incl. the bare CI runner. Now it always reports the honest `unsupported` state and records `present on PATH` / `absent on PATH` as availability evidence. Fixes the remote `test` job. Tests updated (`agent_tests.rs`, `agent_contract.rs`) so the agent suite no longer needs `codex`. |
| `fcd0764` | Align remote CI with the local rehearsal; correct the MSRV floor | harness exit 0 | Declared `rust-version` and `msrv` job toolchain bumped to 1.88 (resolved deps need rustc 1.88). Added `--test-threads=1` to remote `test` job to match the local harness. |
| `b8b509c` | Fix ci.yml YAML parse error (unquoted colon in msrv job name) | — | The unquoted `binding: ` in the `msrv` job `name:` made the whole workflow fail to parse, so remote CI never ran. Quoting fixed it. |
| `7be7c90` | Make local CI standalone and green: verify-local.sh + decoupled gate | harness exit 0 | Decoupled the gate from sibling/absolute machine paths; vendored governance scripts into forge; added fail-fast guards for a missing gate runtime. |

### Remote run status (run `38724589576`, push of `6663389`)
Jobs reported (run still finishing `native-scaffold`):
- pass: fmt, clippy, msrv, deny, artifact
- fail: **test**, contract-parity, readiness, gate, surfaces, governance

#### Reproducing the `test` job locally (faithful bare-runner mirror)
Ran the exact fast-suite command with a PATH that drops every dev tool /
runtime the bare runner lacks (codex, opencode, ariadex, sisyphusfy,
dotnet, flutter, driftwatchdog). Result: **only `checker_contract` fails**
(`forge check` tests need `driftwatchdog`). Everything else passes.

- `codex`: FIXED by `6663389` (agent suite no longer needs it).
- `dotnet`/`flutter` scaffold tests: already self-skip when the SDK is
  absent (guarded), so they pass on the runner.
- `web_assurance_browser_contract`: static contract test, no browser;
  passes when `driftwatchdog` is absent (accepts an "honest gap" 200).
- `checker_contract`: runs `forge check`, which needs `driftwatchdog`
  (forge's own first-party gate runtime). On a bare runner it errors →
  the `test` job is red for the SAME missing runtime that fails `gate`.

**Conclusion:** the `test` job's remaining failure is `driftwatchdog`, the
same root cause as the `gate` job. `driftwatchdog` is forge's own gate
engine (the same runtime `verify-local.sh`'s gate step uses locally), so
the consistent fix is to provision it in CI — not to stub the tests.

Pre-existing remote failures (unrelated to these commits): `governance`,
`surfaces`, `readiness`, `contract-parity` (`platform-contracts` sibling
absent → fails fast by design). `gate` and `test` both reduce to the
missing `driftwatchdog` runtime.

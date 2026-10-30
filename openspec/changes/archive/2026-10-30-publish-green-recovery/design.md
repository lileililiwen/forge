## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The packaging extractor | `forge` (vendored driftwatchdog) | Yes |
| Readiness fixture rendering | `forge` | Yes |
| The artifact job's path check | `forge` | Yes |
| The NuGet cache and the kit bytes | environment / kit | No |

## Defect 1: The Invalid Regex

The extractor was supposed to capture YAML block scalars:

```python
pattern = re.compile(
    r"^[ \t-]*run:[ \t]*\n([ \t]+.+(?:\n\1[ \t]+.*|\n[ \t]*)*)",
    re.MULTILINE,
)
```

`\1` refers to group 1 while group 1 is still open. CPython 3.12 raises
`cannot refer to an open group`; the test only reaches the block when
`shellcheck` exists, so a developer host without it never compiled the regex.

The replacement walks lines: match a `run: |`/`>-` key, take every following
line indented more than the key, then dedent by the smallest non-empty indent.
This is what the regex was trying to express, and it is deterministic. The
current template yields five scripts, all `sh -n` clean.

## Defect 2: The Missing Feed

`render_files` returns `(relative path, text)`. The binary feed is a separate
read because a `.nupkg` is a ZIP and cannot travel as a `String`. `generate`
already reads it; readiness did not.

`render_feed_files` mirrors the kit resolution and returns
`(relative path, bytes)`. Readiness writes both lists through the same
create-parent-and-write loop. The new test asserts an `aspnet-web` fixture
carries a non-empty `packages/platform-feed/*.nupkg`; the real
`forge readiness check --profile aspnet-web` still passes.

Why local verification missed it: NuGet resolves a package from the global
cache before it needs the configured local source, so a fixture that omits the
feed still restores on a warm host. The regression test checks the fixture
bytes, not the restore outcome, so it does not depend on cache state.

## Defect 3: The Archive Shape

`scripts/package.sh` stages:

```text
forge-<version>-<target>/
  forge
  LICENSE
  README.md
  CHANGELOG.md
  share/forge/contracts/**
  share/forge/runtime-templates/**
  share/forge/governance/**
```

The check allowed `^forge-[^/]*/[^/]*$` only. The correct predicate is "root
files plus the share tree": `^forge-[^/]*/([^/]*|share/|share/forge(/.*)?)$`.
A synthetic listing confirms the valid entries pass and `…/evil/../x` still
fails.

## Defect 4: The Validator Was Not Installed

`platform-dotnet-quality` runs `modules/dotnet-platform-libs/scripts/quality-gate.sh`,
which calls `openspec` for change-name validation. The job never installed it,
so it failed with exit 127 after every .NET test had passed. The job now sets
up Node and installs `@fission-ai/openspec@1.6.0`, the same pinned version the
main `governance` job uses.

## Defect 5: The Relocated Target Directory

`source_root()` walks `current_exe()` ancestors looking for `Cargo.toml` and
`modules/`. A build cache that symlinks `target/` into a shared directory
breaks that walk, and `forge module list` correctly refuses with
`module-catalog-unavailable` — the installed-binary path is meant to refuse.
On a developer host, the same checkout is compiled into the binary, so the
fallback to `CARGO_MANIFEST_DIR` recovers the source checkout while a binary
moved to another host still refuses (that path does not exist there).

The concern runner and the runtime-template resolver had the same assumption in
a second form: both used the source resource only when
`executable.starts_with(CARGO_MANIFEST_DIR)`. Under the cache, that test fails;
the concern runner looked for `<cache>/share/forge/governance/check_concern.py`
and the template resolver refused the checkout's bundle. Both now prefer the
source resource whenever it exists, which is the same rule the installed
prefix already followed.

## Failure Boundaries

- The extractor test is the packaging suite itself; a malformed block yields a
  shellcheck error or a missing script.
- The readiness test fails if the feed resolver returns nothing.
- The artifact check still rejects nested paths outside `share/forge`.

## Verification Strategy

1. `cargo test -p forge --lib readiness::` passes (including the new test).
2. `forge readiness check --profile aspnet-web` passes.
3. The packaging suite runs with and without shellcheck: without it the branch
   is skipped; with it the extractor is exercised. The extractor was also run
   standalone on the current template and produced five `sh -n`-clean scripts.
4. The artifact regex is tested against a synthetic listing.
5. `cargo test -p forge --lib module_catalog` proves the compile-time fallback,
   and `forge module list` returns the in-tree modules.
5. The repository suite, Gate, formatting and OpenSpec validation pass.

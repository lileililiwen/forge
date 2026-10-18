# Project Agent Instructions

- Read `HANDOFF.md` before work.
- Use OpenSpec for non-trivial changes.
- Use BFS → DFS → BFS for impact, implementation, and regression review.
- Run the declared verification command before completion.
- Work directly on `main` by default; do not create a branch or worktree unless the user asks for one or designates one. Preserve unrelated uncommitted work and never sweep it into a commit.
- `main` is the only development branch. There is one owner and one agent at a time, so there is no concurrent-workload reason to branch; a branch only pollutes the commit line, invites conflicts, and gets forgotten unmerged. Do not open one "just to be safe". When a branch does exist because the owner designated it, the work is not finished until `main` carries it and the branch is deleted.
- Do not use `rm` to delete anything; on this host it is a recoverable-delete shim that intercepts the call and can fail or leave the path behind. Clean up scratch work from Python: `tempfile.TemporaryDirectory()` as a context manager, or `shutil.rmtree(path, ignore_errors=True)`. Neither prompts.

## Project facts

- Stack: framework, language, versions — read from the manifest, kept current.
- Toolchain: the package manager/build tool this repo uses; run its commands, never a competing tool's.
- Verify: the declared verification command (`.project.json`) that must pass before completion.
- Layout: first-level source directories, one line each with purpose and naming convention.

Any directory, tool, or command change updates this block in the same commit —
an outdated facts block misleads every agent more than a missing one.

## Claims and evidence

- Build/test/skeleton success is not runtime or release evidence.
- A README claim is a factual assertion and is checked against the file tree. Do not write a claim you cannot evidence; state what is true instead. Silence is honest, an unverifiable claim is not.
- A checker that reports OK without performing a comparison is worse than no checker, because it retires the question while answering nothing. A verifier that no gate runs does not gate: wire every check into the gate that blocks, or it is documentation.
- A check that can be switched off is not a check. Do not add an opt-out, an exemption list, or an environment-variable escape to a check that is meant to hold. A checker that depends on an environment variable being set fails on any runner that lacks it — prefer a committed, repo-relative input.

## Shared infrastructure

- Assess shared infrastructure before duplication; see `docs/adoption-grant.md` and `docs/capabilities.md` in the governance repository. Assess a named sibling or a published package for an existing capability before writing a local implementation, and name the specific sibling, project id, or package you assessed — an un-named assessment is not an assessment.
- Do not edit a sibling repository without scope authorization. This restriction governs *source* changes to another repository. It does not govern *consuming* a sibling's published artifact: a pinned, checksum-verified package coordinate, its registration in this repository's own composition root, and this repository's own tests are consumer-side changes. Under a valid `adoption_grant` in this repository's `.project.json`, replacing a local implementation with a registered shared provider is authorized without separate cross-repo approval, and must be recorded in this repository's `capabilities[].consumes[]` and in `adoption/adoptions.json`. A source-level coupling — a `ProjectReference`, a relative path, or a `../` dependency on a sibling checkout — remains forbidden regardless of any grant, as does bumping a provider's version.
- Keep the capability local when ownership, privacy, lifecycle, or dependency direction makes that the correct boundary. Record that decision as a `declines[]` entry in `.project.json` with a rationale from the closed vocabulary and an evidence reference that resolves in this repository; prose in a document is not a decline.
- Consuming a shared package must not drag in a dependency you do not need. If a shared package would impose a version floor or an ORM/provider reference on this repository, that is a defect in the package, not a cost you should absorb — report it rather than working around it.

## Commit protocol

- Keep at most one active OpenSpec change; complete and archive it before creating another.
- Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit. Stop after the second commit; do not push unless asked.

Detailed rules: `.ai-rules/workflow.md` and `.ai-rules/completion.md`.

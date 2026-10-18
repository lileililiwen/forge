# Documentation Capture Plan

## Surface

Workspace Governance is a CLI and documentation repository. It has no
human-facing application route, so a browser screenshot is not applicable.
The self-check command is the representative operator surface.

## Capture

- Source: repository root, `scripts/workspace_check.py --self-check`.
- Command: `python3 scripts/workspace_check.py --self-check`.
- Viewport, selector, and wait settings: not applicable to CLI output.
- Artifact: `docs/assets/terminal-self-check.txt`.
- Source revision: `b05e5d4` (`enforce sequential OpenSpec changes` and its
  HANDOFF-only completion commit); captured 2026-10-12 UTC.
- Privacy review: output contains only the aggregate self-check result; no
  project names, user data, credentials, or machine-specific paths are included.

Refresh this capture and its source revision after a change to the command,
checker output, or operator workflow.

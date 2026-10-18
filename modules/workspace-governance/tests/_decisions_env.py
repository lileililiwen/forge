"""Shared test helper: stage a per-test `decisions.json` for the audit.

The `load_decisions_register` audit hook in `workspace_check.py` honours the
`WORKSPACE_DECISIONS_PATH` env var so tests can isolate the audit from the
governance root's real register. Without this, the real register is loaded
against the test's tiny `projects.json` and the cross-reference check fails
because the test registry does not list the register's owner/downstream
ids.

`use_empty_decisions_register()` writes a valid empty register to a temp
file and points the env var at it. Pair with `restore_decisions_register()`
in `tearDownModule` (or a `try` / `finally`) so production runs are
untouched.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path
from typing import IO


_TEMP_FILES: list[IO[str]] = []


def use_empty_decisions_register() -> Path:
    """Stage a valid empty register and point the env var at it. Returns the temp file path."""
    fd, path = tempfile.mkstemp(prefix="decisions-empty-", suffix=".json")
    os.close(fd)
    Path(path).write_text(json.dumps({"version": 1, "decisions": []}), encoding="utf-8")
    os.environ["WORKSPACE_DECISIONS_PATH"] = path
    return Path(path)


def use_decisions_register(payload: dict) -> Path:
    """Stage a custom register payload (already a dict) and point the env var at it."""
    fd, path = tempfile.mkstemp(prefix="decisions-", suffix=".json")
    os.close(fd)
    Path(path).write_text(json.dumps(payload), encoding="utf-8")
    os.environ["WORKSPACE_DECISIONS_PATH"] = path
    return Path(path)


def restore_decisions_register() -> None:
    os.environ.pop("WORKSPACE_DECISIONS_PATH", None)

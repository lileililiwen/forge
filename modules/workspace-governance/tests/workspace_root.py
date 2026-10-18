"""Resolve the workspace inspected by fleet-level regression tests."""

from __future__ import annotations

import os
from pathlib import Path


def resolve_workspace_root(governance_root: Path) -> Path:
    """Honor the operator root, else support standalone and Forge module layouts."""
    configured = os.environ.get("WORKSPACE_ROOT")
    if configured:
        return Path(configured).resolve()
    if governance_root.parent.name == "modules" and (governance_root.parent.parent / ".git").exists():
        return governance_root.parent.parent.parent.resolve()
    return governance_root.parent.resolve()

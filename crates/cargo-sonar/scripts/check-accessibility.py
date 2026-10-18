#!/usr/bin/env python3
"""Run Forge's accessibility concern check for this project."""
import subprocess
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
raise SystemExit(subprocess.run(["forge", "concern", "check", "accessibility", str(PROJECT_ROOT)]).returncode)

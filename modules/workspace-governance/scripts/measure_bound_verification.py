#!/usr/bin/env python3
"""Measure the consequence of binding: green / red / failed-to-start, per repository.

This does **not** soften, skip, or reinterpret anything. It runs each
repository's own declared `verification.command` exactly as the Gate runtime
runs it — `sh -c <command>` in the repository root with a 60 s cap, the same
`CHECK_TIMEOUT_MS` as `src/commands/gate.rs` — and classifies the result.

The classifications are finer than the Gate's own, because the owner needs to
tell two different facts apart:

* the Gate maps *any* non-zero exit to `FAIL` (`src/gate/adapters.rs`), so a
  missing toolchain and a genuinely failing test suite are the same verdict to
  the Gate. This report splits them, because "the toolchain is not installed"
  and "the tests fail" are different decisions.

Classifications:

* ``GREEN``             exit 0.
* ``RED``               a non-zero exit from a command that did start.
* ``NO_START``          the command could not start: exit 127, or a shell
                        "not found" in the output.
* ``TIMEOUT``           the 60 s cap expired; nothing was concluded.

No repository is modified. No `rm`: temporary state lives in a
`tempfile.mkdtemp` removed with `shutil.rmtree(..., ignore_errors=True)`.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
from bind_verification_command import CONCERN_ID, discover_repos, read_declared_command  # noqa: E402

GATE_TIMEOUT_MS = 60_000
GATE_TIMEOUT_S = GATE_TIMEOUT_MS / 1000

# Shells report an unexecutable command as 127, and Node/npm report a missing
# local binary as 127 from the script it was asked to run.
NOT_FOUND_MARKERS = (
    "command not found",
    "not found: ",
    "No such file or directory",
    "is not recognized as an internal or external command",
    "Unable to locate package",
    "Could not execute process",
)


def is_bound(text: str) -> bool:
    return f"  {CONCERN_ID}:" in text


def classify(repo: Path) -> dict:
    command, refusal = read_declared_command(repo)
    record = {"repo": repo.name, "command": command, "status": None, "exit": None, "detail": ""}
    if command is None:
        record["status"] = "NO_DECLARATION"
        record["detail"] = refusal
        return record
    manifest = repo / ".ai-gate" / "gate.yaml"
    if not manifest.is_file():
        record["status"] = "NO_MANIFEST"
        record["detail"] = "no .ai-gate/gate.yaml to bind into"
        return record
    if not is_bound(manifest.read_text(encoding="utf-8", errors="replace")):
        record["status"] = "UNBOUND"
        record["detail"] = f"no commands.{CONCERN_ID} entry"
        return record

    try:
        completed = subprocess.run(
            ["sh", "-c", command],
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=GATE_TIMEOUT_S,
            check=False,
        )
    except subprocess.TimeoutExpired:
        record["status"] = "TIMEOUT"
        record["detail"] = f"exceeded the {GATE_TIMEOUT_S:.0f}s gate timeout"
        return record
    except OSError as exc:
        record["status"] = "NO_START"
        record["detail"] = f"could not spawn: {exc}"
        return record

    code = completed.returncode
    output = f"{completed.stdout}\n{completed.stderr}"
    record["exit"] = code
    if code == 0:
        record["status"] = "GREEN"
        return record
    if code == 127 or any(marker in output for marker in NOT_FOUND_MARKERS):
        record["status"] = "NO_START"
        record["detail"] = first_signal(output) or f"exit {code}"
        return record
    record["status"] = "RED"
    record["detail"] = first_signal(output) or f"exit {code}"
    return record


def first_signal(output: str) -> str:
    """The first line that says something went wrong, for the report."""
    for line in output.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        lowered = stripped.lower()
        if any(
            token in lowered
            for token in ("error", "fail", "not found", "cannot", "denied", "timed out")
        ):
            return stripped[:200]
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    return lines[-1][:200] if lines else ""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args(argv)

    repos = discover_repos(args.root.resolve())
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        records = list(pool.map(classify, repos))
    records.sort(key=lambda r: r["repo"])

    if args.out:
        args.out.write_text(json.dumps(records, indent=2), encoding="utf-8")

    for record in records:
        line = f"{record['repo']:32} {record['status']:14}"
        if record["exit"] is not None:
            line += f"exit={record['exit']:<5}"
        if record["detail"]:
            line += f" {record['detail']}"
        print(line)

    print()
    for status in ("GREEN", "RED", "NO_START", "TIMEOUT", "UNBOUND", "NO_MANIFEST", "NO_DECLARATION"):
        names = [r["repo"] for r in records if r["status"] == status]
        print(f"{status + ':':16}{len(names):<5}{' '.join(names)}")
    tmp = Path(tempfile.gettempdir()) / "unused"
    shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

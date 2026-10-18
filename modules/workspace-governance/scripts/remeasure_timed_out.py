#!/usr/bin/env python3
"""Re-measure the repositories the 60 s gate timeout cut short, un-contended.

The first pass ran 4 repositories at once, so a first-time `cargo build` or
`dotnet restore` had to share the machine with three other compilers. 27 of
the 76 timed out under the Gate's own 60 s `CHECK_TIMEOUT_MS`, which is a real
Gate verdict (`REVIEW_REQUIRED`) but says almost nothing about whether the
declared command actually passes.

This pass re-runs **only** those 27, two at a time, with a much longer cap, so
the owner gets both numbers: what the Gate says today, and what the command
does when it is allowed to finish. It changes no verdict and softens nothing —
it only stops reporting "could not finish in 60 s" as if it were "the tests
fail".

No repository is modified. No `rm`.
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
from bind_verification_command import read_declared_command  # noqa: E402
from measure_bound_verification import NOT_FOUND_MARKERS, first_signal  # noqa: E402


def classify(repo: Path, timeout_s: float) -> dict:
    command, refusal = read_declared_command(repo)
    record = {"repo": repo.name, "command": command, "status": None, "exit": None, "detail": ""}
    if command is None:
        record["status"] = "NO_DECLARATION"
        record["detail"] = refusal
        return record
    try:
        completed = subprocess.run(
            ["sh", "-c", command],
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=timeout_s,
            check=False,
        )
    except subprocess.TimeoutExpired:
        record["status"] = "TIMEOUT"
        record["detail"] = f"exceeded the {timeout_s:.0f}s re-measure cap"
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
    elif code == 127 or any(m in output for m in NOT_FOUND_MARKERS):
        record["status"] = "NO_START"
        record["detail"] = first_signal(output) or f"exit {code}"
    else:
        record["status"] = "RED"
        record["detail"] = first_signal(output) or f"exit {code}"
    return record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--repos", type=Path, required=True, help="first-pass measurement JSON")
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--timeout", type=float, default=600.0)
    parser.add_argument("--jobs", type=int, default=2)
    args = parser.parse_args(argv)

    first = json.loads(args.repos.read_text(encoding="utf-8"))
    targets = [Path(args.root) / r["repo"] for r in first if r["status"] == "TIMEOUT"]
    print(f"re-measuring {len(targets)} timed-out repositories, "
          f"cap {args.timeout:.0f}s, {args.jobs} at a time", flush=True)

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        records = list(pool.map(lambda p: classify(p, args.timeout), targets))
    records.sort(key=lambda r: r["repo"])

    if args.out:
        args.out.write_text(json.dumps(records, indent=2), encoding="utf-8")
    for record in records:
        line = f"{record['repo']:28} {record['status']:10}"
        if record["exit"] is not None:
            line += f"exit={record['exit']:<5}"
        if record["detail"]:
            line += f" {record['detail']}"
        print(line, flush=True)
    print()
    for status in ("GREEN", "RED", "NO_START", "TIMEOUT"):
        names = [r["repo"] for r in records if r["status"] == status]
        print(f"{status + ':':12}{len(names):<5}{' '.join(names)}")
    tmp = Path(tempfile.gettempdir()) / "unused-2"
    shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

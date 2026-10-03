#!/usr/bin/env python3
"""Bound each saved-capture strategy independently; no native shell/capture work."""
import json
import pathlib
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "structdiff-size-results"
SIZES = ["80x24", "120x40", "160x50", "80x50", "160x25"]
BINARY = ROOT / "target/release/ship-jsonpatch-prototype"
records = []


def run(size, strategy, timeout):
    folder = OUT / size / strategy
    folder.mkdir(parents=True, exist_ok=True)
    command = [str(BINARY), "--saved-size", size, strategy]
    start = time.monotonic()
    timed_out = False
    with (folder / "run-output.txt").open("w") as log:
        try:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                    timeout=timeout, check=False)
            code = result.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            code = None
    record = dict(size=size, strategy=strategy, command=command, timeout_seconds=timeout,
                  elapsed_seconds=time.monotonic() - start, exit_code=code, timed_out=timed_out)
    (folder / "run-meta.json").write_text(json.dumps(record, indent=2) + "\n")
    records.append(record)
    (OUT / "run-meta.json").write_text(json.dumps(records, indent=2) + "\n")
    print(json.dumps(record), flush=True)
    return not timed_out and code == 0


# Exactly one diff per size before any ordered sampling. No percentile claim.
probed = {size: run(size, "ordered-probe", 10) for size in SIZES}
for size in SIZES:
    run(size, "json", 45)
    run(size, "default", 15)
    if probed[size]:
        run(size, "ordered", 60)
    else:
        records.append(dict(size=size, strategy="ordered", skipped=True,
                            reason="single-diff probe failed or exceeded 10 seconds"))
        (OUT / "run-meta.json").write_text(json.dumps(records, indent=2) + "\n")

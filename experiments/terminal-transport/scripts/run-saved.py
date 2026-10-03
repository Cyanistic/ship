#!/usr/bin/env python3
"""Reuse preserved captures in a fresh output directory, without spawning PTYs."""
import argparse
import datetime
import json
import os
import pathlib
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
SIZES = ["80x24", "120x40", "160x50", "80x50", "160x25"]
MODES = ["compression", "json", "default", "ordered", "ordered-probe"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", choices=SIZES, default="160x50")
    parser.add_argument("--mode", choices=MODES, default="compression")
    args = parser.parse_args()
    binary = ROOT / "target/release/ship-jsonpatch-prototype"
    if not binary.is_file():
        parser.error("build first: cargo build --release --locked --target-dir target")

    runs = ROOT / ".runs"
    runs.mkdir(exist_ok=True)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = pathlib.Path(tempfile.mkdtemp(prefix=f"{stamp}-{args.size}-{args.mode}-", dir=runs))
    # The unchanged harness resolves inputs and outputs against its working directory.
    # Only inputs point back to evidence; all outputs stay in this new directory.
    (out / "size-results").symlink_to(os.path.relpath(ROOT / "size-results", out), target_is_directory=True)
    command = [str(binary)]
    if args.mode == "compression":
        command += ["--compression", args.size]
    else:
        command += ["--saved-size", args.size, args.mode]
    timeout = {"compression": 90, "json": 45, "default": 15, "ordered": 60, "ordered-probe": 10}[args.mode]
    start = time.monotonic()
    code = None
    timed_out = False
    print(f"Output: {out}", flush=True)
    with (out / "run-output.txt").open("w") as log:
        try:
            code = subprocess.run(command, cwd=out, stdout=log, stderr=subprocess.STDOUT,
                                  timeout=timeout, check=False).returncode
        except subprocess.TimeoutExpired:
            timed_out = True
    record = dict(size=args.size, mode=args.mode, command=command, working_directory=str(out),
                  input=str(ROOT / "size-results" / args.size / "captures.json"),
                  timeout_seconds=timeout, elapsed_seconds=time.monotonic() - start,
                  exit_code=code, timed_out=timed_out)
    (out / "run-meta.json").write_text(json.dumps(record, indent=2) + "\n")
    print((out / "run-output.txt").read_text(), end="")
    print(json.dumps(record, indent=2))
    return 1 if timed_out else code


if __name__ == "__main__":
    raise SystemExit(main())

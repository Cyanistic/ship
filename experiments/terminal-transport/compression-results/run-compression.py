#!/usr/bin/env python3
"""Run only saved captures. Bounded release processes; never overwrite prior results."""
import json, pathlib, subprocess, time
ROOT = pathlib.Path(__file__).resolve().parent.parent
runs = []
for size in ['80x24', '120x40', '160x50', '80x50', '160x25']:
    out = ROOT / 'compression-results' / size
    out.mkdir(exist_ok=True)
    cmd = [str(ROOT / 'target/release/ship-jsonpatch-prototype'), '--compression', size]
    start = time.monotonic()
    with (out / 'run-output.txt').open('w') as log:
        try:
            result = subprocess.run(cmd, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=90)
            meta = dict(size=size, command=cmd, exit_code=result.returncode, timed_out=False)
        except subprocess.TimeoutExpired:
            meta = dict(size=size, command=cmd, exit_code=None, timed_out=True)
    meta.update(elapsed_seconds=time.monotonic()-start, timeout_seconds=90)
    (out / 'run-meta.json').write_text(json.dumps(meta, indent=2)+'\n')
    runs.append(meta)
    (ROOT / 'compression-results/run-meta.json').write_text(json.dumps(runs, indent=2)+'\n')
    print(meta, flush=True)
    if meta['exit_code'] != 0:
        raise SystemExit(1)

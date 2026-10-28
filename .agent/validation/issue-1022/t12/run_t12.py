"""#1022 T12: the sustained runs in Auto against fixed native and fixed minimum quality.
Runs the release binary on each route in turn and samples its working set every 10 s
from outside. Writes each run's folder, <name>.stdout.txt and memory.txt. Run from the
repo root."""
import subprocess
import sys
import time
from pathlib import Path

EXE = Path("target/release/universe-app.exe")
D = Path(".agent/validation/issue-1022/t12")

RUNS = [(f"r4-{v}", D / f"r4-{v}.route") for v in ("auto", "native", "min")]
for i in range(1, 4):
    for v in ("auto", "native", "min"):
        RUNS.append((f"short-{v}-{i}", D / f"short-{v}.route"))


def working_set_kb(pid):
    out = subprocess.run(["tasklist", "/fi", f"PID eq {pid}", "/fo", "csv", "/nh"],
                         capture_output=True, text=True).stdout
    try:
        field = out.strip().split('","')[-1].strip('"')
        return int("".join(ch for ch in field if ch.isdigit()))
    except ValueError:
        return None


only = set(sys.argv[1:])
for name, route in RUNS:
    if only and name not in only:
        continue
    with open(D / f"{name}.stdout.txt", "w") as log:
        proc = subprocess.Popen([str(EXE), "--route", str(route), str(D / name)],
                                stdout=log, stderr=subprocess.STDOUT)
        peak = 0
        while proc.poll() is None:
            kb = working_set_kb(proc.pid)
            if kb:
                peak = max(peak, kb)
            time.sleep(10)
    line = f"{name} exit={proc.returncode} peak_working_set_mb={peak / 1024:.0f} (sampled every 10 s)"
    print(line, flush=True)
    with open(D / "memory.txt", "a") as m:
        m.write(line + "\n")

"""#1022 T3 baselines: runs the release binary on the route and shot files in turn and
samples each run's working set from outside (the game reads no OS memory counters,
ADR 0053). Writes every run's output folder plus memory.txt. Run from the repo root."""
import subprocess
import sys
import time
from pathlib import Path

EXE = Path("target/release/universe-app.exe")
D = Path(".agent/validation/issue-1022/t3")
ROUTES = Path(".agent/validation/issue-1022")

RUNS = [
    ("r4-normal", ["--route", str(ROUTES / "routes/r4.route"), str(D / "r4-normal")]),
    ("r4-min", ["--route", str(D / "r4-min.route"), str(D / "r4-min")]),
]
for i in range(1, 4):
    RUNS.append((f"short-normal-{i}", ["--route", str(ROUTES / "t2/short.route"), str(D / f"short-normal-{i}")]))
    RUNS.append((f"short-min-{i}", ["--route", str(D / "short-min.route"), str(D / f"short-min-{i}")]))
RUNS.append(("cold", ["--screens", str(D / "cold"), "--shots", str(D / "cold.shots"), "--timing"]))


def working_set_kb(pid):
    out = subprocess.run(
        ["tasklist", "/fi", f"PID eq {pid}", "/fo", "csv", "/nh"],
        capture_output=True, text=True,
    ).stdout
    try:
        field = out.strip().split('","')[-1].strip('"')
        return int("".join(ch for ch in field if ch.isdigit()))
    except ValueError:
        return None


only = set(sys.argv[1:])
memory = []
for name, args in RUNS:
    if only and name not in only:
        continue
    log = open(D / f"{name}.stdout.txt", "w")
    proc = subprocess.Popen([str(EXE), *args], stdout=log, stderr=subprocess.DEVNULL)
    peak = 0
    while proc.poll() is None:
        kb = working_set_kb(proc.pid)
        if kb:
            peak = max(peak, kb)
        time.sleep(10)
    log.close()
    memory.append(f"{name} exit={proc.returncode} peak_working_set_mb={peak / 1024:.0f} (sampled every 10 s)")
    print(memory[-1], flush=True)
(D / "memory.txt").open("a").write("\n".join(memory) + "\n")

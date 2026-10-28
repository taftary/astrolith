"""#1022 T9: R2 readiness on the 20 cold placements, before (T3) and after (T9).

Reads the TIMING lines named cold-NN of each timing.txt (the warm-up is left
out), takes the nearest-rank p95 of ring_ms and ready_ms, and checks R2:
p95 ring within 3 s, p95 ready within 5 s, and each p95 no worse than the
baseline by more than the larger of 20 % or 250 ms. Run from the repo root.
"""
import math
from pathlib import Path

ROOT = Path(".agent/validation/issue-1022")
BEFORE = ROOT / "t3/cold/timing.txt"
AFTER = ROOT / "t9/cold/timing.txt"


def values(path, key):
    out = []
    for line in path.read_text().splitlines():
        parts = line.split()
        if len(parts) < 2 or parts[0] != "TIMING" or not parts[1].startswith("cold-"):
            continue
        fields = dict(p.split("=", 1) for p in parts[2:] if "=" in p)
        v = fields.get(key, "none")
        out.append(math.inf if v == "none" else float(v))
    return out


def p95(xs):
    xs = sorted(xs)
    rank = max(math.ceil(len(xs) * 95 / 100), 1) - 1
    return xs[rank]


lines = []
for key, limit in (("ring_ms", 3000.0), ("ready_ms", 5000.0)):
    before, after = values(BEFORE, key), values(AFTER, key)
    b, a = p95(before), p95(after)
    allowed = b + max(0.2 * b, 250.0)
    lines.append(
        f"{key} placements={len(after)} p95_before={b:.0f} p95_after={a:.0f} "
        f"limit={limit:.0f} within_limit={'yes' if a <= limit else 'no'} "
        f"allowed_vs_baseline={allowed:.0f} within_baseline={'yes' if a <= allowed else 'no'}"
    )
text = "\n".join(lines) + "\n"
(ROOT / "t9/r2.txt").write_text(text)
print(text, end="")

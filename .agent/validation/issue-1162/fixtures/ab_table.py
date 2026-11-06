"""#1162 T7: one table per ABBA set (before, after, after, before), from raw frames.

Usage (repo root): py .agent/validation/issue-1162/fixtures/ab_table.py <set dir> [<set dir> ...]
Interval medians per scenario from each run's route/frames.txt (fixtures/compare.py);
change = mean of the two after runs / mean of the two before runs - 1.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import compare as c  # noqa: E402


def table(d):
    runs = sorted(Path(d).glob("[0-9]*-*/"), key=lambda p: int(p.name.split("-")[0]))
    st = {r.name: c.scenario_stats(r) for r in runs}
    al = {r.name: c.all_stats(r) for r in runs}
    out = [f"### `{Path(d).name}`", "", "| scenario | " + " | ".join(r.name for r in runs) + " | change | >100 ms |",
           "|---|" + "---|" * len(runs) + "---|---|"]
    for n in list(st[runs[0].name]) + ["all"]:
        g = (lambda r: al[r]) if n == "all" else (lambda r: st[r][n])
        b = [g(r.name)["interval"] for r in runs if "before" in r.name]
        a = [g(r.name)["interval"] for r in runs if "after" in r.name]
        ch = (sum(a) / len(a)) / (sum(b) / len(b)) - 1
        out.append(f"| {n} | " + " | ".join(f"{g(r.name)['interval']:.2f}" for r in runs)
                   + f" | {ch * 100:+.1f}% | " + "/".join(str(g(r.name)["over"]) for r in runs) + " |")
    return "\n".join(out)


if __name__ == "__main__":
    print("\n\n".join(table(d) for d in sys.argv[1:]))

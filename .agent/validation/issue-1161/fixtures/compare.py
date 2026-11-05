"""#1161 T8: before/after comparison from the raw run files.

Usage (from the repo root):
  py .agent/validation/issue-1161/fixtures/compare.py
Writes after/comparison.md (the four batch routes, before vs after) and
paired/paired.md (the alternating repeats). Medians come from each run's
raw route/frames.txt (columns: time, scenario, interval, cpu work, main,
render, wait). Auto settings come from route/telemetry-v1.csv, memory from
memory.jsonl. Numbers are copied by this script, not typed by hand.
"""
import csv
import json
import statistics
from pathlib import Path

D = Path(".agent/validation/issue-1161")
ROUTES = ["surfaces-fixed", "surfaces-terrain-low", "surfaces-terrain-high", "surfaces-auto"]
WORLDS = ["home", "mercury", "desert", "iron", "ice", "lava"]
SETTINGS = ["terrain", "markers", "atmosphere", "render_scale"]


def frames(run):
    rows = {}
    for line in (run / "route" / "frames.txt").read_text().splitlines():
        p = line.split()
        if len(p) != 7:
            continue
        rows.setdefault(p[1], []).append([float(v) for v in (p[0], *p[2:])])
    return rows


def med(rows, col):
    return statistics.median(r[col] for r in rows)


def scenario_stats(run):
    out = {}
    for name, rows in frames(run).items():
        if name in ("warmup", "waypoint", "end"):
            continue
        out[name] = {
            "interval": med(rows, 1),
            "cpu": med(rows, 2),
            "render": med(rows, 4),
            "over": sum(r[1] > 100 for r in rows),
        }
    return out


def all_stats(run):
    rows = [r for n, rs in frames(run).items() if n not in ("warmup", "waypoint", "end") for r in rs]
    return {"interval": med(rows, 1), "cpu": med(rows, 2), "render": med(rows, 4), "over": sum(r[1] > 100 for r in rows)}


def ready(run):
    s = json.loads((run / "summary.json").read_text())
    return {r["scenario"]: r.get("ready_ms") for r in s.get("readiness", [])}


def settled(run):
    """The settings at the end of each telemetry leg (Auto's choice)."""
    last = {}
    for row in csv.DictReader((run / "route" / "telemetry-v1.csv").open()):
        last[int(row["leg"])] = "/".join(row[k] for k in SETTINGS)
    return last


def world_memory(run):
    """Median working set (MB) while at each world, by the telemetry clock."""
    starts = {}
    for row in csv.DictReader((run / "route" / "telemetry-v1.csv").open()):
        starts.setdefault(int(row["leg"]), float(row["elapsed_s"]))
    mem = [json.loads(line) for line in (run / "memory.jsonl").read_text().splitlines() if line.strip()]
    out = {}
    # Legs: 0 warm, then per world: ground, waypoint (no frames), low, orbit.
    for i, world in enumerate(WORLDS):
        first = 1 + 4 * i
        a = starts[first]
        b = starts.get(first + 4, float("inf"))
        ws = [m["working_set_bytes"] / 1e6 for m in mem if a <= m["elapsed_s"] < b]
        out[world] = statistics.median(ws) if ws else float("nan")
    return out


def pct(a, b):
    return f"{(b / a - 1) * 100:+.1f}%" if a else "n/a"


def comparison():
    out = ["# #1161 T8: before vs after, per scenario", "",
           "Medians from each run's raw `route/frames.txt`, written by `fixtures/compare.py`; change = after/before - 1 (ms). "
           "Before: `before/` (exe 46ee6f4f…, commit 7005cbd). After: `after/` (`after/exe.sha256`, commit in `after/commit.txt`).",
           "Limit (criterion 9): fixed-quality interval median within +5%, no new stalls over 100 ms."]
    for route in ROUTES:
        b, a = D / "before" / route, D / "after" / route
        sb, sa = scenario_stats(b), scenario_stats(a)
        rb, ra = ready(b), ready(a)
        out += ["", f"## {route}", "",
                "| scenario | interval before | after | change | render before | after | cpu before | after | >100 ms before/after | ready ms before/after |",
                "|---|---|---|---|---|---|---|---|---|---|"]
        for name in sb:
            x, y = sb[name], sa.get(name)
            if y is None:
                continue
            out.append(f"| {name} | {x['interval']:.2f} | {y['interval']:.2f} | {pct(x['interval'], y['interval'])} | "
                       f"{x['render']:.2f} | {y['render']:.2f} | {x['cpu']:.2f} | {y['cpu']:.2f} | {x['over']}/{y['over']} | {rb.get(name)}/{ra.get(name)} |")
        x, y = all_stats(b), all_stats(a)
        out.append(f"| **all** | {x['interval']:.2f} | {y['interval']:.2f} | {pct(x['interval'], y['interval'])} | "
                   f"{x['render']:.2f} | {y['render']:.2f} | {x['cpu']:.2f} | {y['cpu']:.2f} | {x['over']}/{y['over']} | warm {rb.get('warmup')}/{ra.get('warmup')} |")
    # Auto's settled settings, leg by leg.
    b, a = settled(D / "before" / "surfaces-auto"), settled(D / "after" / "surfaces-auto")
    out += ["", "## Auto settles (surfaces-auto, end of each leg)", "",
            f"Settings shown as {' / '.join(SETTINGS)}.", "", "| leg | before | after | same |", "|---|---|---|---|"]
    for leg in sorted(set(b) | set(a)):
        out.append(f"| {leg} | {b.get(leg)} | {a.get(leg)} | {'yes' if b.get(leg) == a.get(leg) else 'NO'} |")
    # Memory at each world.
    out += ["", "## Working set at each world (surfaces-fixed, MB median while there)", "",
            "| world | before | after | change |", "|---|---|---|---|"]
    mb, ma = world_memory(D / "before" / "surfaces-fixed"), world_memory(D / "after" / "surfaces-fixed")
    for w in WORLDS:
        out.append(f"| {w} | {mb[w]:.0f} | {ma[w]:.0f} | {pct(mb[w], ma[w])} |")
    (D / "after" / "comparison.md").write_text("\n".join(out) + "\n")


def paired():
    p = D / "paired"
    out = ["# #1161 T8: paired repeats (before build vs after build, alternating)", "",
           "Medians per scenario from raw `route/frames.txt`, written by `fixtures/compare.py`. `exe.sha256` names both builds. "
           "Run order: before r1, after r1, before r2, after r2 (surfaces-fixed), then before and after on surfaces-auto."]
    for route, rounds in (("surfaces-fixed", [1, 2]), ("surfaces-auto", [1])):
        runs = [(f"{side} r{r}", p / f"{side}-{route}-r{r}") for side in ("before", "after") for r in rounds]
        stats = {label: scenario_stats(run) for label, run in runs}
        totals = {label: all_stats(run) for label, run in runs}
        labels = [label for label, _ in runs]
        out += ["", f"## {route}", "", "| scenario | metric | " + " | ".join(labels) + " | mean change |",
                "|---|---|" + "---|" * len(labels) + "---|"]
        names = list(stats[labels[0]])
        for name in names + ["all"]:
            for metric in ("interval", "render", "cpu", "over"):
                vals = [(totals[l] if name == "all" else stats[l][name])[metric] for l in labels]
                if metric == "over":
                    out.append(f"| {name} | >100 ms | " + " | ".join(str(v) for v in vals) + " | |")
                    continue
                n = len(rounds)
                before, after = sum(vals[:n]) / n, sum(vals[n:]) / n
                out.append(f"| {name} | {metric} | " + " | ".join(f"{v:.2f}" for v in vals) + f" | {pct(before, after)} |")
    (p / "paired.md").write_text("\n".join(out) + "\n")


if __name__ == "__main__":
    comparison()
    paired()
    print("COMPARE-OK")

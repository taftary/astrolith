"""#1161: per-scenario frame and memory tables for one side (before or after).

Usage: py .agent/validation/issue-1161/fixtures/tables.py <before|after>
Writes <side>/tables.md from each route's summary.json, resources.json and
memory.jsonl. Numbers are copied from those files, not typed by hand.
"""
import json
import sys
from pathlib import Path

D = Path(".agent/validation/issue-1161")
ROUTES = ["surfaces-fixed", "surfaces-terrain-low", "surfaces-terrain-high", "surfaces-auto"]


def stat(scn, key, field):
    return scn.get(key, {}).get(field, float("nan"))


def main(side):
    out = [f"# #1161 {side}: frame and memory tables", ""]
    out.append("Copied by `fixtures/tables.py` from each route's `summary.json`, `resources.json` and `memory.jsonl` (ms; MB = 10^6 bytes).")
    for route in ROUTES:
        base = D / side / route
        s = json.loads((base / "summary.json").read_text())
        out += ["", f"## {route}", "", "| scenario | interval mean | interval p95 | render mean | cpu mean | over 100 ms | ready ms |", "|---|---|---|---|---|---|---|"]
        ready = {r["scenario"]: r.get("ready_ms") for r in s.get("readiness", [])}
        for name, scn in s["scenarios"].items():
            out.append(
                f"| {name} | {stat(scn, 'interval_ms', 'mean'):.2f} | {stat(scn, 'interval_ms', 'p95'):.2f} | "
                f"{stat(scn, 'render_ms', 'mean'):.2f} | {stat(scn, 'cpu_work_ms', 'mean'):.2f} | {scn.get('over_100ms')} | {ready.get(name)} |"
            )
        a = s["all"]
        out.append(f"| **all** | {a['interval_ms']['mean']:.2f} | {a['interval_ms']['p95']:.2f} | {a['render_ms']['mean']:.2f} | {a['cpu_work_ms']['mean']:.2f} | {a['over_100ms']} | |")
        r = json.loads((base / "resources.json").read_text())["observations"]
        mem = [json.loads(line)["working_set_bytes"] for line in (base / "memory.jsonl").read_text().splitlines() if line.strip()]
        out.append("")
        out.append(
            f"Resources (first / last / peak): meshes {r['meshes']['first']:.0f}/{r['meshes']['last']:.0f}/{r['meshes']['peak']:.0f}, "
            f"images {r['images']['first']:.0f}/{r['images']['last']:.0f}/{r['images']['peak']:.0f}, "
            f"ground materials {r['ground_materials']['first']:.0f}/{r['ground_materials']['last']:.0f}/{r['ground_materials']['peak']:.0f}, "
            f"terrain MB {r['terrain_bytes']['last'] / 1e6:.1f} last / {r['terrain_bytes']['peak'] / 1e6:.1f} peak. "
            f"Working set MB: peak {max(mem) / 1e6:.0f}, last {mem[-1] / 1e6:.0f} ({len(mem)} samples)."
        )
    (D / side / "tables.md").write_text("\n".join(out) + "\n", encoding="utf-8", newline="\n")
    print(f"TABLES-OK {side}")


if __name__ == "__main__":
    main(sys.argv[1])

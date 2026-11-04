"""#1085 T4: per-scenario Spec v1 classification of the alternating pairs.

Reads t4/<cfg>-p<k>-base|proto/{summary,manifest}.json and calls
profile_report.classify once per (config, scenario). Writes t4/t4-pairs.json
and prints Markdown tables. Run from the repository root.
"""

import json
import math
import statistics
import sys
from pathlib import Path

sys.path.insert(0, "scripts/validation")
import profile_report  # noqa: E402

T4 = Path(".agent/validation/issue-1085/t4")
CONFIGS = ["rank-normal", "rank-normal-uncapped", "rank-low", "rank-low-uncapped"]
SCENES = ["universe", "system", "orbit", "descent", "turns", "ground", "ground-night", "world-two"]
PAIRS = {"rank-low": 5}  # Spec v1 extension: ambiguous scenes after three pairs


def load(cfg, k, side):
    folder = T4 / f"{cfg}-p{k}-{side}"
    manifest = json.loads((folder / "manifest.json").read_text())
    summary = json.loads((folder / "summary.json").read_text())
    expected = "inflight1" if side == "proto" else "none"
    assert manifest["prototype"] == expected, (folder, manifest["prototype"])
    assert manifest["complete"] and manifest["settings_unchanged"], folder
    return manifest, summary


def p95(values):
    values = sorted(v for v in values if v is not None)
    return values[math.ceil(0.95 * len(values)) - 1] if values else None


def metrics(manifest, summary, scene):
    row = summary["scenarios"][scene]
    legs = [r["ready_ms"] for r in summary["readiness"] if r["scenario"] == scene]
    return {"p95": row["interval_ms"]["p95"], "mean": row["interval_ms"]["mean"],
            "readiness_p95": p95(legs) or 0.0,
            "readiness_incomplete": sum(v is None for v in legs),
            "memory_peak": manifest["process"]["observed_peak_working_set_bytes"],
            "wait_mode": row["wait_mode"]["label"], "over_100ms": row["over_100ms"]}


def main():
    out = {}
    for cfg in CONFIGS:
        runs = {side: [load(cfg, k, side) for k in range(1, PAIRS.get(cfg, 3) + 1)] for side in ("base", "proto")}
        out[cfg] = {}
        print(f"\n### {cfg}\n")
        print("| scenario | base p95 (runs) | proto p95 (runs) | p95 change | base mean | proto mean |"
              " mean change | wait modes base / proto | verdict | rollout numbers |")
        print("|---|---|---|---:|---:|---:|---:|---|---|---|")
        for scene in SCENES:
            per = {side: [metrics(m, s, scene) for m, s in runs[side]] for side in runs}
            m = {key: ([r[key] for r in per["base"]], [r[key] for r in per["proto"]])
                 for key in ("p95", "mean", "readiness_p95", "memory_peak")}
            result = profile_report.classify({scene: m}, PAIRS.get(cfg, 3))
            rollout = result["rollout_numbers"][scene]
            modes = (",".join(r["wait_mode"] for r in per["base"]),
                     ",".join(r["wait_mode"] for r in per["proto"]))
            out[cfg][scene] = {"runs": per, "classify": result}
            bp, pp = (statistics.median(v) for v in m["p95"])
            bm, pm = (statistics.median(v) for v in m["mean"])
            failed = [k for k, ok in rollout.items() if not ok]
            print(f"| {scene} | {bp:.2f} ({', '.join(f'{v:.1f}' for v in m['p95'][0])}) "
                  f"| {pp:.2f} ({', '.join(f'{v:.1f}' for v in m['p95'][1])}) "
                  f"| {100 * (pp - bp) / bp:+.1f}% | {bm:.2f} | {pm:.2f} | {100 * (pm - bm) / bm:+.1f}% "
                  f"| {modes[0]} / {modes[1]} | {result['verdict']} "
                  f"| {'ok' if not failed else 'over: ' + ', '.join(failed)} |")
    (T4 / "t4-pairs.json").write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()

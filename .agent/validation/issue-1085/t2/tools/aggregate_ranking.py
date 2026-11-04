import json, statistics as st, sys
from pathlib import Path
root = Path(sys.argv[1])
scen = ["universe","system","orbit","descent","turns","ground","ground-night","world-two"]
keys = ["interval_ms","main_ms","render_ms","combined_wait_ms"]
def med_mad(v):
    m = st.median(v); return m, st.median([abs(x-m) for x in v])
for cfg in ["normal","normal-uncapped","low","low-uncapped"]:
    runs = [json.loads((root/f"rank-{cfg}-r{k}"/"summary.json").read_text()) for k in (1,2,3)]
    print(f"\n## {cfg}  (median ± MAD of per-run means, ms; p95 median; over100 sum; wait label)")
    print("| scenario | interval | main | render | wait | interval p95 | >100ms | wait mode |")
    print("|---|---:|---:|---:|---:|---:|---:|---|")
    for s in scen:
        cells=[]
        for k in keys:
            m,d = med_mad([r["scenarios"][s][k]["mean"] for r in runs]); cells.append(f"{m:.2f} ± {d:.2f}")
        p95 = st.median([r["scenarios"][s]["interval_ms"]["p95"] for r in runs])
        o = sum(r["scenarios"][s].get("over_100ms",0) for r in runs)
        lab = "/".join(r["scenarios"][s]["wait_mode"]["label"] for r in runs)
        print(f"| {s} | " + " | ".join(cells) + f" | {p95:.2f} | {o} | {lab} |")

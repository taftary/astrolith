import sys, collections
for run in sys.argv[1:]:
    d = collections.defaultdict(dict)
    for line in open(f"{run}/route/gpu.txt"):
        if "elapsed_gpu" not in line: continue
        head, _, path = line.strip().partition(" path=")
        w = dict(x.split("=",1) for x in head.split()[1:])
        d[w["scenario"]][path.split("/")[1]] = float(w["mean"])
    print(f"\n## {run} GPU ms per pass (mean)")
    passes = sorted({p for s in d.values() for p in s}, key=lambda p: -sum(s.get(p,0) for s in d.values()))
    print("| scenario | total | " + " | ".join(passes) + " |")
    print("|---|---:|" + "---:|"*len(passes))
    for s, row in d.items():
        print(f"| {s} | {sum(row.values()):.2f} | " + " | ".join(f"{row.get(p,0):.2f}" for p in passes) + " |")

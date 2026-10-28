# #1022 T9 evidence (#1066): terrain loading budgets (R2)

Release build, the T3 cold-ground shot file (`../t3/cold.shots`: a
warm-up plus 20 placements, seed 1022), `--timing`, 1600 x 900.

## R2 readiness, 20 placements [r2.txt, from `compare_cold.py`]

```
ring_ms placements=20 p95_before=2014 p95_after=1933 limit=3000 within_limit=yes allowed_vs_baseline=2417 within_baseline=yes
ready_ms placements=20 p95_before=7433 p95_after=7378 limit=5000 within_limit=no allowed_vs_baseline=8920 within_baseline=yes
```

- Support ring under the player: p95 1933 ms, within R2's 3 s.
- Selected detail (the whole wanted ground): p95 7378 ms, over R2's 5 s.
  **Reported miss.** Today's game misses it too (7433 ms in T3), and this
  change stays within the baseline allowance.

## What was found on the way [exp/]

The first version also counted every finished-but-unapplied build against
the in-flight limit. On the full run that pushed the p95 to 3005 ms
(ring) and 13073 ms (ready). Four variants on six placements (`exp.shots`,
`exp/<variant>/timing.txt`) separated the causes:

- `budget10` (the time budget at 10 %), `share15` (at 15 %) and
  `unlimited` (no time budget) gave the same slow times;
- `oldinflight` (finished builds not counted) gave back the T3 times.

So the time budget costs no readiness here. The slowdown came from
finished builds holding dispatch slots for one frame: builds that finish
in a frame apply in that same frame, after the dispatch. The kept rule
counts only the backlog carried over from the previous frame. That still
bounds the queue, and on the full run readiness is back at baseline (above).

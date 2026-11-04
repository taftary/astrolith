# #1085 T4 (#1151): matched comparison, baseline vs `inflight1`

**Build:** the same release `universe-app.exe` for both sides, SHA-256 `e5447043bd1ff0f2607c7a20f5a90996f7f6d860b5a7127acc3572be7bdcdc77` (T3 commit 28f28da, code identical to feat/1085 dcdc5db).
- **Baseline:** no switch (#819 pacing, two frames in flight).
- **Prototype:** `ASTROLITH_PROTOTYPE=inflight1`.

Every manifest has the expected `prototype` (`none` / `inflight1`), `complete: true` and `settings_unchanged: true`, checked by `tools/aggregate_t4.py`.

**Conditions:** Intel UHD 620, Vulkan, 1600x900, 60 Hz, Balanced on AC, 5 workers, GPU timers off. The overnight batch ran 2026-11-03 22:58 to 2026-11-04 06:08 UTC, with the PC left idle.

## Method

- **Fixtures:** the four T2 rank fixtures (`scripts/validation/fixtures/profile/`), each with eight 120-second scenarios after a 30-second warm-up:
  - `rank-normal` (Normal, VSync on);
  - `rank-normal-uncapped` (VSync off);
  - `rank-low` (Low, 720p, VSync on);
  - `rank-low-uncapped`.
- **Pairs:** three alternating pairs per fixture (`batch-t4.sh`). Odd pairs run the baseline first and even pairs the prototype first.
- **Extension:** after three pairs, Spec v1 found `rank-low` universe and world-two ambiguous, so `rank-low` was extended to five pairs (`batch-t4-extend.sh`, 06:13-07:19 UTC). All `rank-low` rows below use five pairs.
- **Long runs:** `sustained-normal` (20 min) and `transitions` (ten planet/quality switches), once per side.
- **Classification:** `profile_report.classify` (Spec v1), run once per fixture and scenario on the per-run values:
  - **Better** needs the median p95 at least 5% lower *and* a drop above twice the baseline MAD.
  - **Promising** means better; **inconclusive** means ambiguous after five pairs; **rejected** means neither.
  - **Rollout numbers:** mean and p95 within max(5%, 0.5 ms); readiness p95 within max(10%, 100 ms); peak memory within 5%.
  - No bottleneck (GPU) time was recorded, so the test is p95 only.
  - Readiness is the scenario's leg `ready_ms`. Memory is the run's peak working set.
- **Wait modes** per run are listed in the tables. Where they differ between sides, the pair is not same-mode, and that is disclosed below.

Tables: `tables.md`; per-run values and full verdicts: `t4-pairs.json`. Interval values are in ms; the runs column lists p95 per run in run order.


### rank-normal

| scenario | base p95 (runs) | proto p95 (runs) | p95 change | base mean | proto mean | mean change | wait modes base / proto | verdict | rollout numbers |
|---|---|---|---:|---:|---:|---:|---|---|---|
| universe | 50.83 (50.5, 50.8, 51.0) | 48.05 (48.1, 48.0, 47.5) | -5.5% | 39.95 | 30.64 | -23.3% | slow,slow,slow / slow,slow,slow | promising | ok |
| system | 51.24 (50.2, 51.5, 51.2) | 50.90 (50.9, 51.1, 50.5) | -0.7% | 40.15 | 40.08 | -0.2% | slow,slow,slow / slow,slow,slow | rejected | ok |
| orbit | 50.96 (32.2, 51.3, 51.0) | 39.52 (39.4, 39.5, 40.4) | -22.4% | 39.84 | 34.25 | -14.0% | slow,slow,slow / slow,slow,slow | promising | ok |
| descent | 47.77 (41.1, 47.8, 47.9) | 48.45 (48.5, 48.3, 48.6) | +1.4% | 36.78 | 39.80 | +8.2% | slow,mixed,mixed / slow,slow,slow | rejected | over: mean |
| turns | 31.51 (29.1, 31.5, 32.2) | 31.64 (31.7, 31.6, 31.6) | +0.4% | 27.16 | 30.61 | +12.7% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean |
| ground | 28.96 (26.8, 33.9, 29.0) | 30.56 (30.8, 30.6, 30.5) | +5.5% | 25.74 | 29.32 | +13.9% | slow,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| ground-night | 28.53 (25.7, 46.8, 28.5) | 29.54 (29.2, 29.5, 29.6) | +3.5% | 24.96 | 28.41 | +13.8% | slow,slow,mixed / slow,slow,slow | rejected | over: mean |
| world-two | 36.02 (35.5, 36.0, 37.0) | 40.11 (40.1, 40.1, 40.1) | +11.4% | 33.14 | 37.10 | +12.0% | slow,slow,slow / slow,slow,slow | rejected | over: mean, p95 |

### rank-normal-uncapped

| scenario | base p95 (runs) | proto p95 (runs) | p95 change | base mean | proto mean | mean change | wait modes base / proto | verdict | rollout numbers |
|---|---|---|---:|---:|---:|---:|---|---|---|
| universe | 29.94 (30.0, 29.7, 29.9) | 31.06 (31.1, 33.6, 30.7) | +3.7% | 23.06 | 25.55 | +10.8% | mixed,mixed,mixed / mixed,mixed,mixed | rejected | over: mean |
| system | 17.44 (17.4, 17.5, 17.4) | 17.47 (17.4, 17.5, 17.5) | +0.2% | 16.68 | 16.67 | -0.0% | fast,fast,fast / fast,fast,fast | rejected | ok |
| orbit | 35.26 (35.3, 35.6, 35.2) | 41.53 (42.2, 41.4, 41.5) | +17.8% | 30.60 | 35.54 | +16.2% | slow,slow,slow / slow,slow,slow | rejected | over: mean, p95 |
| descent | 44.07 (44.1, 44.3, 44.1) | 48.64 (48.6, 48.6, 49.7) | +10.4% | 35.54 | 39.96 | +12.4% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| turns | 31.34 (31.4, 31.3, 31.1) | 31.74 (32.0, 31.6, 31.7) | +1.3% | 27.18 | 30.62 | +12.6% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean |
| ground | 29.12 (29.2, 29.1, 29.1) | 30.78 (30.8, 30.8, 30.8) | +5.7% | 25.79 | 29.31 | +13.7% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| ground-night | 28.54 (28.3, 28.5, 28.5) | 29.34 (29.3, 29.3, 29.7) | +2.8% | 24.97 | 28.37 | +13.6% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean |
| world-two | 35.29 (35.2, 35.3, 35.3) | 39.96 (39.9, 40.0, 40.0) | +13.2% | 32.93 | 37.12 | +12.7% | slow,slow,slow / slow,slow,slow | rejected | over: mean, p95 |

### rank-low

| scenario | base p95 (runs) | proto p95 (runs) | p95 change | base mean | proto mean | mean change | wait modes base / proto | verdict | rollout numbers |
|---|---|---|---:|---:|---:|---:|---|---|---|
| universe | 50.91 (50.9, 50.9, 51.2, 50.8, 51.0) | 48.89 (49.2, 49.5, 48.0, 48.9, 48.1) | -4.0% | 39.98 | 39.92 | -0.2% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | inconclusive | ok |
| system | 51.47 (51.6, 51.2, 51.1, 51.5, 51.5) | 51.21 (51.2, 51.4, 51.1, 50.3, 51.3) | -0.5% | 40.14 | 40.10 | -0.1% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | inconclusive | ok |
| orbit | 51.63 (51.6, 51.5, 51.6, 51.4, 51.8) | 32.69 (32.7, 32.8, 32.6, 36.3, 32.6) | -36.7% | 39.86 | 28.15 | -29.4% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | promising | ok |
| descent | 50.19 (50.1, 50.3, 50.0, 50.3, 50.2) | 33.84 (33.8, 33.0, 34.0, 32.5, 33.9) | -32.6% | 39.88 | 27.27 | -31.6% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | promising | ok |
| turns | 49.42 (49.5, 49.5, 47.0, 49.4, 49.3) | 24.93 (24.8, 25.0, 25.2, 24.4, 24.9) | -49.6% | 39.88 | 23.87 | -40.1% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | promising | ok |
| ground | 48.82 (48.8, 48.9, 48.7, 48.8, 48.5) | 25.92 (30.6, 25.9, 25.9, 25.8, 26.4) | -46.9% | 39.88 | 24.99 | -37.3% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | promising | over: readiness_p95 |
| ground-night | 48.96 (49.0, 48.9, 49.0, 49.0, 48.9) | 25.29 (25.5, 25.3, 25.2, 24.6, 26.2) | -48.3% | 39.90 | 24.38 | -38.9% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | promising | over: readiness_p95 |
| world-two | 48.99 (49.1, 49.0, 49.0, 48.8, 48.8) | 48.02 (48.0, 48.2, 48.4, 48.0, 47.8) | -2.0% | 39.85 | 39.86 | +0.0% | slow,slow,slow,slow,slow / slow,slow,slow,slow,slow | inconclusive | over: readiness_p95 |

### rank-low-uncapped

| scenario | base p95 (runs) | proto p95 (runs) | p95 change | base mean | proto mean | mean change | wait modes base / proto | verdict | rollout numbers |
|---|---|---|---:|---:|---:|---:|---|---|---|
| universe | 21.81 (21.7, 21.8, 22.2) | 24.19 (23.9, 25.1, 24.2) | +10.9% | 18.71 | 21.03 | +12.4% | mixed,mixed,mixed / mixed,mixed,mixed | rejected | over: mean, p95 |
| system | 17.43 (17.4, 17.4, 17.4) | 17.49 (17.4, 17.5, 17.5) | +0.3% | 16.68 | 16.67 | -0.0% | fast,fast,fast / fast,fast,fast | rejected | ok |
| orbit | 24.16 (24.1, 24.5, 24.2) | 32.44 (29.6, 33.9, 32.4) | +34.3% | 21.65 | 28.04 | +29.5% | slow,slow,slow / slow,slow,slow | rejected | over: mean, p95 |
| descent | 29.07 (29.0, 29.2, 29.1) | 33.54 (33.5, 33.5, 31.9) | +15.4% | 24.16 | 28.10 | +16.3% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| turns | 23.55 (23.9, 23.6, 23.3) | 24.94 (25.1, 24.9, 24.9) | +5.9% | 20.79 | 24.10 | +15.9% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| ground | 24.54 (24.5, 24.6, 24.5) | 25.90 (29.8, 25.8, 25.9) | +5.5% | 21.52 | 24.96 | +16.0% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| ground-night | 23.68 (23.8, 23.7, 23.7) | 25.38 (25.2, 25.4, 26.0) | +7.2% | 20.82 | 24.33 | +16.9% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |
| world-two | 28.35 (28.4, 28.3, 28.4) | 32.43 (32.4, 32.2, 32.7) | +14.4% | 26.30 | 30.50 | +16.0% | mixed,mixed,mixed / slow,slow,slow | rejected | over: mean, p95 |

## Sustained (20 min, Normal, VSync on)

From `sustained-*/summary.json`, ms:

| side | all mean | all p95 | first 5 min mean / p95 | final 5 min mean / p95 | >100 ms | peak working set |
|---|---:|---:|---|---|---:|---:|
| baseline | 33.97 | 48.63 | 39.67 / 50.49 (slow) | 31.25 / 43.00 (mixed) | 4 | 1059 MB |
| inflight1 | 34.59 | 48.42 | 34.26 / 47.80 (slow) | 34.96 / 47.20 (slow) | 3 | 1067 MB |

**Neither side drifts.** The difference between the first and final five minutes follows the scene order, not time:
- the baseline is slow in universe, orbit and descent, and mixed in turns and ground, in both passes;
- `inflight1` stays slow in every block.

Readiness legs are similar on both sides (orbit 367-440 ms, others 63-123 ms).

## Transitions (ten switches between planet 1 at minimum quality and planet 0 at Normal)

| side | mean | p95 | worst | >100 ms | detail ready per switch | peak working set |
|---|---:|---:|---:|---:|---|---:|
| baseline | 39.91 | 51.51 | 172.8 | 10 | 1151-1261 ms | 918 MB |
| inflight1 | 36.16 | 50.16 | 148.8 | 10 | 1153-1310 ms | 895 MB |

Every switch's ring and detail completed on both sides (no `none`).

## Reading

1. **VSync off: rejected in every scenario, on both fixtures.** One frame in flight removes CPU/GPU overlap.
   - The mean rises by 10-30% in every scenario except system, which sits at the 60 fps cap on both sides.
   - The worst case is `rank-low-uncapped` orbit: p95 +34.3%.
   - `inflight1` must not apply with VSync off.

2. **Low, VSync on: promising in five of eight scenarios.**
   - Orbit, descent, turns, ground and ground-night: p95 -33% to -50%, mean -29% to -40%. For example, ground goes from 39.88 to 24.99 ms mean and 48.82 to 25.92 ms p95.
   - Universe, system and world-two: inconclusive at five pairs, with no meaningful change.
   - Rollout numbers pass everywhere except readiness on the ground scenes and world-two. Detail readiness rises by 140-214 ms (median ground 1265 → 1454 ms, ground-night 1319 → 1459 ms, world-two 1195 → 1409 ms), just over the max(10%, 100 ms) bound.
   - All pairs are slow/slow, so these are same-mode comparisons.

3. **Normal, VSync on: promising in universe (p95 -5.5%, mean -23.3%) and orbit (p95 -22.4%, mean -14.0%); rejected elsewhere.**
   - World-two: p95 +11.4%.
   - Descent, turns, ground and ground-night: mean +8% to +14%.
   - **The baseline did not land in one state.** Two examples:
     - *Ground:* every T2 baseline run (and T3's exploration run of this same executable) was slow at about 39.8 ms. Here, two of three baseline runs were mixed at about 25-28 ms.
     - *Orbit:* 28.3 ms in pair 1 and 39.9 ms in pairs 2 and 3.
   - In the mixed state, the baseline main thread is busy for about 25 ms per frame and the wait is short. For example, turns has main 25.0 ms and wait 4-6 ms; `inflight1` has main 11.9 ms and wait 26 ms.
   - So in descent, turns, ground and ground-night, the baseline/`inflight1` pairs are mostly mixed/slow, not same-mode. Their "rejected" compares `inflight1` against the baseline's better state.
   - `inflight1` gave the same number in every run (ground p95 30.8, 30.6, 30.5 ms).

4. **Sustained and transitions:** no growth or drift on either side.
   - Transitions are 9% faster on mean with `inflight1`, with the same count of frames over 100 ms.
   - Peak memory is within 1-3% everywhere.

## Verdict (Spec v1, numbers only)

| fixture | verdict | rollout numbers |
|---|---|---|
| Normal, VSync on | promising (universe, orbit) | not met: mean over in descent, turns, ground, ground-night, world-two; p95 over in ground, world-two |
| Normal, VSync off | rejected | not met |
| Low, VSync on | promising (orbit, descent, turns, ground, ground-night) | not met: readiness over (+140-214 ms) in ground, ground-night, world-two |
| Low, VSync off | rejected | not met |

**`inflight1` is not ready for rollout as tested.**
- What it does well: with VSync on, it removes the 40 ms lock (frames alternating two and three refreshes) wherever the baseline falls into it. It also makes VSync-on delivery the same from run to run, with about half the main-thread time.
- Where it loses: wherever the baseline runs in its busy mixed state, and everywhere with VSync off.

Visual and behaviour checks are T5's; nothing drawn differs between the sides.

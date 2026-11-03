# #1085 T2 step 1: main-thread regression bisect

Reference PC, 2026-11-03. Intel UHD Graphics 620, 60 Hz, power plan Balanced (381b4222), AC present.
Fixture: `.agent/validation/issue-1087/evidence/short-normal/fixture.route` (the #1087 short-normal route: 30 s warm-up, 120 s ground, vsync on).
Each build: `cargo build --locked --release -p universe-app` at the detached commit, then
`python scripts/validation/profile_run.py --route <fixture> --out .agent/validation/issue-1085/t2/bisect-<commit> --timeout 400 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "..."`.

| Commit | Merge | Exe SHA-256 (prefix) | Ground frames | Interval mean (ms) | Main mean (ms) | Render mean (ms) | Wait mean (ms) | >100 ms |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 79bc26c | #1092 (#1087) | 0972050dec6252f7 | 3053 | 39.30 | 23.57 | 3.86 | 34.65 | 0 |
| 8c83c9b | #1102 | not built: docs and guides only (`git diff --stat 79bc26c 8c83c9b` touches no crate) | | | | | | |
| d0d3719 | #1113 (#996) | 0ca3353d66e5cc9a | 1473 | 81.50 | 80.82 | 3.00 | 0.26 | 2 |
| d053bf3 | #1115 (#996 R2) | 034aebb0ec92a4a8 | 1467 | 81.77 | 81.09 | 2.98 | 0.26 | 4 |
| 152f477 | #1126 (#1104) | 3efb14744fdbeb8e | 1481 | 81.04 | 80.36 | 3.02 | 0.26 | 1 |

Numbers come from each `bisect-<commit>/summary.json` (`scenarios.ground`); the full SHA-256 is in each `manifest.json`.

**The regression starts at d0d3719 (#1113, "Real spacing from L2 to L6").** The machine is not the cause: the 79bc26c build run on the same day reproduces the #1087 result (main thread 23.57 ms against 25.34 ms on 2026-10-29).

## Cause (local probe, never committed)

A temporary timer in `nest::generate_cell` (built with `--profile profiling`, run directly with the same fixture for 90 s) counted about 4,000 calls per second, using about 2.1 s of CPU time per second across threads. Backtraces of calls slower than 2 ms are in `probe/backtraces.txt`. Two per-frame callers:

- `debug_overlay::sync_overlay` → `PlanetFrame::for_planet` → `terrain::planet_recipe` → `home::is_home_planet_cell` → `home::chain_matches_journey`. This runs in normal play.
- `route_run::drive_route` → `shot_place::planet_of` → `home::chain_matches_journey`. This runs in measurement routes only, so route numbers are inflated too.

`chain_matches_journey` (added by #1113) regenerates every cell on the chain to replay the autopilot pick. Generating L8 and L9 calls `is_home_system_cell` / `is_home_planet_cell`, which call `chain_matches_journey` again, so the walk recurses. L7-L9 are outside `CELL_CACHE` (L2-L6), so these cells are rebuilt each time (one L9 call took 447,610 µs). The probe build was reverted; the task branch has no tracked change.


# Ranking on the frozen baseline (#1149)

**Baseline:** commit 44d1d29 (main abbe97c with the #1153 fix, plus the ranking fixtures), release `universe-app.exe` SHA-256 `3ce17ca45d74c3b4c92a8a7ebc5b7e7bb55031c9a127b28fcf9199de178b73af`, in every `manifest.json`.

- **Machine:** Intel UHD 620, Vulkan, 1600x900 window, 60 Hz display.
- **Power:** Windows power plan Balanced (381b4222), on AC, observed 2026-11-03 by the main agent.

**Runs:** the four fixtures in `scripts/validation/fixtures/profile/rank-*.route`. Each is a 30 s warm-up, then eight 120 s scenarios: universe, system, orbit, descent, turns, ground, ground-night, world-two.

- Each fixture ran three times, alternating (`rank-<config>-r{1,2,3}/`).
- One GPU-timer run each for the two VSync-off fixtures (`gpu-*-uncapped/`).
- Order and commands: `tools/batch-t2.sh`.

The configs:

- **Fixed Normal:** planet and marker detail Normal, atmosphere Normal, native 3D resolution.
- **Fixed low-cost:** planet and marker detail Low, 3D resolution 720, atmosphere kept at Normal (the floor).
- **All four:** shadows and bloom off, as in new-install defaults.
- **"capped":** VSync on (the default).
- **"uncapped":** `set vsync = off`.

**Correction to the plan wording:** "uncapped" runs still have the game's 60 fps frame cap (setting `target_fps`, default 60).

- The route's `target` line only sets the line the summary judges against; it caps nothing.
- That is why the VSync-off system scene sits at exactly 16.68 ms with 0.4 ms of wait: it is the frame cap.
- No other scene goes under 16.7 ms, so the cap hides no cost.

## Per-scenario CPU and waits (median ± MAD of 3 run means)

From `tools/aggregate_ranking.py` over each run's `summary.json`, reproduced in `ranking-cpu.md`. Values in ms.

- **main:** main-world schedules, `First` to `Last`.
- **render:** the render schedule minus its waits.
- **wait:** the GPU pacing wait plus the window-image wait (`frame_work.rs`).

The threads overlap, so the parts never add up to the interval.


### normal  (median ± MAD of per-run means, ms; p95 median; over100 sum; wait label)
| scenario | interval | main | render | wait | interval p95 | >100ms | wait mode |
|---|---:|---:|---:|---:|---:|---:|---|
| universe | 39.92 ± 0.02 | 11.69 ± 0.12 | 6.10 ± 0.09 | 32.04 ± 0.03 | 50.53 | 3 | slow/slow/slow |
| system | 40.06 ± 0.06 | 7.92 ± 0.39 | 6.62 ± 0.34 | 31.65 ± 0.15 | 51.08 | 0 | slow/slow/slow |
| orbit | 28.91 ± 0.68 | 9.15 ± 0.04 | 6.76 ± 0.48 | 21.30 ± 0.79 | 32.71 | 3 | slow/slow/slow |
| descent | 35.17 ± 1.30 | 20.62 ± 0.16 | 8.10 ± 0.08 | 24.25 ± 1.24 | 42.63 | 0 | slow/mixed/mixed |
| turns | 28.64 ± 1.52 | 23.23 ± 1.58 | 7.11 ± 0.22 | 9.60 ± 5.18 | 46.52 | 0 | mixed/mixed/mixed |
| ground | 39.84 ± 0.02 | 11.67 ± 0.10 | 5.17 ± 0.44 | 33.61 ± 0.59 | 48.85 | 1 | slow/slow/mixed |
| ground-night | 39.87 ± 0.01 | 11.65 ± 0.11 | 5.13 ± 0.07 | 33.66 ± 0.19 | 48.87 | 1 | slow/slow/slow |
| world-two | 39.76 ± 0.03 | 11.51 ± 0.08 | 5.63 ± 0.02 | 32.93 ± 0.11 | 48.39 | 3 | slow/slow/slow |

### normal-uncapped  (median ± MAD of per-run means, ms; p95 median; over100 sum; wait label)
| scenario | interval | main | render | wait | interval p95 | >100ms | wait mode |
|---|---:|---:|---:|---:|---:|---:|---|
| universe | 22.88 ± 0.05 | 14.45 ± 0.07 | 6.63 ± 0.00 | 9.17 ± 0.34 | 29.58 | 3 | mixed/mixed/mixed |
| system | 16.68 ± 0.00 | 5.30 ± 0.02 | 4.37 ± 0.04 | 0.36 ± 0.00 | 17.47 | 0 | fast/fast/fast |
| orbit | 30.57 ± 0.50 | 10.16 ± 1.85 | 7.38 ± 1.38 | 22.14 ± 0.69 | 35.28 | 3 | slow/slow/slow |
| descent | 35.54 ± 0.12 | 21.02 ± 2.42 | 8.25 ± 1.43 | 23.10 ± 1.12 | 43.55 | 0 | slow/mixed/mixed |
| turns | 27.13 ± 0.02 | 24.86 ± 0.01 | 8.00 ± 0.10 | 4.87 ± 0.78 | 31.01 | 0 | mixed/mixed/mixed |
| ground | 25.73 ± 0.06 | 23.13 ± 0.05 | 9.14 ± 0.02 | 5.09 ± 0.17 | 29.03 | 3 | slow/mixed/mixed |
| ground-night | 24.85 ± 0.13 | 22.39 ± 0.12 | 8.97 ± 0.03 | 5.05 ± 0.08 | 28.34 | 3 | slow/mixed/mixed |
| world-two | 33.03 ± 0.03 | 21.87 ± 0.05 | 8.28 ± 0.04 | 22.75 ± 0.04 | 35.66 | 3 | slow/slow/slow |

### low  (median ± MAD of per-run means, ms; p95 median; over100 sum; wait label)
| scenario | interval | main | render | wait | interval p95 | >100ms | wait mode |
|---|---:|---:|---:|---:|---:|---:|---|
| universe | 39.96 ± 0.02 | 9.99 ± 0.02 | 6.18 ± 0.01 | 32.31 ± 0.00 | 50.94 | 3 | slow/slow/slow |
| system | 40.14 ± 0.00 | 7.64 ± 0.03 | 6.67 ± 0.02 | 31.72 ± 0.03 | 51.45 | 0 | slow/slow/slow |
| orbit | 39.84 ± 0.01 | 8.58 ± 0.01 | 7.82 ± 0.01 | 30.35 ± 0.02 | 51.62 | 3 | slow/slow/slow |
| descent | 39.87 ± 0.01 | 10.08 ± 0.03 | 6.53 ± 0.01 | 32.04 ± 0.01 | 50.07 | 0 | slow/slow/slow |
| turns | 39.89 ± 0.01 | 10.83 ± 0.03 | 5.57 ± 0.01 | 33.17 ± 0.00 | 49.35 | 0 | slow/slow/slow |
| ground | 39.88 ± 0.01 | 11.25 ± 0.01 | 5.67 ± 0.03 | 33.13 ± 0.03 | 48.89 | 0 | slow/slow/slow |
| ground-night | 39.89 ± 0.00 | 11.35 ± 0.05 | 5.44 ± 0.01 | 33.35 ± 0.00 | 48.90 | 0 | slow/slow/slow |
| world-two | 39.86 ± 0.01 | 11.01 ± 0.13 | 5.74 ± 0.08 | 32.99 ± 0.10 | 48.99 | 3 | slow/slow/slow |

### low-uncapped  (median ± MAD of per-run means, ms; p95 median; over100 sum; wait label)
| scenario | interval | main | render | wait | interval p95 | >100ms | wait mode |
|---|---:|---:|---:|---:|---:|---:|---|
| universe | 18.11 ± 0.05 | 11.92 ± 0.04 | 6.34 ± 0.01 | 5.97 ± 0.01 | 21.16 | 3 | mixed/mixed/mixed |
| system | 16.68 ± 0.00 | 5.07 ± 0.00 | 4.45 ± 0.00 | 0.38 ± 0.00 | 17.45 | 0 | fast/fast/fast |
| orbit | 21.60 ± 0.10 | 9.11 ± 0.15 | 7.35 ± 0.08 | 12.19 ± 0.02 | 24.08 | 3 | slow/slow/slow |
| descent | 24.17 ± 0.04 | 17.22 ± 0.00 | 8.82 ± 0.01 | 10.94 ± 0.09 | 29.09 | 0 | mixed/mixed/mixed |
| turns | 20.74 ± 0.01 | 18.74 ± 0.02 | 8.94 ± 0.03 | 2.50 ± 0.05 | 23.42 | 0 | mixed/mixed/mixed |
| ground | 21.48 ± 0.00 | 19.44 ± 0.05 | 8.85 ± 0.05 | 3.44 ± 0.35 | 24.50 | 2 | mixed/mixed/mixed |
| ground-night | 20.91 ± 0.05 | 19.03 ± 0.04 | 8.73 ± 0.06 | 2.68 ± 0.09 | 24.00 | 3 | mixed/mixed/mixed |
| world-two | 26.38 ± 0.03 | 22.70 ± 0.03 | 9.06 ± 0.02 | 12.80 ± 0.22 | 28.56 | 3 | mixed/mixed/mixed |


## Per-scenario GPU passes (VSync off, one run per config)

From `tools/gpu_passes.py` over `gpu-*-uncapped/route/gpu.txt` (`elapsed_gpu` means, ms), reproduced in `ranking-gpu.md`. With timers on, interval means match the untimed runs within 0.5 ms (ground Normal 26.01 against 25.73).


### gpu-normal-uncapped GPU ms per pass (mean)
| scenario | total | main_opaque_pass_3d | tonemapping | upscaling | main_transparent_pass_3d | clustering | ui | bin_unpacking | early_mesh_preprocessing |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| warmup | 12.27 | 10.06 | 1.28 | 0.43 | 0.06 | 0.21 | 0.12 | 0.09 | 0.02 |
| universe | 9.25 | 5.33 | 1.55 | 0.50 | 1.43 | 0.25 | 0.15 | 0.02 | 0.01 |
| system | 4.12 | 0.01 | 2.59 | 0.60 | 0.32 | 0.41 | 0.20 | 0.00 | 0.00 |
| orbit | 9.99 | 7.83 | 1.23 | 0.44 | 0.11 | 0.21 | 0.12 | 0.02 | 0.02 |
| descent | 19.70 | 17.51 | 1.27 | 0.43 | 0.06 | 0.21 | 0.12 | 0.07 | 0.02 |
| turns | 13.63 | 11.39 | 1.27 | 0.45 | 0.07 | 0.21 | 0.12 | 0.09 | 0.02 |
| ground | 12.45 | 10.19 | 1.27 | 0.45 | 0.08 | 0.22 | 0.12 | 0.09 | 0.02 |
| ground-night | 11.51 | 9.20 | 1.27 | 0.49 | 0.09 | 0.21 | 0.12 | 0.09 | 0.02 |
| world-two | 17.91 | 15.70 | 1.26 | 0.44 | 0.06 | 0.21 | 0.12 | 0.09 | 0.02 |

### gpu-low-uncapped GPU ms per pass (mean)
| scenario | total | main_opaque_pass_3d | tonemapping | ui | upscaling | clustering | main_transparent_pass_3d | bin_unpacking | early_mesh_preprocessing | main_transparent_pass_2d |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| warmup | 9.70 | 7.26 | 0.83 | 0.79 | 0.44 | 0.22 | 0.05 | 0.09 | 0.02 | 0.00 |
| universe | 7.03 | 3.69 | 1.04 | 0.95 | 0.47 | 0.27 | 0.57 | 0.02 | 0.02 | 0.00 |
| system | 4.29 | 0.01 | 1.67 | 1.37 | 0.55 | 0.41 | 0.28 | 0.00 | 0.00 | 0.00 |
| orbit | 6.32 | 3.98 | 0.80 | 0.77 | 0.42 | 0.22 | 0.09 | 0.02 | 0.02 | 0.00 |
| descent | 11.70 | 9.30 | 0.82 | 0.78 | 0.43 | 0.22 | 0.06 | 0.07 | 0.02 | 0.00 |
| turns | 9.57 | 7.12 | 0.83 | 0.79 | 0.44 | 0.22 | 0.06 | 0.10 | 0.02 | 0.00 |
| ground | 10.31 | 7.86 | 0.82 | 0.79 | 0.44 | 0.22 | 0.07 | 0.09 | 0.02 | 0.00 |
| ground-night | 9.66 | 7.18 | 0.83 | 0.79 | 0.44 | 0.22 | 0.08 | 0.10 | 0.02 | 0.00 |
| world-two | 13.74 | 11.31 | 0.82 | 0.78 | 0.43 | 0.22 | 0.06 | 0.09 | 0.02 | 0.00 |


## Diagnostic toggles (ground scenario only)

Fixtures in `toggles/fixtures/`: 30 s warm-up plus 120 s ground, fixed Normal, GPU timers on. One run each, in order (`tools/batch-tg.sh`, then tg-cap30-setting).

These isolate costs only; they are not the ranking. Ground row of each `toggles/<run>/summary.json` and `route/gpu.txt`, in ms:

| run | change from base | interval | p95 | main | render | wait | wait mode | GPU total | main opaque | other GPU passes ≥ 0.3 ms |
|---|---|---:|---:|---:|---:|---:|---|---:|---:|---|
| tg-base | VSync off | 24.41 | 26.00 | 19.69 | 6.84 | 16.23 | slow | 11.91 | 9.74 | tonemapping 1.21, upscaling 0.45 |
| tg-atmo-low | atmosphere Low | 25.77 | 29.15 | 23.13 | 9.57 | 5.31 | mixed | 12.41 | 10.16 | tonemapping 1.27, upscaling 0.45 |
| tg-atmo-high | atmosphere High | 25.99 | 29.05 | 23.36 | 9.75 | 5.04 | mixed | 12.40 | 10.15 | tonemapping 1.27, upscaling 0.45 |
| tg-shadows | shadows on | 32.94 | 34.95 | 25.60 | 10.96 | 20.11 | slow | 15.37 | 13.04 | tonemapping 1.26, upscaling 0.47 |
| tg-bloom | bloom on | 28.52 | 31.28 | 24.67 | 10.45 | 13.69 | mixed | 15.61 | 10.22 | bloom 3.14, tonemapping 1.27, upscaling 0.44 |
| tg-shadows-bloom | shadows + bloom on | 35.90 | 37.71 | 24.71 | 11.44 | 22.68 | slow | 18.41 | 12.99 | bloom 3.08, tonemapping 1.25, upscaling 0.48 |
| tg-cap30 | route `target 30` (caps nothing, see above) | 25.80 | 29.02 | 23.12 | 9.51 | 5.48 | mixed | 12.38 | 10.14 | tonemapping 1.27, upscaling 0.45 |
| tg-cap30-setting | real 30 fps frame cap (`set target_fps = 30`), VSync off | 33.33 | 34.39 | 10.89 | 4.20 | 0.38 | fast | 12.26 | 10.05 | tonemapping 1.26, upscaling 0.44 |
| tg-capped | VSync on | 39.86 | 49.01 | 11.39 | 5.31 | 33.50 | slow | 14.86 | 12.20 | tonemapping 1.57, upscaling 0.47 |

### What the toggles show

**tg-base landed in a different wait mode** from the other VSync-off runs.

- Its wait is 16.2 ms "slow", against about 5 ms "mixed" in the others. Its main thread is also shorter (19.7 against 23.1 ms).
- The ranking runs (`rank-normal-uncapped-r*`: 3 of 3 at 25.7 ms, main 23.1 ms) agree with tg-atmo-low, tg-atmo-high and tg-cap30. Those are the comparison base below.
- tg-base is reported, not dropped.

**Atmosphere tier: no measurable cost on ground. Not a candidate.**

- Low, Normal and High differ by at most 0.2 ms in interval, and not at all in the timed passes (main opaque 10.15-10.16).
- The atmosphere tables are not in Bevy's pass timers, so this toggle is the only measure: ≤ 0.3 ms on ground.

**Shadows and bloom are not baseline costs.** Both are player settings, off in the baseline.

| Toggle | Interval | GPU |
|---|---:|---:|
| Shadows on | +7.1 ms | +3.0 ms (opaque pass +2.9; the shadow-map pass itself is untimed) |
| Bloom on | +2.7 ms | +3.1 ms (bloom pass) |
| Both on | +10.1 ms | — |

The two alone add up to +9.8 ms against +10.1 ms together, so there is no interaction.

**Transparency is small.** There is no toggle, so the timer is the measure:

- 0.06-0.09 ms on the planet scenes;
- 1.43 ms (Normal) or 0.57 ms (low-cost) in the universe scene.

**Ground shading: the main opaque pass is the upper bound.** The baseline has no toggle, because a lean variant is exactly what T3 would build. The pass covers ground, figure and markers:

- 9.2-10.2 ms on ground scenes, VSync off;
- 15.7-17.5 ms in descent and world-two, VSync off;
- 12.2 ms on ground with VSync on.

**The main thread is not slow work, it is a busy CPU.** Same scene, same settings:

| Setting | Main thread |
|---|---:|
| VSync on (tg-capped) | 11.39 ms |
| Real 30 fps cap, VSync off (tg-cap30-setting) | 10.89 ms |
| Running flat out (VSync off at the 60 fps cap) | 23.1 ms |

- The work per frame is the same, so the doubling comes from running the processor without a pause.
- Likely causes: the CPU and GPU share one power budget on this chip, and the main thread competes with the render thread and the patch workers.
- Which share is which cannot be measured here: the runner has no package-power or per-core clock counters.

## Wait-mode explanation

**The plan's hypothesis is rejected.** The plan guessed that GPU work sits right at 16.7 ms, so frames flip between one and two vblanks.

- The measured GPU work on ground is 11.9-12.5 ms with VSync off and 14.9 ms with VSync on.
- With VSync on, frames do not flip between one and two vblanks. They sit between two and three: mean about 40 ms, p95 about 49 ms.
- This holds in 6 of 8 scenes, in both configs, in all three runs.

**Confirmed instead: VSync-on frame delivery costs more than the work.**

1. **The GPU runs slower with VSync on.** The same ground frame's passes take 14.86 ms with VSync on against 12.26-12.41 ms with VSync off (opaque 12.20 against 10.05-10.16, tonemapping 1.57 against 1.26-1.27).
   - With a 30 fps cap and VSync off, the GPU idles about 21 ms per frame and still runs at 12.26 ms.
   - So this slowdown is not explained by idle time alone, as ADR 0044 assumed. It comes with the VSync present path.
2. **The chain does not fit in one or two refreshes.** The chain is main thread (about 11.4 ms), then render thread (about 5.3 ms), then GPU (about 14.9 ms), with two frames in flight (ADR 0044). It does not overlap enough to fit one vblank, so frames land on the second or third refresh.
3. **The same work delivers faster without VSync.** With the same CPU and GPU, VSync off and a real 30 fps cap, frames are a steady 33.33 ms with 0.38 ms of wait ("fast" mode). With VSync on they are 39.86 ms, p95 49.01.
   - The extra 6.5 ms beyond a steady 33.3 ms belongs to how frames reach the screen with VSync on, not to any drawing cost.
   - So does the whole distance from 16.7 ms.

**The two modes seen in #1087 (26-28 against about 38 ms) are this split.**

- **VSync-off-like runs:** main-thread or GPU bound, short waits.
- **VSync-on runs:** long waits.
- T1's wait-mode label separates them.
- Mixed runs appear in descent and turns, where streaming work (main 20-23 ms) is long enough to change the chain.

## Ranked costs per scenario

Fixed Normal with VSync on (what a player gets by default), with the VSync-off GPU passes for the drawing share. "Delivery wait" is the frame-delivery wait from the capped runs. All values in ms.

| scenario | interval | 1st | 2nd | 3rd | 4th |
|---|---:|---|---|---|---|
| universe | 39.92 | delivery wait 32.04 | main 11.69 | render 6.10 | GPU opaque 5.33 (+ tonemapping 1.55, transparent 1.43) |
| system | 40.06 | delivery wait 31.65 | main 7.92 | render 6.62 | GPU tonemapping 2.59 (opaque 0.01) |
| orbit | 28.91 | delivery wait 21.30 | main 9.15 | GPU opaque 7.83 | render 6.76 |
| descent | 35.17 | delivery wait 24.25 | main 20.62 (streaming) | GPU opaque 17.51 | render 8.10 |
| turns | 28.64 | main 23.23 (streaming) | GPU opaque 11.39 | delivery wait 9.60 | render 7.11 |
| ground | 39.84 | delivery wait 33.61 | main 11.67 | GPU opaque 10.19 (12.20 with VSync on) | render 5.17 |
| ground-night | 39.87 | delivery wait 33.66 | main 11.65 | GPU opaque 9.20 | render 5.13 |
| world-two | 39.76 | delivery wait 32.93 | main 11.51 | GPU opaque 15.70 | render 5.63 |

**Lowering detail does not change what a player sees on this PC.** With the fixed low-cost config and VSync on, every scenario is 39.76-40.14 ms with a 30.35-33.35 ms wait, because delivery dominates. With VSync off, low-cost helps:

| Scene (VSync off) | Normal | Low-cost |
|---|---:|---:|
| ground | 25.73 ms | 21.48 ms |
| descent | 35.54 ms | 24.17 ms |
| world-two | 33.03 ms | 26.38 ms |

**Areas not measured, and why:**

- **Atmosphere tables and the shadow-map pass:** not in Bevy's GPU pass timers. Bounded by toggles instead.
- **Driver and present time:** no counter.
- **CPU power and clock sharing with the GPU:** no package-power or clock counters.
- **Phones:** no device runs (#1054).

## Candidate by the plan's rule

**The largest single cost is the VSync-on frame-delivery wait.**

- It is 31.7-33.7 ms of a 40 ms frame in 6 of 8 scenarios (21.3 and 24.3 ms in orbit and descent).
- Ground drawing (main opaque pass) is 10.2 ms with VSync off and 12.2 ms with VSync on. Even removed entirely, it would leave the VSync-on frame on the same two-to-three-refresh pattern.

The three conditions of the rule:

- **(a) A bounded change can address it.** The VSync-on present path and its frame pacing are a few systems in `crates/universe-app/src/gpu.rs` (`FramePacingPlugin`, two frames in flight) and the window's present settings. Prototype directions for T3, behind `ASTROLITH_PROTOTYPE` / `prototype`, measured against this baseline:
  - a tear-free present mode other than first-in-first-out, if the adapter offers one;
  - the surface's maximum frame latency;
  - where the pacing wait sits relative to the window-image acquire.
- **(b) The Spec v1 preservation list holds:** nothing drawn changes, only when it reaches the screen.
- **(c) No player setting changes:** VSync stays a player setting with the same meaning (no tearing).

**Switch:** the plan expected a lean ground fragment path. Per the plan's rule ("If T2 instead ranks frame delivery or atmosphere first, that becomes the candidate"), the T3 candidate is **VSync-on frame delivery**.

- The lean ground path is not dropped as an idea. It is ranked third or lower in every scenario at the default settings.
- It matters only with VSync off, where low-cost detail and the busy-CPU effect are larger levers.

# 0044 — Frame time: two frames in flight, timing runs, the vsync floor

- Status: accepted
- Date: 2026-10-20
- Issue: #819 (Spec v2)

## Context

The owner asked for steady, faster frames (#819): a steady 60 frames per second (16.7 ms) as the goal, on the dev PC (Intel UHD 620) at 1600x900, release build, vsync on (Q1 A, Q2 A, Q3 B, Q4 A). Anything that keeps the picture is allowed; turning a feature off needs the owner (Q5 A). In #777 frames alternated between about 15 and 90 ms. #780 then made Vulkan wait for the GPU at the end of every frame (ADR 0041).

## Decision

- **Timing runs.** `--screens <dir> --shots timing/views.txt --timing` times 180 frames at each settled view and prints `TIMING <name> mean_ms= p95_ms= worst_ms= alt_pct=` (also written to `timing.txt`).
  - The run's window is 1600x900 (`TIMING_SIZE`) and updates continuously, as a focused window does.
  - `timing/views.txt` holds the fixed views: L1-L9 dive, then the planet at the morning point (orbit, 100 km, 1 km, ground, walking).
- **Two frames in flight (Vulkan).** Before a frame takes its window image, it waits for the frame before the previous one. The GPU draws one frame while the CPU prepares the next, and no more than two frames are ever queued. This replaces the wait at the end of every frame (ADR 0041).
  - `gpu::FramePacingPlugin`: `InFlight<T>` holds the last submissions. The submission index type is fixed by two functions, so no direct graphics dependency is added (Bevy does not re-export it).
  - A non-blocking poll at frame end hands finished read-backs to their callbacks.
- **Few patch builds queued.** At most two ground patch builds per worker thread are in flight (`patches::max_builds`).
  - Bevy finishes screenshots on the same worker pool. An unbounded queue held the `player-third` capture past its frame cap once frames got faster.

## Baseline

Release build, 1600x900, vsync on, `timing/views.txt`. Before: the #780 pacing (wait at frame end). After: two frames in flight. Mean frame time in ms, mean of two runs each:

| View | Baseline | After | Vsync off (after) |
|---|---|---|---|
| L1-L9 dive | 39.7 | 40.0 | 7.6-11.4 |
| orbit | 46.1 | 39.6 | 43.1 |
| sky-100km | 34.2 | 35.2 | 32.2 |
| sky-1km | 44.1 | 38.6 | 44.9 |
| ground | 38.0 | 38.2 | 33.7 |
| walking | 51.2 | 39.7 | 37.1 |

Run-to-run noise is about ±4 ms on the planet views (sky-100km measured 30.6 and 39.8 in two runs of the same build).

## Alternation

The 15/90 ms alternation of #777 is gone: `alt_pct` is 0.0 on every view, before and after. It came from the Vulkan backlog that ADR 0041 removed. The 95th percentile is at most 1.3 times the mean on every view (limit 1.5).

## Steps

Each step was timed on the same views:

1. Wait for the GPU just before submitting (one frame in flight): orbit and walking faster, but `player-third` timed out in the full screenshot run.
2. Wait just before taking the window image: the same timeout.
3. Cause found: the screenshot read-back task waited behind hundreds of patch builds on the worker pool. Fix: `max_builds`. The full screenshot run passes again (26 files).
4. Two frames in flight (kept): orbit 46→40, 1 km 44→39, walking 51→40 ms. The full screenshot run passes.
5. Bloom texture 256 instead of 512: no change at vsync on. Not kept.
6. Continuous update mode for timing runs (an unfocused window is slowed to a low-power pace): no change in the numbers. Kept, so a timing run measures what a player sees.

## Result against 16.7 ms

Not reached on this PC with vsync on.

- **The vsync floor.** With vsync on, no view goes below about 40 ms, even when its work takes far less: the dive views run at 7.6-11.4 ms with vsync off.
  - The GPU slows down when vsync leaves it idle. The same dive frame's passes take 10.0 ms with vsync on (bloom 6.4, tonemapping 2.6) and 4.9 ms with vsync off (bloom 3.0, tonemapping 1.2).
  - The light views therefore settle at two to three display refreshes per frame (33-50 ms).
  - Players can already turn vsync off (Settings > Display > Window).
- **Remaining cost per part at the planet** (vsync off, after):

| Part | Cost |
|---|---|
| Main thread (game update) | about 7 ms, alongside the GPU |
| Render thread (prepare and submit) | 4-9 ms, alongside the GPU |
| Main opaque pass (ground, figure) | 6-15 ms |
| Bloom | 3.0-3.6 ms |
| Tonemapping | 1.2-1.5 ms |
| Upscaling, clustering, other timed passes | about 1 ms |
| Not in the pass timers (atmosphere tables, sun shadow, driver) | the rest, 12-25 ms of the 30-45 ms frame |

- **What would be needed next:**
  - Timers on the atmosphere and shadow passes, to split the untimed part.
  - A cheaper ground pass: fewer triangles near the horizon, or a cheaper material.
  - Lower-cost atmosphere or shadow settings. These change the picture, so they need the owner (Q5 A).

## Same picture

The full screenshot run was taken before (branch head, #780 pacing) and after (two frames in flight) and compared by eye: `planet-l10`, `approach-l9`, `sky-1km` and `player-third` match. The faint dark sky dots of #780 vary from run to run in both. The pacing and the build cap change when a frame is drawn, not what is drawn.

## Consequences

- One frame more of latency on Vulkan than ADR 0041's frame-end wait, in exchange for the GPU never idling while the CPU prepares the next frame.
- Ground patches stream in no more slowly than before: the worker threads stay busy, and the queue only holds fewer waiting builds.
- Timing runs are the way to check any later speed change: same views, same size, same lines.

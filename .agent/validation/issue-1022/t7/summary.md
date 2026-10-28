# #1022 T7 evidence (#1064): the frame cap

Release build, `--screens --shots t7.shots --timing` (1600 x 900, 60 Hz
display), one run per target and vsync setting:
`ASTROLITH_TARGET_FPS=60|30`, `ASTROLITH_TIMING_NO_VSYNC=0|1`. Numbers
are copied from each folder's `timing.txt`. `cap_mean_ms` is the cap's
sleep per frame: idle CPU, kept out of the work.

## Space view (light: work p95 about 10 ms), where the cap matters

| run | mean frame | p95 frame | cap sleep (idle) |
|---|---|---|---|
| 60 FPS, vsync off [fps60-vsync-off] | 16.72 ms | 18.49 ms | 7.04 ms |
| 30 FPS, vsync off [fps30-vsync-off] | 33.33 ms | 37.72 ms | 21.55 ms |
| 60 FPS, vsync on [fps60-vsync-on] | 39.88 ms | 52.70 ms | 0.00 ms |
| 30 FPS, vsync on [fps30-vsync-on] | 40.02 ms | 51.74 ms | 1.38 ms |

- Cap error with vsync off: the mean is 0.05 ms over 16.67 at 60 FPS
  and 0.00 ms off 33.33 at 30 FPS. No busy-wait: the cap sleeps.
- With vsync on, this PC's Vulkan driver presents about every 40 ms even
  when the work is 10 ms (the vsync floor found in #819, ADR 0044). The
  cap is not the limit there; at 60 FPS on a 60 Hz display it adds no
  cap at all (vsync paces alone).

## Planet views (orbit, ground)

The work (p95 about 38 to 44 ms) is over both targets, so the cap sleeps
0.00 ms in every run: the cap never slows a frame that is already late.

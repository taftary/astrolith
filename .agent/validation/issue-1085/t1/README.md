# #1085 T1 evidence: route GPU passes and wait modes

Tooling proof runs on the reference PC. This is **not** the frozen baseline (T2) or a comparison.

- Build: `cargo build --locked --release -p universe-app` on `task/1148-route-gpu-waits` (main 2381721 plus the T1 changes), 7m 02s. Executable SHA-256 `9dec14718c74519d790997280eb71c1cef3b58ac235e020c5d7447c5211c0bbf` (both manifests).
- Environment observed 2026-11-03 by the main agent: Intel UHD Graphics 620, driver 31.0.101.2130, refresh 60 Hz. Power plan Balanced (Utilisation normale, 381b4222-…), AC present (BatteryStatus 2). Point observations only; thermals not observed.

## Runs (ground scenario, 120 s scored after 30 s warm-up)

| Run | Fixture | Frames | Interval mean / p95 / worst (ms) | Main mean (ms) | Render mean (ms) | Wait mean (ms) | Wait mode | >100 ms | Peak working set (bytes) |
| --- | --- | ---: | --- | ---: | ---: | ---: | --- | ---: | ---: |
| `short-normal-gpu` (`ASTROLITH_GPU_TIMERS=1`) | `short-normal.route` | 1453 | 82.54 / 92.83 / 110.62 | 81.66 | 4.00 | 0.36 | fast | 17 | 888909824 |
| `short-normal-uncapped` | `short-normal-uncapped.route` | 1493 | 80.40 / 83.40 / 102.72 | 79.54 | 3.64 | 0.33 | fast | 2 | 878518272 |

GPU pass means for the ground scenario, from `short-normal-gpu/route/gpu.txt` (1453 samples each): `main_opaque_pass_3d` 16.707 ms, `tonemapping` 2.623 ms, `upscaling` 0.681 ms, `clustering` 0.396 ms, `ui` 0.201 ms, `bin_unpacking` 0.166 ms, `main_transparent_pass_3d` 0.098 ms, `early_mesh_preprocessing` 0.037 ms.

Both runs completed with `settings_unchanged: true`.

## What this shows

- **T1 done criteria:** route runs now write per-scenario GPU pass means (`gpu.txt`), and every `summary.json` report carries a wait-mode label.
- **Main-thread finding (for T2, cause not established):** both runs spend about 80 ms of main-thread work per frame. The #1087 `short-normal` run (2026-10-29, same fixture content) recorded interval mean 26.49 ms and main mean 25.34 ms. The uncapped run had no GPU timers, so the timers are not the cause. Settled main-world counts are similar (#1087: 7 meshes, 13 images; here: 8 meshes, 13 images). Commits merged since #1087 (79bc26c..2381721): #1102, #1113, #1115, #1126 (#1104), #1142, #1143, #1145, #1147. The machine state (CPU clock, other load) was not continuously observed. T2 must reproduce this and find the cause before any ranking.
- With CPU work at about 80 ms, frames never wait on the GPU (wait mode fast in both runs). The GPU pass times are therefore not hidden behind vsync, but they also don't decide frame delivery in this state.

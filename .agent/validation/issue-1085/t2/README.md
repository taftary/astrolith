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

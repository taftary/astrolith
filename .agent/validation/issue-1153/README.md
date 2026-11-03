# #1153 evidence: journey answers remembered

Reference PC, 2026-11-03: Intel UHD Graphics 620, 60 Hz, power plan Balanced (381b4222), AC present.
Build: `cargo build --locked --release -p universe-app` at a481755 (T1 merged into `fix/1153-journey-check-per-frame`), 6m 59s. Executable SHA-256 `2f9d1f1dac205480ec04106f3bd338fe6b529bddf7528ddd0999ade29a75d04a`.
Run: `python scripts/validation/profile_run.py --route .agent/validation/issue-1087/evidence/short-normal/fixture.route --out .agent/validation/issue-1153/short-normal --timeout 400 --workers 5 --width 1600 --height 900 --refresh-hz 60 --power-note "..."`. The manifest reads `completed`, `settings_unchanged: true`.

Ground scenario (120 s scored after 30 s warm-up), from `short-normal/summary.json`:

| Build | Frames | Interval mean / p95 / worst (ms) | Main mean (ms) | Render mean (ms) | Wait mean (ms) | >100 ms |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| a481755 (this fix) | 3014 | 39.83 / 50.71 / 58.33 | 8.54 | 6.65 | 31.87 | 0 |
| 152f477 (before, #1085 T2 bisect) | 1481 | 81.04 | 80.36 | 3.02 | 0.26 | 1 |
| 79bc26c (before #1113, #1085 T2 bisect) | 3053 | 39.30 | 23.57 | 3.86 | 34.65 | 0 |

Spec v1 AC1 (main mean ≤ 30 ms): **met, 8.54 ms**. The main thread is now lighter than before #1113, because the journey check already ran every frame then, only more cheaply. The frame interval is back to the vsync-paced ~40 ms, now set by waiting on the GPU (wait mean 31.87 ms). That rendering cost is what #1085 studies.

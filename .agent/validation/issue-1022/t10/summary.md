# #1022 T10 evidence (#1067): Auto in the game

Release build, real window, `--route`, 1600 x 900, vsync on (this PC's
60 Hz display). Each `calm-heavy-<fps>.route` holds a calm space view (L4)
for 120 s, then the planet's ground for 60 s. The controller's log lines
are in `calm-heavy-<fps>.stdout.txt`; the frame summaries in
`calm-heavy-<fps>/route.txt`.

## Steps seen (from the logs)

- **Calm view, both targets.** Auto starts at the floor and climbs one
  step about every 12 s, in the recovery order of point 8: resolution
  twice, terrain loading work, terrain, markers, atmosphere, shadows,
  bloom. Space-view work p95 is 10.55 ms in both runs.
- **Ground, 60 FPS** (work p95 38.47 ms, over the 16.7 ms target).
  Reductions start after three overloaded seconds:
  - kept, because the work fell: resolution, terrain loading, terrain;
  - rolled back, because the work did not fall: resolution again,
    terrain loading again, markers, shadows.
- **Ground, 30 FPS** (work p95 37.13 ms, near the 33.3 ms target):
  resolution down once, then up again 13 s later when there was room.

## What this PC shows

At 60 FPS the frame interval stays near 50 ms in both scenes. That is
the vsync floor of ADR 0044, not the work. So Auto raises quality in the
calm view at no cost to smoothness, and lowers it on the ground where
the work is the load. A short-route run in Auto (`short-auto/`) stayed at
the floor after one loading step: on that route even the space segment's
work (p95 15.89 ms) is above the 85 % spare line (14.17 ms) at 60 FPS.

## Decisions taken while wiring (stated on #1022)

- **A slow frame is judged by its own work when work timing is reliable**
  (R1: intentional waits are excluded). On this PC's vsync floor every
  frame waits about 40 ms, so judging by the interval would push every
  scene to the floor for nothing.
- **Memory pressure is read at the planet only.** Elsewhere the ground's
  state is a reset.
- **Pauses:** the menus and a hidden (occluded) window. An unfocused but
  visible window keeps adapting: timing and route runs are unfocused.

## Headless

`--verify` prints three `QUALITY` lines, kept in the golden:
- a heavy synthetic load from the best quality comes down, then only
  retries after cooldowns;
- a calm one climbs from the floor to the ceiling;
- the heavy run repeats byte for byte.

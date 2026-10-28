# #1022 T12 evidence (#1069): the sustained runs and the last shots

## Build and environment

- Release build of `feat/1022-adaptive-quality` at d1c8187.
- This dev PC: Intel UHD Graphics 620, Vulkan, 60 Hz display, vsync on
  (`present=AutoVsync`), 5 ground build workers, timer `cpu-work`.
  Every `route.txt` carries this `ROUTE-RUN` line.
- Run order: `run_t12.py` ran the three long runs, then the short ones
  alternating Auto, native and minimum, three times. The working set was
  sampled from outside every 10 s (`memory.txt`).
- **Mobile is not covered.** Nothing here is evidence for phones or
  tablets; that stays with #1054.

The runs:
- **Auto:** `*-auto.route`, `set quality_mode = auto`.
- **Fixed native:** `*-native.route`, Manual with an existing install's
  saved choices: Normal detail, shadows and bloom on, Native resolution.
- **Fixed minimum:** `*-min.route`, Manual at Auto's floor: 720 lines,
  Low terrain and markers, Normal atmosphere, effects off.

## Long route (r4: warm-up 60 s, then 2 x 5 scenarios x 120 s, target 60 FPS)

Copied from each `r4-*/route.txt`:

| run | ROUTE-ALL p95 frame | work p95 | worst | over 100 ms | within 110 % |
|---|---|---|---|---|---|
| Auto | 46.38 ms | 32.46 ms | 310.75 ms | 6 | 0.1 % |
| fixed native | 49.22 ms | 47.68 ms | 375.45 ms | 20 | 0.1 % |
| fixed minimum | 47.77 ms | 30.19 ms | 324.17 ms | 6 | 0.1 % |

First five and final five minutes, p95 frame (`ROUTE-FIRST5`, `ROUTE-FINAL5`):

| run | first five | final five |
|---|---|---|
| Auto | 48.13 ms | 34.61 ms |
| fixed native | 47.87 ms | 50.53 ms |
| fixed minimum | 48.48 ms | 32.99 ms |

Per scenario, p95 frame / work p95 (`ROUTE-SCENARIO`):

| scenario | Auto | fixed native | fixed minimum |
|---|---|---|---|
| universe | 50.49 / 17.04 ms | 48.06 / 29.58 ms | 50.68 / 17.30 ms |
| orbit | 30.85 / 29.54 ms | 47.55 / 46.55 ms | 29.12 / 28.00 ms |
| descent | 33.44 / 32.21 ms | 53.39 / 52.30 ms | 47.29 / 30.29 ms |
| turns | 35.35 / 34.91 ms | 46.37 / 45.24 ms | 47.54 / 29.72 ms |
| ground | 34.78 / 34.18 ms | 48.03 / 47.37 ms | 47.50 / 32.73 ms |

The ground readiness of each planet leg is on the `ROUTE-LEG` lines of
each `route.txt`.

## Short route, three runs each, alternating (`ROUTE-ALL`)

| run | p95 frame | work p95 |
|---|---|---|
| short-auto-1 | 48.27 ms | 27.17 ms |
| short-auto-2 | 48.34 ms | 28.73 ms |
| short-auto-3 | 48.36 ms | 30.66 ms |
| short-native-1 | 52.32 ms | 50.50 ms |
| short-native-2 | 50.58 ms | 48.86 ms |
| short-native-3 | 51.76 ms | 49.84 ms |
| short-min-1 | 48.37 ms | 29.24 ms |
| short-min-2 | 48.38 ms | 31.81 ms |
| short-min-3 | 48.28 ms | 30.09 ms |

## Memory [memory.txt]

Peak working set: r4-auto 891 MB, r4-native 954 MB, r4-min 884 MB.
Short runs: Auto 860 to 862 MB, native 939 to 942 MB, minimum 858 to
860 MB. The Auto runs change resolution (and replace the 3D picture)
several times yet peak no higher than the fixed minimum's. This is the
real-window check on repeated resolution changes; the unit test
`a_resize_loop_keeps_one_picture` covers resizes.

## What Auto chooses on this PC

Auto's log lines (`r4-auto.stdout.txt`):
- it starts at the floor;
- in the long run it takes 5 steps and 2 rollbacks, all in the loading
  work and the resolution;
- each short run takes 1 step.

So on this PC, at 60 FPS, **Auto stays at or near its floor**: 720
lines, Low terrain and markers, Normal atmosphere (owner decision A),
shadows and bloom off. The planet scenes' work (p95 about 30 to 35 ms)
is far over the 16.7 ms target. Even the universe scenes' work (p95
17.04 ms) is over the 85 % spare line, so Auto finds no room to climb
at 60 FPS. At 30 FPS (T10 evidence, `../t10/`) it climbs in calm views.

Against fixed native, Auto's frames are faster:
- whole run p95 46.38 against 49.22 ms;
- final five minutes 34.61 against 50.53 ms.

Its work is close to the fixed minimum's (32.46 against 30.19 ms p95).
On this PC, 60 FPS is out of reach in every mode, as the owner expected
(2026-10-27). How Auto looks and feels here is for the owner to judge.

## Shots: DPI, portrait, below the floor [shots-*/]

`ASTROLITH_RENDER_SCALE=0.7`, the T6 shot file, `--timing`:

- `shots-dpi150/` (`--size 1920x1080 --scale 1.5`): the HUD is drawn at
  the 1.5 scale and stays sharp; the 3D markers sit where they do at
  scale 1.0 (compare `../t6/scale-0.7/space-l4.png`).
- `shots-portrait/` (`--size 900x1600`): portrait; the shorter side is
  900, so 70 % is raised to the 720 floor; the HUD is sharp.
- `shots-below-floor/` (`--size 960x540`): the window is under the floor,
  so the 3D scene stays native.

The PNGs stay out of git and go to the `validation-evidence` release
with the pull request.

# #1022 T3 baselines (#1060)

Today's game, before any adaptive change, on the development PC (header of
every run: `size=1600x900 scale=1.00 adapter="Intel(R) UHD Graphics 620"
backend=Vulkan present=AutoVsync workers=5 timer=cpu-work`). Built at
`a4240c4` (release). Run by `run_baselines.py`, one run at a time, in this
order: r4-normal, r4-min, short-normal-1, short-min-1, short-normal-2,
short-min-2, short-normal-3, short-min-3, cold.

- **normal**: the owner's usual settings as today's defaults give them.
- **min**: the same route with `planet_detail=low`, `marker_detail=low`,
  `shadows=off` (`r4-min.route`, `short-min.route`).
- **r4**: warm-up 60 s, then two passes of five scenarios, 120 s each
  (`../routes/r4.route`). **short**: one pass of the five scenarios
  (`../t2/short.route`). **cold**: 20 random ground placements, seed 1022
  (`cold.shots`, `--screens --shots --timing`).

Every number below is copied from the run's own file named in brackets.

## Long route (r4), whole run

| | p95 frame | p95 work | worst | frames over 100 ms |
|---|---|---|---|---|
| normal [r4-normal/route.txt `ROUTE-ALL`] | 49.13 ms | 47.54 ms | 353.11 ms | 18 |
| min [r4-min/route.txt `ROUTE-ALL`] | 47.37 ms | 38.85 ms | 315.51 ms | 6 |

First five vs final five scenarios, p95 work: normal 45.09 / 48.89 ms, min
39.06 / 38.86 ms (`ROUTE-FIRST5`, `ROUTE-FINAL5`).

## Short route, three runs each (`ROUTE-ALL`)

| run | p95 frame | p95 work |
|---|---|---|
| short-normal-1 | 50.46 ms | 48.79 ms |
| short-normal-2 | 51.21 ms | 49.23 ms |
| short-normal-3 | 50.47 ms | 48.56 ms |
| short-min-1 | 48.04 ms | 33.41 ms |
| short-min-2 | 48.24 ms | 33.13 ms |
| short-min-3 | 48.14 ms | 33.16 ms |

## Reading

- Minimum detail cuts the work a frame needs (short route p95 work about
  49 ms down to about 33 ms) but not the frame interval (p95 frame stays
  near 48 ms): with vsync on a 60 Hz screen, a frame of 33 ms of work still
  misses two refreshes. On this PC no existing setting alone reaches 30 or
  60 frames a second; the next steps (3D resolution, the controller) are
  what can move the interval.
- The worst frames (over 300 ms) fall in the universe scenario and the
  warm-up, in both normal and min.
- The universe scenario is light (p95 work 15.54 to 28.79 ms); the planet
  scenarios carry the load.

## Cold placements [cold/timing.txt]

Twenty fresh ground spots: `work_p95_ms` from 39.88 (cold-03) to 49.11
(cold-04); `ready_ms` from 1672 (cold-19) to 7794 (cold-04).

## Memory [memory.txt]

Peak working set, sampled every 10 s from outside: r4-normal 942 MB, r4-min
879 MB, short-normal 937 to 946 MB, short-min 909 to 911 MB, cold 952 MB.

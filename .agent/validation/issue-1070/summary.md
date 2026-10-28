# #1070 evidence: per-pixel atmosphere from 50 km up (Spec v1)

Release builds, real window, 1280 x 720 for the shots, the clock held.
`before-*` is `main` at 1b43678; `after-*` is `fix/1070-high-haze`. The
PNGs stay out of git and go to the `validation-evidence` release with the
pull request.

## Shots (heights above the ground; the switch counts from sea level)

`high-haze.shots` and `switch.shots`, at lat 10 lon 100 (11:49, daylight),
plus the built-in `sky` group (`*-sky/`):

| shot | before | after |
|---|---|---|
| h-1km | today's ground haze | identical |
| h-44km, h-46km, h-51km, h-53km | stripes (the table) | the same stripes: still below the switch here |
| h-55km (after only) | — | smooth |
| h-60km | stripes | smooth |
| sky-100km (built-in, 13:03) | stripes | smooth |
| sky-terminator (ground at sunset) | today's sunset | identical |

- The switch happens between 53 and 55 km above the ground at this spot,
  so the ground here lies a few kilometres below sea level, where the
  switch is counted from (Spec v1 says "about 50 km").
- Crossing the switch, the stripes go and the tone stays the same; the
  per-pixel haze is a little smoother and lighter near the horizon.
- The stripes remain below the switch (seen at 44 to 53 km here). Spec v1
  only asks for 60 km and up; they are reported, not fixed.
- The first shot run, at lat 0 lon 0 (`*-shots/`), fell in the night and
  is kept only as a record.

## Cost (`--timing`, 1600 x 900, two runs each, `*-timing-N/timing.txt`)

| run | 60 km mean / p95 | 100 km mean / p95 |
|---|---|---|
| before-timing-1 | 39.79 / 47.70 ms | 39.83 / 48.00 ms |
| before-timing-2 | 39.71 / 47.67 ms | 39.79 / 47.83 ms |
| after-timing-1 | 47.23 / 55.18 ms | 46.97 / 54.13 ms |
| after-timing-2 | 47.40 / 55.01 ms | 46.95 / 54.10 ms |

**Per-pixel is slower on this PC:** about 7 ms more per frame on average
at 60 km and 100 km, in both runs. Reported, as Spec v1 asks.
(The timing runs let the clock run; the views were at night here, so the
cost is the method's, not the picture's.)

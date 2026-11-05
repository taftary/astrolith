# #1161 correction round 1 (#1212, #1213, #1214)

Validator FAIL on 495506b (MET 19/23): https://github.com/taftary/astrolith/issues/1161#issuecomment-6666490702. This round answers C3/C15 (the moon), C14 (home snow) and the C9 evidence note.

- **Program:** `exe.sha256` (23fe1d96…), built from the moon commit 9a58131 (`commit.txt` names the branch head when the pictures were saved; the code is the same).
- **Shots:** `r1.shots`, run with `universe-app --screens <dir> --shots .agent/validation/issue-1161/r1/r1.shots`. The two `range-*` shots were taken in a second run of the same program.

## Owner decisions this round
On the Issue, owner comment https://github.com/taftary/astrolith/issues/1161#issuecomment-6666649845:
- **Q11 A:** the rare frames over 100 ms while ground loads are accepted. That covers the paired Normal run (1, at ice-low) and the warm-up (Low 2 to 4, Auto 2 to 3).
- **Q12 A:** from the planet, the moon is a true-size small grey disc (#995). Its seas and craters show up close.

## The moon (#1212; criteria C3 and C15)
**What was wrong:**
- The moon ball had only 362 points. Bevy's icosphere has `10 (s + 1)^2 + 2` points, and `MOON_SUBDIVISIONS` was 5. The T6 note's "2,562 points" was wrong.
- Most painted craters fell between the points, so the moon showed soft blobs and no round craters.

**Fix:**
- **Mesh:** `MOON_SUBDIVISIONS` is now 31, which gives 10,242 points about 2 degrees apart. The test `a_moon_ball_is_fine_enough_for_its_craters` checks the count.
- **Young craters:** 20% of the craters are young (`MOON_FRESH_SHARE`). Each has a bright bowl and ejecta, up to `MOON_RAY` 0.88 bright.
- **Old craters:** they keep a darker floor inside a faint rim.
- **Overlaps:** floors, rims and the young-crater brightness are each continuous, so edges stay smooth where craters overlap.
- **Seas:** their edges are softer.
- **Test:** `moons_are_painted_with_seas_and_craters` now also requires bright craters (brightness over 0.8).

**Pictures:** `moon-trio.png`, cropped from `pictures/`. Left to right:
- `moon-space`: the T8 view, half lit;
- `moon-lit`: fully lit;
- `moon-close`: up close.

The moon is grey with darker seas, bright young craters and dark crater floors. `moon-planet` shows the true-size disc from the home planet (Q12 A).

## Home snow (#1213; criterion C14)
`snow-sheet.png` and `range-sheet.png`, from `pictures/`:
- **`range-60k` and `range-200k`:** the range around the home planet's highest point, 6,210 m at lat 19.12 lon -114.14, found by sampling 400,000 directions. Snow covers the high ground, with brown rock and lowland around it.
- **`peak-a`, `peak-above-a` and `peak-flank-a`:** snow on the peak and its high plateau.
- **`cap-south` and `cap-south-up`:** the southern polar cap in its summer sun (lat -75 and -72, local noon), white.

`home-pole` in `../after/pictures` (lat 85 north) reads tan because it is lit by a low, warm sun.

The T8 views (`../after/pictures/home-plain` and `home-mountain`) show the green-brown lowland and the grey-blue rock seen from 3 km. Together with these views, the home planet shows lowland, rock, and snow up high and at the poles. No change to the snow line was needed.

## Evidence corrections (#1214; C9)
- `../after/README.md` now states the frames over 100 ms correctly:
  - 1 in paired after r1 (ice-low); 0 in after r2 and in both before rounds;
  - warm-up frames over 100 ms: Low 2 to 4, Auto 2 to 3 (batch);
  - High 0 to 1 (batch, Mercury ground).

  Q10 A and Q11 A accept these.
- **Timing check for this round:** `surfaces-fixed/` is one run of the Normal route on this build (see the table below).

| run (Normal route, whole-route medians) | interval ms | render ms | cpu ms | frames over 100 ms |
|---|---|---|---|---|
| before (batch, `../before/surfaces-fixed`) | 44.90 | 6.18 | 10.66 | 0 |
| T8 after (batch, `../after/surfaces-fixed`) | 37.20 | 8.27 | 21.35 | 0 |
| T8 paired after r1 / r2 | 36.41 / 39.73 | 8.18 / 7.95 | 16.67 / 13.93 | 1 / 0 |
| R1 (`surfaces-fixed`, this build) | 43.19 | 6.57 | 11.50 | 0 |

Medians are from each run's raw `route/frames.txt`, read with `fixtures/compare.py` (`all_stats`).
- **The R1 run lies inside the before and after range:** the interval median moves between the 33, 40 and 50 ms VSync steps from run to run, and the render median is the lowest of the after runs.
- **No frame over 100 ms.**
- **Mesh count unchanged:** 8 at peak.
- **Working set at each world (MB):** home 898, Mercury 902, desert 903, iron 905, ice 906, lava 908, against 897 to 908 before. The 10,242-point moon adds no measurable cost.

# #1160 T1 (#1195): baseline before any light change

- **Code:** branch `feat/1160-planet-light` at cdddbfe. This is main 5a4eed5 plus one registry row, so the game code is identical to main.
- **Build:** release `universe-app.exe`, SHA-256 in `exe.sha256` (`fe429c96…e104e2`).
- **PC:** Intel UHD 620, Vulkan, 60 Hz, AC power on Balanced. The PC was idle during the runs.
- **Fixtures:** `../fixtures/`.
  - `light.shots`: the 19 #1085 T5 shots plus `terminator` and `low-sun`.
  - Four routes: `light-fixed` (Manual, Atmosphere Normal), `light-atmo-low`, `light-atmo-high` and `light-auto`. Each runs a 30 s warm-up, then 60 s each of day, dusk, moonlit, orbit, terminator and descent.
  - Shadows and bloom are off in the fixed routes, as in #1085 rank-normal.
- **Repeat:** `bash .agent/validation/issue-1160/fixtures/batch.sh before`.

## Results
- `pictures/`: `SCREENS-OK files=22` (21 PNG plus `screens.txt`).
- All four routes report `complete: true` and `settings_unchanged: true`. The raw frames are in `<route>/route/frames.txt`, with `summary.json` and `resources.json` alongside.
- Medians and p95 per scenario are in `tables.md`.

## What the baseline shows
- **Frame interval:** it sits near the 40 ms VSync lock known from #1085 (median about 36-41 ms) in every scenario and tier. The interval therefore hides small GPU cost changes. The comparisons for AC6 use the interval median plus the **render** and **cpu work** medians.
- **Atmosphere tiers today:** Low, Normal and High cost almost the same on this PC. For example, day render medians are 4.92, 4.82 and 4.95 ms, and orbit 7.59, 7.45 and 7.77 ms. AC8 ("low measurably cheaper than high") can only be checked for any *new* cost added by #1160; the existing tiers do not separate measurably here.
- **Moonlit scenario:** its cpu work is high (about 30-36 ms median) in every fixed run, and 15 ms under Auto. This is ground building at lat -60. It is already the case before #1160 and does not count against it.

## Pictures: the problems to fix
- `dusk.png` and `dawn.png`: a hard-edged pink or orange band in the sky. Dawn also has a thin bright horizon line.
- `low-sun.png` (new): a bright wedge in the sky with straight, hard, slanted edges, against flat grey sky. This looks like a flat drawn shape, not atmosphere scattering. T2 investigates it.
- `moonlit.png`: near black.
- `orbit.png` and `terminator.png` (new): the day side is washed out to white-blue and the night side is flat black.
- `day.png`: flat ground colour, which is out of scope (Q1 A, #1161).

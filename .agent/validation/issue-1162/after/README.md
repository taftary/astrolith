# #1162 after evidence (T7, #1222)

- **Program:** the final build, `exe.sha256` (cc8ffb87…), built from 84dae0a, the commit named in `commit.txt`.
- **Before:** the f943143 build (ffc9fc87…, `../before/`).
- **Run:** `bash .agent/validation/issue-1162/fixtures/batch.sh after <program>`.
- **ABBA repeats:** `fixtures/ab.sh`, in `../paired/fixed` and `../paired/auto`.
- **Tables:** `comparison.md` and `../paired/paired.md`, written by `fixtures/compare.py` from the raw files.

## Owner decisions
- **Spec v2 answers:**
  - Q1–Q7 all A, Q8 A;
  - S1 yes, G1 yes.
- **Q9 A** (https://github.com/taftary/astrolith/issues/1162#issuecomment-6681748169): the measured water cost is accepted under criterion 9. The slower drawing of dry and ice worlds after a home visit is tracked in #1223.
- **Q9 follow-up** (https://github.com/taftary/astrolith/issues/1162#issuecomment-6683101463): Auto settling one step lower in render scale, and first arrival. The owner answered A (https://github.com/taftary/astrolith/issues/1162#issuecomment-6685170026): both are accepted under criterion 9.
- **Q10 A** (https://github.com/taftary/astrolith/issues/1162#issuecomment-6685643589): the MORPH check measures dry land only (limit 1.0 px). The coast's jump of about 1.1 px (1.081 px on the first pick) is accepted under criterion 12 and tracked in #1225.

## Pictures
`pictures/`, from `../fixtures/water.shots`, with the contact sheet `after-sheet.png`. The matched before set is `../before/pictures`, sheet `../before/before-sheet.png`, taken with the before program and the same shot list.
- **Home from orbit** (`home-orbit`): deep blue seas around the continents. The sun's glint shows on the sea, and the coast is smooth.
- **Home coast on the way down** (`home-coast-200k`, `-30k`, `-3k`, `home-coast`, `home-beach`): the coast keeps its place. The water is lighter over the shallows near the shore. Small waves move close up.
- **Night** (`home-night`): dark water.
- **On the sea** (`home-shallow`, `home-shallow-up`): flat water, with the waves close by.
- **Polar sea** (`home-polar`, `home-polar-up`): pale flat sea ice.
- **Lakes** (`home-lake-up`, `home-lake`): a round lake with flat water and the sun's glint.
- **Partly filled lake** (`home-ring-up`, `home-ring`): a lake with its pale dry ring.
- **Ice world** (`ice-*`):
  - its seas are flat frozen ground;
  - from orbit they read as smoother, bluer ice;
  - its frozen lakes are flat ice in the ice world's own colours.

  T7 took the water shader off ice worlds for speed. With it, ice-only frames cost +12.7%; without it, +1.0% (`../ab5`, `../ab-ab-ice-3`).
- **Desert world** (`desert-*`): the same low coast stays dry.

Places for the shots come from `water-home.txt`, `water-ice.txt` and `water-desert.txt` (`universe-app --list-water`).

## Speed (criterion 9, with Q9 A)
**Fixed Normal, ABBA** (`../paired/paired.md`):
- whole route **+7.0%**;
- standing on the sea +12.4%;
- home coast, from 300 m and from orbit: -4.4%, +1.0% and +2.2%;
- ice +9.1% to +15.1%, desert +4.5% to +7.8%.

**Auto, ABBA:** whole route **-4.1%**.

**Batch runs** (`comparison.md`, one run each side, hours apart, so machine drift is included):
- Normal +7.6%, Low +5.1%, High +9.0%, Auto +7.3%;
- Low stays cheaper than High: 36.49 against 47.99 ms whole-route.

**Long frames:** frames over 100 ms are about the same before and after: Normal 10/9 and Auto 5/4 in the batch runs.

**Auto settles** (`comparison.md`):
- one step lower in render scale on legs 10–13 (0.8 instead of 0.9 or 1.0) and on leg 7 (0.9 instead of 0.8); the other settings are the same.

**First arrival:**
- ABBA fixed: 2,605 ms after against 2,886 ms before (means);
- ABBA Auto: 2,786 against 2,499;
- batch: 2,871 against 2,428.

**Speed work and what was tried:** `../speed.md`.

## Memory (criterion 10)
- **Working set:** `comparison.md` lists it for each visit of the several-world route (home, ice, desert, then each again). It changes by +0.2% to +0.5% at every visit, and nothing grows with the visits.
- **Textures** (`water-fixed/resources.json`, before and after): images peak at 13 in both, so no new textures.

## Checks
`--verify` passes. Its MORPH check now compares mountains and plains on fully dry level-3 patches, since the sea is flat. On the first pick, a patch with a coast, popping read 1.081 px against its 1.0 px limit. The coast's agreement across levels is checked by the tests named in criterion 3. The owner accepted this under Q10 A; bringing the coast under 1.0 px is #1225.

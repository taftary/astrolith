# #1161 T8 (#1210): after evidence

- **Code:** `task/1210-after-evidence` at e27b785 (`commit.txt`), the full feature: T2 classifier, T3 craters, T4 ground material, T5 lava, T6 moons, T7 Help, and the T8 speed changes (ground base colour per vertex, noise without a local array).
- **Program:** `exe.sha256` (d5503f29…). Before: `../before/exe.sha256` (46ee6f4f…, commit 7005cbd).
- **Repeat (from the repo root):**
  1. `bash .agent/validation/issue-1161/fixtures/batch.sh after`
  2. `bash .agent/validation/issue-1161/fixtures/paired.sh <before.exe> <after.exe>`
  3. `py .agent/validation/issue-1161/fixtures/tables.py after`
  4. `py .agent/validation/issue-1161/fixtures/compare.py` (writes `comparison.md` and `../paired/paired.md`)
  5. `py .agent/validation/issue-1161/fixtures/sheet.py .agent/validation/issue-1161/after/pictures .agent/validation/issue-1161/after/after-sheet.png`
- **PC and settings:** Intel UHD 620, Vulkan, 60 Hz, AC power on Balanced, idle. The same as before.

## Owner decisions after Spec v2
- **Q9 A** (iron worlds): iron worlds keep their air and their shape, so they have dark metal ground with no craters. Real craters stay on airless worlds; moons have painted ones.
- **Q10 A** (https://github.com/taftary/astrolith/issues/1161#issuecomment-6664131299): the remaining orbit cost on Normal and the High-quality scene costs are accepted under criterion 9.
- **Q10 follow-up A** (question https://github.com/taftary/astrolith/issues/1161#issuecomment-6665866148, answer A https://github.com/taftary/astrolith/issues/1161#issuecomment-6666173576): Auto settling one step lower on part of the route is accepted as part of Q10 A. See the Auto section below.

## Pictures (criteria 1, 3, 6, 7, 8; test plan steps 1 to 5)
`pictures/` (`SCREENS-OK files=39`) and `after-sheet.png`, matched shot for shot with `../before/before-sheet.png`. Rows: home, Mercury, desert, iron, ice, lava, and the home moon. Columns: plain, mountain (3 km up), pole, cube edge (lat 0 lon 45), orbit, night.

| world | before | after |
|---|---|---|
| home (Earth-like) | pale grey everywhere | green-brown lowland with mid-scale variation; grey-blue rock seen from 3 km; from orbit, continents in green and brown |
| Mercury (desert, airless) | pale grey, no craters | rust and sand tones; a crater bowl with rim and floor in the mountain view; craters and maria on the disc from orbit; white polar cap |
| desert | pale grey | sand with fine ripples near the camera; pink-tan disc; white polar cap |
| iron (Q9 A) | pale grey | dark metal-grey ground with fine detail; no craters |
| ice | pale grey | white ice with long blue-grey cracks, seen from 3 km and on the ground; white-blue disc |
| lava | pale grey; dark night side | dark crust with orange glowing cracks and low fields; the night view glows in a network of cracks; orange-red disc with dark crust |
| home moon | smooth grey ball | grey ball with darker seas and bright craters, from space and from the planet |

- **Cube edge:** the edge views show continuous ground with no seam on every world. The test `the_close_detail_meets_across_patch_edges` checks the close detail meets across patch edges to within 1 cm.
- **Close detail:** it fades in near the camera (the plain views) and is absent in the 3 km and orbit views, with no visible step in any picture. Its reach is Low 150 m, Normal 300 m and High 600 m (test `terrain_quality_sets_how_far_the_close_detail_reaches`).
- **Lava glow:** it uses the own-light (emissive) input, not the sunlight. It stays steady, since the shader reads no clock (test `only_lava_has_its_own_light_and_it_stays_steady`). The night view shows it clearly.
- **Pole views, pre-existing:** the home, ice and lava pole pictures show a rectangle of ground in a different shade near the camera. The same rectangle is in the before pictures (`../before/before-sheet.png`, pole column), so this work did not cause it. Home's polar cap reads tan because the pole sun is low and warm (#1160 light).

## Speed (criteria 9 and 11)
All numbers are copied by script from the raw run files: `comparison.md` (four batch routes, before vs after), `../paired/paired.md` (alternating repeats), and `tables.md` and `../before/tables.md` (means, stalls, readiness and resources).

- **Paired repeats, Normal (`surfaces-fixed`, two rounds each, alternating):**
  - The whole-route interval median is +0.2% (`paired.md`, row "all").
  - Orbit views are +1.0% to +4.0%: home +4.0%, Mercury +3.9%, desert +3.9%, iron +3.9%, ice +1.0%, lava +1.1%.
  - Render costs about 1 ms more per frame on average (+18.5% for the whole route). That is the ground material's cost, and Q10 A accepts it.
  - No frame over 100 ms in any paired run.
- **Batch routes (one run per side, different times):**
  - The interval medians move with the VSync lock (33, 40 or 50 ms steps), so these single runs are noisier than the paired ones.
  - Normal −17.1%, Low −16.2% and Auto −21.3% (more frames hit the 33 ms step).
  - High +8.8%, with one frame over 100 ms (Mercury ground). Q10 A accepts the High cost.
- **Low against High (S1):** Low is cheaper. In `tables.md` the whole-route means are interval 37.65 against 48.59 ms and render 8.16 against 9.10 ms. The close detail's reach is set by Terrain quality, and Auto drives it through terrain.
- **First arrival:** the warm-up readiness in the paired runs is the same within noise:
  - before: 2,554, 2,355 and 2,525 ms;
  - after: 2,675, 2,253 and 2,560 ms;
  - mean 2,478 against 2,496 ms.

  The batch runs read 2,168 to 2,770 ms on Normal, but they ran at different times and the paired runs separate that noise.
- **Auto:**
  - In the paired Auto run, Auto keeps render scale at 0.9 instead of 1.0 on part of the route (legs 8 to 19), and Terrain on Low where the before build briefly chose Normal (legs 16, 24 and 25).
  - The batch Auto run shows the same pattern (`comparison.md`, "Auto settles").
  - This is the same ~1 ms of drawing cost, and Auto lowers settings to stay smooth. The owner accepted it (Q10 follow-up A, above).

## Memory (criterion 10)
- **No new textures:** the image count is 7 first, 13 last and 13 peak on every route, as before. No new meshes either (8/8/8). Ground materials stay at 32 at most, and terrain memory is the same (15.9 MB last, 17.4 MB peak on Normal).
- **Several worlds:** the `surfaces-fixed` route visits all six fixture worlds, one of each type plus Mercury. The median working set while at each world is:

  | world | before (MB) | after (MB) |
  |---|---|---|
  | home | 897 | 894 |
  | Mercury | 901 | 899 |
  | desert | 903 | 900 |
  | iron | 904 | 900 |
  | ice | 906 | 902 |
  | lava | 908 | 904 |

  It is −0.3% to −0.5% at each world. It rises by about 2 MB per world on both sides, the same before and after.
- **What the change adds:** one uniform (`GroundLook`, binding 102) per ground material, written only when the world changes; a world type per open planet; and vertex colours on the moon meshes, built once per open cell. It adds no textures, render passes or per-world resources.

## Checks (criterion 12)
On the final head e27b785, all pass:
- `cargo fmt --all --check`;
- `cargo clippy --locked --workspace --all-targets -- -D warnings`;
- `cargo test --locked --workspace`, the quick tier: universe-core 546 passed, universe-render 296 passed, no failures;
- `cargo doc --locked --workspace --no-deps` with warnings as errors;
- `cargo run --locked -p universe-app -- --verify` (`VERIFY-OK`).

CI and the validator run the slow tier on the PR head.

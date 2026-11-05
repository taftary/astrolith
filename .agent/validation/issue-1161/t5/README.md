# #1161 T5 (#1207): lava glow

**Code (`patch_grid_fragment.wgsl`, `lava_glow`):** on lava worlds only (`GroundLook.glow.w` = `LAVA_GLOW` 2.5, 0 elsewhere), the ground sets its own light (`emissive`). That input was kept separate for this in #1160 C5, so the surface colour, the sun and the moons are untouched. Molten ground is drawn darker where it glows.
- **Glow pattern:** cracks at two scales (the ridges of a noise field on the body direction, with 1,500 m and 250 m cells), plus the low lava fields under the plains at 40%.
- **Far view:** a crack network finer than a few pixels fades to its mean glow, so the far view stays calm. An earlier try speckled from 20 km up; `lava-sheet.png` is the final version.
- **Steady:** no clock is read, so nothing animates (Q6 A). An emissive input lights only its own pixels, so no other world is lit.

**Test:** `ground_look_tests::only_lava_has_its_own_light_and_it_stays_steady`. Only lava has an own light, it is orange, and the fragment shader reads no clock.

**Pictures:** `pictures/` (`lava.shots`, `SCREENS-OK files=11`) and `lava-sheet.png`. Top row: day plain, day orbit, night at 200 m. Bottom row: night at 20 km, night orbit, iron night (no glow).
- **At night:** glowing crack networks on black crust.
- **From high up:** the network blends into a steady glow, and the lava seas in the low basins glow orange (the guide's lava row: "a sea of lava filling low ground").
- **By day:** the crust is dark with a faint glow.

**Checks:** workspace tests and clippy pass; `--verify` gives `VERIFY-OK`.

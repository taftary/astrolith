# #1161 T3 (#1205): craters as height on airless worlds

**Code:** `relief_craters.rs` adds a crater layer to `Relief`, only when the world has no air (Q4 A).
- **Placement:** 7 size classes. Each class lays a 3D cell lattice over the unit direction, and half its cells hold a crater, which gives the saturated `D^-2` count.
- **Depth:** `0.2 D` for simple craters, `1.044 D^0.301` km for complex ones (with a flat floor). The switch is at `D_tr = 16533.8 g^-1.0303` m.
- **Rim and ejecta:** a rim of `0.04 D` (at most half the depth), and ejecta as `(R/r)^3` out to two radii.
- **Lookup:** 2 x 2 x 2 cells per class, with no allocation.

**Tests** (`relief_craters_tests.rs`, all pass):
- `worlds_with_air_keep_every_bit_of_their_heights`: every air world among the first 60 listed reads the frozen reference maths bit for bit (criterion 4).
- `airless_worlds_carry_craters_within_their_bounds`: Mercury has bowls deeper than 1 km, and every height stays inside `height_bounds_m` (criterion 5).
- `the_crater_profile_has_no_step`: the bowl meets the rim, the rim meets the ejecta, and the ejecta reach zero. The depths follow the guide.
- `craters_match_across_every_cube_edge`: an edge point reached from two faces gives the same height bits (criterion 5).
- `the_player_walks_a_crater_wall_with_feet_on_the_ground` (slow tier): walking, running, a jump and the landing on Mercury's steepest sampled crater wall all keep the feet on the drawn ground, within the `GROUND` limit of 0.01 m (criterion 5). `headless_ground` now uses the world's own radius (Earth unchanged: `GROUND ... PASS`).
- `craters_cost_a_bounded_share_of_a_height_read`: in the debug test, 20,000 reads take 29.5 ms for the base height and 37.9 ms for the crater layer. That cost applies only on airless worlds (Mercury is the one airless planet among the 4,000 listed).

**Other:** the classifier module is renamed from `surface` to `world_type`, because the #770 test forbids a `mod surface`.

**Pictures:** `pictures/` (`craters.shots`, `SCREENS-OK files=8`) and `mercury-before-after.png`. The top two rows are before and the bottom two after, each in the order plain, mountain, pole / edge, orbit, night.
- **From 3 km up:** bowls with rims and ejecta, from about 1 km to tens of km across.
- **From orbit:** they barely show. T4's material shading (slope, rim) is what makes them read from far.
- **Home:** home-plain is unchanged (air world).

**Checks:** workspace tests, clippy `-D warnings`, fmt and docs pass; `--verify` gives `VERIFY-OK`.

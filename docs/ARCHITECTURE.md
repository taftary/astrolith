# Architecture

Bird's-eye view, entry points, and codemap of this repository. Anyone
asking "where is the thing that does X" should find the answer here in
under a minute.

Where is the code that decides when a marker opens? `universe-core`, the
`nav` module, `should_open`.

## Bird's-eye view

A Bevy application that renders a nested dive through universe scales
L1-L10. Four crates: `universe-core` holds deterministic universe logic
(std only), `universe-render` holds Bevy indicator rendering,
`universe-app` holds the window binary plus the headless `--verify` and
`--capture` drivers plus the windowed `--screens` driver, and `universe-verify` is the Bevy-free headless gate
printing the same report. Dependency direction is `core <- render <- app`
with `universe-verify` depending on `universe-core` only.

**Architecture Invariant**: dependency direction is `core <- render <- app`,
never otherwise.

**Architecture Invariant**: `universe-core` knows nothing about Bevy, wgpu,
winit, or any rendering or windowing crate.

**Architecture Invariant**: determinism. The same root seed and path
produce byte-identical snapshots on every run and platform (three tiers in
`E-DET-TIERS`).

**Architecture Invariant**: bounded memory. At most `STREAM_CAP` (32) cells
are live at any time (open chain, horizon, previews).

Universe reference docs live under `guides/universe/`; the frozen ladder
and pins stay in `docs/universes/`. The L1 visual and lifetime reference
index is `guides/universe/L01/documents/README.md`, the
L2 visual and lifetime reference index is
`guides/universe/L02/documents/README.md`, and the L3 visual and lifetime
reference index is `guides/universe/L03/documents/README.md`, the L4
visual and lifetime reference index is
`guides/universe/L04/documents/README.md`, and the L5 visual and lifetime
reference index is `guides/universe/L05/documents/README.md`, and the L6
visual and lifetime reference index is
`guides/universe/L06/documents/README.md`, and the L7
visual and lifetime reference index is
`guides/universe/L07/documents/README.md`, and the L8
visual and lifetime reference index is
`guides/universe/L08/documents/README.md`, and the L9
visual and lifetime reference index is
`guides/universe/L09/documents/README.md`, and the L10
visual and lifetime reference index is
`guides/universe/L10/documents/README.md`, and the L11
visual and lifetime reference index is
`guides/universe/L11/documents/README.md`, and the L12
visual and lifetime reference index is
`guides/universe/L12/documents/README.md`, and the L13
visual and lifetime reference index is
`guides/universe/L13/documents/README.md`, and the L14
visual and lifetime reference index is
`guides/universe/L14/documents/README.md`.

## Entry points

- Window: run `universe-app` with no flags. Opens the `Universe MVP`
  window: hover highlights a marker, click targets it (and deselects the
  player, #884), wheel or arrows dive, Spacebar runs the autopilot journey
  L1-L10 (then 10 m above the selected player, #884), `F` toggles free
  flight (WASD plus right-drag look with no pitch limit, Q/E roll at the
  roll speed, `Shift+1-9` steps speed, `Ctrl+1-8` saves a view, `1-8`
  recalls it), P toggles the player selection, Esc opens the menu (Resume,
  Guides, Settings, Quit; #473). Settings and saved views persist in the
  platform config folder (`%APPDATA%\Astrolith\settings.toml` on Windows).
  A wrong command line prints one usage line to stderr and exits 2.
- Headless verification: `universe-app --verify`. Replays the autopilot
  from the fixed root seed, regenerates every opened cell twice,
  byte-compares, checks the open/close inverse and the pre-entry preview,
  prints canonical snapshots, exits 0 on `VERIFY-OK`, 1 on `VERIFY-FAIL`.
- Headless capture: `universe-app --capture <dir>`. Replays the same
  journey and writes one `level-L<N>.txt` plus one `frame-L<N>.ppm` per
  level plus `capture.log`. Exits 0 on `CAPTURE-OK`, 1 on `CAPTURE-FAIL`,
  2 for a missing `<dir>`.
- Screens mode: `universe-app --screens <dir>` (ADR 0021). Opens the real
  window with default settings, shows each UI screen from the render shot
  list, saves one PNG per screen plus `screens.txt`, exits 0 on
  `SCREENS-OK`, 1 on `SCREENS-FAIL`. Local only (needs a graphics adapter).

## Codemap

### `universe-core` (`crates/universe-core/src/`)

Pure deterministic universe logic. std only. Public API is the workspace's
stable surface (`E-CORE-API`).

- `coords`: `Level` (validated newtype, the pattern for `E-NEWTYPE`),
  `Frame`, neighbor and zoom arithmetic.
- `seed`: deterministic seed derivation and the small PRNG.
- `nest`: marker-tree nesting (R6), budgets, ratios, brightness curves,
  path seeds, preview sets.
- `frame`: observer-frame geometry: orientation, planet radius, horizon
  (split from `nest` under `E-FILE-SIZE`, #394), and `Opened` with its
  point and direction maps across a frame (moved from `nest`, #403); a
  planet's entry also turns by its `spin` about its `axis` (`turn_about`,
  #981, ADR 0052). Local
  up blends from the planet axis to the ground normal by altitude at L10
  (`local_up`, `UP_BLEND_NEAR/FAR`, #430). Radius is the true radius from
  the record through the one L10 metre (Earth about 0.49, #774).
- `planet_frame`: one planet-centred metre frame per L10 planet (#774):
  `PlanetPos` (metres, `f64`), `PlanetFrame::for_planet` (radius from
  `body_recipe`, ECEF-like, no spin), `from_cell`/`to_cell` (the one place),
  `LocalFrame` (up, north, east), `LocalPos`, `CameraRel`,
  `l10_camera_offset` plus `l10_drawn_m_f32` (the exact render path the
  window shares, camera-relative metres since #780 F2 A), and saved-view
  conversion.
- `precision`: planet-frame shake measurement (#774): `shake` (worst mm
  within 1 km, pixels at 1600x900 farther, over heights and face
  centre/edge/corner) and `shake_all` (Earth plus two seeded planets),
  printed as the headless `PRECISION` line.
- `cube_sphere`: tangent-adjusted cube-to-sphere mapping for the L10 planet
  (#775): `Face` (`PosX`/`NegX`/`PosY`/`NegY`/`PosZ`/`NegZ`, `+Y` north),
  `FacePos`, `to_sphere`/`from_sphere` (exact inverse), `grid_point` with
  canonical edge folding, `test_height`, and `spacing` (min/max/ratio plus
  centre/edge/corner), printed as the headless `CUBE-SPHERE` line.
- `lod`: quadtree LOD keys, geometry and error functions per cube face
  (#776, split under `E-FILE-SIZE`; relief-aware selection #799, morph #777):
  `PatchKey`, `patch_screen_error_px` and
  `patch_screen_error_with_relief_px` (`max(quad_sag, relief_error)` over the
  relief-grown nearest distance).
- `lod_select`: quadtree selection per cube face (#777 T1, split from `lod`):
  `QuadTree` (six roots, split above the threshold and merge below half with
  the one relief-aware error, merges only when the parent data is known,
  one-level neighbour rule across faces, `max_level` per planet, Earth 18),
  `UpdateScratch`, `NeighbourDir`, `edge_mask`/`edge_masks` (Q3 A, via
  `lod_morph::morph_mask_in`),
  `draw_selection` (parents stay), `update_with_relief_reused` and the
  culled `update_culled_reused` with `append_visible_leaves_to` (#778,
  ADR 0037: hidden leaves never split, hidden parents merge). Since #817
  (ADR 0038) the split and merge rule is CDLOD: split when the patch box
  is under the next finer level's range (`with_ranges`), merge at its
  exact complement. Since #948 (T2) `append_unbuilt_cover_to` lists the
  leaves with no built mesh outside the build list, so the background
  queue can drain hidden cover last.
- `lod_box`: a patch's box at its real height range, aligned with the
  patch (#817, ADR 0038): `PatchBox` (axes, extents, `distance`) and
  `patch_box` (3 × 3 directions at the lowest and highest relief, grown
  by cap bulge and parent chord sag; a lower bound of every vertex
  distance).
- `lod_range`: CDLOD ranges (#817, ADR 0038): `LevelRanges` (roughness
  lattice per face, `scale_at`, `scale_max`, `range_m`), `edge_m`,
  `scale_for`, `scale_cap` (1× to level 6, ×1.41 per level to 4×) and
  `scale_at_dir`/`scale_at_dir_cached` for the patch build.
- `lod_cull`: frustum and horizon visibility for the selection (#778,
  ADR 0037): `View`, `Bounds`, `patch_bounds` (conservative sphere over the
  cap, chord sag and parent shape), `above_horizon` (lowest-relief
  occluder), `patch_visible`, and `Cull` (view, built heights, relief
  bounds fallback, occluder radius).
- `lod_ring`: the ground ring and the build order (#918, ADR 0049):
  `RING_M` (500 m), `box_distance_m` (relief box distance, independent of
  the view), the nearest-first build list with the ring's leaves first, and
  `next_ring_m` (the ring shrinks while what it needs is over the memory
  cap, `RING_MIN_M`).
- `headless_ring`: headless `RING` line (#918 T2, AC2, AC4): `ring_metrics`
  and `verify_ring` (2 m over the plains, a full turn in 30° steps: 0
  builds and 0 ring changes, first build the nearest leaf).
- `lod_morph`: per-vertex CDLOD geomorph rule (#777, ADR 0036), the one
  formula the shader mirrors: `vertex_factor` (own distance, band ending at
  `err x focal / detail` where the parent splits, last 30% blended, Q7 A;
  pinned on coarser edges, held on finer edges), `morph_mask_in` (coarser
  and finer edge nibbles against any drawn set, `O(depth)`),
  `odd_edge_bits` and `relief_normal` (terrain normals from the relief).
- `lod_patch_morph`: per-patch morph shape (#777, split from `lod_mesh`):
  parent positions (odd-odd on the parent's drawn diagonal), parent and own
  terrain normals, per-vertex morph errors (edges from their own points),
  skirts, and `blended_vertex` (one vertex as the shader draws it); a
  tile-only build skips the normals the shared grid makes itself (#920).
- `grid_rebuild`: the shared grid's `f32` rebuild (#920, ADR 0050):
  `GridFrame`, `offset_from_steps` (offsets from the patch centre from small
  differences only, as the vertex shader runs it), `exact_offset` and
  `naive_offset` for the precision probe (`headless_grid_precision`).
- `grid_tile`: one patch's tile in the shared heights texture (#920, ADR
  0050): the layout (37 x 63 texels of 32 bits, 9,324 bytes), `grid_tile`
  and `build_patch_and_tile` (packed from the heights the build sampled,
  full or tile-only), `f16_bits`, `pack_normal`, and the CPU mirrors the
  tests hold the shader to (`tile_offset`, `tile_morph_err`,
  `tile_parent_normal`).
- `system_space`: one star system in one space at true scale (#995
  study, ADR 0051): `SystemSpace`, `SystemBody`, `system_space`, masses
  (`body_mass_kg`, `kepler_mass_kg`), gravity zones (`laplace_radius_m`,
  `influence_radius_m`, `zone_nesting`) and true-scale sizes
  (`screen_radius_px`, `height_for_px`), with the study probes.
- `system_handover`: star, orbit and spin frames with enter/leave bands
  (#995 study): `Frame`, `locate`, `to_star`, `frame_for`, and the
  handover and precision probes.
- `headless_handover`: the `HANDOVER` verify line (#995): the gravity-zone
  open and close jumps, the turn, the distance change and the arrival sun.
- `clock`: the sky's clock (#981, ADR 0052): `Clock` (game seconds, speed
  step, play/pause), `GAME_PER_REAL` (one Earth day in ten minutes at 1x),
  `SPEED_STEPS`, `speed_label`, the pure `orbit_angle_at` and
  `spin_angle_at`, and the home calendar `readout`.
- `sky_motion`: bodies moving with the clock (#981, ADR 0052): `Mover`
  (rebuilt only on a regenerate), `Universe::advance` (planets around their
  star in L8 and L9, moons around their planet in L10, the open planet's
  entry riding its planet and turning with its spin about its tilted axis)
  and `Universe::sync_zones` (a moving gravity zone reaching a still camera
  opens the planet).
- `headless_ride`, `headless_clock`: the `RIDE`, `ORBIT` and `CLOCK` verify
  lines (#981): riding a month, the zone sweep, periods and the day's
  sun turn.
- `lod_mesh`: patch meshes on the quadtree (#776, T2; relief #799; parent
  shape and skirts #777 T2): `PatchMesh` (33 x 33 points on #775's mapping,
  `(radius + h) x dir` with `f64` centre plus `f32` metre offsets, Q5 A;
  `parent_offsets`/`parent_normals`/`morph_err_m` for geomorphing, skirt
  rings below every edge at `max(4 x patch error, 1 m)` with their own
  indices, built by `lod_patch_morph`; `full` false for a tile-only build,
  #920) and
  `build_patch` (pure, any thread), recording `relief_error_m`
  (double-resolution gap) and the height range for the relief-aware error.
- `relief`: deterministic terrain relief on the cube-sphere (#799):
  `Relief::for_planet`/`for_recipe` (seed, gravity scale), `height_m` (one
  pure `f64` function on the unit direction: continent mask, hybrid hills,
  belted ridged ranges, warping; giants zero), `ReliefTint`
  (Deep/Low/High/Snow) and `tint_for_height` (headless only since #892: the
  window draws one ground colour).
- `headless_relief`: headless `RELIEF` line for the relief (#799, split under
  `E-FILE-SIZE`): highest/lowest, land share, modes, gravity bound, seam and
  pole checks.
- `headless_morph`: headless `MORPH` line for cracks and morphing (#777 T4,
  #817): scripted 2,000 km to 1 m and back at 60 fps at the default flight
  speed through the culled CDLOD selection, builds and per-vertex blend;
  worst popping (px) and normal change (degrees) over the whole path,
  cracks and worst edge gap, mountain versus plains depth and still-camera
  stability; `build_keys` builds on every core.
- `headless_morph_measure`: blended frames and their measures, matched by
  grid position (#777 review): `crack_stats` (shared edge points and
  T-junctions) and `surface_change` (popping and normals against the
  covering patch, both ways).
- `headless_lod`: headless `LOD` line (#776, moved from `headless` under
  `E-FILE-SIZE`); `cross_level` is the real crack count on its descent.
- `headless_cull`: headless `CULL` and `DEPTH` lines (#778, ADR 0037): the
  culled #777 descent and climb looking 20° down, builds capped at
  `BUILD_BUDGET`, worst patches, triangles and builds, culled leaves, and
  the drawn-hidden and visible-dropped counts (must be 0); `f32` reverse-Z
  depth separation at 1 cm, 1 m and 1,000 km with the real near rule.
- `drawn_surface`: the ground as drawn (#779, ADR 0039): `DrawnSurface`
  (drawn patches as shared meshes, their edge masks and the morph camera),
  `ground_at` (finest drawn patch under a point, its triangle blended with
  `blended_vertex` as the shader draws it, radial ray), `place_on_ground`
  and `set_camera`.
- `body_frame`: the body's local tangent frame near the ground (#779,
  ADR 0039): `step_near_ground` (below `LOCAL_BELOW_M` the step runs in a
  `LocalFrame`, re-anchored at `REANCHOR_M`) and `BodyAnchor`.
- `headless_ground`: headless `GROUND` line (#779): 100 m walk and 100 m
  run with one jump over the mountain slope and a plain, through the CDLOD
  selection and the drawn surface; feet, landing and frame-switch gaps.
- `home`: journey matching, home fixtures, and the autopilot (split from
  `nest`, #156).
- `gen`: pure generator contracts, marker forms, portal offsets, the sphere clamp.
- `density`, `astro`, `terrain`: L1-L3 density, galaxy-to-star sampling,
  L10 terrain (bare planet since #375; one surface record from the pure
  `body_recipe`, anchor-tile region portals at L9 and L10, ADR 0025). The
  planet has no relief (#593): it is the icosphere split once, 80 flat
  faces with corners on the sphere (`FACET_DEPTH`, `facet_plane`,
  `surface_radius`, `surface_floor_radius`), and finer splits lie flat
  inside those faces. Since #995 (ADR 0051) `planet_recipe` sizes a
  planet from its system's `PlanetInfo`, moons sit at their true orbits
  and sizes, and `kepler_period_days` gives their periods.
- `ground`: removed (#770, ADR 0031); the ground is the drawn surface since #779 (`drawn_surface`, ADR 0039).
- `landing`: removed (#770, ADR 0031); `DiveMode::Landing` behaves like `Targeted` until the rebuild.
- `tail`: deleted (#742, T0): building/room generation, collision arms,
  outlines, furniture, and their tests are purged; L11-L14 stay numbered
  but unreachable, falling through to the uniform scatter.
- `icosphere`, `surface`: removed (#770, ADR 0031); the old flat-faced surface is gone.
  and from the open L10 cell with the navigation's own arithmetic,
  answers the ground height, and grows each cell's nested clusters
  (#458, ADR 0024, superseded by 0028; the detail window samples it).
- `system`, `sysgen`: L5-L8 stellar and planetary sampling data plus the
  L4-L9 indicator generator (counts, portal order, home fixtures).
- `layouts`: L4-L6 marker layouts (galaxy forms, cloud rings, star radii;
  split from `sysgen` under `E-FILE-SIZE`, #384).
- `orbits`: L7-L9 system layouts (rings, shells, orbits, star bodies, #384). #995 (ADR 0051): `metres_per_cell`, `true_cell` and `true_radius_cell` place L8 and L9 planets, belts and the star at true scale; `planet_slot` gives the home journey's planet the Earth's slot; `orbits_check_l8` checks true scale.
- `snapshot`: canonical text snapshots (`snapshot_generated`).
- `cache`: fixed-capacity LRU cell store.
- `verify`: determinism and border-agreement predicates.
- `noise`: deterministic noise helpers.
- `player`: where the player stands (#607, #873; relief surface plus 2 m #799):
  `PlayerSpot` (the named chain of the body's cell, its feet and heading),
  `seeded` (a planet and a spot on its relief surface plus 2 m, fixed per
  seed, on the morning side under its sun, #780 F1 A: `MORNING_RAD`,
  `MORNING_SPREAD_RAD`, `morning_point`), `destination` (anchored, so the autopilot rises back out instead of
  re-aiming), `arrival_pose` (10 m straight above along planet up looking
  down, #873 Q6 C 10 m, replacing #607 Q3 A behind-and-above) and `is_near`,
  plus the one third-person offset (`THIRD_BACK`, `THIRD_UP`) and
  `third_person_camera` (#754, Q4 A).
- `figure`: the player's low-poly figure (#755): `FigurePart` (head, torso,
  arm, leg), `FigureTriangle` with face normals, `figure_triangles` (full)
  and `first_person_triangles` (no head), `is_accent`, `HEAD_BOTTOM/TOP/HALF`
  and `FIRST_PERSON_NEAR_SHARE` (the camera near share).
- `light`: the planet's one sunlight (#780, ADR 0040): `star_direction`
  (planet to its star at L10, the camera up-right fallback above),
  `sun_direction` (the planet's sun at L10 only, moving with the sky's
  clock since #981), `from_sun` (the place
  where the sun stands an angle from overhead), `SUN_ILLUMINANCE`,
  `NIGHT_SHARE` (the moonlit night floor, 28% of daylight, #908; was
  about 7%) and `OFF_PLANET_FLOOR`; the eyes' adaptation at the planet
  (#908): `view_light_share`, `adapted_ev_offset` (`ADAPT_SHARE`, half the
  stops to noon) and `ease_ev` (`ADAPT_TAU_S`, about 5 s).
- `atmosphere`: one air model per planet (#780, ADR 0040): `AirModel`
  (Rayleigh, Mie, ozone and ground albedo from the recipe's air, bottom at
  the relief low bound, sea-level density below sea level), `densities`,
  `densities_at_falloff` (what the window's medium reads), and `sample`, a
  CPU single-scattering reference (`SkySample`: sky overhead, ground,
  haze).
- `headless_light`: headless `LIGHT` line (#780): the #777 path from
  2,000 km to 1 m and back under the fixed sun, the largest frame step of
  sky, ground and haze against `STEP_LIMIT`, the night-to-day ground ratio,
  and a bit-for-bit repeat.
- `motion`: gradual velocity and heading response for the player body
  (#756): `approach` (starts, stops and turns bend instead of snapping),
  `turn_toward` (a half turn in about a third of a second), the
  acceleration and braking times, and `inertia_rate`.
- `nav`: `Universe`, `DiveEvent`, `DiveMode` (`Passing`, `Targeted`,
  window-only `Landing`, #430), `dive_step`, `should_open`,
  `should_close`, `crossed_portals`,
  `sibling_in_open_units`, `drawn_radius`, `open_marker_radius`,
  `JourneyStep`, `replay_autopilot`, navigation constants, `Seed`,
  `MarkerIndex`, the allocation-free `sync`, and the analytic preview
  context (`preview_surface_context`, #430). Cached L10 frame/relief plus
  space (`planet_space`, `planet_frame`, `planet_relief`); dive slides
  around the planet and never drops below 3 m above relief (#873 T3-T4). #995 (ADR 0051): `DiveMode::Journey`, `planet_zone_m`, `l8_planet_zone`, `planet_zone_cells_for_chain`, `ZONE_LEAVE_FACTOR`, `Universe::open_planet` (star close-up and planet in one exact shift from the gravity zone) and `journey_target`; in a planetary system nothing opens by angle and the planet closes past 1.2 zones.
- `dive_orbit`: dive orbit and turn math (#873, ADR 0045; turn ease #884,
  ADR 0047): `dive_orbit` (same-distance circling, `DIVE_ORBIT_RATE`,
  `orbit_rate`), `advance_roll` and `roll_up` (in-place turn),
  `ease_roll` (`ROLL_TAU_S`, stops exactly, no drift after release),
  `slide_around_sphere` and `planet_avoid_radius` (no pass-through,
  `PLANET_CLEARANCE_SHARE`), `clamp_above_ground` (3 m floor,
  `GROUND_CLEARANCE_M`, `PLAYER_ARRIVAL_ABOVE_M` 10 m). Real ground
  (#892, ADR 0048): `lowest_ground_avoid_radius` (the avoid ball sits
  below the lowest ground, no sea level), `ground_radius_cells` and
  `ground_dive_step` (the centre dive and the orbit measure their gap from
  the ground under the camera, so they slow near it and rest at 3 m).
- `preview`: pre-entry preview cache (`PreviewCache` plus its background
  `merge`), preview-set computation, `preview_positions`, `preview_error`. Planets are never previewed (#995).
- `stream`: streaming working set (`STREAM_CAP`, `StreamLedger`,
  `working_chains`, proximity-first unload: farthest first, recency then
  chain order on ties; #742 T5).
- `destination`: the camera's final destination (#423): `Destination`,
  the seeded surface chain from L1 to the L10 planet along the journey
  picks, `Way` (next marker on the way, arrived, off), the click rule
  (targets but never opens at L10), and the allocation-free per-frame
  `on_the_way`. At L8 the way through the star close-up names the planet itself; clicking a planet or the star targets it (#995).
- `flight`: free-flight synthesis (`FreePose`, `FreeKeys`, look, speed,
  `nearest_portal`; spaceship #884, ADR 0047: `free_rotate` with no pitch
  clamp for a full loop, `free_roll` at `free_roll_rate` (`FREE_ROLL_RATE`
  times the roll speed), `free_right`, `free_step_axes` along the view's
  own axes, gravity never turns the view), the pure heading
  helpers the window camera carries across frames (`ease_heading`,
  `orthogonal_up`, #403), and the scripted leg (`FreeLegReplay`,
  `replay_free_leg`). `nearest_body_distance` (#995: the near plane in a planetary system).
- `gravity`: real gravity from every massive body (#559): real masses
  (`mass_kg`, `planet_mass_kg`), the summed pull and the strongest body
  (`pull`), the planet under the body pulling with its real surface
  gravity.
- `body`: the player body in normal mode (#558): true-size walk, jump,
  fly, drone or plane turning (`body_step`), the ground under each level
  (`level_ground`), collisions with buildings, room walls and furniture
  (`resolve_collisions`), and places opening around a walking body
  (`walk_levels`); since #559 it falls under the real pull with its
  momentum (drift, orbit, falling opens places). `body_step_scaled` is the
  one step with a metre scale (#779, used in the local frame).
- `keymap`: one key or mouse button per action (#560; roll + exposure
  #884, ADR 0047): `Action` (the 28 actions, with the time keys T, [ and ]
  since #981, including `RollLeft`/`RollRight`
  Q/E and `Exposure` on B with migration of old E files), `Binding` (a name
  from the fixed `BINDING_NAMES` table, so the core stays Bevy-free), and
  `Keymap` with the spec defaults, `bind` (a taken key swaps), and `apply`
  (saved `key.<action>` lines; a damaged entry keeps its default).
  `Settings.keymap` carries it.
- `touch`: touch layout and gesture math (#561): `Layout` (the stick and
  round buttons for a window size and hand, above the scale readout),
  `Shown` (which controls each mode shows), `stick` (directions, the run
  edge), `pinch_log_factor`, `is_tap`.
- `labels`: `level_label`, `scale_label`, `scale_anchor`,
  `window_title_for_level`, `next_level_line`, `tile_note`, `ground_ruler`
  (#543); the panel's ground heights (#892, ADR 0048): `ground_level`
  (`GROUND_LEVEL_FLOORS_M`: L11 from 100 km, L12 from 1 km, L13 from 10 m,
  L14 below), `ground_label`, `height_line`, `ground_scale_label`. `STAR_CLOSE_UP_RADII`, `system_level` and `nearest_line` (#995: L9 by distance, the nearest body).
- `contrast`: WCAG luminance and contrast against black, the Machado
  protanopia simulation, the two-stop ladder check, and the display tint
  map for dark reds (#446).
- `names`: seeded marker names, home-chain real names, surface `"<kind> <n>"`
  names, and `"kind - name"` label text (#446). A planet's L8 marker takes its close-up name (#995).
- `pick`: pick ranking (destination portal first, then nearest), repeat-click
  cycling, the label rule, and the pick/label size tokens (#446).
- `pacing`: the frame cap's rules (#1022 T7, ADR 0054): `cap_plan` (no
  cap when vsync paces a matching display, the cap at whole multiples and
  uneven refreshes), `pacing_note` (plain words for an uneven or too-slow
  display) and `next_deadline` (no catch-up burst after a late frame).
- `quality`: the effective graphics quality Auto draws (#1022 T5):
  `EffectiveQuality` (resolution steps of 10 % of native down to the
  720-pixel floor, terrain loading work, terrain, markers, atmosphere,
  shadows, bloom), `Limits` (ceiling, floor, memory pressure), `Control`
  with the recovery and reduction orders, and `Bottleneck` hints. Never
  saved: the saved choices are `settings_quality`.
- `quality_window`: the controller's one-second windows (#1022 T5, R1):
  slow frames over 110 % of the target interval, overloaded / delivered /
  spare tests, p95 interval and work, in fixed buffers (`E-HOT-NOALLOC`).
- `quality_controller`: the Auto controller (#1022 T5): three overloaded
  windows to reduce (three seconds apart), ten spare windows to recover
  (a probe without work timing), trials judged after a two-second settle
  and rolled back when useless, ten-second time-out, 60 / 120 / 240 s
  cooldowns, pauses, target and ceiling changes, the performance-limit
  status and its restriction. Trace tests in `quality_tests`.
- `settings_quality`: graphics quality preferences split from `settings`
  (`E-FILE-SIZE`, #1022 T4): `QualityMode`, `TargetFps`, `Tier`,
  `Resolution3d` and `PlanetDetail`, re-exported from `settings`.
- `settings_camera`: camera presets split from `settings` (`E-FILE-SIZE`,
  #899): `CameraSmoothing` (#756 S1) and `BehindFollow` (Off / Quick /
  Smooth, #899 S1), re-exported from `settings`.
- `settings_ground`: the Ground memory limit split from `settings`
  (`E-FILE-SIZE`, #918 S1, ADR 0049): `GroundMemory` (8 / 16 / 32 / 64 /
  128 MB since #920 S1,
  default 64, file id `ground_memory_limit`), re-exported from `settings`.
- `settings`: player settings, ranges, defaults, per-category reset, the 8
  saved-view slots (free poses store forward + up, old yaw/pitch files still
  load), the tolerant `key = value` file format (#473), the player figure
  `BodyColour` presets (`GreyOrange`, `White`, `Blue`, `Red`, #755 S1), the
  dive orbit speed (`DiveOrbitSpeed` 0.5-2.0 default 1.0, #873 S1), and the
  free-mode roll speed (`FreeRollSpeed` 0.5-2.0 default 1.0, #884 T5 S1).
- `guides`: About and Help page text with the approved words; the scale
  ladder is built from `labels`, so it cannot drift (#473), plus the four
  ground heights after L10 (`GROUND_LADDER_ROWS`, #892 G2); Help >
  Navigation opens with the panel's L11-L14 heights (#892 G1). Help > Free
  flight carries the spaceship text (loop over the top, Q/E roll, no
  gravity turn, #884 G1) and Help > Navigation ends with the planet-centre
  plus selection text (3 m floor, selected body, Space then 10 m, P toggle,
  click deselects, #884 G2); no page names E for exposure (#884 G3).
- `tests/properties.rs`: property tests, one sentence per property.

API boundary: other crates use core's public items only; core exposes no
rendering concept.

### `universe-render` (`crates/universe-render/src/`)

Bevy indicator rendering only. Depends on `bevy` and `universe-core`.
Exposes exactly its plugins, its two resource newtypes, and the helpers
`universe-app` needs (`E-CORE-API`).

- `lib.rs`: crate doc, `UniverseRenderPlugin`, `DivePlugin` (grouping
  `MenuPlugin`, `PanelPlugin`, `SettingsUiPlugin`, `GuidesUiPlugin`),
  `DiveSystems::{Input, Camera, Draw}`, the `Universe` and `PreviewCache`
  resource newtypes over `universe-core::nav`, the `Flight`, `SavedSlots`,
  `CursorPortals`, `ClickCycle`, `LabelCache`, `ExposureLevel`,
  `BillboardState`, `PlayerState` (#754), `FigureState` (#755), and `StreamTasks` resources, the
  public `SettingsStore`, and the angle/cap re-exports.
- `menu.rs`: `Screen` state (`Universe`, `Menu`, `Guides`, `Settings`), the
  menu button, the Menu screen, and the Esc rule; the `Input` set runs only
  in `Universe`, so screens pause the universe and block clicks and keys
  (#473).
- `fonts.rs`: bundled UI fonts (#473, ADR 0023): Inter and Lucide embedded
  from `assets/fonts/` into `UiFonts`, the `TextRole` tag that sets each
  screen text's font and size from tokens, and the `Icon` codepoint table.
- `panel/`: shared Guides/Settings shell (title, Back, close, expandable
  category list with selection bar, scrolling content, narrow collapse under
  720 px) plus the `SETTINGS_TREE` and `GUIDES_TREE` list data (#473); the
  `scrollbar` child module draws the draggable content thumbs (#518).
- `settings_ui.rs`: Settings content, the fullscreen/vsync loader
  (`PendingWindow`), and the store apply/fan-out; keys and widgets agree
  through `SettingsStore` (#473). Display > Scene carries the Body colour
  cycle button (`SettingBodyColour`, #755 S1) and the Planet detail cycle
  button (`SettingPlanetDetail`, #776 S1); Controls > Flight carries the
  Dive orbit speed stepper (`DiveOrbitSpeed` 0.5-2.0 default 1.0, #873 S1)
  and the Free-mode roll speed stepper (`FreeRollSpeed` 0.5-2.0 default 1.0,
  #884 T5 S1); Developer carries Debug axes and
  the Debug overlay toggle (`DebugOverlay`, #776 S2, Q6 A) mirroring `F3`.
- `keys_ui.rs`: the editable Controls > Keys page (#560): one row per
  action, click to capture the next key or mouse press (`KeyCapture`,
  which consumes that press before the menu's Esc rule; Esc cancels), the
  swap, and Reset keys.
- `touch.rs`: touch gestures (#561): `read_touches` turns `TouchInput`
  (or, with Developer > Simulate touch, the left mouse button and cursor)
  into `TouchState` before the input chain; each finger takes a role (stick,
  round button, mode reminder, menu button, free screen: one finger looks
  or taps, two pinch), and `handle_input`, `pick_hover` and
  `normal_mode_step` read it next to the key map.
- `touch_ui.rs`: the on-screen stick and round buttons drawn from the core
  layout, shown on Touch controls always or after a touch on automatic,
  hand-swapped (#561).
- `views_ui.rs`: the one save and recall rule for views, used by the keys
  and by the Views page's Save here and Go buttons (#561).
- `bindings.rs`: maps core binding names to Bevy `KeyCode` and
  `MouseButton` (Shift, Ctrl and Alt on either side) and answers
  `pressed` / `just_pressed` per action through `Actions` (#560); every
  game key reads through it.
- `guides_ui.rs`: Guides pages with the approved text, ladder built from
  core `guides` (#473).
- `camera.rs`: `spawn_indicator_camera`, `sync_camera` (normal mode poses
  through `player_camera`, dive and free keep their own easing, #754),
  `adapt_eyes` and `sync_exposure` (the manual level plus the eyes' offset,
  `EyeAdaptation`, `exposure_ev`; L10 only, setting Eye adaptation, #908). The up eases toward the local up (`local_up` at
  `UP_RATE`, #430) and the heading toward the marker on the way
  (`LOOK_RATE`); the dive roll eases toward the dragged angle and derives
  the drawn up without writing back (`ease_roll`, `roll_up`,
  `Navigation.roll`/`roll_drawn`, #884 T1, replacing #873 T2 in-place
  accumulation). Free keeps its own forward/up with no gravity easing and
  no pitch clamp (#884 T2). `camera_projection` sets the 70° field of view the pixel
  measures assume (#778 F1 A); at L10 `near_plane_cells` keeps the near
  plane at most 5% of the height above the drawn terrain, floored at 1 mm
  (#778 Q1 A, ADR 0037). In a planetary system the nearest body bounds the near plane (#995).
- `input.rs`: `handle_quit` (no-op stage: Esc never quits since #473),
  `pick_hover`, `handle_input`,
  `Navigation` (plus `roll`/`roll_drawn` and left-drag `orbit_acc`, #884 T1;
  free pose lives in `Flight`, #884 T2),
  `Autopilot`, `FlightMode`, `Flight` (`free_forward`/`free_up` spaceship,
  #884 T2), `SavedView` (free forward + up, old yaw/pitch files still load),
  `SavedSlots`, `CursorPortals` (ranked portals under the cursor),
  `ClickCycle` (repeat-click state; exposure, speed, debug axes and saved
  views all write through the settings store since #473),
  `ModeNote` and `Latches` (#557: the mode keys switch normal mode, free
  flight and the dive, Save view / Bring a view back + 1-8, look
  sensitivity and invert, hold-or-toggle latches). Every key comes from
  the saved key map through `bindings` (#560; roll Q/E, exposure B, #884
  T3). Hover is the ranked head
  (destination portal first, then nearest, #446); clicks resolve repeats
  into cycles through portals only and deselect the player (#884 T4).
  `Navigation` holds the final
  `Destination` and re-derives the target on the way every input frame
  (`target_on_the_way`, ADR 0018); manual window dives land
  (`DiveMode::Landing`), the autopilot keeps the replay line
  (`dive_mode`, ADR 0019); window free flight passes through everything,
  the ground included (#558). In dive mode WASD plus the left-drag orbit
  at the same distance (threshold so clicks still pick) and the right-drag
  turns in place with release stop and smoothing (#884 T1); free look
  rotates around the view's own axes with no pitch limit and Q/E roll at
  the roll speed with no gravity turn (#884 T2, Q3 A). Touch one-finger
  drag orbits, tap picks (#873 T2, Q1-Q2).
  Dive and orbit never enter the L10 planet and never drop below 3 m above
  relief (cached frame/relief, #873 T3-T4).
- `hud.rs`: persistent scale readout (`HudText`, `spawn_hud`, `sync_hud`; ADR 0015), with no next line since nothing opens below any level (#729); at the planet `hud_lines_at` names the height band, the height above the ground and its scale from the rendered camera (`camera_height_m`, every mode; #892, ADR 0048), with F3 on at the planet the `ground_memory_line` (#918, ADR 0049), and the top-left mode reminder (`ModeReminderText`, `sync_mode_reminder`; #557). In one star system `system_hud` names the level by distance and adds the nearest body line (#995, ADR 0051).
- `normal_mode.rs`: normal mode (#558): `place_body`, `normal_mode_step`
  (moves the body through `universe_core::body`, advances the handover ease;
  placing and saving live in `player_state`, #754), and `draw_body` (the
  faint pull arrow only, #559; the figure itself lives in `figure_draw`,
  #755). At L10 the body steps through `step_near_ground` on the drawn
  ground, and a pending body drops onto it once it is built (#779). In
  third person A/D turn the body car-style (`body_step`, #899 Spec v5) and
  the behind-camera follow turns the camera with the body, then eases any
  mouse offset out while you move (`BehindFollow`).
- `style.rs` `RenderOrigin` / `Anchored` / `place_anchored`:
  camera-relative drawing (ADR 0026); world units are metres at L10 and
  open cells elsewhere (#780 F2 A): `scale`, `len`, and `CellSized` for
  meshes built in cell units.
- `sun.rs`: the one sunlight (#780, ADR 0040): `Sun`, `spawn_sun`,
  `sync_sun` (direction from `light::sun_direction`, the night floor as
  ambient light, near shadows at L10 while Settings > Display > Scene >
  Shadows is on, in metre cascades `SHADOW_CASCADES_M`), `floor_brightness`
  and `sun_now`.
- `air.rs`: the planet's atmosphere (#780, ADR 0040): `medium_for` (a
  `ScatteringMedium` from core's densities), `sync_air` (the medium first,
  then an `Atmosphere` anchored at the planet centre and the camera's
  `AtmosphereSettings`), `sync_sky_mode` (lookup tables inside the air,
  raymarched from `SPACE_MODE_ABOVE_M` above it, and the aerial-haze reach
  from `aerial_reach_m`), and `clear_stale_sky` (the engine's leftover sky
  after L10 closes, ADR 0046).
- `stars.rs`: emissive billboards for bright portal tints (`StarBillboard`,
  `BillboardState`, `draw_star_billboards`; ADR 0015). A star's quad steps aside past `BILLBOARD_BODY_ANGLE` (#995).
- `stream.rs`: background preview generation off the frame thread
  (`StreamTasks`, `sync_previews`, ledger mirror).
- `draw.rs`: `draw_axes`, `draw_open_cell`,
  `draw_previews` (context previews inside markers, oriented rects/boxes,
  #430), `draw_parent_siblings` (per-object tint hues since #157;
  form level of detail since #384), `portal_ring` (one ring per portal at
  the true child position, none on populations, #446), dot and brightness
  floors from tokens (portals clamp to the actionable floor, children sit
  one ladder step under the shell, #446). In a planetary system the star and planets stay lit and draw at true size with the marker floor (#995).
- `forms.rs`: `Form` to gizmo mapping, the LOD gate, frame mapping (#384),
  true-scale grid curvature and oriented rect/box draws (#430).
- `tokens.rs`: every visual value named by role (#446; relief tints #799):
  line and dot floors, era tint tables and semantic aliases, ladder steps,
  pick sizes, label threshold and cap, ring scales, the one ground colour
  `RELIEF_GROUND_TINT` (#892 Q9 C; the #799 height tints are gone). Gizmo width applied explicitly from `LINE_MIN_PX`.
- `labels.rs`: pooled portal label text nodes (`PortalLabel`, `LabelCache`,
  `spawn_labels`, `sync_labels`; largest-first `LABEL_CAP`, #446).
- `patch.rs`: removed (#770, ADR 0031); no detail window until the rebuild.
- `player.rs`: the player in the window (#607, #873, #884, ADR 0047):
  `player_setup` (selected at start, planet target; P toggles selection,
  1 away flies there first with takeover), `player_glide` (exponential
  approach to the 10 m arrival pose when selected, #873 Q6 C; focus clears
  after arrival so the target is the planet centre again with selection
  kept; hands over on takeover; poses through `player_camera` Gliding,
  #754), `go_player_click` (Go selects, Reset reselects seeded),
  `player_here` (tests). State lives in `player_state`.
- `player_state.rs`: one player state (#754; selection #884, ADR 0047):
  `PlayerState` (spot, seeded, `selected` true at start, gliding, takeover
  plus body, third or first, active, handover, pull, local-frame anchor
  and pending ground drop, #779), `leave_player`, `follow_player`,
  `enter_player` in leave, follow, enter order, Go to player and Reset
  read the same resource.
- `player_camera.rs`: one function for the player's camera (#754):
  `player_camera` (`Gliding`, `First`, `Third`, `Handover`, `Released`),
  `arrival_tilt`, `HANDOVER_S`, `smoothstep`; `sync_camera` only applies it.
- `planet_approach.rs`: deleted (#729, ADR 0028); the L9 approach fade
  lives in `patch.rs` now.
- `surface_split.rs`: deleted (#729, ADR 0028); splits are gone, the dome
  has fixed resolution.
- `planet.rs`: L10 moons as meshes (#776, ADR 0034: the six fixed faces are
  gone, LOD patches draw the sphere; moons stay, seed-keyed, unlit). Moons draw at their true size (#995).
- `patches.rs`: LOD patch workers and drawing (#776; relief tints
  #799; morph #777 T3): `PatchTree` (selection plus `Relief`), `PatchTasks`
  (worker builds nearest first with the 500 m ring, at most `max_builds` in
  flight, far builds after the ring, #819, #918; since #948 unbuilt hidden
  cover queues last, nearest first, on leftover room without tripping the
  memory bias, keeping one flick of headroom; apply budget
  `PATCH_APPLY_BUDGET`; the Ground memory limit in effect, `cap_bytes`, read
  from the setting every frame; the detail bias), `draw_selection_into`
  (parents stay; an unbuilt merged leaf draws its four cached children),
  `sync_patches`
  (Camera set, relief-aware selection from the rendered camera
  `RenderOrigin.camera`, culled by the camera view from `camera_view`, visible leaves only wanted and built, masks against the
  draw set, early return on a steady draw set, #778; full CPU builds within
  `NEAR_BUILD_M` of the camera or of the ground below the feet, tile-only
  builds beyond, #920), `PatchPiece` and `PatchMask` entities
  (one per patch of the whole tree's draw set, the cover, since #948 R2:
  the selection follows last frame's camera, so hidden built ground is
  already drawn when a fast turn reveals it, and the GPU culls off-screen
  patches by their `Aabb` with the current camera; `Anchored`, `E-ENTITY-NAME`, the shared grid mesh, the
  level's ground material and the tile's `MeshTag`, the mask written into the
  tile, #920). `GroundSurface` holds the
  `DrawnSurface` rebuilt from the cover at L10, and `ground_under` reads it
  (falling back to `level_ground` elsewhere, #779, ADR 0039).
- `patch_cache.rs`: resident ground patches and the memory cap (#918 T5,
  ADR 0049): `PatchCache` (every patch keeps its tile, ADR 0050, and a slim
  `PatchInfo`; full CPU meshes only within `NEAR_CPU_M` of the camera or of
  the ground below the feet, #920; byte count, a tile per patch on the GPU;
  `evict_farthest` over the limit, never a needed patch or a root;
  `relief_or_children` so unbuilt parents can merge) and `bias_step` (the
  detail coarsens in slow steps while the cap holds builds back).
- `patch_material.rs`: what the ground's materials share (#777, ADR 0036;
  #920, ADR 0050): `ground_base` (the one ground colour, matte, one-sided),
  `GroundScale` (the band scale) and `PatchMaterialPlugin` (registers the
  grid, caps mesh slabs).
- `patch_grid.rs`: ground patches from one shared grid (#920, ADR 0050):
  `PatchGridMaterial = ExtendedMaterial<StandardMaterial, PatchGrid>` with
  embedded `patch_grid.wgsl`, `patch_grid_prepass.wgsl` and
  `patch_grid_common.wgsl` (the vertex stage rebuilds each patch from its
  tile: positions, own normals from four neighbours, stored parent
  normals, the CDLOD blend), `GridAtlas` (the paged `R32Uint` heights
  texture sized from the Ground memory limit, tiles, the two shared meshes,
  one material per level made one a frame finest first so near ground draws
  first, ordered tile and mask writes), `GridInbox` (tiles packed on the
  workers), `build_task`, `upload` and `aabb_of`; the headless app keeps a
  texture-less `GridAtlas`.
- `debug_overlay.rs`: F3 LOD overlay (#776, Q6 A, S2): `OverlayInfo` (seven
  items, plus ground memory and its limit, #918), `level_colour`,
  `sync_overlay` (Draw set, gizmo wireframe through every edge point plus
  level colours when on).
- `sky.rs`: removed (#770, ADR 0031); limb preview deleted.
- `screens.rs`: `ScreensPlugin` for `--screens` (#492, ADR 0021): the shot
  list `SHOTS` (including the third-person and first-person player figure
  shots, #755, the L10 planet sphere shot, #775, and the LOD overlay shot,
  #776), the setup/settle/capture sequencer with a per-shot frame cap, and
  `ScreensQueue` of captured pictures. No file I/O.
- `screens_sky.rs`: sky shots for light and air (#780): `SkyView` (height,
  angle from the sun, tilt), the seven `SKY_*` shots from orbit to the
  night side, and `place_for_sky` under the planet's real sun; also
  `place_above_ground` for the 1 m planet shot.
- `screens_select.rs`: shot groups for `--screens <dir> <groups>` (#847,
  ADR 0043): `ShotGroup` (menus, planet, sky, walking), `ShotSelection`
  (`parse`, `takes`), `SMOKE` (one shot per group), `shot_count_for`, and
  `ready_early` (ground shots go once the drawn ground is built, after the
  run's first ground shot).
- `sky.rs`: the sky's clock in the window (#981, ADR 0052): `SkyClock`,
  `advance_sky` (ticks, moves the bodies, carries a zone open like a dive
  open), `restore_sky_time` and `keep_sky_time` (the settings file's
  `game_time`, on quit and every 30 s).
- `time_bar.rs`: the top-middle time bar (#981, ADR 0052): play or pause,
  slower, the speed, faster and the readout; hidden with the mode
  reminder switch, a second row on phone-width windows; `time_bar_click`
  claims its presses and reads the T, [ and ] keys.
- `shot_file.rs`: shot files for `--screens <dir> --shots <file>` (#863;
  `time=H` sets the sky's hours, #981):
  `ShotSpec` (name, level, `ShotPlace`, `ShotLook`, figure),
  `parse_shot_file` (refuses a wrong line as `line N: <problem>`), and the
  `ShotFile` resource that replaces the built-in list.
- `shot_place.rs`: placement for shot-file shots (#863): `template`,
  `lat_lon_dir`, `heading_tilt_dir`, `yaw_facing`, `pose_for` and the
  `place_file_shot` system; also `player_for_shot` for the built-in player
  shots.
- `timing.rs`: frame timing for `--screens <dir> --shots <file> --timing`
  (#819): `summarise` (mean, 95th percentile, worst, short-then-long
  alternation), `timing_line` (with `ring_ms` and `ready_ms` at the ground,
  #918), the `TimingRun` resource, `TIMING_FRAMES`
  and `TIMING_SIZE` (1600x900). The fixed views are `timing/views.txt`.
  Each line also carries the frame's work without waits, and `timing_header`
  opens the file with the `TIMING-RUN` line read from the window and the
  adapter (#1022 T1, ADR 0053).
- `frame_work.rs`: a frame's CPU work without its waits (#1022 T1, ADR
  0053): `FrameWorkPlugin` times the main world (`First` to `Last`) and the
  render schedule minus its wait region (`FrameWorkSet::WaitBegins` to after
  `prepare_windows`); `FrameWork::sample` gives a `WorkSample` whose work is
  the longer thread.
- `route.rs`: route files for `--route` (#1022 T2, ADR 0053): `parse_route`
  (shot lines plus `for=` and `scenario=`, `journey` legs, `target` and `set`
  lines), `interpolate` (heights by a constant factor), `RouteFrame`,
  `frame_line` and `summary` (warm-up, each scenario, the whole, the first
  and final five minutes).
- `route_run.rs`: the window side of `--route` (#1022 T2): `RouteRun` and
  `drive_route`, placing the camera every frame in real time and recording
  frames and the ground's readiness per planet leg; `RoutePlugin`.
- `effective.rs`: the effective graphics (#1022 T8): `EffectiveGraphics`
  (what is drawn now; Manual and, until T10, Auto follow the saved
  choices; never saved), `manual_quality`, the bloom fade (0.5 s by
  intensity) and the atmosphere samples per tier. `sun`, `continuity`
  (eased marker cutoff), `patches` (`terrain_detail_px`: tier times memory
  bias) and `air` read it.
- `frame_cap.rs`: the frame cap (#1022 T7, ADR 0054): `FrameCapPlugin`
  (window runs only) sleeps after the main world's measured work to the
  target's next deadline, records the sleep as `WorkSample::cap_ms`, and
  keeps `PacingStatus` with the display note.
- `render_scale.rs`: the internal 3D resolution (#1022 T6, ADR 0054):
  `RenderScale` (effective, never saved; Manual from the saved 3D
  resolution), `RenderScalePin` (`ASTROLITH_RENDER_SCALE` for evidence),
  and `RenderScalePlugin`. Below native the 3D camera draws into a scaled
  image whose target scale factor keeps window logical coordinates, and a
  present camera stretches it under the native-size UI.
- `figure_draw.rs`: one lit low-poly figure for the waiting and the active
  body (#755): `FigurePose`, `figure_pose` (one code path), `figure_mesh_of`
  (canonical mesh with CPU vertex colours), `refresh_colours` (in-place),
  `figure_rotation`, `FigureState`, `draw_figure` (camera-relative via
  `Anchored`, ADR 0026; lit by the sun since #780).
- `style.rs`: `point_color_for_level`, `sibling_color_for_level`,
  `tint_color` (dark reds display-mapped through `contrast`, #446),
  `scaled`, `to_vec3`. Reads `tokens.rs` and defines no visual value.

**Architecture Invariant**: only the L10 planet creates meshes, materials,
or textures (one morph patch mesh with skirts per drawn LOD patch plus one
body per moon, #776 ADR 0034 and #799 ADR 0035 and #777 ADR 0036, plus moons
at L10 per ADR 0016), the player figure creates one unlit mesh with CPU
vertex colours (#755), and any open cell may create emissive billboard quads
for bright portal tints (ADR 0015); everything else draws indicators only
(`E-RENDER-NO-MESH`).

API boundary: the app imports plugins and the documented helpers; systems
and internals are `pub(crate)`.

### `universe-app` (`crates/universe-app/src/`, `tests/`)

The binary. Depends on `bevy`, `universe-core`, `universe-render`.
Exposes nothing.

- `main.rs`: flag parsing (`--verify`, `--capture <dir>`, `--screens <dir>`, usage with exit
  2) and the window `App` with the Bevy error handler. The window run loads
  the settings file into the render `SettingsStore` before the plugins run
  (#473).
- `persist.rs`: settings path resolution, load-with-defaults, and the
  save-on-change system watching the render store. `--verify` and `--capture`
  never call it, so the golden files stay untouched (#473).
- `verify.rs`: `run_verify` and its checks.
- `capture.rs`: `run_capture`.
- `screens.rs`: `run_screens` and `run_shot_file` (#492; groups #847; shot
  files #863, read and checked before the window opens): folder probe,
  default-settings window, the chosen `ShotSelection` or `ShotFile`, PNG
  write with size and not-blank checks, `screens.txt`.
- `gpu.rs`: `render_plugin` and `FramePacingPlugin` for both window builds
  (#499; #780, ADR 0041; #848, ADR 0042): the engine's automatic backend on
  every platform (Vulkan on the dev PC), `WGPU_BACKEND` overrides; on
  Vulkan at most two frames are in flight: a frame waits for the one
  before the previous before it takes its window image (#819, ADR 0044).
  The build pool leaves at least two threads to the frame (#1022 T1, ADR
  0053).
- `route.rs`: `run_route` for `--route <file> <dir>` (#1022 T2, ADR 0053):
  the route file read before the window opens, default settings plus its
  `set` lines, `frames.txt` and `route.txt` written at the end, `ROUTE-OK`.
- `options.rs`: the trailing window options `--build-workers <n>`, `--size
  <w>x<h>` and `--scale <f>` in any order (#994, #1022 T1, ADR 0053).
- `protocol.rs`: every stdout line format and the exit-code constants
  (`E-ONE-PRINTER`, `E-STDOUT-PROTOCOL`).
- `tests/golden.rs` + `tests/golden/`: the M0 baseline. `--verify` stdout
  byte-equal to `verify.txt`; `--capture` file listing equal to
  `capture.txt` (`E-GOLDEN`).

### `universe-verify` (`crates/universe-verify/src/`, `tests/`)

The Bevy-free headless gate (#86). Depends on `universe-core` only.
Exposes nothing.

- `main.rs`: bare or `--verify` runs the shared report, anything else
  prints usage to stderr and exits 2.
- `tests/parity.rs`: the new binary's stdout byte-equal to the M0 golden
  file (read-only), plus the exit-2 path.

API boundary: `universe-verify` and `universe-app --verify` print the
same `universe_core::headless` report bytes; neither links the engine
for the gate.

## Cross-cutting concerns

- Determinism: fixed root seed (`DEMO_SEED`), fixed timestep
  (`VERIFY_DT = 1/60 s`), fixed seeds in tests, six-decimal snapshots,
  `total_cmp` ordering. Three tiers (`E-DET-TIERS`); transcendental calls
  confined to core generators (`E-TRANSCENDENTAL`).
- Bounded memory: the streaming working set (open chain, horizon,
  previews) capped at `STREAM_CAP`; per-frame
  allocation forbidden in `Update` systems (`E-HOT-NOALLOC`); the preview
  cache reuses its buffers for a static camera.
- Testing: ladder in `E-TEST-LADDER`; hermetic tests (`E-TEST-HERMETIC`);
  golden baseline (`E-GOLDEN`); properties (`E-PROTEST`); `App`-level and
  schedule-order tests in render (`E-BEVY-TEST`); visual evidence from
  the real-window `--screens` shots (`screens.rs`), checked by the main
  agent and the owner test.
- Error handling: no panics outside tests (`E-NO-PANIC`); `Option` versus
  `Result` (`E-OPT-RESULT`); the window app panics on system bugs in
  debug and logs them in release (`E-SYS-RESULT`).
- Diagnostics: `bevy::log` macros (`E-LOG-TRACING`); reserved output
  vocabulary (`E-VOCAB`); `RUST_BACKTRACE=1` in automated runs
  (`E-BACKTRACE`).
- Simulation timing: fixed timestep for simulation systems; 60 fps target
  budget lives in `docs/universes/stack.md` (`E-BUDGET`).
- Command line: exit 0 pass, 1 check failure, 2 wrong usage (`E-EXIT-CODE`,
  the one approved behavior change, Q7).

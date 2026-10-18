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
  window: hover highlights a marker, click targets it, wheel or arrows dive,
  Spacebar runs the autopilot journey L1-L10, `F` toggles free flight (WASD
  plus right-drag look, `Shift+1-9` steps speed, `Ctrl+1-8` saves a view,
  `1-8` recalls it), Esc opens the menu (Resume, Guides, Settings, Quit;
  #473). Settings and saved views persist in the platform config folder
  (`%APPDATA%\Astrolith\settings.toml` on Windows). A wrong command
  line prints one usage line to stderr and exits 2.
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
  point and direction maps across a frame (moved from `nest`, #403). Local
  up blends from the planet axis to the ground normal by altitude at L10
  (`local_up`, `UP_BLEND_NEAR/FAR`, #430). Radius is the true radius from
  the record through the one L10 metre (Earth about 0.49, #774).
- `planet_frame`: one planet-centred metre frame per L10 planet (#774):
  `PlanetPos` (metres, `f64`), `PlanetFrame::for_planet` (radius from
  `body_recipe`, ECEF-like, no spin), `from_cell`/`to_cell` (the one place),
  `LocalFrame` (up, north, east), `LocalPos`, `CameraRel`,
  `l10_camera_offset` plus `l10_drawn_f32` (the exact render path the window
  shares), and saved-view conversion.
- `precision`: planet-frame shake measurement (#774): `shake` (worst mm
  within 1 km, pixels at 1600x900 farther, over heights and face
  centre/edge/corner) and `shake_all` (Earth plus two seeded planets),
  printed as the headless `PRECISION` line.
- `cube_sphere`: tangent-adjusted cube-to-sphere mapping for the L10 planet
  (#775): `Face` (`PosX`/`NegX`/`PosY`/`NegY`/`PosZ`/`NegZ`, `+Y` north),
  `FacePos`, `to_sphere`/`from_sphere` (exact inverse), `grid_point` with
  canonical edge folding, `test_height`, and `spacing` (min/max/ratio plus
  centre/edge/corner), printed as the headless `CUBE-SPHERE` line.
- `home`: journey matching, home fixtures, and the autopilot (split from
  `nest`, #156).
- `gen`: pure generator contracts, marker forms, portal offsets, the sphere clamp.
- `density`, `astro`, `terrain`: L1-L3 density, galaxy-to-star sampling,
  L10 terrain (bare planet since #375; one surface record from the pure
  `body_recipe`, anchor-tile region portals at L9 and L10, ADR 0025). The
  planet has no relief (#593): it is the icosphere split once, 80 flat
  faces with corners on the sphere (`FACET_DEPTH`, `facet_plane`,
  `surface_radius`, `surface_floor_radius`), and finer splits lie flat
  inside those faces.
- `ground`: removed (#770, ADR 0031); no ground until the rebuild (step 6, #779).
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
- `orbits`: L7-L9 system layouts (rings, shells, orbits, star bodies, #384).
- `snapshot`: canonical text snapshots (`snapshot_generated`).
- `cache`: fixed-capacity LRU cell store.
- `verify`: determinism and border-agreement predicates.
- `noise`: deterministic noise helpers.
- `player`: where the player stands (#607): `PlayerSpot` (the named chain
  of the body's cell, its feet and heading), `seeded` (a planet, region and
  spot on the region's ground, fixed per seed), `destination` (anchored, so
  the autopilot rises back out instead of re-aiming), `arrival_pose`
  (behind and above the body, sharing `third_person_camera` with the
  third-person view, #754) and `is_near`, plus the one third-person offset
  (`THIRD_BACK`, `THIRD_UP`) and `third_person_camera` (#754, Q4 A).
- `figure`: the player's low-poly figure (#755): `FigurePart` (head, torso,
  arm, leg), `FigureTriangle` with face normals, `figure_triangles` (full)
  and `first_person_triangles` (no head), `is_accent`, `HEAD_BOTTOM/TOP/HALF`
  and `FIRST_PERSON_NEAR_SHARE` (the camera near share).
- `light`: star light for the figure (#755): `star_direction` (planet to its
  star at L10, the camera up-right fallback above), `MIN_LIGHT`, `shade`.
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
  context (`preview_surface_context`, #430).
- `preview`: pre-entry preview cache (`PreviewCache` plus its background
  `merge`), preview-set computation, `preview_positions`, `preview_error`.
- `stream`: streaming working set (`STREAM_CAP`, `StreamLedger`,
  `working_chains`, proximity-first unload: farthest first, recency then
  chain order on ties; #742 T5).
- `destination`: the camera's final destination (#423): `Destination`,
  the seeded surface chain from L1 to the L10 planet along the journey
  picks, `Way` (next marker on the way, arrived, off), the click rule
  (targets but never opens at L10), and the allocation-free per-frame
  `on_the_way`.
- `flight`: free-flight synthesis (`FreePose`, `FreeKeys`, look, speed,
  `nearest_portal`), the pure heading
  helpers the window camera carries across frames (`ease_heading`,
  `orthogonal_up`, #403), and the scripted leg (`FreeLegReplay`,
  `replay_free_leg`).
- `gravity`: real gravity from every massive body (#559): real masses
  (`mass_kg`, `planet_mass_kg`), the summed pull and the strongest body
  (`pull`), the planet under the body pulling with its real surface
  gravity.
- `body`: the player body in normal mode (#558): true-size walk, jump,
  fly, drone or plane turning (`body_step`), the ground under each level
  (`level_ground`), collisions with buildings, room walls and furniture
  (`resolve_collisions`), and places opening around a walking body
  (`walk_levels`); since #559 it falls under the real pull with its
  momentum (drift, orbit, falling opens places).
- `keymap`: one key or mouse button per action (#560): `Action` (the 21
  actions), `Binding` (a name from the fixed `BINDING_NAMES` table, so the
  core stays Bevy-free), and `Keymap` with the spec defaults, `bind`
  (a taken key swaps), and `apply` (saved `key.<action>` lines; a damaged
  entry keeps its default). `Settings.keymap` carries it.
- `touch`: touch layout and gesture math (#561): `Layout` (the stick and
  round buttons for a window size and hand, above the scale readout),
  `Shown` (which controls each mode shows), `stick` (directions, the run
  edge), `pinch_log_factor`, `is_tap`.
- `labels`: `level_label`, `scale_label`, `scale_anchor`,
  `window_title_for_level`, `next_level_line`, `tile_note`, `ground_ruler`
  (#543).
- `contrast`: WCAG luminance and contrast against black, the Machado
  protanopia simulation, the two-stop ladder check, and the display tint
  map for dark reds (#446).
- `names`: seeded marker names, home-chain real names, surface `"<kind> <n>"`
  names, and `"kind - name"` label text (#446).
- `pick`: pick ranking (destination portal first, then nearest), repeat-click
  cycling, the label rule, and the pick/label size tokens (#446).
- `settings`: player settings, ranges, defaults, per-category reset, the 8
  saved-view slots, the tolerant `key = value` file format (#473), and the
  player figure `BodyColour` presets (`GreyOrange`, `White`, `Blue`, `Red`,
  #755 S1).
- `guides`: About and Help page text with the approved words; the scale
  ladder is built from `labels`, so it cannot drift (#473).
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
  cycle button (`SettingBodyColour`, #755 S1).
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
  `sync_exposure`. The up eases toward the local up (`local_up` at
  `UP_RATE`, #430) and the heading toward the marker on the way
  (`LOOK_RATE`).
- `input.rs`: `handle_quit` (no-op stage: Esc never quits since #473),
  `pick_hover`, `handle_input`,
  `Navigation`, `Autopilot`, `FlightMode`, `Flight`, `SavedView`,
  `SavedSlots`, `CursorPortals` (ranked portals under the cursor),
  `ClickCycle` (repeat-click state; exposure, speed, debug axes and saved
  views all write through the settings store since #473),
  `ModeNote` and `Latches` (#557: the mode keys switch normal mode, free
  flight and the dive, Save view / Bring a view back + 1-8, look
  sensitivity and invert, hold-or-toggle latches). Every key comes from
  the saved key map through `bindings` (#560). Hover is the ranked head
  (destination portal first, then nearest, #446); clicks resolve repeats
  into cycles through portals only. `Navigation` holds the final
  `Destination` and re-derives the target on the way every input frame
  (`target_on_the_way`, ADR 0018); manual window dives land
  (`DiveMode::Landing`), the autopilot keeps the replay line
  (`dive_mode`, ADR 0019); window free flight passes through everything,
  the ground included (#558).
- `hud.rs`: persistent scale readout (`HudText`, `spawn_hud`, `sync_hud`; ADR 0015), with no next line since nothing opens below any level (#729), and the top-left mode reminder (`ModeReminderText`, `sync_mode_reminder`; #557).
- `normal_mode.rs`: normal mode (#558): `place_body`, `normal_mode_step`
  (moves the body through `universe_core::body`, advances the handover ease;
  placing and saving live in `player_state`, #754), and `draw_body` (the
  faint pull arrow only, #559; the figure itself lives in `figure_draw`,
  #755).
- `style.rs` `RenderOrigin` / `Anchored` / `place_anchored`:
  camera-relative drawing (ADR 0026).
- `stars.rs`: emissive billboards for bright portal tints (`StarBillboard`,
  `BillboardState`, `draw_star_billboards`; ADR 0015).
- `stream.rs`: background preview generation off the frame thread
  (`StreamTasks`, `sync_previews`, ledger mirror).
- `draw.rs`: `draw_axes`, `draw_open_cell`,
  `draw_previews` (context previews inside markers, oriented rects/boxes,
  #430), `draw_parent_siblings` (per-object tint hues since #157;
  form level of detail since #384), `portal_ring` (one ring per portal at
  the true child position, none on populations, #446), dot and brightness
  floors from tokens (portals clamp to the actionable floor, children sit
  one ladder step under the shell, #446).
- `forms.rs`: `Form` to gizmo mapping, the LOD gate, frame mapping (#384),
  true-scale grid curvature and oriented rect/box draws (#430).
- `tokens.rs`: every visual value named by role (#446): line and dot
  floors, era tint tables and semantic aliases, ladder steps, pick sizes,
  label threshold and cap, ring scales. Gizmo width applied explicitly
  from `LINE_MIN_PX`.
- `labels.rs`: pooled portal label text nodes (`PortalLabel`, `LabelCache`,
  `spawn_labels`, `sync_labels`; largest-first `LABEL_CAP`, #446).
- `patch.rs`: removed (#770, ADR 0031); no detail window until the rebuild.
- `player.rs`: the player in the window (#607): `player_setup` (first
  destination, Go to player key, 1 away flies there first), `player_glide`
  (exponential approach to the arrival pose, hands over on takeover; poses
  through `player_camera` Gliding, #754), `go_player_click` (Views buttons),
  `player_here` (tests). State lives in `player_state`.
- `player_state.rs`: one player state (#754): `PlayerState` (spot, seeded,
  gliding, takeover plus body, third or first, active, handover, pull),
  `leave_player`, `follow_player`, `enter_player` in leave, follow, enter
  order, Go to player and Reset read the same resource.
- `player_camera.rs`: one function for the player's camera (#754):
  `player_camera` (`Gliding`, `First`, `Third`, `Handover`, `Released`),
  `arrival_tilt`, `HANDOVER_S`, `smoothstep`; `sync_camera` only applies it.
- `planet_approach.rs`: deleted (#729, ADR 0028); the L9 approach fade
  lives in `patch.rs` now.
- `surface_split.rs`: deleted (#729, ADR 0028); splits are gone, the dome
  has fixed resolution.
- `planet.rs`: L10 planet sphere plus moons as meshes (#775, ADR 0033:
  six cube-sphere faces at fixed `SPHERE_GRID`, one unlit flat colour,
  seed-keyed; moons stay)
- `sky.rs`: removed (#770, ADR 0031); limb preview deleted.
- `screens.rs`: `ScreensPlugin` for `--screens` (#492, ADR 0021): the shot
  list `SHOTS` (including the third-person and first-person player figure
  shots, #755, and the L10 planet sphere shot, #775), the setup/settle/capture
  sequencer with a per-shot frame cap, and `ScreensQueue` of captured pictures.
  No file I/O.
- `figure_draw.rs`: one lit low-poly figure for the waiting and the active
  body (#755): `FigurePose`, `figure_pose` (one code path), `figure_mesh_of`
  (canonical mesh with CPU vertex colours), `refresh_colours` (in-place),
  `figure_rotation`, `FigureState`, `draw_figure` (camera-relative via
  `Anchored`, ADR 0026; unlit so nothing else changes).
- `style.rs`: `point_color_for_level`, `sibling_color_for_level`,
  `tint_color` (dark reds display-mapped through `contrast`, #446),
  `scaled`, `to_vec3`. Reads `tokens.rs` and defines no visual value.

**Architecture Invariant**: only the L10 planet creates meshes, materials,
or textures (one flat-tone sphere as six face meshes plus one body per moon,
#775 ADR 0033, plus moons at L10 per ADR 0016), the player figure creates
one unlit mesh with CPU vertex colours (#755), and any open
cell may create emissive billboard quads for bright portal tints (ADR
0015); everything else draws indicators only (`E-RENDER-NO-MESH`).

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
- `screens.rs`: `run_screens` (#492): folder probe, default-settings
  window, PNG write with size and not-blank checks, `screens.txt`.
- `gpu.rs`: `render_plugin` for both window builds (#499, ADR 0022):
  DirectX 12 on Windows, Bevy's default elsewhere, `WGPU_BACKEND` overrides.
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

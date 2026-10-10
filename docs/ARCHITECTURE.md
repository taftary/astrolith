# Architecture

Bird's-eye view, entry points, and codemap of this repository. Anyone
asking "where is the thing that does X" should find the answer here in
under a minute.

Where is the code that decides when a marker opens? `universe-core`, the
`nav` module, `should_open`.

## Bird's-eye view

A Bevy application that renders a nested dive through universe scales
L1-L14. Four crates: `universe-core` holds deterministic universe logic
(std only), `universe-render` holds Bevy indicator rendering,
`universe-app` holds the window binary plus the headless `--verify` and
`--capture` drivers, and `universe-verify` is the Bevy-free headless gate
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
  Spacebar runs the autopilot journey L1-L14, `F` toggles free flight (WASD
  plus right-drag look, `Shift+1-9` steps speed, `Ctrl+1-8` saves a view,
  `1-8` recalls it), Esc quits. A wrong command
  line prints one usage line to stderr and exits 2.
- Headless verification: `universe-app --verify`. Replays the autopilot
  from the fixed root seed, regenerates every opened cell twice,
  byte-compares, checks the open/close inverse and the pre-entry preview,
  prints canonical snapshots, exits 0 on `VERIFY-OK`, 1 on `VERIFY-FAIL`.
- Headless capture: `universe-app --capture <dir>`. Replays the same
  journey and writes one `level-L<N>.txt` plus one `frame-L<N>.ppm` per
  level plus `capture.log`. Exits 0 on `CAPTURE-OK`, 1 on `CAPTURE-FAIL`,
  2 for a missing `<dir>`.

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
  (`local_up`, `UP_BLEND_NEAR/FAR`, #430).
- `home`: journey matching, home fixtures, and the autopilot (split from
  `nest`, #156).
- `gen`: pure generator contracts, marker forms, portal offsets, the sphere clamp.
- `density`, `astro`, `terrain`: L1-L3 density, galaxy-to-star sampling,
  L10 terrain (bare planet since #375).
- `ground`: window-only ground clearance split from `terrain` under
  `E-FILE-SIZE` (`GROUND_STANDOFF`, `relief_radius`, `ground_height`,
  `clamp_above_ground`, #430).
- `landing`: window-only bend-to-land curve (`landing_step`,
  `landing_normal_for`, `landing_floor_cap` for `DiveMode::Landing`, #430).
- `tail`: L11-L14 sparse scatters and room (#375).
- `system`, `sysgen`: L5-L8 stellar and planetary sampling data plus the
  L4-L9 indicator generator (counts, portal order, home fixtures).
- `layouts`: L4-L6 marker layouts (galaxy forms, cloud rings, star radii;
  split from `sysgen` under `E-FILE-SIZE`, #384).
- `orbits`: L7-L9 system layouts (rings, shells, orbits, star bodies, #384).
- `snapshot`: canonical text snapshots (`snapshot_generated`).
- `cache`: fixed-capacity LRU cell store.
- `verify`: determinism and border-agreement predicates.
- `noise`: deterministic noise helpers.
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
  `working_chains`, longest-unseen-first unload).
- `destination`: the camera's final destination (#423): `Destination`,
  the named marker chain from L1 to a room along the seeded journey picks,
  `Way` (next marker on the way, arrived, off), the click rule, and the
  allocation-free per-frame `on_the_way`.
- `flight`: free-flight synthesis (`FreePose`, `FreeKeys`, look, speed,
  `nearest_portal`), the pure heading
  helpers the window camera carries across frames (`ease_heading`,
  `orthogonal_up`, #403), the window-only clamped step
  (`free_flight_step_clamped`, #430), and the scripted leg (`FreeLegReplay`,
  `replay_free_leg`).
- `labels`: `level_label`, `scale_label`, `scale_anchor`,
  `window_title_for_level`.
- `contrast`: WCAG luminance and contrast against black, the Machado
  protanopia simulation, the two-stop ladder check, and the display tint
  map for dark reds (#446).
- `names`: seeded marker names, home-chain real names, surface `"<kind> <n>"`
  names, and `"kind · name"` label text (#446).
- `pick`: pick ranking (destination portal first, then nearest), repeat-click
  cycling, the label rule, and the pick/label size tokens (#446).
- `tests/properties.rs`: property tests, one sentence per property.

API boundary: other crates use core's public items only; core exposes no
rendering concept.

### `universe-render` (`crates/universe-render/src/`)

Bevy indicator rendering only. Depends on `bevy` and `universe-core`.
Exposes exactly its plugins, its two resource newtypes, and the helpers
`universe-app` needs (`E-CORE-API`).

- `lib.rs`: crate doc, `UniverseRenderPlugin`, `DivePlugin`,
  `DiveSystems::{Input, Camera, Draw}`, the `Universe` and `PreviewCache`
  resource newtypes over `universe-core::nav`, the `Flight`, `SavedSlots`,
  `CursorPortals`, `ClickCycle`, `LabelCache`, `ExposureLevel`,
  `BillboardState`, and `StreamTasks` resources, and the
  angle/cap re-exports.
- `camera.rs`: `spawn_indicator_camera`, `sync_camera`, `sync_exposure`.
  The up eases toward the local up (`local_up` at `UP_RATE`, #430) and the
  heading toward the marker on the way (`LOOK_RATE`).
- `input.rs`: `handle_quit`, `pick_hover`, `handle_input`,
  `Navigation`, `Autopilot`, `FlightMode`, `Flight`, `SavedView`,
  `SavedSlots`, `CursorPortals` (ranked portals under the cursor),
  `ClickCycle` (repeat-click state; exposure steps on `E` / `Shift+E` via
  `ExposureLevel`; debug axes on `X` since #384). Hover is the ranked head
  (destination portal first, then nearest, #446); clicks resolve repeats
  into cycles through portals only. `Navigation` holds the final
  `Destination` and re-derives the target on the way every input frame
  (`target_on_the_way`, ADR 0018); manual window dives land
  (`DiveMode::Landing`), the autopilot keeps the replay line
  (`dive_mode`, ADR 0019); window free flight holds the ground standoff
  (`free_flight_step_clamped`).
- `hud.rs`: persistent scale readout (`HudText`, `spawn_hud`, `sync_hud`; ADR 0015).
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
- `planet.rs`: L10 bare planet body and moons as meshes
  (`PlanetMesh`, `PlanetMeshState`, `draw_planets`; ADR 0016), the fading
  air rim plus the limb preview around the targeted region
  (`draw_air_rim`, `draw_limb_preview`; ADR 0019).
- `sky.rs`: horizon rings and sky arcs for the tail surface levels
  (`draw_horizon`, `draw_sky`; #394), previewed horizon/sky/haze inside
  markers (`draw_preview_context`) and kept parent context at the marker's
  own shell curve (`draw_parent_context`; ADR 0019).
- `style.rs`: `point_color_for_level`, `sibling_color_for_level`,
  `tint_color` (dark reds display-mapped through `contrast`, #446),
  `scaled`, `to_vec3`. Reads `tokens.rs` and defines no visual value.

**Architecture Invariant**: only the open L10 cell creates planet meshes,
materials, or textures (bare body, moons per ADR 0016), and any open
cell may create emissive billboard quads for bright portal tints (ADR
0015); everything else draws indicators only (`E-RENDER-NO-MESH`).

API boundary: the app imports plugins and the documented helpers; systems
and internals are `pub(crate)`.

### `universe-app` (`crates/universe-app/src/`, `tests/`)

The binary. Depends on `bevy`, `universe-core`, `universe-render`.
Exposes nothing.

- `main.rs`: flag parsing (`--verify`, `--capture <dir>`, usage with exit
  2) and the window `App` with the Bevy error handler.
- `verify.rs`: `run_verify` and its checks.
- `capture.rs`: `run_capture`.
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
  schedule-order tests in render (`E-BEVY-TEST`); visual proof in
  `scripts/validation/frame_proof.py` (before/after capture frames as
  pictures on the parent Issue).
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

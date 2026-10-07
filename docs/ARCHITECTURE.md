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

Universe reference docs live under `docs/universes/`; the L1 visual and
lifetime reference index is `docs/universes/levels/L01/README.md`, the
L2 visual and lifetime reference index is
`docs/universes/levels/L02/README.md`, and the L3 visual and lifetime
reference index is `docs/universes/levels/L03/README.md`, the L4
visual and lifetime reference index is
`docs/universes/levels/L04/README.md`, and the L5 visual and lifetime
reference index is `docs/universes/levels/L05/README.md`, and the L6
visual and lifetime reference index is
`docs/universes/levels/L06/README.md`, and the L7
visual and lifetime reference index is
`docs/universes/levels/L07/README.md`, and the L8
visual and lifetime reference index is
`docs/universes/levels/L08/README.md`.

## Entry points

- Window: run `universe-app` with no flags. Opens the `Universe MVP`
  window: hover highlights a marker, click targets it, wheel or arrows dive,
  Spacebar runs the autopilot journey L1-L10, `F` toggles free flight (WASD
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
- `home`: journey matching, home fixtures, and the autopilot (split from
  `nest`, #156).
- `gen`: pure generator contracts.
- `density`, `astro`, `terrain`: L1-L3 density, galaxy-to-star sampling,
  L10 terrain.
- `system`, `sysgen`: L5-L8 stellar and planetary sampling data plus the
  L4-L9 indicator generator (counts, portal order, home fixtures).
- `snapshot`: canonical text snapshots (`snapshot_generated`).
- `cache`: fixed-capacity LRU cell store.
- `verify`: determinism and border-agreement predicates.
- `noise`: deterministic noise helpers.
- `nav`: `Universe`, `DiveEvent`, `DiveMode`, `dive_step`, `should_open`,
  `should_close`, `should_close_horizon`, `crossed_portals`,
  `sibling_in_open_units`, `drawn_radius`, `open_marker_radius`,
  `JourneyStep`, `replay_autopilot`, navigation constants, `Seed`,
  `MarkerIndex`, and the allocation-free `sync`.
- `preview`: pre-entry preview cache (`PreviewCache` plus its background
  `merge`), preview-set computation, `preview_positions`, `preview_error`.
- `stream`: streaming working set (`STREAM_CAP`, `StreamLedger`,
  `working_chains`, longest-unseen-first unload).
- `flight`: free-flight synthesis (`FreePose`, `FreeKeys`, look, speed,
  `nearest_portal`) and the scripted leg (`FreeLegReplay`,
  `replay_free_leg`).
- `labels`: `level_label`, `scale_label`, `scale_anchor`,
  `window_title_for_level`.
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
  `ExposureLevel`, `BillboardState`, and `StreamTasks` resources, and the
  angle/cap re-exports.
- `camera.rs`: `spawn_indicator_camera`, `sync_camera`, `sync_exposure`.
- `input.rs`: `handle_quit`, `pick_hover`, `handle_input`,
  `Navigation`, `Autopilot`, `FlightMode`, `Flight`, `SavedView`,
  `SavedSlots` (exposure steps on `E` / `Shift+E` via `ExposureLevel`).
- `hud.rs`: persistent scale readout (`HudText`, `spawn_hud`, `sync_hud`; ADR 0015).
- `stars.rs`: emissive billboards for bright portal tints (`StarBillboard`,
  `BillboardState`, `draw_star_billboards`; ADR 0015).
- `stream.rs`: background preview generation off the frame thread
  (`StreamTasks`, `sync_previews`, ledger mirror).
- `draw.rs`: `draw_axes`, `draw_open_cell`,
  `draw_previews`, `draw_parent_siblings` (per-object tint hues since #157).
- `planet.rs`: L10 planet bodies, atmosphere rims, and moons as meshes
  (`PlanetMesh`, `PlanetMeshState`, `draw_planets`; ADR 0014).
- `style.rs`: `point_color_for_level`, `sibling_color_for_level`,
  `tint_color`, `scaled`, `to_vec3`, `PICK_PIXELS`.

**Architecture Invariant**: only the open L10 cell creates planet meshes,
materials, or textures (bodies, rims, moons per ADR 0014), and any open
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

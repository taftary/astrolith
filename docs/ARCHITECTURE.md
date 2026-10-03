# Architecture

Bird's-eye view, entry points, and codemap of this repository. Anyone
asking "where is the thing that does X" should find the answer here in
under a minute.

Where is the code that decides when a marker opens? `universe-core`, the
`nav` module, `should_open`.

## Bird's-eye view

A Bevy application that renders a nested dive through universe scales
L1-L11. Three crates, one direction: `universe-core` holds deterministic
universe logic (std only), `universe-render` holds Bevy indicator rendering,
`universe-app` holds the window binary plus the headless `--verify` and
`--capture` drivers.

**Architecture Invariant**: dependency direction is `core <- render <- app`,
never otherwise.

**Architecture Invariant**: `universe-core` knows nothing about Bevy, wgpu,
winit, or any rendering or windowing crate.

**Architecture Invariant**: determinism. The same root seed and path
produce byte-identical snapshots on every run and platform (three tiers in
`E-DET-TIERS`).

**Architecture Invariant**: bounded memory. At most `2 + PREVIEW_CAP`
generations are alive at any time.

## Entry points

- Window: run `universe-app` with no flags. Opens the `Universe MVP`
  window: hover highlights a marker, click targets it, wheel or arrows dive,
  Spacebar runs the autopilot journey L1-L11, Esc quits. A wrong command
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
- `gen`: pure generator contracts.
- `density`, `astro`, `terrain`: L1-L3 density, galaxy-to-star sampling,
  L11 terrain.
- `snapshot`: canonical text snapshots (`snapshot_generated`).
- `cache`: fixed-capacity LRU cell store.
- `verify`: determinism and border-agreement predicates.
- `noise`: deterministic noise helpers.
- `nav`: `Universe`, `DiveEvent`, `dive_step`, `should_open`,
  `should_close`, `sibling_in_open_units`, `drawn_radius`,
  `open_marker_radius`, `PreviewCache`, `preview_positions`,
  `preview_error`, `JourneyStep`, `replay_autopilot`, navigation constants,
  `Seed`, `MarkerIndex`, and the allocation-free `sync`.
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
  resource newtypes over `universe-core::nav`, and the angle re-exports.
- `camera.rs`: `spawn_indicator_camera`, `sync_camera`.
- `input.rs`: `handle_quit`, `pick_hover`, `handle_input`,
  `Navigation`, `Autopilot`.
- `draw.rs`: `draw_axes`, `sync_previews`, `draw_open_cell`,
  `draw_previews`, `draw_parent_siblings`.
- `style.rs`: `point_color_for_level`, `sibling_color_for_level`,
  `scaled`, `to_vec3`, `PICK_PIXELS`.

**Architecture Invariant**: no meshes, materials, or textures are created
here (`E-RENDER-NO-MESH`).

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

## Cross-cutting concerns

- Determinism: fixed root seed (`DEMO_SEED`), fixed timestep
  (`VERIFY_DT = 1/60 s`), fixed seeds in tests, six-decimal snapshots,
  `total_cmp` ordering. Three tiers (`E-DET-TIERS`); transcendental calls
  confined to core generators (`E-TRANSCENDENTAL`).
- Bounded memory: live generations capped at `2 + PREVIEW_CAP`; per-frame
  allocation forbidden in `Update` systems (`E-HOT-NOALLOC`); the preview
  cache reuses its buffers for a static camera.
- Testing: ladder in `E-TEST-LADDER`; hermetic tests (`E-TEST-HERMETIC`);
  golden baseline (`E-GOLDEN`); properties (`E-PROTEST`); `App`-level and
  schedule-order tests in render (`E-BEVY-TEST`).
- Error handling: no panics outside tests (`E-NO-PANIC`); `Option` versus
  `Result` (`E-OPT-RESULT`); the window app panics on system bugs in
  debug and logs them in release (`E-SYS-RESULT`).
- Diagnostics: `bevy::log` macros (`E-LOG-TRACING`); reserved output
  vocabulary (`E-VOCAB`); `RUST_BACKTRACE=1` in automated runs
  (`E-BACKTRACE`).
- Simulation timing: fixed timestep for simulation systems; 60 fps target
  budget lives in `docs/universe/stack.md` (`E-BUDGET`).
- Command line: exit 0 pass, 1 check failure, 2 wrong usage (`E-EXIT-CODE`,
  the one approved behavior change, Q7).

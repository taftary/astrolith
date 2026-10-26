# Universe stack, pins, and architecture rules

Source: Issue #35 (spec v4 approved, owner decisions, plan comment).
Status: starting point for M1–M5. Deliberate upgrades only.

## Language and toolchain pin

- Language: Rust, stable channel, edition 2024.
- Toolchain: exact stable `1.97.1`, pinned in `rust-toolchain.toml`.
- Lints: `rustfmt` and `clippy` components installed via the toolchain file.

## Bevy pin

- Bevy: exact `=0.19.1`, the newest 0.19 patch release (stable, not 0.20-rc).
- Declared once in the root `Cargo.toml` under `[workspace.dependencies]`;
  `universe-render` and `universe-app` inherit it via `bevy.workspace = true`.
- `default-features = false`: only the features below are enabled.
- The full dependency tree (including transitive versions) is recorded in
  the root `Cargo.lock`, and CI builds with `--locked`.

## Enabled Bevy features and why each

Read from the pinned `bevy 0.19.1` manifest (`[features]` table) and
`bevy_internal 0.19.1` (`src/default_plugins.rs`, `src/lib.rs`).
Each feature maps to plugins added by `DefaultPlugins`:

| Feature | Why it is enabled |
|---|---|
| `std` | Desktop platform base; gates terminal handler and task pools |
| `multi_threaded` | Multi-threaded task pools and pipelined rendering |
| `bevy_log` | `LogPlugin`: diagnostics output |
| `bevy_asset` | `AssetPlugin`: shader and pipeline asset loading for the renderer |
| `bevy_window` | `WindowPlugin`: window handle and window events |
| `bevy_winit` | `WinitPlugin`: opens the OS window and runs the event loop |
| `x11` | Linux X11 backend for winit (CI runs ubuntu-latest) |
| `wayland` | Linux Wayland backend for winit |
| `bevy_render` | `RenderPlugin`: GPU renderer abstraction |
| `bevy_image` | `ImagePlugin`: GPU textures and render targets the pipelines need |
| `bevy_camera` | `CameraPlugin`: camera components and visibility |
| `bevy_color` | `Color` type for gizmo drawing (part of Bevy's own `common_api`) |
| `bevy_core_pipeline` | `CorePipelinePlugin`: presents rendered frames to the window |
| `bevy_gizmos` | `GizmoPlugin`: immediate-mode lines and points API |
| `bevy_gizmos_render` | `GizmoRenderPlugin`: draws gizmos through the renderer |
| `bevy_pbr` | `PbrPlugin`: required host for the 3D gizmo line pipeline (`LineGizmo3dPlugin` only installs when `PbrPlugin` is loaded; our code still creates no meshes) |
| `bevy_mesh` | `MeshPlugin` plus the `Mesh3d`/`MeshMaterial3d` components and sphere primitives for the L10 bare planet body and moons (ADR 0016, #375; gizmos stay for everything else) |
| `bevy_post_process` | `PostProcessPlugin`: the `Bloom` camera component for bright sources plus the filmic tonemap stage (ADR 0015, #157; `Tonemapping::TonyMcMapface` needs no LUT feature) |
| `bevy_ui` | `UiPlugin`: the persistent scale HUD overlay (level, distance, scale bar as `Node` plus `Text`, #157; window title keeps its readout) |
| `bevy_ui_render` | `UiRenderPlugin`: draws UI nodes to the window; `bevy_ui` alone only runs layout and interaction, leaving every button, readout, and label blank (owner report, #473 follow-up) |
| `bevy_state` | `StatesPlugin` via `App::init_state`: the `Screen` state (`Universe`, `Menu`, `Guides`, `Settings`) with `in_state` gating pausing the dive pipeline behind the screens (#473) |
| `default_font` | Embedded `FiraMono-subset` font for every `Text` node (menu glyph and screens, HUD readout, portal labels): without it `bevy_text` ships no font and all UI text renders blank (owner report, #473 follow-up) |
| `png` | PNG encoding of the `--screens` window pictures (#492, ADR 0021) |

Screen text uses bundled fonts instead (#473, ADR 0023): Inter (SIL OFL 1.1) and Lucide icons (ISC), embedded from `crates/universe-render/assets/fonts/` with their licence files. `default_font` still sets the HUD readout and the portal labels.

Deliberately off: audio, text-to-speech, sprites, glTF, picking,
scene serialization, animation, input focus, gamepad, dev tools.
Indicators plus the #375 bare planet meshes and the #157 bloom/HUD need none of them.
Any addition must be pinned here with its justification first.

## Graphics backend per platform

Chosen in `crates/universe-app/src/gpu.rs` for both window builds (normal
play and `--screens`; ADR 0042, #848, following ADR 0022 and ADR 0041). No
extra Bevy feature: the backends come with `bevy_render`'s own wgpu
defaults.

| Platform | Backend | Why |
|---|---|---|
| All, Windows included | The engine's automatic choice (Vulkan on the dev PC) | The common practice for wgpu games; on the dev PC it picks Vulkan, the only backend there that draws the atmosphere correctly (ADR 0041) |

- `WGPU_BACKEND` (for example `dx12` or `vulkan`) always overrides the choice, so a developer can test another backend on purpose.
- On Vulkan at most two frames are in flight: a frame waits for the one before the previous before it takes its window image (`FramePacingPlugin`, ADR 0041, ADR 0044).
- `.cargo/config.toml` sets `VK_IMPLICIT_LAYER_PATH` and `VK_LOADER_LAYERS_DISABLE` for cargo runs. This hides the stale Steam layer loader lines, and implicit layers (overlays) do not load.

## Workspace crates and dependency rules

Moved to `docs/ARCHITECTURE.md` (codemap) and `docs/engineering.md`
(crate rules) in #85 (Q6). Direction of allowed dependencies:
core <- render <- app.

## Architecture rules

Moved to `docs/ARCHITECTURE.md` (invariants) and `docs/engineering.md`
(rules) in #85 (Q6). Headline: headless core, one plugin per feature,
DRY, SOLID, documented public items, fixed timestep, data-driven content.

## Budgets

- Frame rate: 60 fps target (owner decision).
- Draw-call budget: TBD (set before content lands, M3 at the latest).
- Texture budget: TBD (no textures in M1–M5; indicators only).
- Memory: live cell count bounded by the LOD threshold (`θ_min`),
  never by universe size (notion invariant).
- Ground memory: the planet's ground patches (CPU plus GPU, counted from
  the buffers) stay under the Ground memory limit, 64 MB by default
  (8 / 16 / 32 / 64 / 128 MB in Settings; #918, ADR 0049; a 9.3 KB tile per patch in one shared grid, #920, ADR 0050).

TBD numbers above are open decisions, not proven results. The owner
tests plausibility and feel; automated checks prove determinism,
border agreement, and green CI.

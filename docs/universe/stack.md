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

Deliberately off: audio, UI, text, sprites, meshes, glTF, picking,
scene serialization, animation, input focus, gamepad, dev tools.
Indicators only (axes, points) need none of them.
Any addition must be pinned here with its justification first.

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

TBD numbers above are open decisions, not proven results. The owner
tests plausibility and feel; automated checks prove determinism,
border agreement, and green CI.

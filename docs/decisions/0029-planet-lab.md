# 0029 — Planet lab, a separate development program

- Status: proposed
- Date: 2026-10-14
- Issue: #640 (Spec v1, owner answers Q1-Q10, session 2026-10-14)

## Context

The owner wants a development view focused only on planet and moon
generation (L10-L14): a classic 3D editor with a nodes tree, a tools
panel, menus, a grid, axes and camera views, running on PC only. Players
never see it. The game's own UI is Bevy UI, which is slow to build rich
editor panels with (trees, fields, sliders, menus).

## Decision

1. A new crate `crates/planet-lab` (binary `planet-lab`, `publish =
   false`). It adds a new dependency direction, `planet-lab -> render ->
   core`. No game crate depends on it, and a test pins that. The binary
   builds only for Windows, Linux and macOS (`compile_error!` elsewhere).
2. The lab draws the planet with the game's own planet code, through a
   public plugin of `universe-render`. That plugin brings in later tasks
   of #640.
3. `bevy_egui` 0.42 (MIT) is the lab's UI toolkit: the release for Bevy
   0.19, with `default-features = false` and only `render`. It is a
   lab-only dependency, never used by a game crate. `deny.toml` lets
   `planet-lab` wrap the `bevy` facade beside `universe-render` and
   `universe-app`.
   - egui's default fonts are left out. Their OFL-1.1 and Ubuntu-font-1.0
     licences are not on the allow-list. The lab installs the game's
     bundled Inter font instead (ADR 0023).
   - `bevy_egui` turns on `bevy_image`'s `zstd_rust`, which pins `ruzstd`
     0.8 (RUSTSEC-2026-0344, a decompression bomb). Neither program
     decodes untrusted `.zst` or KTX2 data, so the advisory is ignored in
     `deny.toml` with that reason. It lifts with the next Bevy ADR.
4. On Windows the lab uses Vulkan by default. On the dev PC's Intel GPU,
   the egui pass loses the DirectX 12 device within seconds ("device
   lost"), while the same window without egui runs fine on DirectX 12.
   Vulkan logs the known loader and validation lines there (#499), but it
   runs. The game keeps DirectX 12 (ADR 0022). `WGPU_BACKEND` still
   overrides both.

## Consequences

- The lockfile gains `bevy_egui`, `egui` and their dependencies (listed
  in the dependency pull request with the `cargo tree` diff). The game's
  dependency tree is unchanged.
- The CI dependency-direction step also refuses a game crate depending on
  `planet-lab`.
- A later Bevy upgrade must move `bevy_egui` to the release for that
  Bevy version in the same pull request.

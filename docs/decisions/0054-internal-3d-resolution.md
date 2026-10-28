# 0054 - Internal 3D resolution through an image target, and the frame cap

- Status: accepted
- Date: 2026-10-28
- Issue: #1022 (Spec v4 points 3, 5 and 6, R3; tasks T6 #1063 and T7 #1064)

## Context

Auto quality and the Manual 3D resolution setting draw the 3D scene below the window's size while text, menus and labels stay at native size (Spec v4 point 3, R3). The scene is one 3D camera with HDR, bloom, filmic tonemapping, the atmosphere and the gizmo lines; until now the UI also rode on that camera.

The spike looked at two routes in Bevy 0.19:

- **`MainPassResolutionOverride`.** It shrinks only the main pass, inside the full-size target. Post-processing (bloom, tonemapping) still runs at full size over a partly filled texture, and no upscaling pass comes with it. Using it means writing that pass plus making each effect read the smaller viewport. Rejected.
- **An image target.** The 3D camera draws into an image of the scaled size, and a second camera stretches the image over the window. Every effect runs at the image's size unchanged. The image target's scale factor (`ImageRenderTarget::scale_factor`) can make the camera's logical size equal the window's. Kept.

## Decision

- `render_scale::RenderScalePlugin`, part of `UniverseRenderPlugin`, holds `RenderScale`: the effective linear scale, never saved. Manual sets it from the saved 3D resolution (a preset as tall as the window or taller counts as Native, Q25). Auto keeps native until its controller is connected (T10). `RenderScalePin` (`ASTROLITH_RENDER_SCALE` in `--screens` runs) fixes it for evidence.
- **At native** the 3D camera draws straight into the window and carries the UI, exactly as before. The present camera is off.
- **Below native** (size from `universe_core::quality::scaled_size`: 10-point steps, the 720-pixel floor, never above native, the aspect kept):
  - the 3D camera draws into an `Rgba8UnormSrgb` image with no CPU copy;
  - its target scale factor is the image width over the window's logical width, so `world_to_viewport` returns window logical pixels. Labels, picking and the LOD's pixel basis need no change;
  - a `Camera2d` present camera (order 1, its own empty render layer, no MSAA, no tonemapping) shows the image in a full-window `ImageNode` under all other UI. It is the default UI camera, so menus, the HUD and labels draw at native size;
  - gizmo line widths scale with the picture, so lines look the same width once stretched.
- A size change removes the old image from the assets, which releases its texture. The UI-camera marker is placed explicitly because Bevy otherwise picks the highest-order window camera, active or not.

## Consequences

- One extra full-window draw when scaled, and none at native.
- Upscaling is bilinear. A sharper upscaler can replace the present step later without touching the 3D camera.
- Tests: the same world points project to the same window points at 100 % and 70 %; a resize loop keeps at most one picture; moving the effective scale never changes the saved settings. Real-window shots at 1920 x 1080 compare 100 % and 70 % (`.agent/validation/issue-1022/t6/`).

## Frame cap (T7)

- `universe_core::pacing` decides the cap, purely:
  - with vsync on a display whose refresh equals the target, no cap is added: vsync paces alone;
  - at a whole multiple (120 Hz for 60 FPS), the cap takes every other refresh;
  - at any other refresh, the cap still holds the target. `pacing_note` says in plain words why frames cannot be even. The target is never changed silently;
  - with vsync off or an unknown refresh, the cap alone paces.
- `frame_cap::FrameCapPlugin`, added only by the window runs, sleeps in `Last` after `FrameWorkSet::MainEnds` until the next deadline. A late frame starts a new deadline, so there is no catch-up burst.
  - The sleep is `std::thread::sleep`, which on Windows waits on a high-resolution timer, with no busy-wait.
  - It is recorded as `WorkSample::cap_ms` (`cap_mean_ms` in timing lines), apart from the work, so the controller never counts it.
  - `PacingStatus` keeps the pacing and its note for Settings (S9).
- Measured on the dev PC (`.agent/validation/issue-1022/t7/summary.md`):
  - with vsync off the cap holds 60 and 30 FPS within a fraction of a millisecond on average;
  - with vsync on, this PC's Vulkan present stays near 40 ms whatever the cap, the floor of ADR 0044.

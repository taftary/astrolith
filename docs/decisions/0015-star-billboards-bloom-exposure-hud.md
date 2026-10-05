# 0015 — Star billboards, bloom, exposure, and scale HUD

- Status: accepted
- Date: 2026-10-05
- Issue: #157 (Spec v1 approved; second amendment to ADR 0002 after ADR 0014, which stays accepted for L10 planet meshes)

## Context

ADR 0002 says `universe-render` draws indicators only and creates no
meshes, materials, or textures; ADR 0014 relaxes it for exactly the open
L10 cell. The realism review (#143, theme 5.7) asks for per-object colors
with a white Sun, HDR emissive sources with bloom and filmic tonemap, a
manual exposure control, and a persistent scale readout, which gizmo
spheres with one color per era and tonemapping off cannot show.

## Decision

`universe-render` may additionally:

- tint every gizmo marker from its core `ColorInfo` (stars by O-M class,
  galaxies by kind and density; planets keep their biome colors);
- spawn one camera-facing emissive quad per bright portal tint
  (brightness at or above the Sun) with a dimmer record past the
  far-view gate, rebuilt only when the open cell changes;
- render the frame through `Bloom` plus the filmic `TonyMcMapface`
  tonemap with a manual exposure resource (`E` dims, `Shift+E`
  brightens, opening at the pre-#157 look);
- draw a persistent scale HUD overlay (level name, distance order,
  rung-growing bar) beside the window title.

Previews stay gizmos; populations stay gizmo-only; nothing moves; the
dive route and timing never change. The `bevy_post_process` feature
carries bloom and the `bevy_ui` feature carries the HUD text, each pinned
with its justification in `docs/universe/stack.md`.

## Consequences

Per-star blackbody colors and per-galaxy type colors reach the screen
from deterministic core data with snapshot bytes to match. Bright
sources bloom without clipping through the filmic curve. The exposure
keys clash with nothing taken. Frame proof covers the visual change;
golden files move only on the new tint bytes.

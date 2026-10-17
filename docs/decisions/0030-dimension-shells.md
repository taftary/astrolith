# 0030 — Dimension shells around the planet

- Status: accepted
- Date: 2026-10-17
- Issue: #729 (correction round 2)

## Context

The dive ends at L10 with only the hemisphere dome, but the owner
notion wants the L10-L14 dimensions fixed as planet radius plus a value,
scaling down continuously while diving to L10. Locked answers: display
only (entering stays impossible), concentric shells, continuous motion.
The round-2 plan (four faint wireframe shells, fading in and out) was
approved before building.

## Decision

Four wireframe shell spheres centred on the planet, one per dimension,
at planet radius plus a fixed share (L11 outermost … L14 innermost),
each in its era tint:

- Each shell draws only while the camera is outside it, at the approach
  fade of its angular size times a smooth crossing factor, so passing
  through never pops. From the ground all four are gone.
- Display only: no portals, no entering, no labels, no stdout lines.
  Guides, HUD, and goldens are untouched; the dive cap stands.
- Shell radii and strengths are pure functions (`shell_radii`,
  `shell_strength`) with headless tests; the draw is immediate gizmos
  off the frame thread budget like other lines.

## Consequences

The L9 approach shows nested shells around the target planet; the dive
sinks through them to the dome. This record refines ADR 0029 (which
stays accepted for the dome and rim).

# 0029 — Hemisphere dome with a shown rim

- Status: accepted
- Date: 2026-10-17
- Issue: #729 (correction round 1)

## Context

ADR 0028 drew the planet as a small follow patch beside nothing. Owner
feedback on the merged result: the semi-sphere must be complete, not a
part, its plane edges forming a circle, with the edges shown; further
decisions are the AI's.

## Decision

The dome becomes the complete hemisphere facing the follow focus:

- **Extent.** Angular radius exactly PI/2 from the centre. Same follow
  rule (body walking, camera ground point flying), same fade, same base
  tone, same rebuild triggers. Core `patch_centre` is unchanged.
- **Grid.** A polar grid (pole vertex, 24 rings to PI/2, 96 segments:
  4,512 triangles), because a tangent-plane grid cannot reach PI/2.
  Fixed resolution, pure `f64`, deterministic; every sample stays
  planet-fixed, so terrain never slides.
- **Rim.** A gizmo line loop through the actual rim vertices, lifted a
  hair (`RIM_LIFT`) off the surface so the lines never z-fight the mesh,
  in the new `RIM_TINT`. Nothing draws beyond the rim.
- **Rebuild hygiene.** The stored camera is compared in surface-cell
  units like the live value; comparing mapped against unmapped rebuilt
  the mesh every frame at L9 and the churn kept it off screen.

## Consequences

This record refines ADR 0028 (which stays accepted for patch-only).
Close-range screenshots change from a small ground patch to a horizon
with a rim circle; the L9 approach shows the dome disc with its rim.

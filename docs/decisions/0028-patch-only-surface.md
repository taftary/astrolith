# 0028 — Patch-only planet surface

- Status: accepted
- Date: 2026-10-16
- Issue: #729

## Context

ADR 0024 put the planet-to-building leg on one icosphere mesh, and ADR
0027 added a follow patch beside its ball. The owner notion (#729) calls
the whole icosphere wrong: the L10-L13 levels stay but are displayed
differently, only the semi-plane patch draws, nothing below L10 opens,
and the work continues from clean things. Locked answers: dive to L10 as
today, patch dome alone there, no entering, delete the icosphere code
outright, rewrite verify output, goldens, guides, and the ADRs.

## Decision

The render-side icosphere is deleted: `surface_mesh.rs`,
`surface_split.rs`, and `planet_approach.rs` are gone (about 1,840
lines), with their tests. In its place one dome mesh draws:

- **Dome.** `patch.rs` builds a fixed 48×48 grid bent onto the sphere
  around the follow focus (the walking body, else the camera's ground
  point), sampling the planet-fixed surface, in the base surface tone
  with flat normals. No ball, no markers, no edge lines, no skirts: the
  grid is continuous, so neighbours never crack.
- **When.** At L10 the open cell's frame draws at full strength; at L9
  the nearest fading planet draws first, scaled by the cell ratio. The
  open cell's own markers, previews, siblings, and portal labels draw
  nowhere at L10: the dome is the whole surface.
- **Rebuild.** The dome rebuilds when the camera or the focus moves past
  a share of its height; turns alone never rebuild (splits are gone).
- **Dive cap.** `MAX_NAV_LEVEL` and `MAX_OPEN_LEVEL` are 10. The
  autopilot, the seeded destination, and the player spot end at the L10
  planet; goldens are regenerated with the reasons in the pull request.
- **Stdout.** The `LEVEL`/`SNAPSHOT` lines stop at L10, `JOURNEY`
  reaches L10, `RATIO` stops at L9→L10, `HORIZON` names L10 only, and
  the `PARENT-CONTEXT` line is removed. Formats are unchanged; the
  removed legs are recorded here as the decision `E-STDOUT-PROTOCOL`
  requires.

## Consequences

This record supersedes ADR 0024 and ADR 0027; both stay on file marked
superseded. Close-range L10 screenshots change from faceted ball plus
patch to one smooth dome. `E-LADDER-FROZEN` keeps `docs/universes/`
untouched: the frozen ladder still describes L11-L14 while the code caps
at L10, until the owner approves a rewrite.

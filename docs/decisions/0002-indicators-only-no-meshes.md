# 0002 — Indicators only, no meshes

- Status: accepted
- Date: 2026-10-03
- Issue: #35 (spec v4)

## Context

The MVP visualizes universe structure, not surfaces. Meshes, materials,
and textures would pull in asset, material, and texture pipelines for no
product need.

## Decision

`universe-render` draws indicators only (axes, points, lines via gizmos).
It creates no meshes, materials, or textures (`E-RENDER-NO-MESH`). The
enabled Bevy feature list in `docs/universe/stack.md` stays minimal, each
feature justified there.

## Consequences

The renderer stays small and headless-testable. Any mesh need is a new
decision, not a quiet addition.

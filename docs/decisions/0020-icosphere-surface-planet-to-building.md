# 0020 — Icosphere surface from planet to building

- Status: proposed
- Date: 2026-10-10
- Issue: #458 (Spec v2 approved)

## Context

ADR 0002 says `universe-render` draws indicators only and creates no
meshes, materials, or textures. ADR 0016 narrows one exception: the open
L10 cell builds one flat-tone unlit body plus one small body per moon.
ADR 0017 maps every tail form to a gizmo primitive, so L11-L13 ground is a
bowl with `Form::Grid`, `Form::Rect`, `Form::Thread`, and `Form::Ring`.
ADR 0019 keeps meshes L10-only while covering entries with limb, rim, and
kept parent context.

The owner notion (#458) unifies the planet-to-building leg on one surface:
the planet, its regions, cities, and plots are one continuous icosphere,
heights from the planet's own height field, entrances as highlighted
triangle groups. Separate flat rectangles from L10 to L13 go away.

## Decision

`universe-core` gains a pure deterministic icosphere (`icosphere.rs`):
a 20-face base icosahedron, triangle addressing by base face plus child
path, 1->4 split at edge midpoints re-projected to the unit sphere.
One `surface_radius(seed, direction)` over `terrain::height_at` (5
octaves, same relief range at every level) is shared by generators,
meshes, and `ground::clamp_above_ground`. Clusters are seeded flood fills
restricted to the parent cluster, so they nest exactly and cover the whole
parent; enterable counts stay today's (6 regions per planet, about half
the markers per tail level).

L10 meshes its region depth, L11 the entered region to city depth, L12
the city to plot depth, L13 plots with one `Form::Box` per enterable plot;
L14 room is unchanged. Each level meshes its own triangles in the open
cell's local frame in f64, narrowed to f32 only at upload.

`universe-render` extends the planet exception to L10-L13: one filled
unlit mesh per open level L10-L13 with highlighted tiles in a brighter
tone as vertex colors. Kept parent mesh and child previews use the same
triangles, so entries and exits change nothing (R7 / ADR 0019). The ground
grid draws go away; horizon, sky, previews, limb, and air rim stay, drawn
against the new surface. `ground`, `landing`, and `frame::local_up` read
the icosphere surface and cluster normals.

## Consequences

Supersedes the mesh clause of ADR 0016 (meshes L10-only becomes meshes
L10-L13 for the open surface levels) and the Patch/Rect/Grid/Thread/Ring
tail forms of ADR 0017 on the L10-L13 leg (entrances are highlighted
triangle groups; rivers, roads, lakes, and decorative boxes are removed
for now). ADR 0002 stays accepted for everything else; L1-L9 indicators
and the L14 room are untouched.

`verify.txt`, `capture.txt`, and the frame proof move for L10-L13 once,
each change explained in the pull request; L1-L9 and L14 stay
byte-identical. Triangle budget is split near the camera only, capped per
frame for 60 FPS.

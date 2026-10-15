# 0031 — Generated planets and moons

- Status: accepted
- Date: 2026-10-15
- Issue: #685 (Spec v2, owner decisions Q1-Q5, candidates S1, S2, G1 all yes)

## Context

Spec v2 asks for every planet and moon generated from its seed and its
system's age: fourteen planet types with relief, plates, craters,
landforms, climate colours, clouds, giant bands, rings and moon systems,
behind a new Planet look, with the home planet generated too. The
research sits in `guides/universe/L10/generation/` (sources only, never
shown). The work landed in fourteen task branches (#686-#699) on
`feat/685-generated-worlds`.

## Decision

1. Two worlds behind one setting (`PlanetLook { Normal, Squares,
   Generated }`, stored strings `normal`/`squares`/`generated`).
   Normal and Squares keep today's universe byte-identically (the
   Earth-Moon home fixture, class sizes, smooth surface); Generated
   switches core into `WorldMode::Generated`, where every planet and
   moon, the home one included (Q4), comes from the generator. The
   default stays Normal until acceptance.
2. One pure recipe per body in `universe-core/src/world/` (std only,
   `f64`, deterministic): classifier (Chen-Kipping), star ages, air
   (cosmic shoreline over the star's life), energy-balance climate,
   plates (Voronoi, boundary profiles), craters by surface age,
   rivers-first landforms with pure lakes, per-type relief tuning
   against hypsometry targets, cloud cover and drift, giant bands,
   storms and rings, moon systems (origins, limits, budgets, heating,
   resonances, binaries, Trojans), and lifetime (activity fade,
   uplift-age erosion, locking, decay, dwarf dry-out).
3. One ground seam: every reader of the ground calls
   `SurfaceFrame::ground_radius` (flat faces in Classic, pure relief in
   Generated, scaled by the relief-height setting in the planet's own
   metre). Mesh corners, walked ground and the headless check agree
   (AC3); heights never depend on split depth.
4. Render stays visual-only on top: vertex-coloured relief, two drifting
   cloud shells behind the Clouds setting, air-model sky and rim, giant
   band fields, ring annuli with viewing-angle opacity, and potato moons
   tinted by type. Clouds never feed gameplay, so GPU float noise is
   acceptable there.
5. The engine lab shows the recipe rows and a pinned type gallery (first
   seed per planet and moon kind, re-pinned by running the ignored
   `pin_scan` if generation moves them); the game and lab `--screens`
   modes each gain Generated shots.
6. Guides gains Help > Planets (the G1 text); Relief height (0.5-2x) and
   Clouds (on/off) join Display > Scene, saved like the other settings.

## Consequences

- Classic mode is byte-identical: goldens, `--verify` and the home
  fixture tests are untouched by Generated work.
- New kinds of test: determinism for seed and age, LOD purity,
  classifier and moon-kind coverage, caps with hypsometry modes, plate
  boundary rules, crater-age laws, river and lake rules, the shoreline
  cases, moon limits, lifetime monotonicity, gallery pin re-resolution,
  and the approved guide text.
- Switching the default to Generated after owner acceptance is a
  separate small pull request.
- Baked erosion and baked lakes stay out (Q2 non-goal); animated
  seasons, caves, life and cities were never in scope.

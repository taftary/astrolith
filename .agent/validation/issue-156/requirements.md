# Requirements (CONFIRMED) — issue #156

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/156#issuecomment-5991318364,
owner approval https://github.com/taftary/astrolith/issues/156#issuecomment-5991340721).
One criterion per acceptance criterion, in owner words. Criteria about
variety, home fixtures, and crowding are judged by the
validator directly (headless output plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: L10 shows squashed balls with hills, valleys, and shorelines instead of 24 flat points; Earth reads as blue oceans with land, clouds, and ice under a thin rim of air (verify: file crates/universe-core/src/terrain.rs contains EARTH_FLATTENING)
- [ ] C2: surface climates follow heat plus wetness (ocean, shore, desert, grass, forest, peaks, cold plains, ice) with Earth-like worlds at about 71 percent ocean and other bodies without the two-group split (verify: file crates/universe-core/src/terrain.rs contains OCEAN_FRACTION_EARTH)
- [ ] C3: approaching a planet grows its detail tile by tile with no cracks at tile edges and no stutter on the dev PC; far balls and close surfaces are the same world (verify: file crates/universe-render/src/planet.rs contains planet_mesh_for)
- [ ] C4: planets carry moons inside safe distances (Earth with its Moon); moons never highlight, never take the target, never open (verify: file crates/universe-core/src/terrain.rs contains MOON_ORBIT_KM)
- [ ] C5: planets carry tilt and spin from the observed pattern as data; nothing orbits or spins on screen yet; the home path ends at the fixed Earth-Moon with procedural neighbours kept clear (verify: file crates/universe-core/src/nest.rs contains is_home_planet_cell)
- [ ] C6: same seed gives the same planets on rerun and across Windows/Linux; a regenerated cell byte-matches; counts and portal mapping outside L10 are unchanged (verify: contains VERIFY-OK in verify.log)
- [ ] C7: --verify ends in VERIFY-OK; --capture succeeds; the golden rewrite is explained line by line in the PR; the mesh permission update names this Issue; window rendering outside L10 keeps its current style (verify: file docs/decisions/0014-planet-meshes-for-l10.md contains 0002)

# Requirements (CONFIRMED) - issue #384

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/384#issuecomment-6079585828,
owner approval https://github.com/taftary/astrolith/issues/384#issuecomment-6079597254).
One criterion per acceptance criterion, in owner words. Criteria about
what each level looks like are judged by the validator directly (frame
proof plus code reads where stated); the script marks those UNVERIFIABLE
and they never count as MET from a log line. Snapshot tokens named in the
probes: `F <form> ...` (form), `Q dx dy dz` (portal offset inside the
body), `E void|wall|filament|node` (cosmic-web environment).

- [ ] C1: Every marker in every saved snapshot carries a form token; no level draws a bare wireframe sphere for a non-spherical thing (verify: contains F disk in verify.log)
- [ ] C2: Every marker that opens carries a portal location, and opening, closing, preview, and sibling placement follow it: diving into a marker and out again lands at the same place (verify: contains INVERSE in verify.log)
- [ ] C3: No generated point sits farther than half a cell from its cell centre (verify: contains in-sphere=PASS in verify.log)
- [ ] C4: Far markers are dots; from 0.006 rad they show their form; from 0.02 rad they preview; at 0.14 rad they open; brightness is continuous across all four stages (verify: file crates/universe-core/src/nest.rs contains FORM_ANGLE: f64 = 0.006)
- [ ] C5: L1 shows the observable sphere with a graticule and eight octant sub-spheres (verify: file crates/universe-render/src/forms.rs contains fn draw_graticule)
- [ ] C6: L2 and L3 snapshots carry an environment per point; filaments link to two neighbours; voids, walls, nodes, clusters, and groups are each drawn differently (verify: contains F thread in verify.log)
- [ ] C7: L4 spirals are tilted ellipses with a bar and arms, ellipticals nested ellipses, irregulars clumps, each at its own size; the home galaxy's portal sits in its disk, not at its centre (verify: contains HOME-PORTAL L4 off-centre=PASS in verify.log)
- [ ] C8: L5 shows the disk ellipse, two arms, and clouds sized by mass; L6 stars are sized by mass with companion arcs (verify: file crates/universe-core/src/sysgen.rs contains Form::Arc)
- [ ] C9: L7 shows the belt and disc rings and two Oort shell circles; L8 shows one orbital plane with every planet on its own non-overlapping orbit circle, ordered by distance on a log scale, sized by radius, belts as bands, the heliopause as the outer shell (verify: contains ORBITS L8 coplanar=PASS ordered=PASS spaced=PASS in verify.log)
- [ ] C10: L9 draws the star body at the centre, sized by mass and coloured by class, with the planets in the same plane as L8 (verify: contains F body in verify.log)
- [ ] C11: L10 region portals sit on the planet's surface as patches; moons have orbit circles; equator and spin axis are drawn; surface sample points are in the snapshot but not on screen (verify: contains F patch in verify.log)
- [ ] C12: L11 sits on a curved patch; L12 and L13 on flat grids; regions, cities, and buildings are outlines (buildings with height); L14 has a room box around its furniture (verify: contains F box in verify.log)
- [ ] C13: The debug axes are hidden by default and toggle with X (verify: file crates/universe-render/src/input.rs contains KeyCode::KeyX)
- [ ] C14: --verify ends with VERIFY-OK, the journey reaches L14 through the same home fixtures, and the same seed gives byte-identical snapshots on repeated runs (verify: contains VERIFY-OK in verify.log)
- [ ] C15: The golden files are rewritten once in the pull request with the change explained per line class (verify: file crates/universe-app/tests/golden/verify.txt contains F disk)
- [ ] C16: Frame proof for every level L1 to L14 is posted on the Issue (verify: file scripts/validation/frame_proof.py contains EXPECTED_COUNT = 15)
- [ ] C17: The decision records for marker nesting (0004) and portal-vs-population (0010) are amended to name the body/portal split; indicators-only (0002) is unchanged (verify: file docs/decisions/0017-body-and-portal-forms.md contains Amends ADR 0004 and ADR 0010)

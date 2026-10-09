# Requirements (CONFIRMED) - issue #394

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/394#issuecomment-6086438229,
owner approval https://github.com/taftary/astrolith/issues/394#issuecomment-6086447810).
One criterion per acceptance criterion, in owner words. Criteria about
what each level looks like are judged by the validator directly (frame
proof plus code reads where stated); the script marks those UNVERIFIABLE
and they never count as MET from a log line. Verify lines named in the
probes are introduced by this Issue: `TAIL-ANCHORS`, `MILESTONES`,
`ATMOSPHERE`, `CURVATURE`, `HORIZON`, `PORTAL-ON-STRUCTURE`,
`PARENT-CONTEXT`; the surface snapshot token `T` grows by the atmosphere
fields (`T height flattening tilt spin air tint`).

- [ ] C1: The ladder anchor table lists a published object and size for L11, L12, L13, and L14 (500 km range or basin, 10 km city, 30 m building, 5 m room), and the size ratios used by the program follow them (verify: contains TAIL-ANCHORS L11=5.70 L12=4.00 L13=1.50 L14=0.70 in verify.log)
- [ ] C2: Invisible milestones follow the 1.5-decade rule on the anchored gaps: none between planet and region, two between region and city, two between city and building, none between building and room; the gap note and the L11 guide say the same (verify: contains MILESTONES L10-L11=0 L11-L12=2 L12-L13=2 L13-L14=0 in verify.log)
- [ ] C3: Every planet in a saved snapshot carries an atmosphere thickness and tint; moons and small rocks are airless, Earth-sized rock is thin and pale blue, giants are thick and pale amber; the home planet carries Earth's values (verify: contains ATMOSPHERE home=thin in verify.log)
- [ ] C4: The open L10 planet shows a thin tinted rim of concentric circles outside its body, fading outward; an airless world shows none (verify: file crates/universe-render/src/planet.rs contains fn draw_air_rim)
- [ ] C5: L10 region portals sit on the land at spots of distinct height, on the surface, not at the six face centres; each carries its exact spot and the region opens there (verify: contains PORTAL-ON-STRUCTURE L10=PASS in verify.log)
- [ ] C6: The ground at L11, L12, and L13 curves by the real planet radius at that level's scale (a gentle bowl at L11, nearly flat at L12, flat to the eye at L13); the hand-picked L11 constant is gone (verify: contains CURVATURE L11= in verify.log)
- [ ] C7: A horizon line is drawn at L11, L12, and L13 at the true distance for the viewer's height; it lies beyond the cell on entry and closes in as you descend (verify: contains HORIZON L11=beyond-cell in verify.log)
- [ ] C8: L11 shows a sky arc with haze bands, L12 a sky arc without haze, L13 the horizon line only (verify: file crates/universe-render/src/sky.rs contains fn draw_sky)
- [ ] C9: City portals sit on the region's river and coast points; building portals sit on city block edges along streets; the room portal sits inside the building box at a floor height. All placements are in the snapshot (verify: contains PORTAL-ON-STRUCTURE L10=PASS L11=PASS L12=PASS L13=PASS in verify.log)
- [ ] C10: Every L10-L13 portal carries its exact spot; diving into it and out again lands at the same spot (the existing round-trip check passes for the tail) (verify: contains INVERSE in verify.log)
- [ ] C11: After opening, the level you came from stays drawn faintly around you at the true ratio (planet limb and air around the region, region outline and ground around the city, city blocks around the building, building walls around the room), at one constant faint brightness, until the next open (verify: contains PARENT-CONTEXT L11=PASS L12=PASS L13=PASS L14=PASS in verify.log)
- [ ] C12: No new meshes, no rim shell mesh: air, sky, horizon, and parent context are gizmo indicators; the planet body and moons are the only meshes, as today (verify: file crates/universe-render/src/lib.rs contains no rim entity survives)
- [ ] C13: Levels L1-L9 are unchanged on screen and in their snapshots (verify: contains L9 in verify.log)
- [ ] C14: --verify ends with VERIFY-OK, the journey reaches L14 through the same home fixtures, and the same seed gives byte-identical snapshots on repeated runs (verify: contains VERIFY-OK in verify.log)
- [ ] C15: The golden files are rewritten once in the pull request with the change explained per line class (verify: file crates/universe-app/tests/golden/verify.txt contains TAIL-ANCHORS)
- [ ] C16: Frame proof for every level L10 to L14 is posted on the Issue (verify: file scripts/validation/frame_proof.py contains EXPECTED_COUNT = 15)
- [ ] C17: The ladder document carries the R11 amendment (anchors L11-L14, gap note); ADR 0002, 0016, and 0017 are unchanged; the decision that chose flat L12-L13 in #384 is recorded as superseded on this Issue (verify: file docs/universes/ladder.md contains R11 amendment)

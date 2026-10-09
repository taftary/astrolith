# Requirements (CONFIRMED) - issue #375

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/375#issuecomment-6076176242,
owner approval https://github.com/taftary/astrolith/issues/375#issuecomment-6076189107).
One criterion per acceptance criterion, in owner words. Criteria about
what the planet and the new levels look like are judged by the validator
directly (frame proof plus code reads where stated); the script marks
those UNVERIFIABLE and they never count as MET from a log line.

- [ ] C1: The planet shows no climate zones, no sea line, and no glowing rim: one flat-tone ball with its hills and its moons (verify: file crates/universe-render/src/planet.rs contains PLANET_TONE)
- [ ] C2: The planet's saved snapshot carries no biome letter, no sea level, and no air field (verify: file crates/universe-core/src/snapshot.rs contains T height flattening tilt spin)
- [ ] C3: From the planet, a marker can be targeted and opened into L11 Regions (verify: contains JOURNEY opened=L11 - Regions in verify.log)
- [ ] C4: The dive continues L11 to L12 to L13 to L14 by opening markers, and L14 Room is the end: nothing in it can be opened (verify: contains JOURNEY opened=L14 - Room in verify.log)
- [ ] C5: Pressing Space from L1 reaches L14 Room on the fixed route without stopping, and the same route is taken on every run (verify: contains JOURNEY reached=L14 - Room levels-opened=13 PASS in verify.log)
- [ ] C6: Diving out from L14 returns up through L13, L12, L11, L10 and on to L1, landing at the same place each time (verify: contains INVERSE in verify.log)
- [ ] C7: The HUD, window title, and scale bar name every level L1 to L14 with its own anchor; "Beyond MVP" appears nowhere (verify: file crates/universe-core/src/labels.rs contains L14 - Room)
- [ ] C8: Each of L11 to L14 has its own tint; the room's furniture appears as outlines (verify: file crates/universe-render/src/draw.rs contains draw_room_outlines)
- [ ] C9: The same seed gives byte-identical snapshots for every level L1 to L14 on repeated runs (verify: contains LEVEL L14 - Room in verify.log)
- [ ] C10: universe --verify ends with VERIFY-OK and its report covers L1 to L14 (verify: contains VERIFY-OK in verify.log)
- [ ] C11: Golden files are updated in the pull request with an explanation of the change (verify: file crates/universe-app/tests/golden/verify.txt contains nested dive L1-L14)
- [ ] C12: A new decision record supersedes ADR 0014 and records the end of the ladder at L14 (verify: file docs/decisions/0016-bare-planet-and-ladder-to-room.md contains Supersedes ADR 0014)

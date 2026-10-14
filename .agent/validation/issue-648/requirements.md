# Requirements (CONFIRMED) - issue #648

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/648#issuecomment-6162192161,
owner approval https://github.com/taftary/astrolith/issues/648#issuecomment-6162675069),
with the decisions D1-D4. How the lab and the game look on the real window
is judged by the main agent's `--screens` pictures and the owner test; the
validator takes no pictures (#533).

- [ ] C1: In the planet lab on Earth, the distance to the centre at a corner of the ground reads 6,371 km, and between about 5,950 and 6,371 km anywhere on the ground (AC1) (verify: contains earth_ground_reads_its_real_radius ... ok in cargo-test.log)
- [ ] C2: On any other planet, the distance to the centre at a ground corner matches the planet's size readout (AC2) (verify: contains every_planet_ground_sits_at_its_real_radius ... ok in cargo-test.log)
- [ ] C3: Heights use the planet's real size, and the level follows them: L11 below 1,000 km, L12 below 100 km, L13 below 1 km, L14 below 10 m on Earth (AC3) (verify: contains shells_follow_heights_in_planet_metres ... ok in cargo-test.log)
- [ ] C4: The player stands 1.8 m tall against Earth's real radius; speeds in m/s and the 100 m squares stay in real metres (AC4) (verify: contains body_and_squares_use_the_planet_metre ... ok in cargo-test.log)
- [ ] C5: Surface gravity on Earth still reads 9.8 m/s2, and a body lands and stands on the ground from L10 to L14 (AC5) (verify: contains bodies_stand_on_the_ground_in_planet_metres ... ok in cargo-test.log)
- [ ] C6: From orbit and at L1-L10 the game looks exactly as today; the same seed gives the same universe on every run (AC6, AC8) (verify: contains VERIFY-OK in verify.log)
- [ ] C7: Earth's radius is 6,371 km in both the game and the lab (AC7) (verify: contains one_earth_radius_everywhere ... ok in cargo-test.log)
- [ ] C8: An ADR records the planet metre and replaces the single ladder metre rule from #632 (AC9) (verify: file docs/decisions/0030-planet-metre.md contains one metre per planet)

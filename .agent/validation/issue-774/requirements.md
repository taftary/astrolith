# Requirements (CONFIRMED) - issue #774

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/774#issuecomment-6247255966,
owner approval https://github.com/taftary/astrolith/issues/774#issuecomment-6247359464),
with Q1-Q6 (all A) and #770's Phase 2 decisions (Q3 A, Q6 A, Q8 A, Q9 A).
Nothing changes on screen (Q6 A); the precision is proven by measurement.

- [ ] C1: Each L10 planet has a planet frame in metres, centred on it, with its radius from its own record (Earth 6,371 km); the same seed gives the same frame (AC1, Q1 A, Q2 A, Q3 A) (verify: contains planet_frame_per_planet_in_metres ... ok in cargo-test.log)
- [ ] C2: A local frame (up, north, east) exists at any point, its axes at right angles and of unit length (AC2) (verify: contains local_frame_is_orthonormal ... ok in cargo-test.log)
- [ ] C3: L10 cell units to the planet frame and back return the start point within 1 µm at every tested height (AC3) (verify: contains cell_to_planet_round_trip_within_a_micrometre ... ok in cargo-test.log)
- [ ] C4: Every point within 1 km of the camera shakes by less than 1 mm after the full chain, at every height from 1 cm to 2,000 km over a face centre, an edge and a corner; farther, under one pixel at 1600×900 (AC4, Q5 A) (verify: contains shake_under_a_millimetre_near_the_camera ... ok in cargo-test.log)
- [ ] C5: The camera, body, spot, moons, markers and saved views at L10 go through the one conversion, and an old settings file with saved views still loads to the same place (AC5, Q4 A) (verify: contains l10_positions_share_one_conversion ... ok in cargo-test.log)
- [ ] C6: The headless check prints a `PRECISION` line with the worst shake in millimetres per height (AC6, Q6 A) (verify: contains PRECISION in verify.log)
- [ ] C7: Everything else behaves as after #770, and the headless game check passes (AC7) (verify: contains VERIFY-OK in verify.log)

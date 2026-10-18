# Requirements (CONFIRMED) - issue #775

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/775#issuecomment-6248707242,
owner approval https://github.com/taftary/astrolith/issues/775#issuecomment-6248727245),
with Q1-Q7 (all A). Spec v3 keeps v1's AC1–AC8 and only rewords point 5,
test steps 3 and 6, and the arrival floor (review items 1 and 4); C1–C8
below already match v3. What the sphere looks like on the real window is judged
by the owner test and the main agent's screenshots; the validator takes no
pictures (#533).

- [ ] C1: Each L10 planet is six faces mapped onto the sphere by the tangent-adjusted mapping at its record radius, and the backward step returns a point to its face and place within 1 µm (AC1, Q1 A) (verify: contains cube_sphere_maps_and_inverts ... ok in cargo-test.log)
- [ ] C2: Zero mismatches along every face seam and corner for positions, normals and test heights (AC2, Q3 A, Q5 A) (verify: contains seams_match_bit_for_bit ... ok in cargo-test.log)
- [ ] C3: Largest/smallest point spacing over the whole sphere under 1.5, with centre, edge and corner values reported (AC3, Q6 A) (verify: contains spacing_stays_under_one_and_a_half ... ok in cargo-test.log)
- [ ] C4: At L10 the planet is drawn as a plain smooth sphere in one flat colour, unlit, at fixed resolution, through the one conversion of #774 (AC4, Q2 A) (verify: contains the_planet_sphere_is_drawn_at_l10 ... ok in cargo-test.log)
- [ ] C5: Arrival is above the true surface (at least 3× the radius) for every planet including giants, Earth's arrival is unchanged, and the dive closes far away relative to the planet (AC5, Q7 A) (verify: contains arrival_and_close_follow_the_planet ... ok in cargo-test.log)
- [ ] C6: Moons sit between 1.5× and 4× the radius in their order, the ring just outside the surface, and L10 markers are measured against the planet's space (AC6, Q4 A, Q7 A) (verify: contains moons_and_ring_sit_outside_the_planet ... ok in cargo-test.log)
- [ ] C7: The headless check prints a `CUBE-SPHERE` line (seam mismatches, spacing ratio, round trip) (AC7) (verify: contains CUBE-SPHERE in verify.log)
- [ ] C8: Everything else behaves as after #774, and the headless game check passes (AC8) (verify: contains VERIFY-OK in verify.log)

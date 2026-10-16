# Requirements (CONFIRMED) - issue #685 Spec v3

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/685#issuecomment-6208822339,
owner approval https://github.com/taftary/astrolith/issues/685#issuecomment-6208830597),
replacing Spec v2 purity (AC3/C3) with dynamic height (LOD + Levels).
How the generated planets look on the real window is judged by the main agent's
`--screens` pictures and the owner test; the validator takes no pictures (#533).

- [ ] C1: From far above (L10) to the ground (L14), the same mountains keep their shape while smaller hills and hollows fade in continuously, with no popping or jumps, matching how the sphere mesh splits (verify: contains dynamic_height_fades_continuously_without_pops ... ok in cargo-test.log)
- [ ] C2: High up only big shapes show (continents, ranges); low down small shapes join (hills, craters, rivers, fine detail). Far ground never shimmers or crawls (verify: contains far_ground_uses_only_big_shapes ... ok in cargo-test.log)
- [ ] C3: In Normal mode (press 1 on the ground) you stand on the drawn ground, at every shell from L10 to L14 (verify: contains walked_ground_equals_full_detail_drawn ... ok in cargo-test.log)
- [ ] C4: Shells read from the relief ground: L14 below 10m, L13 below 1km, L12 below 100km, L11 below 1000km on Earth (scaled by planet size except L14 always 10m), with no flicker at a boundary (verify: contains shells_read_relief_ground_with_order ... ok in cargo-test.log)
- [ ] C5: Restart the game: the same planets look exactly the same (verify: contains generation_is_deterministic_for_seed_and_age ... ok in cargo-test.log)

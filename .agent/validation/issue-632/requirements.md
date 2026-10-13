# Requirements (CONFIRMED) - issue #632

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/632#issuecomment-6156103832,
owner approval https://github.com/taftary/astrolith/issues/632#issuecomment-6156111573),
with the clarification answers Q1-Q6 and the Guides answers G1 yes, G2 yes.
Spec v2 replaces Spec v1. How the descent feels on the real window is judged
by the main agent's screenshots and the owner test.

- [ ] C1: On a planet the level follows only the height above the ground: below 1,000 km L11, below 100 km L12, below 1 km L13, below 10 m L14 on Earth (AC1) (verify: contains the_level_follows_the_height_above_the_ground ... ok in cargo-test.log)
- [ ] C2: Going up, a level is left only above 1.5 times its height (AC1) (verify: contains a_level_is_left_above_one_and_a_half_times_its_height ... ok in cargo-test.log)
- [ ] C3: On other planets L11 to L13 scale with the planet radius and L14 is always 10 m (AC2) (verify: contains the_heights_scale_with_the_planet_and_l14_stays_ten_metres ... ok in cargo-test.log)
- [ ] C4: The rule is the same anywhere on the planet, at the equator, the poles and the far side (AC3) (verify: contains the_level_is_the_same_anywhere_on_the_planet ... ok in cargo-test.log)
- [ ] C5: The rule is the same in dive, free, normal walk and fly (AC3) (verify: contains the_level_is_the_same_in_every_mode ... ok in cargo-test.log)
- [ ] C6: Climbing above the L11 leave height puts you back in L10 (AC4) (verify: contains climbing_above_l11_returns_to_l10 ... ok in cargo-test.log)
- [ ] C7: Nothing on screen appears, disappears or jumps when the level changes (AC5) (verify: contains a_level_change_moves_nothing_on_screen ... ok in cargo-test.log)
- [ ] C8: The HUD shows "next:" at L10 to L13 and the readout 10^6, 10^5, 10^3, 10^1 m at L11 to L14 (AC6) (verify: contains the_tail_readout_matches_the_shell_heights ... ok in cargo-test.log)
- [ ] C9: L11 to L14 hold no region, city or building markers, no scenery and no room outlines (AC7) (verify: contains the_shell_levels_hold_only_the_planet ... ok in cargo-test.log)
- [ ] C10: A dive from L10 goes down to its target point on the ground, the player by default, and rests in L14 above the ground (AC8) (verify: contains the_dive_rests_in_l14_above_the_target_point ... ok in cargo-test.log)
- [ ] C11: Down arrow rises back up through the levels to L10 (AC8) (verify: contains rising_returns_through_the_levels_to_l10 ... ok in cargo-test.log)
- [ ] C12: Help > About > The scale ladder shows the four new L11 to L14 rows (AC9, G1) (verify: contains the_scale_ladder_shows_the_shell_heights ... ok in cargo-test.log)
- [ ] C13: Help > Navigation ends with the new sentence word for word (AC9, G2) (verify: contains navigation_explains_levels_by_height ... ok in cargo-test.log)
- [ ] C14: L1 to L10 behave exactly as before, and the same seed gives the same universe on every run (AC10) (verify: contains VERIFY-OK in verify.log)

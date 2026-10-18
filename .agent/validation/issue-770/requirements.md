# Requirements (CONFIRMED) - issue #770

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/770#issuecomment-6245693467,
owner approval https://github.com/taftary/astrolith/issues/770#issuecomment-6245702171),
with the draft's decisions of 2026-10-17 (far ball deleted, L1-L10 skeleton
kept, a broken intermediate state accepted, goldens updated), Q1-Q10 and
S1, G1, G2 yes. What the L10 view looks like on the real window is judged by
the owner test; the validator takes no pictures (#533).

- [ ] C1: The removed list is gone from the code, and the kept list is still there and works: the L1-L10 journey, markers, names, autopilot, saved views, dive, free flight and the player work of #754-#756 (AC1, Q2 A) (verify: contains old_surface_removed_journey_kept ... ok in cargo-test.log)
- [ ] C2: At L10 no planet is drawn, the moons still are, and nothing panics or logs an error when you reach or leave L10 (AC2, decision 1) (verify: contains no_planet_drawn_at_l10_moons_stay ... ok in cargo-test.log)
- [ ] C3: The levels stop at L10 everywhere: `MAX_LEVEL` 10, labels, the ladder, the snapshots (AC3) (verify: contains levels_stop_at_l10 ... ok in cargo-test.log)
- [ ] C4: The headless check, the capture, the goldens and the parity check pass on the cleaned state, and two runs give the same output (AC4, decision 4) (verify: contains VERIFY-OK in verify.log)
- [ ] C5: The player keeps its spot and can be taken over; with no ground the body floats and flies, and the camera and controls of #754-#756 work (AC5, decision 3, Q1 A) (verify: contains player_floats_without_ground ... ok in cargo-test.log)
- [ ] C6: `guides/universe/L10/generation/` is unchanged, and no guide page references removed code (AC6, Q7 A) (verify: contains guides_reference_no_removed_code ... ok in cargo-test.log)
- [ ] C7: The seven Phase 2 Issues exist, each linking its design-doc section and stating its targets, and the cleanup ADR lists them (AC7, Q4 A, Q8 A) (verify: file docs/decisions/0031-planet-surface-cleanup.md contains #780)
- [ ] C8: Settings > Display > Scene has no Ground guides row; an old settings file with `ground_guides` still loads (AC8, S1) (verify: contains ground_guides_setting_removed ... ok in cargo-test.log)
- [ ] C9: Help > Navigation and About > What is Astrolith show the G1 and G2 texts (AC9, G1, G2) (verify: contains gap_texts_for_navigation_and_about ... ok in cargo-test.log)

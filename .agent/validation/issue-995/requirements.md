# Requirements — Issue #995 (one continuous star system, L8-L10)

From Spec v2 (https://github.com/taftary/astrolith/issues/995#issuecomment-6412004684), approved in https://github.com/taftary/astrolith/issues/995#issuecomment-6412012793; Phase 2 started on the owner's `go` (system data wins).

- [ ] C1: Phase 1 study: a headless prototype held one star system in one frame at true scale and measured precision, handovers and on-screen sizes (AC1) (verify: contains zooms_hand_over_without_a_jump ... ok in cargo-test.log)
- [ ] C2: The numbers and a go or no-go recommendation were posted, and nothing further was built before the owner's `go` (AC2) (verify: file docs/decisions/0051-one-star-system-space.md contains a study first (`go` after the study report))
- [ ] C3: From the system view you zoom continuously to a planet, a moon or the star with no portal swap; the journey crosses L8 to L10 with no L9 opening (AC3) (verify: contains JOURNEY reached=L10 - Planets and moons levels-opened=8 PASS in verify.log)
- [ ] C4: At every handover nothing on screen jumps: the headless check prints the position, direction and distance change on the gravity-zone open and close, all within limits, and the journey arrives in daylight (AC4) (verify: contains HANDOVER planet=6 open_jump_mm=0.000 close_jump_mm=0.000 in verify.log)
- [ ] C5: Small bodies show their marker and label, and the real body takes over as it grows, with no gap or double image (AC5, Q1 A) (verify: contains true_scale_sizes_from_the_system_view ... ok in cargo-test.log)
- [ ] C6: The HUD names L8, L9 or L10 by distance and shows the distance to the nearest body (AC6, Q2 A, Q4 A) (verify: contains the_system_hud_names_the_level_by_distance_and_the_nearest_body ... ok in cargo-test.log)
- [ ] C7: L10 to L14, walking and the ground behave exactly as today (AC7) (verify: contains verify_stdout_matches_golden ... ok in cargo-test.log)
- [ ] C8: Frame time on this PC is no worse than today (ADR 0044 rule) with no new per-frame memory growth (AC8) (verify: file .agent/validation/issue-995/timing.txt contains AC8 frame time: PASS)
- [ ] C9: The headless game check passes; the golden journey changes once, as the ladder amendment says (AC9) (verify: contains VERIFY-OK in verify.log)
- [ ] C10: Help > Navigation shows the G1 text and About > The scale ladder shows the G2 line, word for word (AC10, G1 yes, G2 yes) (verify: contains guides_explain_one_star_system ... ok in cargo-test.log)
- [ ] C11: Moons sit at their true orbits inside their planet's gravity zone (Q3 A) (verify: contains moons_and_ring_sit_outside_the_planet ... ok in cargo-test.log)
- [ ] C12: Each planet has one size: the system view and the ground agree (owner: "go", system data wins) (verify: contains true_scale_sizes_from_the_system_view ... ok in cargo-test.log)
- [ ] C13: Instead of a sub-dimension portal it continues scaling (owner notion) (verify: contains a_zone_is_left_further_out_than_it_is_entered ... ok in cargo-test.log)

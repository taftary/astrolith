# Requirements (CONFIRMED) - issue #598

Correction round 1. Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/598#issuecomment-6152523359,
owner approval https://github.com/taftary/astrolith/issues/598#issuecomment-6152531140),
with the owner's answers R1-R3. Spec v2 replaces Spec v1. One or more
criteria per acceptance criterion, in the spec's words. How the split looks
on the real window is judged by the main agent's screenshots and the owner
test.

- [ ] C1: Triangles fully outside the view, with the margin, are not split past the default (AC1) (verify: contains only_triangles_in_view_split_past_the_default ... ok in cargo-test.log)
- [ ] C2: Far away, the planet stays the icosphere split once (AC1) (verify: contains far_planet_is_the_icosphere_split_once ... ok in cargo-test.log)
- [ ] C3: A triangle in view splits while it is larger than about 40 px on screen and stops once it is smaller (AC2) (verify: contains triangles_in_view_stop_near_forty_pixels_on_screen ... ok in cargo-test.log)
- [ ] C4: No level, place or target limits the depth: plain ground close by splits past the next level's depth (AC2) (verify: contains plain_ground_close_by_goes_past_the_next_level_depth ... ok in cargo-test.log)
- [ ] C5: The ground seen from the same height shows about the same triangle count at L10, L11, L12 and L13, within 25% (AC3) (verify: contains the_same_height_gives_about_the_same_count_at_every_level ... ok in cargo-test.log)
- [ ] C6: Entering a place keeps the same triangles on both sides (AC4) (verify: contains entry_keeps_the_same_triangles_on_both_sides ... ok in cargo-test.log)
- [ ] C7: The mesh never exceeds 6,000 triangles, even on the ground (AC5) (verify: contains the_budget_holds_even_on_the_ground ... ok in cargo-test.log)
- [ ] C8: With the budget full the centre of the view comes first, then the rest by size on screen (AC5) (verify: contains a_full_budget_keeps_the_view_centre_then_the_largest ... ok in cargo-test.log)
- [ ] C9: A turn past about 10 degrees asks for a rebuild; a smaller turn or a new target alone does not (AC6) (verify: contains only_a_turn_past_ten_degrees_asks_for_a_rebuild ... ok in cargo-test.log)
- [ ] C10: In the app the surface rebuilds on a turn past 10 degrees and not on a still camera or a small turn (AC6) (verify: contains the_surface_rebuilds_on_a_turn_only_when_past_ten_degrees ... ok in cargo-test.log)
- [ ] C11: The same seed still gives the same universe on every run (AC7) (verify: contains VERIFY-OK in verify.log)

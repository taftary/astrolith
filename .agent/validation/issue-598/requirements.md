# Requirements (CONFIRMED) - issue #598

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/598#issuecomment-6140678983,
owner approval https://github.com/taftary/astrolith/issues/598#issuecomment-6140841280),
with answers Q1-Q6. One or more criteria per acceptance criterion, in the
spec's words. How the split looks on the real window is judged by the main
agent's screenshots and the owner test.

- [ ] C1: Triangles fully outside the view (with the margin) are not split past the default; triangles in view are (AC1) (verify: contains only_triangles_in_view_split_past_the_default ... ok in cargo-test.log)
- [ ] C2: Far away, the planet stays the icosphere split once (AC1) (verify: contains far_planet_is_the_icosphere_split_once ... ok in cargo-test.log)
- [ ] C3: The target's triangles split down to the target's level depth whatever their size on screen (AC2) (verify: contains the_target_splits_to_its_depth_even_when_small ... ok in cargo-test.log)
- [ ] C4: Past the default only enterable clusters, the open cell and the target split; plain ground and closed populations stay at their level depth (AC3) (verify: contains plain_ground_stays_at_its_level_depth_and_enterable_places_go_deeper ... ok in cargo-test.log)
- [ ] C5: No triangle goes deeper than the next level's depth, and the mesh never exceeds 12,000 triangles (AC4, AC5) (verify: contains nothing_goes_past_the_next_level_depth_or_the_budget ... ok in cargo-test.log)
- [ ] C6: Entering a place keeps the same triangles on both sides (AC4) (verify: contains entry_keeps_the_same_triangles_on_both_sides ... ok in cargo-test.log)
- [ ] C7: With the budget full the target's triangles come first, then those nearest the screen centre, then the rest of the view by size (AC5) (verify: contains a_full_budget_keeps_the_target_then_the_view_centre ... ok in cargo-test.log)
- [ ] C8: A turn past about 10 degrees or a new target asks for a rebuild, a smaller turn does not (AC6) (verify: contains a_turn_past_ten_degrees_or_a_new_target_asks_for_a_rebuild ... ok in cargo-test.log)
- [ ] C9: In the app the surface rebuilds on a turn past 10 degrees and not on a still camera or a small turn (AC6) (verify: contains the_surface_rebuilds_on_a_turn_only_when_past_ten_degrees ... ok in cargo-test.log)
- [ ] C10: The same seed still gives the same universe on every run (AC7) (verify: contains VERIFY-OK in verify.log)

# Requirements (CONFIRMED) - issue #430

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/430#issuecomment-6096053141,
owner approval https://github.com/taftary/astrolith/issues/430#issuecomment-6096085601).
One criterion per acceptance criterion, in owner words. Window feel (test
plan steps 1-11) is judged by the owner test; the script marks a criterion
without a probe UNVERIFIABLE and it never counts as MET from a summary
claim. No criterion asserts a pull-request run observation.

- [ ] C1: Far from the planet the camera's up is the planet's axis; close to the surface it is the ground's up under the camera, blending in between (verify: contains local_up_is_axis_far_and_normal_near_at_l10 in cargo-test.log)
- [ ] C2: The picture never rolls suddenly at any level, including when looking straight down at a region (verify: contains up_rate_bounds_one_frame_and_looking_down_keeps_up in cargo-test.log)
- [ ] C3: Entering a region changes nothing about roll; the carried up already is the region's up (verify: contains region_entry_needs_no_roll in cargo-test.log)
- [ ] C4: The last part of the approach to a region, city or building bends to the ground's up and arrives above the place looking down at about 3.6 cell units up (verify: contains tail_landings_open_on_the_normal_at_rest in cargo-test.log)
- [ ] C5: Aiming still feels direct: the path is straight for most of the approach and bends only near the end (verify: contains straight_above_the_bend_start in cargo-test.log)
- [ ] C6: Galaxies, stars and planets from orbit keep the straight line (verify: contains no_normal_matches_the_straight_step_exactly in cargo-test.log)
- [ ] C7: After entry the camera turns smoothly from looking down to the next marker without a snap (verify: contains window_dive_lands_down_the_tail_with_bounded_roll_and_turn in cargo-test.log)
- [ ] C8: Diving never goes under the planet's relief or the ground, scrolling in, out, or along straight cuts (verify: contains landing_dive_never_cuts_the_planet_but_targeted_passes_through in cargo-test.log)
- [ ] C9: Free flight keeps the same standoff and cannot fly under the relief or the ground (verify: contains window_free_flight_holds_the_ground_standoff in cargo-test.log)
- [ ] C10: A room is approached from above the roof and the camera never passes through a wall (verify: contains tail_round_trips_keep_parent_context_and_no_new_meshes in cargo-test.log)
- [ ] C11: Inside a marker, before entry, the next place's ground, horizon, sky, outlines and shell are already faintly there and grow stronger on approach (verify: contains preview_context_matches_the_opened_child_at_its_scale in cargo-test.log)
- [ ] C12: After entry the previous place's surroundings stay drawn at the right scale (verify: contains kept_context_meets_the_marker_curve_at_entries_and_exits in cargo-test.log)
- [ ] C13: The previewed ground curve matches the curve after entry, and previewed rectangles and boxes follow the patch orientation (verify: contains grid_curvature_scales_with_linear_sizes in cargo-test.log)
- [ ] C14: On entering a region the planet body stays as the edge seen from the region; nothing switches at the moment of entry (verify: contains limb_preview_fades_in_and_meets_the_kept_limb_at_open in cargo-test.log)
- [ ] C15: Across every entry and exit from planet to room nothing appears, disappears or jumps at the moment of crossing (verify: contains tail_round_trips_keep_parent_context_and_no_new_meshes in cargo-test.log)
- [ ] C16: Spacebar keeps its straight journey and the headless verify output and capture file list match the saved references exactly (verify: contains VERIFY-OK in verify.log)
- [ ] C17: One decision record holds the camera model, amending ADR 0018 and the #403 amendment of ADR 0012 and leaving ADR 0013 unchanged (verify: file docs/decisions/0019-continuous-camera-planet-to-room.md contains amends ADR 0018)

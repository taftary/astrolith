# Requirements (CONFIRMED) - issue #446

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/446#issuecomment-6097371003,
owner approval https://github.com/taftary/astrolith/issues/446#issuecomment-6097377015).
One criterion per acceptance criterion, in owner words. Window feel (test
plan steps 1-9, including the Windows colour-filter checks) is judged by the
owner test; the script marks a criterion without a probe UNVERIFIABLE and it
never counts as MET from a summary claim. No criterion asserts a
pull-request run observation.

- [ ] C1: Every marker line is drawn at least 2 px wide; no marker line is thinner (verify: contains gizmo_width_comes_from_the_token in cargo-test.log)
- [ ] C2: Every marker dot has a minimum size on screen, so the smallest marker you can act on stays visible (verify: contains dot_floor_holds_twelve_pixels_and_actionable_never_dims in cargo-test.log)
- [ ] C3: Portals, the selection ring, the portal mark and labels are at least 3:1 against black at their dimmest drawn brightness (verify: contains every_actionable_tint_reaches_three_to_one_at_its_dimmest_step in cargo-test.log)
- [ ] C4: No marker colour reads as black for red-blind (protan) vision; dark reds are replaced by a lighter orange-red wherever they are drawn (verify: contains no_tint_maps_near_black_under_protanopia in cargo-test.log)
- [ ] C5: Three named steps: the entered body (shell) is brightest, children sit just below it, and the background context sits at least two stops (four times dimmer) under the children (verify: contains ladder_steps_sit_two_stops_apart in cargo-test.log)
- [ ] C6: Every portal shows a small ring at the true position of what is inside it, the same shape at every level; populations show no mark (verify: contains portal_rings_draw_headless_at_l1_l6_and_l11 in cargo-test.log)
- [ ] C7: A portal shows a text label when it is large enough on screen, when it is selected, or when it is under the pointer; smaller portals and populations show no label (verify: contains labels_belong_to_qualifying_portals_and_read_kind_name in cargo-test.log)
- [ ] C8: A label shows the kind and the name; the home chain keeps its real names; every other portal has a generated name, the same on every run for the same seed (verify: contains home_chain_keeps_its_real_names in cargo-test.log)
- [ ] C9: With colour removed, portals and populations are still told apart by shape: the portal mark and the label exist only on portals (verify: contains ring_centre_is_the_oriented_child_position_and_populations_carry_none in cargo-test.log)
- [ ] C10: Each level keeps one family of shapes; importance is shown by size and brightness, never by changing the shape (verify: contains every_tail_rung_has_its_own_tint in cargo-test.log)
- [ ] C11: The selected marker gets an outline ring at the line floor, plus a halo around its label; hover shows a brighter version of the same ring; neither relies on a colour change alone (verify: contains repeated_clicks_cycle_ranked_portals_and_rings_draw_at_the_floor in cargo-test.log)
- [ ] C12: With the mouse, the pick area is at least 24 px; a touch pick size of 44 pt / 48 dp is defined and used as soon as touch input exists (verify: contains pick_tokens_match_the_spec_size in cargo-test.log)
- [ ] C13: When pick areas overlap, the portal on the way to the current destination wins first, then the nearest portal; a population is never picked over a portal (verify: contains destination_portal_wins_then_nearest in cargo-test.log)
- [ ] C14: Clicking again on the same spot selects the next portal under the pointer, in the same order, and wraps around; moving the pointer away resets the order (verify: contains clicks_resolve_ranked_first_then_cycling in cargo-test.log)
- [ ] C15: Every value above is a named value in one place, named by role; the drawing code uses these names and holds no numbers of its own for them (verify: contains no_numeric_colour_literal_remains_outside_tokens in cargo-test.log)
- [ ] C16: The headless verify output and the capture file list match the saved references exactly (verify: contains VERIFY-OK in verify.log)

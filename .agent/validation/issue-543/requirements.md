# Requirements (CONFIRMED) - issue #543

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/543#issuecomment-6119337348,
owner approval https://github.com/taftary/astrolith/issues/543#issuecomment-6119350929),
with the owner's answers Q1 = B, Q2 = B, S1 yes, G1 yes. One criterion per
acceptance criterion, in the spec's words. The look on the real window
(test plan steps 1-5) is judged by the main agent's screenshots and the
owner test.

- [ ] C1: In a room, every piece of furniture stands on the floor and inside the walls (verify: contains furniture_stands_on_the_floor_inside_the_walls ... ok in cargo-test.log)
- [ ] C2: The room's height-to-width shape follows the building that was entered (verify: contains room_takes_the_building_shape_inside_the_cell ... ok in cargo-test.log)
- [ ] C3: The equator ring sits above the highest relief of the planet everywhere (verify: contains equator_ring_clears_the_highest_relief ... ok in cargo-test.log)
- [ ] C4: At the region, city and plot levels, the horizon ticks, the centre cross and the ground ruler show, never brighter than the places you can enter (verify: contains ground_guides_sit_on_the_ground_and_the_horizon ... ok in cargo-test.log)
- [ ] C5: At the region, city, plot and room levels, the place you came from carries a dim mark, never as bright as an enterable place (verify: contains entered_dots_keep_a_footprint_ring ... ok in cargo-test.log)
- [ ] C6: An airless world shows the gray overhead circle at the region and city levels; a world with air is unchanged (verify: contains airless_mark_stays_fainter_than_the_sky ... ok in cargo-test.log)
- [ ] C7: A room shows one compass tick on a wall (verify: contains compass_tick_lies_on_the_wall_and_points_inward ... ok in cargo-test.log)
- [ ] C8: Enterable places at the region to plot levels carry a size note, and the label limit is unchanged (verify: contains region_level_labels_carry_their_tile_count ... ok in cargo-test.log)
- [ ] C9: The readout shows a next line naming the next level from the planet to the plot level (verify: contains next_line_names_the_next_level_from_planet_to_plot ... ok in cargo-test.log)
- [ ] C10: Each planet holds one record of its facts, and planet flattening is gone from the data (verify: contains one_surface_record_carries_the_recipe ... ok in cargo-test.log)
- [ ] C11: Solar-system and planet cells place regions the same one way (verify: contains region_portals_ride_the_icosphere_surface ... ok in cargo-test.log)
- [ ] C12: The same seed still gives the same universe on every run (verify: contains VERIFY-OK in verify.log)
- [ ] C13: Nothing above the planet level changes (verify: contains verify_stdout_matches_golden ... ok in cargo-test.log)
- [ ] C14: Settings > Display > Scene has a Ground guides switch, on by default, that turns the guides off and on and is kept after closing the game (verify: contains widgets_write_through_the_store ... ok in cargo-test.log)
- [ ] C15: Help > Navigation ends with the sentence from G1 (verify: file crates/universe-core/src/guides.rs contains also names the next level you can dive into.)

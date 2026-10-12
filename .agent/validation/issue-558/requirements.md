# Requirements (CONFIRMED) - issue #558

Correction round 1: approved Spec v3
(https://github.com/taftary/astrolith/issues/558#issuecomment-6132064094,
owner approval https://github.com/taftary/astrolith/issues/558#issuecomment-6132391485)
replaces Spec v2 item 4 and AC4 with real speeds (correction answers Q1 B,
Q2 keep the room, Q3 A): C5 changes and C21-C22 are new. Everything else
derives from approved Spec v2
(https://github.com/taftary/astrolith/issues/558#issuecomment-6123202959,
owner approval https://github.com/taftary/astrolith/issues/558#issuecomment-6123209702),
with answers Q1-Q8, the candidate answers (all yes), and the drawing
decision A (camera-relative drawing). One criterion per acceptance
criterion, in the spec's words. The feel on the real window is judged by
the main agent's screenshots and the owner test.

- [ ] C1: Key 1 enters normal mode from any mode, the reminder names it, and V switches between first and third person (verify: contains normal_mode_stands_walks_and_switches_camera ... ok in cargo-test.log)
- [ ] C2: In normal mode at L10-L14 the body stands on the ground (on the planet toward its centre) and walks with W A S D, never sinking below the surface (verify: contains on_the_whole_planet_gravity_pulls_to_the_centre ... ok in cargo-test.log)
- [ ] C3: Space jumps and lands back on the ground (verify: contains a_jump_rises_and_lands_back_on_the_ground ... ok in cargo-test.log)
- [ ] C4: A double-tap of Space takes off into flight, Space and C move up and down while flying, and a double-tap lands (verify: contains double_space_takes_off_and_lands ... ok in cargo-test.log)
- [ ] C5: At the default speed step, walking covers 1.4 m per second, running 4 m/s, flying 10 m/s and flying with Shift 30 m/s, at every level from the planet (L10) to the room (L14) (Spec v3 AC1) (verify: contains walking_running_and_flying_are_real_speeds_at_every_level ... ok in cargo-test.log)
- [ ] C6: In normal mode the body cannot pass through room walls or furniture (verify: contains walls_and_furniture_stop_the_body_and_it_slides ... ok in cargo-test.log)
- [ ] C7: In normal mode the body cannot pass through building boxes (verify: contains buildings_are_solid_at_the_plot_level ... ok in cargo-test.log)
- [ ] C8: In free mode the camera passes through everything, the ground included (verify: contains window_free_flight_passes_through_the_ground ... ok in cargo-test.log)
- [ ] C9: In normal mode, walking close to a place opens it and walking away closes it, with no jump (verify: contains walking_up_to_a_place_opens_it_and_walking_away_closes_it ... ok in cargo-test.log)
- [ ] C10: Drone and plane turning follow the setting, drone by default (verify: contains plane_turning_rolls_and_drone_slides ... ok in cargo-test.log)
- [ ] C11: The body is true size, about 1.8 m, at every level (verify: contains the_body_is_true_size_at_every_level ... ok in cargo-test.log)
- [ ] C12: A true-size body keeps its shape when drawn on the whole planet (verify: contains camera_relative_drawing_keeps_a_true_size_body_on_a_planet ... ok in cargo-test.log)
- [ ] C13: The Flying turns and Starting camera settings exist with their defaults and are kept after closing the game (verify: contains round_trip_keeps_every_value ... ok in cargo-test.log)
- [ ] C14: Help gains the Normal mode page (G1) (verify: file crates/universe-core/src/guides.rs contains Press 1 to walk in your own body.)
- [ ] C15: Help > Navigation reads 1: normal mode (G2) (verify: file crates/universe-core/src/guides.rs contains 1: normal mode. 2: free mode. 3: dive mode.)
- [ ] C16: Help > Free flight ends with the pass-through sentence (G3) (verify: file crates/universe-core/src/guides.rs contains In free mode you pass through everything, even the ground.)
- [ ] C17: The body is drawn as an outline person in third person (verify: contains the_outline_person_stands_on_its_feet_at_true_height ... ok in cargo-test.log)
- [ ] C18: The same seed still gives the same universe on every run (verify: contains VERIFY-OK in verify.log)
- [ ] C19: The Starting camera setting picks normal mode's first camera, third person by default (verify: contains starting_camera_setting_picks_the_first_camera ... ok in cargo-test.log)
- [ ] C20: Settings > Controls > Keys names key 1 as normal mode, with no "coming later" note (the key list moved to the Keys page in #560) (verify: file crates/universe-core/src/keymap.rs contains Action::NormalMode => "Normal mode")
- [ ] C21: Shift+1 to Shift+9 multiply those speeds by 1/16 up to 16, step 5 being 1x (Spec v3 AC2) (verify: contains the_speed_steps_multiply_every_speed ... ok in cargo-test.log)
- [ ] C22: Walking from one level into the next keeps the same real speed, with no jump in speed when a place opens around you (Spec v3 AC3) (verify: contains walking_up_to_a_place_opens_it_and_walking_away_closes_it ... ok in cargo-test.log)

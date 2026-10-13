# Requirements (CONFIRMED) - issue #626

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/626#issuecomment-6153821386,
owner approval https://github.com/taftary/astrolith/issues/626#issuecomment-6153828190),
with the answers Q1-Q4 and the review answers R1-R6. Spec v2 replaces
Spec v1. How the squares look on the real window is judged by the main
agent's screenshots and the owner test; running on a phone is covered by
the owner test.

- [ ] C1: Display > Scene has a Planet look setting, Normal by default (AC1, S1) (verify: contains the_planet_look_switches_the_surface_to_squares_and_back ... ok in cargo-test.log)
- [ ] C2: The Planet look row flips the setting through the store (S1) (verify: contains widgets_write_through_the_store ... ok in cargo-test.log)
- [ ] C3: The setting is kept after closing the game (AC6) (verify: contains round_trip_keeps_every_value ... ok in cargo-test.log)
- [ ] C4: Neighbouring squares alternate black and white along every axis (AC2) (verify: contains neighbouring_squares_alternate_along_every_axis ... ok in cargo-test.log)
- [ ] C5: A square is 100 m and only pure black or pure white (AC2) (verify: contains a_square_is_one_hundred_metres_and_only_black_or_white ... ok in cargo-test.log)
- [ ] C6: Squares smaller than a few pixels fade smoothly to an even gray (AC4) (verify: contains small_squares_fade_to_an_even_gray ... ok in cargo-test.log)
- [ ] C7: The render origin reduced by the 200 m period gives the same squares, at any planet size (AC3) (verify: contains the_origin_reduced_by_the_period_gives_the_same_squares ... ok in cargo-test.log)
- [ ] C8: The shader's view maps render space to planet metres at L10-L13 (AC2, AC3) (verify: contains the_view_maps_render_space_to_planet_metres ... ok in cargo-test.log)
- [ ] C9: The squares stay glued to the planet when the camera moves (AC3) (verify: contains the_squares_stay_glued_to_the_planet_when_the_camera_moves ... ok in cargo-test.log)
- [ ] C10: With Squares the edge lines turn orange, and Normal puts the look and the edges back (AC1, AC5) (verify: contains the_planet_look_switches_the_surface_to_squares_and_back ... ok in cargo-test.log)
- [ ] C11: Normal look turns the squares off and the squares are 100 m (AC1) (verify: contains normal_look_turns_the_squares_off ... ok in cargo-test.log)
- [ ] C12: The screens mode takes the squares shots (AC2) (verify: contains shot_list_matches_the_spec_names_and_sizes ... ok in cargo-test.log)
- [ ] C13: ADR 0027 records the first custom shader (verify: file docs/decisions/0027-planet-squares-shader.md contains # 0027 — Planet squares shader)
- [ ] C14: The same seed still gives the same universe on every run (AC7) (verify: contains VERIFY-OK in verify.log)

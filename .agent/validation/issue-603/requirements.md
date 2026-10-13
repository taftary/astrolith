# Requirements (CONFIRMED) - issue #603

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/603#issuecomment-6141976126,
owner approval https://github.com/taftary/astrolith/issues/603#issuecomment-6142269965),
with answers Q1-Q4. One or more criteria per acceptance criterion, in the
spec's words. How the fade looks on the real window is judged by the main
agent's screenshots and the owner test.

- [ ] C1: The fade strength is zero below half the opening size and full exactly at the opening, rising smoothly between (AC1) (verify: contains approach_fade_runs_from_half_the_open_angle_to_the_open ... ok in cargo-test.log)
- [ ] C2: At L9 the planet surface is hidden while the planet looks small (AC1) (verify: contains far_away_no_planet_surface_shows ... ok in cargo-test.log)
- [ ] C3: The fade follows the planet's size both ways: approaching raises it, moving away lowers it (AC2) (verify: contains the_surface_fades_in_by_distance_and_out_again ... ok in cargo-test.log)
- [ ] C4: During the fade the surface sits at the planet's true size and place and splits by the #598 rule; at the opening the triangles match the inside view (AC3) (verify: contains the_approach_maps_like_the_open_and_draws_the_same_triangles ... ok in cargo-test.log)
- [ ] C5: Leaving the planet fades its surface instead of dropping it (AC2) (verify: contains leaving_the_planet_fades_the_surface ... ok in cargo-test.log)
- [ ] C6: The planet marker's outline fades out as the surface fades in, never both at full strength; other markers keep their outline (AC4) (verify: contains the_marker_outline_fades_out_as_the_surface_fades_in ... ok in cargo-test.log)
- [ ] C7: In the app the approached planet's surface and edge lines appear only in the last part of the approach, go away when backing out, and are absent at L8 (AC1, AC2) (verify: contains an_approached_planet_shows_its_surface_only_in_the_last_part_of_the_approach ... ok in cargo-test.log)
- [ ] C8: Inside the planet (L10-L13) the surface behaves as before (AC5) (verify: contains l11_open_cell_spawns_one_surface_and_leaving_despawns_it ... ok in cargo-test.log)
- [ ] C9: The same seed still gives the same universe on every run (AC6) (verify: contains VERIFY-OK in verify.log)

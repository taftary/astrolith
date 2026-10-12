# Requirements (CONFIRMED) - issue #559

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/559#issuecomment-6124224910,
owner approval https://github.com/taftary/astrolith/issues/559#issuecomment-6124230241),
with answers Q1-Q4 and the candidate answers (S1 yes, G1 yes). One
criterion per acceptance criterion, in the spec's words. The feel on the
real window is judged by the main agent's screenshots and the owner test.

- [ ] C1: The pull on the body is the sum of every massive body's pull with real values, and the strongest one is named (verify: contains masses_follow_their_bodies ... ok in cargo-test.log)
- [ ] C2: At a planet's surface the pull matches its real surface gravity (Earth about 9.8 m/s2), falling off with the square of the distance (verify: contains earth_pulls_nine_point_eight_at_its_surface ... ok in cargo-test.log)
- [ ] C3: The ground of a region still pulls like the planet beneath it (verify: contains the_ground_of_a_region_still_pulls_like_the_planet ... ok in cargo-test.log)
- [ ] C4: Near a planet the body falls toward it and lands (verify: contains a_body_high_above_the_planet_falls_back_and_lands ... ok in cargo-test.log)
- [ ] C5: Far between stars the pull is tiny (verify: contains between_the_stars_the_pull_is_tiny ... ok in cargo-test.log)
- [ ] C6: Without pull the body keeps drifting at its own speed (verify: contains without_ground_or_pull_the_body_drifts_at_its_own_speed ... ok in cargo-test.log)
- [ ] C7: Falling onto a place opens it (verify: contains falling_toward_a_region_opens_it ... ok in cargo-test.log)
- [ ] C8: Moving sideways fast enough near a body makes the body fall around it instead of into it (verify: contains moving_sideways_fast_enough_falls_around_the_planet ... ok in cargo-test.log)
- [ ] C9: The readout line names the strongest pull and hides when Show pull is off; free mode feels no pull (verify: contains the_readout_names_the_pull_on_the_planet_and_show_pull_hides_it ... ok in cargo-test.log)
- [ ] C10: The readout line reads like "pull: Earth 9.8 m/s2" (verify: contains pull_line_names_the_body_and_its_strength ... ok in cargo-test.log)
- [ ] C11: Display > Scene has a Show pull switch, on by default, kept after closing the game (verify: contains round_trip_keeps_every_value ... ok in cargo-test.log)
- [ ] C12: Help > Normal mode ends with the gravity sentence (verify: file crates/universe-core/src/guides.rs contains Gravity comes from every body around you, as in real space: near a planet you fall and walk, far from everything you drift. A faint arrow and the readout show the strongest pull.)
- [ ] C13: The same seed still gives the same universe on every run (verify: contains VERIFY-OK in verify.log)

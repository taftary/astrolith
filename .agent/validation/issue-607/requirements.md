# Requirements (CONFIRMED) - issue #607

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/607#issuecomment-6143125045,
owner approval https://github.com/taftary/astrolith/issues/607#issuecomment-6143131438),
with answers Q1-Q7 and the candidate answers (all yes: S1, S2, G1, G2, G3).
One or more criteria per acceptance criterion, in the spec's words. The
arrival view and the waiting body on the real window are judged by the main
agent's screenshots and the owner test.

- [ ] C1: The same seed always places the player on the same planet, region and spot on the region's ground (L11) (AC1) (verify: contains the_same_seed_always_spawns_at_the_same_spot_on_a_region_ground ... ok in cargo-test.log)
- [ ] C2: The start destination is the player (AC1, AC2) (verify: contains the_camera_starts_aimed_at_the_player ... ok in cargo-test.log)
- [ ] C3: With no destination chosen the autopilot flies from L1 to the player and ends just behind and above the body, in dive mode, looking at it (AC2) (verify: contains space_flies_from_l1_to_the_player_and_ends_behind_and_above_the_body ... ok in cargo-test.log)
- [ ] C4: The arrival pose is behind and above the body (AC2) (verify: contains the_arrival_pose_is_behind_and_above_the_body ... ok in cargo-test.log)
- [ ] C5: Clicking a place still sets it as the destination (AC3) (verify: contains a_click_sets_the_seeded_room_below_and_the_way_marker_keeps_it ... ok in cargo-test.log)
- [ ] C6: Pressing 1 near the body takes it over where it stands (AC4) (verify: contains near_the_body_1_takes_it_over_where_it_stands ... ok in cargo-test.log)
- [ ] C7: Pressing 1 elsewhere flies to the player first, then takes over on arrival (AC4) (verify: contains pressing_1_far_away_flies_there_then_hands_over_the_body ... ok in cargo-test.log)
- [ ] C8: Leaving normal mode leaves the body where it stands; that spot becomes the default target and is kept after a restart; Reset player brings back the seeded spot (AC5) (verify: contains leaving_the_body_keeps_and_saves_its_spot_and_reset_brings_the_seeded_one_back ... ok in cargo-test.log)
- [ ] C9: The Go to player key makes the player the destination again, and the autopilot rises out of another branch and returns (AC6) (verify: contains go_to_player_from_another_branch_rises_out_and_comes_back ... ok in cargo-test.log)
- [ ] C10: The player's destination survives leaving its way (AC6) (verify: contains an_anchored_destination_survives_leaving_its_way ... ok in cargo-test.log)
- [ ] C11: The player's spot is kept with the settings across a restart (AC5, S2) (verify: contains round_trip_keeps_every_value ... ok in cargo-test.log)
- [ ] C12: Help > Getting started ends with the G1 sentence (AC7) (verify: file crates/universe-core/src/guides.rs contains Press Space to let the autopilot fly the whole journey from L1 down to your player, standing on a planet.)
- [ ] C13: Help > Navigation names the Go to player key (AC7, G2) (verify: file crates/universe-core/src/guides.rs contains P: go back to your player.)
- [ ] C14: Help > Normal mode gains the G3 sentences (AC7) (verify: file crates/universe-core/src/guides.rs contains Your body waits where you leave it. Press 1 near it to take it back, or from far away to fly there first.)
- [ ] C15: The Go to player key is on the Keys page with P as its default (S1) (verify: file crates/universe-core/src/keymap.rs contains Action::GoToPlayer => "Go to player")
- [ ] C16: The same seed still gives the same universe on every run (AC8) (verify: contains VERIFY-OK in verify.log)

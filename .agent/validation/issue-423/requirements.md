# Requirements (CONFIRMED) - issue #423

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/423#issuecomment-6094939824,
owner approval https://github.com/taftary/astrolith/issues/423#issuecomment-6094952763).
One criterion per acceptance criterion plus one per owner-expectation
sentence, in owner words. Window feel is judged by the owner test; the
script marks a criterion without a probe UNVERIFIABLE and it never counts
as MET from a summary claim. No criterion asserts a pull-request run
observation. Probes refreshed for correction round 1 (criteria text unchanged):
C14 now probes the end-to-end traversal line, following the #403 precedent of
giving every feel criterion an automated probe.

- [ ] C1: There is always a final destination; from the first frame it is the room the Spacebar journey would reach (verify: contains the_camera_starts_aimed_at_the_spacebar_journeys_room in cargo-test.log)
- [ ] C2: In every dimension the camera looks toward the destination, at the marker in the current view that leads to it, also at the cosmic web (verify: contains the_root_is_on_the_way_and_names_the_first_marker in cargo-test.log)
- [ ] C3: Scrolling in enters only the dimensions on the way; any other marker passed is not entered and does not change the destination (verify: contains scrolling_in_through_a_marker_off_the_way_does_not_enter_it in cargo-test.log)
- [ ] C4: Scrolling out leaves the destination unchanged and keeps looking toward it at every step out (verify: contains manual_exit_carries_the_heading_and_keeps_aiming_at_the_marker_just_left in cargo-test.log)
- [ ] C5: Crossing into or out of a dimension never makes the view jump or turn suddenly and never leaves the camera looking at nothing (verify: contains oriented_entry_carries_heading_and_up_and_aims_at_the_next_marker_on_the_way in cargo-test.log)
- [ ] C6: Clicking a marker makes the destination the room the seeded journey reaches under it; the same click gives the same room every run (verify: contains clicking_another_portal_gives_its_seeded_room_every_time in cargo-test.log)
- [ ] C7: Clicking a marker already on the way keeps the same destination (verify: contains a_click_sets_the_seeded_room_below_and_the_way_marker_keeps_it in cargo-test.log)
- [ ] C8: If the branch under a clicked marker has no room, the deepest place on that branch becomes the destination (verify: contains a_branch_without_portals_ends_at_its_deepest_marker in cargo-test.log)
- [ ] C9: When the room is reached, scrolling in does not move past it; the camera stays there until a new click or a scroll out (verify: contains the_autopilot_stops_once_the_room_is_reached in cargo-test.log)
- [ ] C10: Leaving free flight sets the destination to the room under the nearest marker (verify: contains f_toggles_dive_and_free_flight in cargo-test.log)
- [ ] C11: Spacebar flies to the current destination; from the beginning it makes the same journey as today and the headless output is unchanged (verify: contains VERIFY-OK in verify.log)
- [ ] C12: A saved view remembers the destination and recalling it restores it (verify: contains a_saved_view_brings_its_destination_back in cargo-test.log)
- [ ] C13: The camera looks at the room itself once it is reached (verify: contains the_camera_looks_at_the_marker_on_the_way_else_the_room_on_arrival in cargo-test.log)
- [ ] C14: Owner expectation: navigation between dimensions is always fluid, from the cosmic web to the room and back (verify: contains JOURNEY reached=L14 - Room levels-opened=13 PASS in verify.log)

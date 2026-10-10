# Requirements (CONFIRMED) - issue #403

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/403#issuecomment-6093449561,
owner approval https://github.com/taftary/astrolith/issues/403#issuecomment-6093454512).
One criterion per acceptance criterion plus one per owner-expectation
sentence, in owner words. Window glide-feel criteria are judged by the
validator directly (code reads where stated, frame proof) and by owner
testing; the script marks those UNVERIFIABLE and they never count as MET
from a log line. No criterion asserts a pull-request run observation.
Probes refreshed for correction round 2 (criteria text unchanged): the
camera heading is carried through every open and close, the exit re-lock
searches the cell returned to, and the open cell closes on one angular
rule (owner decision on #403, ADR 0012 amendment).

- [ ] C1: On entry, the view continues smoothly from the previewed interior with no snap and no content swap (verify: file crates/universe-render/src/input.rs contains carry_into_child)
- [ ] C2: The first wheel and arrow moves inside glide continuously and the viewpoint never jumps; the stored target is resolved in the new cell (verify: file crates/universe-render/src/input.rs contains takes the portal ahead)
- [ ] C3: Small moves inside never throw the dive out on their own; a fresh entry holds through the first outward dives (verify: contains small_moves_after_entry_stay_inside_and_the_slack_is_the_same_for_keys_and_wheel in cargo-test.log)
- [ ] C4: A deliberate long outward move still returns to the parent, at the marker that was entered (verify: contains opening_keeps_camera_outside_child_cell in cargo-test.log)
- [ ] C5: Exiting re-locks along the way out in the cell returned to and keeps the view instead of teleporting it (verify: contains manual_exit_carries_the_heading_and_relocks_in_the_cell_returned_to in cargo-test.log)
- [ ] C6: Entering universes feels enhanced: entry and the first moves are one continuous glide (verify: contains oriented_entry_carries_heading_up_and_sample_through_the_frame_and_relocks_ahead in cargo-test.log)
- [ ] C7: The deterministic journey is unchanged: --verify ends VERIFY-OK and the golden files are byte-identical with no rewrite (verify: contains VERIFY-OK in verify.log)

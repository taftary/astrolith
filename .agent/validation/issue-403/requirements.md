# Requirements (CONFIRMED) - issue #403

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/403#issuecomment-6093449561,
owner approval https://github.com/taftary/astrolith/issues/403#issuecomment-6093454512).
One criterion per acceptance criterion plus one per owner-expectation
sentence, in owner words. Window glide-feel criteria are judged by the
validator directly (code reads where stated, frame proof) and by owner
testing; the script marks those UNVERIFIABLE and they never count as MET
from a log line. No criterion asserts a pull-request run observation.

- [ ] C1: On entry, the view continues smoothly from the previewed interior with no snap and no content swap (verify: file crates/universe-render/src/input.rs contains The eased look stays untouched)
- [ ] C2: The first wheel and arrow moves inside glide continuously and the viewpoint never jumps; the stored target is resolved in the new cell (verify: file crates/universe-render/src/input.rs contains takes the portal ahead)
- [ ] C3: Small moves inside never throw the dive out on their own; a fresh entry holds through the first outward dives (verify: contains fresh_entry_ignores_a_few_outward_dives_but_eventual_exit_closes in cargo-test.log)
- [ ] C4: A deliberate long outward move still returns to the parent, at the marker that was entered (verify: contains opening_keeps_camera_outside_child_cell in cargo-test.log)
- [ ] C5: Exiting re-locks the marker it came from and eases the view back instead of teleporting it (verify: contains manual_exit_eases_look_back_to_the_marker in cargo-test.log)
- [ ] C6: Entering universes feels enhanced: entry and the first moves are one continuous glide (verify: contains manual_entry_keeps_eased_look_and_resolves_target in cargo-test.log)
- [ ] C7: The deterministic journey is unchanged: --verify ends VERIFY-OK and the golden files are byte-identical with no rewrite (verify: contains VERIFY-OK in verify.log)

# Requirements (CONFIRMED) - issue #729

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/729#issuecomment-6223783631,
owner approval https://github.com/taftary/astrolith/issues/729#issuecomment-6223791173,
candidates https://github.com/taftary/astrolith/issues/729#issuecomment-6223783229),
with answers Q1-Q5 plus navigation follow-up. One or more criteria per
acceptance criterion, in the spec's words. How the patch looks on the
real window is judged by the main agent's screenshots and the owner test.

- [ ] C1: At L10 only the refined patch dome draws around the player, with no ball, no markers, and no portal outlines anywhere (AC1) (verify: contains l10_draws_only_the_follow_patch ... ok in cargo-test.log)
- [ ] C2: Diving L1 to L10 works exactly as today, and the autopilot stops at the planet surface (AC2) (verify: contains autopilot_stops_at_the_planet_surface ... ok in cargo-test.log)
- [ ] C3: Below L10 nothing can be entered: no portals are offered, and trying to dive deeper does nothing (AC3) (verify: contains nothing_opens_below_l10 ... ok in cargo-test.log)
- [ ] C4: Walking and flying keep the patch under the player with fixed terrain (AC4) (verify: contains patch_stays_under_walking_body_without_drift ... ok in cargo-test.log)
- [ ] C5: The icosphere surface code is gone and ADRs 0024/0027 are retired with a new ADR recording patch-only (AC5) (verify: file docs/decisions/0028-patch-only-surface.md contains 0027)
- [ ] C6: Guides carry the approved G1-G3 words (AC6) (verify: contains guides_carry_the_patch_only_words ... ok in cargo-test.log)
- [ ] C7: The headless verification passes on rewritten goldens (AC6) (verify: contains VERIFY-OK in verify.log)
- [ ] C8: The dome is the complete hemisphere facing the player and its rim shows as a circle (round 1) (verify: contains dome_rim_forms_a_shown_circle ... ok in cargo-test.log)

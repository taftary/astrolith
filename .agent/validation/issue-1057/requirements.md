# Requirements — Issue #1057 (big circles fade when seen edge-on)

From Spec v2 (https://github.com/taftary/astrolith/issues/1057#issuecomment-6467842189), approved in https://github.com/taftary/astrolith/issues/1057#issuecomment-6467871163. Owner answers: Q1 A, Q2 yes (decisions comments).

- [ ] C1: A circle fades by the eye's angle out of its plane: full from 10°, gone under 2°, smooth between; the eye at the centre counts as in the plane; a shell's three circles fade apart (AC2, AC3, owner Q1 A) (verify: contains circles_fade_when_seen_edge_on ... ok in cargo-test.log)
- [ ] C2: The orbit-path colour and the #1056 height fade still hold (AC4) (verify: contains orbit_paths_are_light_in_their_own_hue_and_fade_by_height ... ok in cargo-test.log)
- [ ] C3: Everything else is unchanged: the headless check passes as before (AC4) (verify: contains GROUND walk_gap_mm=0.000 run_gap_mm=0.000 land_gap_mm=0.000 in verify.log)
- [ ] C4: At L7 where the bug was seen, no vertical line through the Sun and no dark bands; from above the circles show; other levels look as before (AC1, AC2, AC4, owner Q2) (judged by the validator from the shot evidence comment on the PR, against `.agent/validation/issue-1057/summary.md`; no script probe)

# Requirements — Issue #1056 (orbit paths: no black line in the sky from the ground)

From Spec v1 (https://github.com/taftary/astrolith/issues/1056#issuecomment-6464365131), approved in https://github.com/taftary/astrolith/issues/1056#issuecomment-6464461985. Owner answers Q1 A, Q2 yes in the decisions comments.

- [ ] C1: Orbit paths are fully drawn from 100 km up and off the planet, gone from 1 km down, half at 10 km, rising steadily between (AC1, AC2, owner Q1 A) (verify: contains orbit_paths_fade_out_between_100_km_and_1_km ... ok in cargo-test.log)
- [ ] C2: Orbit paths draw in the moon's own colour lifted to a light floor, never black, faded by the height weight, and not at all when the weight is zero (AC3, owner Q2) (verify: contains orbit_paths_are_light_in_their_own_hue_and_fade_by_height ... ok in cargo-test.log)
- [ ] C3: Everything else is unchanged: the headless check still passes with the ground lines as before (AC4) (verify: contains GROUND walk_gap_mm=0.000 run_gap_mm=0.000 land_gap_mm=0.000 in verify.log)
- [ ] C4: On the ground at dawn no orbit line crosses the sky; at 1 km none; at 10 km a faint one; at 100 km and from orbit a light line in the moon's colour (AC1-AC3) (judged by the validator from the shot evidence comment on the PR, against `.agent/validation/issue-1056/summary.md`; no script probe)

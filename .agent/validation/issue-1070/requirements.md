# Requirements — Issue #1070 (no stripes in the atmosphere from high above)

From Spec v1 (https://github.com/taftary/astrolith/issues/1070#issuecomment-6471327660), approved in https://github.com/taftary/astrolith/issues/1070#issuecomment-6471352474. Owner answer: Q1 A.

- [ ] C1: The haze switches to per-pixel from about 50 km over sea level and back to the table below 45 km, with a band that never flickers, and thin air keeps today's rule (AC1, AC3) (verify: contains planet_air_comes_and_goes_with_the_planet ... ok in cargo-test.log)
- [ ] C2: Everything else is unchanged: the headless check passes as before (AC2) (verify: contains GROUND walk_gap_mm=0.000 run_gap_mm=0.000 land_gap_mm=0.000 in verify.log)
- [ ] C3: The cost is measured and reported at 60 km and 100 km, before and after (AC4) (verify: file .agent/validation/issue-1070/summary.md contains **Per-pixel is slower on this PC:** about 7 ms more per frame)
- [ ] C4: At 60 km and 100 km by day no stripes; near the ground and at sunset the picture is as today; no visible jump crossing the switch (AC1-AC3) (judged by the validator from the shot evidence comment on the PR, against `.agent/validation/issue-1070/summary.md`; no script probe)

# Requirements — Issue #1153 (journey check rebuilds cells every frame)

From Spec v1 (https://github.com/taftary/astrolith/issues/1153#issuecomment-6606583916), approved in https://github.com/taftary/astrolith/issues/1153#issuecomment-6606598235.

- [ ] C1: On the reference PC, the #1087 short-normal ground route (release build) shows a main-thread mean of 30 ms or less (AC1) (judged by the validator from `.agent/validation/issue-1153/short-normal/summary.json` `scenarios.ground.main_ms.mean`, with the executable SHA-256 in its `manifest.json`; no script probe)
- [ ] C2: Standing still on the ground, the journey check no longer rebuilds cells every frame (AC2) (verify: contains home::tests::asking_twice_whether_the_journey_planet_is_home_builds_no_cell_again ... ok in cargo-test.log)
- [ ] C3: A planet off the journey keeps its answer without rebuilding cells (AC2) (verify: contains home::tests::a_neighbour_planet_stays_not_home_when_asked_again ... ok in cargo-test.log)
- [ ] C4: The world is unchanged: the game's headless check still passes against the golden output (AC3) (verify: contains VERIFY-OK in verify.log)
- [ ] C5: You arrive at the same place on the ground as before, and the planet and ground look the same (test plan step 1) (judged by the validator: the golden output covers the journey, cells and ground values; no script probe)
- [ ] C6: Standing on the ground feels smoother, with no new stutter (test plan step 2) (judged by the validator from `short-normal/summary.json`: `over_100ms` in the ground scenario; no script probe)
- [ ] C7: Opening another planet and coming back works as before (test plan step 3) (judged by the validator: the memo returns the same answer the replay gives, shown by C2 and C3 and the workspace tests; no script probe)

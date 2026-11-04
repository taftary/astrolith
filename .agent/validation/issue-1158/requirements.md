# Requirements — Issue #1158

Spec v1: https://github.com/taftary/astrolith/issues/1158#issuecomment-6630033858
Owner approval: https://github.com/taftary/astrolith/issues/1158#issuecomment-6630185885

- [ ] C1: The shared headless test app runs on a fixed 1/60 s clock instead of the real clock (`E-TEST-HERMETIC`). (judged by the validator from crates/universe-render/src/lib_tests.rs and every headless test app builder; no script probe)
- [ ] C2: The free-flight test (`shift_digit_steps_speed_and_slots_round_trip`) flies only within its starting L1 cell, and passes alone and in the suite. (judged by the validator from the test and its runs alone and in the suite; no script probe)
- [ ] C3: `behind_follow_settles_behind`, `narrow_flow_lists_first_then_details_with_back` and `touch_walks_jumps_and_switches_camera_in_normal_mode` pass in every run; the remaining source of run-to-run difference is found and removed from the test, or a real game problem found on the way is reported instead. (judged by the validator from the tests, the #1158 comments and repeated runs; no script probe)
- [ ] C4: Each fixed test is first shown failing deterministically, then passing (`E-BUG-TEST-FIRST`). (judged by the validator from the #1158 evidence comment; no script probe)
- [ ] C5: The full workspace test suite passes 10 times in a row locally, and the pull request's CI passes on its first run. (judged by the validator from .agent/validation/issue-1158/ten-runs.txt; the pull request's own run result is gate-owned and not judged here; no script probe)
- [ ] C6: The game is unchanged: the headless check (`--verify`) and the other tests give the same results; clippy, docs and format pass. (judged by the validator from the runtime checks and the diff touching test code only; no script probe)
- [ ] C7: Look at the pull request's checks. Expect them to pass without re-runs. (judged by the validator from the local ten-run evidence that no test fails by chance; the pull request's own run result is gate-owned and not judged here; no script probe)
- [ ] C8: Play as usual: fly, walk in third person, open the menu on a narrow window. Expect everything to behave exactly as before. (judged by the validator from the diff touching tests only and `--verify`; no script probe)
- [ ] C9: Script check for C5: the tenth run of the final set passed with no failures (verify: file .agent/validation/issue-1158/ten-runs.txt contains run 10: passed=883 failed_lines=0)
- [ ] C10: Script check for C1: the shared fixed clock exists (verify: file crates/universe-render/src/lib_tests.rs contains pub(crate) fn fixed_clock(app: &mut App))

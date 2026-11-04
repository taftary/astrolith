# Requirements — Issue #1191

Spec v1: https://github.com/taftary/astrolith/issues/1191#issuecomment-6636631123
Owner approval: https://github.com/taftary/astrolith/issues/1191#issuecomment-6636723673

- [ ] C1: A jump started before the ground under the player is ready keeps going when the ground becomes ready. The upward or downward speed is kept, and the player is not put back on the ground. (judged by the validator from the test `jump_continues_when_the_ground_becomes_ready` and its runs; no script probe)
- [ ] C2: When the ground becomes ready during a jump, the player is never left below the real ground. If the player is below it, the height is raised to the ground and the jump continues from there. (judged by the validator from the test `jump_below_the_real_ground_is_raised_and_kept`; no script probe)
- [ ] C3: A player standing (not jumping) when the ground becomes ready is placed on it as before. (judged by the validator from the test `standing_is_placed_on_the_ground_as_before`; no script probe)
- [ ] C4: The bug is first shown with a test that fails deterministically, then passes (`E-BUG-TEST-FIRST`): the ground finishes building on the frame after a jump. (judged by the validator from .agent/validation/issue-1191/t1-fails-before-fix.txt and the passing runs; no script probe)
- [ ] C5: The game is otherwise unchanged: the headless check (`--verify`) and all other tests pass; clippy, docs and format pass. (judged by the validator from the runtime checks; no script probe)
- [ ] C6: Fly to a planet and switch to walking (1). Press Space at once, before you would normally move. Expect a normal jump and landing; the jump is not cut short or undone. (judged by the validator from the C1 test, which reproduces the exact timing headlessly; no script probe)
- [ ] C7: Walk and jump as usual a few seconds later. Expect the same as before. (judged by the validator from the existing walk and jump tests and `--verify`; no script probe)
- [ ] C8: Script check for C4: the tests failed before the fix (verify: file .agent/validation/issue-1191/t1-fails-before-fix.txt contains test result: FAILED. 1 passed; 2 failed)
- [ ] C9: Script check for C1: the kept-jump flag exists (verify: file crates/universe-render/src/player_state.rs contains pub airborne: bool)

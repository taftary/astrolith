# Requirements — Issue #1129 (dive-mode mouse drag up/down follows the hand)

From Spec v1 (https://github.com/taftary/astrolith/issues/1129#issuecomment-6588244255), approved in https://github.com/taftary/astrolith/issues/1129#issuecomment-6588313068.

- [ ] C1: In dive mode with Invert look off, a left-drag upward makes the target scene move up on screen (the camera goes down around the target) and downward makes it move down, matching left/right where the scene follows a sideways drag (AC1) (verify: contains dive_drag_up_and_right_move_the_scene_with_the_hand ... ok in cargo-test.log)
- [ ] C2: With Invert look on, the up/down drag goes the other way; left/right is unchanged (AC2) (verify: contains invert_look_flips_the_dive_drag_up_and_down ... ok in cargo-test.log)
- [ ] C3: A one-finger drag in dive mode follows the same rule as the mouse (AC3) (verify: contains touch_drag_up_in_dive_moves_the_scene_with_the_finger ... ok in cargo-test.log)
- [ ] C4: Left/right drag, W/A/S/D, the right-button turn, the distance to the target and clicking to choose a target behave as before (AC4) (verify: contains dive_left_drag_orbits_and_click_still_picks ... ok in cargo-test.log)
- [ ] C5: Owner: "when I am in dive mode using mouse and I want to go top or bottom it is inverse for the right and left it is fine" (verify: contains dive_drag_up_and_right_move_the_scene_with_the_hand ... ok in cargo-test.log)

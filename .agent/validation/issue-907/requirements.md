# Requirements — Issue #907 (dive turn on the bottom half, free mode up/down)

From Spec v1 (https://github.com/taftary/astrolith/issues/907#issuecomment-6324545262), approved in https://github.com/taftary/astrolith/issues/907#issuecomment-6325088374.

- [ ] C1: In dive mode a right-drag to the right above and below the centre turn the view in opposite directions, a drag circling the centre once turns it by one full turn, and near the centre horizontal movement turns as before (AC1) (verify: contains dive_turn_follows_the_mouse_around_the_centre ... ok in cargo-test.log)
- [ ] C2: In free mode moving the mouse up turns the view up and down turns it down, Invert look flips both, and left and right are unchanged (AC2) (verify: contains free_mouse_up_looks_up ... ok in cargo-test.log)
- [ ] C3: The player cameras keep mouse up looking up (owner check, regression guard) (AC3) (verify: contains player_mouse_up_looks_up ... ok in cargo-test.log)
- [ ] C4: The dive turn still stops on release, free mode still loops and rolls, and the headless game check passes (AC3) (verify: contains VERIFY-OK in verify.log)

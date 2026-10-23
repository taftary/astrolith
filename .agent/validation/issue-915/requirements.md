# Requirements — Issue #915 (dive camera jumps far away when zooming across levels)

From Spec v2 (https://github.com/taftary/astrolith/issues/915#issuecomment-6363391884), approved in https://github.com/taftary/astrolith/issues/915#issuecomment-6363402974.

- [ ] C1: Fast mouse-wheel zoom out from the planet to its parent moves outward steadily with no sudden far jump (AC1) (verify: contains fast_wheel_zoom_out_stays_smooth ... ok in cargo-test.log)
- [ ] C2: Fast mouse-wheel zoom in from the parent to the planet moves inward steadily with no sudden jump, the same way as out (AC2) (verify: contains fast_wheel_zoom_in_stays_smooth ... ok in cargo-test.log)
- [ ] C3: Slow wheel steps move one step at a time as today, in both directions (AC3) (verify: contains slow_wheel_steps_stay_stepwise ... ok in cargo-test.log)
- [ ] C4: Dive still ends at the planet as described in Help, and the headless game check passes (AC4) (verify: contains VERIFY-OK in verify.log)

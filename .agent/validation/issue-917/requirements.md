# Requirements — Issue #917 (free-mode speed follows height above ground)

From Spec v2 (https://github.com/taftary/astrolith/issues/917#issuecomment-6369095370), approved in https://github.com/taftary/astrolith/issues/917#issuecomment-6369114744.

- [ ] C1: Far above the planet free flight covers distance quickly; close to the ground the same keys move gently (AC1) (verify: contains free_speed_follows_height_above_ground ... ok in cargo-test.log)
- [ ] C2: Flying down from high to low with the same keys and speed step slows down steadily, with no sudden jump at a level border L11/L12/L13/L14 (AC2) (verify: contains free_speed_slows_smoothly_across_levels ... ok in cargo-test.log)
- [ ] C3: At one fixed height a higher speed step (e.g. 8) is clearly faster than a lower one (e.g. 3) (AC3) (verify: contains free_speed_steps_multiply_at_fixed_height ... ok in cargo-test.log)
- [ ] C4: Very low or inside the ground, free flight still moves and never gets stuck at zero speed (AC4) (verify: contains free_speed_never_stalls_near_ground ... ok in cargo-test.log)
- [ ] C5: Help > Free flight shows the G1 sentence word for word (AC5) (verify: contains guides_explain_free_height_speed ... ok in cargo-test.log)
- [ ] C6: Dive, walking, saved views and the headless game check behave as before (AC6) (verify: contains VERIFY-OK in verify.log)

# Requirements — Issue #908 (a night you can see in)

From Spec v2 (https://github.com/taftary/astrolith/issues/908#issuecomment-6325127125), approved in https://github.com/taftary/astrolith/issues/908#issuecomment-6326304858.

- [ ] C1: On the night side the ambient light is 25-30% of the day-side light on flat ground (AC1) (verify: contains night_is_moonlit ... ok in cargo-test.log)
- [ ] C2: At L10-L14 the exposure adapts to the view's brightness over about 5 seconds, brighter at night and darker by day, smoothly; off the planet (L1-L9) it is exactly as today (AC2) (verify: contains eyes_adapt_at_the_planet ... ok in cargo-test.log)
- [ ] C3: B / Shift+B still shift the exposure one step darker or brighter on top of the adaptation (AC3) (verify: contains manual_exposure_stays_on_top ... ok in cargo-test.log)
- [ ] C4: Daylight, the sky, the leftover-sky fix, the panel and the dive behave as before, and the headless game check passes (AC4) (verify: contains VERIFY-OK in verify.log)
- [ ] C5: Settings > Display > Scene shows Eye adaptation On / Off, default On, saved and reset like the others, and Off keeps the exposure fixed at the planet (AC5) (verify: contains eye_adaptation_setting_round_trips ... ok in cargo-test.log)
- [ ] C6: Help > Navigation carries the G1 night sentence word for word (AC6) (verify: contains guides_describe_the_moonlit_night ... ok in cargo-test.log)

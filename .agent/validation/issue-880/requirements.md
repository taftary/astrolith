# Requirements — Issue #880 (the planet's sky stays after leaving the planet)

From Spec v1 (https://github.com/taftary/astrolith/issues/880#issuecomment-6308697997), approved in https://github.com/taftary/astrolith/issues/880#issuecomment-6308729317.

- [ ] C1: After leaving L10, no frame draws the planet's sky; a camera without sky settings keeps no sky data in the graphics engine, whether you leave from the ground, inside the air or orbit, slowly or fast (AC1) (verify: contains stale_sky_is_cleared ... ok in cargo-test.log)
- [ ] C2: Diving back to the planet draws its sky again as before (AC2) (verify: contains planet_air_comes_and_goes_with_the_planet ... ok in cargo-test.log)
- [ ] C3: A screenshot run leaving the planet from the ground and from orbit, then several pictures at L9 and L7, shows the normal background in every picture, and the same run before the fix shows the sky (AC3) (verify: file docs/decisions/0046-stale-planet-sky.md contains Proof run)
- [ ] C4: Everything else behaves as before, and the headless game check passes (AC4) (verify: contains VERIFY-OK in verify.log)

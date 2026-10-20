# Requirements (CONFIRMED) - issue #780

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/780#issuecomment-6268036909,
owner approval https://github.com/taftary/astrolith/issues/780#issuecomment-6268053651),
with Q1-Q7 (all A), S1 yes, G1 yes, F1 A and F2 A. The look on the real window (sky,
rim, sunsets, shadows) is judged by the main agent's screenshots and the
owner test; the validator takes no pictures (#533).

- [ ] C1: At the planet one sunlight from the star direction lights the planet ground (by its terrain normals), the moons and the figure with one rule; the figure's CPU shading is gone; L1-L9 are unchanged (AC1, Q1 A) (verify: contains one_sunlight_lights_ground_moons_and_figure ... ok in cargo-test.log)
- [ ] C2: The night-side ground is 5-10% as bright as the day side (AC2, Q2 A) (verify: contains night_ratio= in verify.log)
- [ ] C3: The atmosphere comes from each planet's air (Earth thin blue, thick amber, airless none), one model from orbit to the ground (AC3, Q3 A) (verify: contains atmosphere_follows_the_planet_air ... ok in cargo-test.log)
- [ ] C4: Near the player the terrain and the figure cast shadows (AC4, Q4 A) (verify: contains near_shadows_follow_the_setting ... ok in cargo-test.log)
- [ ] C5: The sun direction is fixed and repeat runs are identical (AC5, Q5 A) (verify: contains sun_direction_is_fixed ... ok in cargo-test.log)
- [ ] C6: The headless `LIGHT` line prints the largest frame-to-frame change in sky colour, ground brightness and haze over the 2,000 km to 1 m descent and back, all under 2%, plus the night-to-day ratio (AC6, Q6 A) (verify: contains LIGHT in verify.log)
- [ ] C7: The 60 fps target holds on the dev PC, or the quality steps and measured costs are recorded (AC7, Q7 A) (verify: file docs/decisions/0040-atmosphere-and-light.md contains Frame time)
- [ ] C8: Settings > Display > Scene has Shadows (On/Off, default On); Off removes the near shadows and keeps the light (AC8, S1) (verify: contains shadows_setting_defaults_on_and_saves ... ok in cargo-test.log)
- [ ] C9: Help > Navigation ends with the day-and-night sentence (AC8, G1) (verify: contains guides_carry_the_day_and_night_sentence ... ok in cargo-test.log)
- [ ] C10: Everything else behaves as after #779 and the headless game check passes (AC9) (verify: contains VERIFY-OK in verify.log)
- [ ] C11: The dive's arrival point and the seeded start spot are on the day side, with the sun at least 30 degrees above the horizon (AC10, F1 A) (verify: contains arrival_is_in_daylight ... ok in cargo-test.log)
- [ ] C12: At the planet the drawing units are metres, and the PRECISION, MORPH, CULL, DEPTH and GROUND lines still pass (AC11, F2 A) (verify: contains planet_is_drawn_in_metres ... ok in cargo-test.log)

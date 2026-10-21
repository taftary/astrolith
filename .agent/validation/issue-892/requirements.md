# Requirements — Issue #892 (planet to ground)

From Spec v2 (https://github.com/taftary/astrolith/issues/892#issuecomment-6314317087), approved in https://github.com/taftary/astrolith/issues/892#issuecomment-6314453688.

- [ ] C1: No water: the old sea floors are ground, and nothing reads a sea level for the camera, the dive stop, walking or the body (AC1) (verify: contains no_sea_level_for_the_ground ... ok in cargo-test.log)
- [ ] C2: Diving in toward the planet's centre stops 3 m above the real ground under the camera, over the old sea floors too; orbiting near the ground stays at least 3 m above it; the camera is never inside the ground (AC2) (verify: contains dive_stops_above_the_real_ground ... ok in cargo-test.log)
- [ ] C3: Every ground vertex has the same base colour; there are no height tints (AC3) (verify: contains ground_has_one_colour ... ok in cargo-test.log)
- [ ] C4: At 2,000 km, 500 km, 50 km, 500 m and 5 m above the ground the panel shows L10 to L14 with their names, the height above the ground and the scale, in dive, free and walking (AC4) (verify: contains panel_follows_the_height ... ok in cargo-test.log)
- [ ] C5: Holding the dive-in arrow from 10 km down, the speed keeps falling as the ground gets closer and the camera rests at 3 m without overshooting (AC5) (verify: contains dive_slows_near_the_ground ... ok in cargo-test.log)
- [ ] C6: Above your body at the stop, pressing 1 lands you on your feet with the view easing down; elsewhere 1 flies to your body first (AC6) (verify: contains one_lands_on_your_feet ... ok in cargo-test.log)
- [ ] C7: Orbit, turn, free mode, saved views, the start journey and the leftover-sky fix behave as before, and the headless game check passes (AC7) (verify: contains VERIFY-OK in verify.log)
- [ ] C8: Help > Navigation starts with the G1 text word for word, and About > The scale ladder lists the four L11-L14 rows after L10 (AC8) (verify: contains guides_explain_the_ground_levels ... ok in cargo-test.log)

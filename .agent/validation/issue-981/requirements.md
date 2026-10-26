# Requirements — Issue #981 (timeline and planet movement)

From Spec v2 (https://github.com/taftary/astrolith/issues/981#issuecomment-6419552414), approved in https://github.com/taftary/astrolith/issues/981#issuecomment-6419589432.

- [ ] C1: The time bar shows play or pause, slower, the speed and faster, top middle in every mode and level, hidden by the same Display switches as the mode button, and usable by tap (AC1) (verify: contains the_bar_hides_with_the_mode_reminder_switch ... ok in cargo-test.log)
- [ ] C2: At 1x one Earth day takes 10 minutes of real time; the speeds step 1x, 10x, 100x, 1,000x and 10,000x; the game starts playing at 1x (AC2) (verify: contains speeds_step_by_tens_and_pause_stops_the_clock ... ok in cargo-test.log)
- [ ] C3: Planets go around their star with their Kepler periods, spin with their day length and tilt, and moons go around their planets; positions are a pure function of time (AC3) (verify: contains ORBIT planet_quarter_deg=90.000000 moon_half_deg=180.000 PASS in verify.log)
- [ ] C4: On a planet or in its gravity zone you ride with it and turn with it; the ground never slides and the sun moves across the sky; outside every zone you stay still; every handover is jump-free (AC4) (verify: contains RIDE planet=6 in verify.log)
- [ ] C5: Pause stops the sky only; walking, flying and diving keep working (AC5) (verify: contains presses_step_the_clock ... ok in cargo-test.log)
- [ ] C6: The readout shows the home planet's day and time; the time is saved on quit and continues on the next start (AC6) (verify: contains the_time_continues_from_the_last_run_and_is_kept_on_quit ... ok in cargo-test.log)
- [ ] C7: Frame time on this PC is no worse than today (ADR 0044 rule), with no new per-frame memory growth and no ground patch rebuild because time moves (AC7) (verify: file .agent/validation/issue-981/timing.txt contains AC7 frame time: PASS)
- [ ] C8: The headless game check passes; the journey runs at a fixed time so the existing lines stay, and new lines check the clock, the orbits and the riding (AC8) (verify: contains CLOCK start="Day 1, 08:00" day_real_s=600 in verify.log)
- [ ] C9: Controls > Keys has the three time rows (T, [, ]), rebindable; Help > Navigation shows the G1 text and the G2 change word for word (AC9) (verify: contains guides_tell_of_the_sky_clock ... ok in cargo-test.log)
- [ ] C10: In the UI top middle there are play, pause and two speed buttons (owner notion) (verify: contains the_bar_hides_with_the_mode_reminder_switch ... ok in cargo-test.log)
- [ ] C11: The player's position updates in space with the movement (owner notion) (verify: contains inside_a_planet_the_camera_rides_along ... ok in cargo-test.log)
- [ ] C12: Careful about performance and optimization (owner notion) (verify: file .agent/validation/issue-981/timing.txt contains AC7 frame time: PASS)
- [ ] C13: The time bar buttons work by mouse click with Simulate touch on and off (owner report, correction round 1) (verify: contains a_click_on_play_pauses_with_simulate_touch_on_and_off ... ok in cargo-test.log)

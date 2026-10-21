# Requirements — Issue #884 (camera comfort)

From Spec v2 (https://github.com/taftary/astrolith/issues/884#issuecomment-6309663985), approved in https://github.com/taftary/astrolith/issues/884#issuecomment-6309680990.

- [ ] C1: In dive mode the right-drag rotation follows the mouse smoothly and stops the moment the button is released; no later frame changes it (AC1) (verify: contains dive_turn_stops_on_release ... ok in cargo-test.log)
- [ ] C2: In free mode looking up or down has no limit (a full loop over the top), and near a planet the view never turns by itself (AC2) (verify: contains free_look_loops_and_ignores_gravity ... ok in cargo-test.log)
- [ ] C3: In free mode Q and E roll the view one way and the other, and release stops it (AC3) (verify: contains free_roll_with_q_and_e ... ok in cargo-test.log)
- [ ] C4: B darkens and Shift+B brightens, E no longer changes exposure, and a saved key map with the old E for exposure moves to B (AC4) (verify: contains exposure_moves_to_b ... ok in cargo-test.log)
- [ ] C5: The target is always the planet; at start the player is selected; Space with the player selected flies to the planet and ends 10 m straight above him looking down, then orbit and zoom work around the planet's center with the player still selected (AC5) (verify: contains space_flies_planet_then_player ... ok in cargo-test.log)
- [ ] C6: P toggles the player's selection, a click on any other point deselects him, and with him deselected Space flies to the planet only (AC6) (verify: contains p_toggles_player_selection ... ok in cargo-test.log)
- [ ] C7: Normal mode, dive zoom and orbit, saved views and the leftover-sky fix behave as before, and the headless game check passes (AC7) (verify: contains VERIFY-OK in verify.log)
- [ ] C8: Settings > Controls > Flight has Free-mode roll speed, 0.5 to 2.0, default 1.0, saved and reset like the others; 2.0 rolls twice as fast as 1.0 (AC8) (verify: contains roll_speed_setting_round_trips ... ok in cargo-test.log)
- [ ] C9: Help > Free flight shows the G1 text and Help > Navigation ends with the G2 text word for word, and no Guides page names E for exposure (AC9) (verify: contains guides_explain_free_flight_and_selection ... ok in cargo-test.log)

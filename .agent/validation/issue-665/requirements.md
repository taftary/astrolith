# Requirements (CONFIRMED) - issue #665

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/665#issuecomment-6165616468,
owner approval https://github.com/taftary/astrolith/issues/665#issuecomment-6165630280),
with the decisions Q1-Q5. How the lab looks on the real window is judged by
the main agent's `engine-lab --screens` pictures and the owner test; the
validator takes no pictures (#533).

- [ ] C1: The Nodes list has a "Shaders" line with "Planet squares" under it, and the menu bar has a Shaders menu (AC1) (verify: contains the_shaders_line_and_menu_list_planet_squares ... ok in cargo-test.log)
- [ ] C2: Selecting the shader shows its on/off, settings, Reset and Copy values in the tools panel (AC1) (verify: contains the_selected_shader_shows_its_controls ... ok in cargo-test.log)
- [ ] C3: With Planet squares on, the body on show (planet, or a moon shown with F) draws the same squares as the game's Squares look; off draws the Normal look; the lab starts with it off (AC2) (verify: contains squares_follow_the_body_on_show_and_start_off ... ok in cargo-test.log)
- [ ] C4: Changing the square size, the two colours or the fade changes the values sent to the shader at once, within the slider limits, with the even fade always smaller than the sharp one (AC3) (verify: contains live_settings_reach_the_shader_within_limits ... ok in cargo-test.log)
- [ ] C5: Reset gives back 100 m, black, white, 8 px and 2 px, and a restart starts from those values (AC3, AC4) (verify: contains reset_and_restart_give_the_game_values ... ok in cargo-test.log)
- [ ] C6: Copy values puts Rust constants with the current values on the clipboard, and the status bar confirms it (AC5) (verify: contains copy_values_writes_rust_constants ... ok in cargo-test.log)
- [ ] C7: The Position, Normal and Distance debug views colour the body on show as described, and Off gives the normal picture back (AC6) (verify: contains debug_views_select_what_the_shader_shows ... ok in cargo-test.log)
- [ ] C8: At L1-L9 the Shaders controls and debug views are greyed out (AC7) (verify: contains shaders_are_greyed_above_the_planet ... ok in cargo-test.log)
- [ ] C9: The game's squares values stay 100 m, black, white, 8 px and 2 px, so its squares and Normal looks give the same picture as before (AC8) (verify: contains the_game_squares_values_are_unchanged ... ok in cargo-test.log)
- [ ] C10: The game is unchanged: the same seed gives the same universe, and no game crate depends on the lab (AC8) (verify: contains VERIFY-OK in verify.log)
- [ ] C11: The engine-lab doc has a Shaders section (verify: file docs/engine-lab.md contains ## Shaders)

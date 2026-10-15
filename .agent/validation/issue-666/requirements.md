# Requirements (CONFIRMED) - issue #666

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/666#issuecomment-6189908577,
owner approval https://github.com/taftary/astrolith/issues/666#issuecomment-6190092094),
with the decisions Q1-Q5. How the lab looks on the real window is judged by
the main agent's `engine-lab --screens` pictures and the owner test; the
validator takes no pictures (#533).

- [ ] C1: The Shaders menu and panel have an "Edit code" entry that opens a separate code window (not the tools panel) showing the game's Planet squares code, with Save, Reset, Copy code and an error line (AC1) (verify: contains the_code_window_shows_the_game_code_with_save_reset_and_copy ... ok in cargo-test.log)
- [ ] C2: Pressing Save redraws the body on show (planet, or a moon shown with F) at once with the edited code; the panel's on/off, settings and debug views keep working with edited code (AC2) (verify: contains save_applies_edited_code_to_the_body_on_show ... ok in cargo-test.log)
- [ ] C3: With an error in the code, pressing Save keeps the last good picture and shows the message in the window; fixing the code and pressing Save redraws correctly (AC3) (verify: contains shader_errors_keep_the_last_good_picture ... ok in cargo-test.log)
- [ ] C4: Reset gives back the game's code at once, and after a restart the lab starts from the game's code with the window closed (AC4) (verify: contains reset_and_restart_give_the_game_code ... ok in cargo-test.log)
- [ ] C5: Copy code puts the exact edited text on the clipboard, and the status bar confirms it (AC5) (verify: contains copy_code_puts_the_edited_text_on_the_clipboard ... ok in cargo-test.log)
- [ ] C6: At L1-L9, Save is greyed out (AC6) (verify: contains save_is_greyed_above_the_planet ... ok in cargo-test.log)
- [ ] C7: The game's shader file and squares values are unchanged, so its squares and Normal looks give the same picture as before (AC7) (verify: contains the_game_shader_file_and_values_are_unchanged ... ok in cargo-test.log)
- [ ] C8: The game is unchanged: the same seed gives the same universe, and no game crate depends on the lab (AC7) (verify: contains VERIFY-OK in verify.log)
- [ ] C9: The engine-lab doc has a Shader code section under Shaders (verify: file docs/engine-lab.md contains ## Shader code)

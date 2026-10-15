# Requirements (CONFIRMED) - issue #667

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/667#issuecomment-6192827884,
owner approval https://github.com/taftary/astrolith/issues/667#issuecomment-6192841269),
with the decisions Q1-Q5. How the lab looks on the real window is judged by
the main agent's `engine-lab --screens` pictures and the owner test; the
validator takes no pictures (#533).

- [ ] C1: The Shaders list and menu show Planet squares plus each created shader by name, with a New shader entry that adds and selects "Shader 1", "Shader 2", and so on (AC1) (verify: contains new_shader_appears_selected_with_a_unique_name ... ok in cargo-test.log)
- [ ] C2: Selecting a library entry shows it on the body on show at once (planet, or a moon shown with F); only one draws at a time (AC2) (verify: contains selecting_an_entry_shows_it_on_the_body_on_show ... ok in cargo-test.log)
- [ ] C3: A new shader draws the Normal look until edited; editing its code and pressing Save redraws at once; an error keeps the last good picture and shows the message; Reset gives back its blank start (AC3) (verify: contains created_shader_errors_keep_the_last_good_text ... ok in cargo-test.log)
- [ ] C4: Each shader keeps its own on/off, settings, code and error while the lab runs; rename keeps names unique; Delete removes a created shader at once (Planet squares cannot be renamed or deleted) (AC4) (verify: contains each_shader_keeps_its_own_settings_and_code ... ok in cargo-test.log)
- [ ] C5: The same on/off, size/colours/fade, Reset, Copy values and debug views work for every shader; Copy code puts the exact edited text on the clipboard and Copy values puts the Rust constants; the status bar confirms each (AC5) (verify: contains copy_texts_follow_the_selected_entry ... ok in cargo-test.log)
- [ ] C6: After a restart only Planet squares remains (off, game values, window closed) (AC6) (verify: contains restart_gives_only_planet_squares ... ok in cargo-test.log)
- [ ] C7: At L1-L9, New shader, the shader controls and Save are greyed out (AC7) (verify: contains new_shader_and_save_are_greyed_above_the_planet ... ok in cargo-test.log)
- [ ] C8: The game's shader file and squares values are unchanged, so its squares and Normal looks give the same picture as before (AC8) (verify: contains the_game_shader_file_and_values_are_unchanged ... ok in cargo-test.log)
- [ ] C9: The game is unchanged: the same seed gives the same universe, and no game crate depends on the lab (AC8) (verify: contains VERIFY-OK in verify.log)
- [ ] C10: The engine-lab doc has a Shader library section under Shaders (verify: file docs/engine-lab.md contains ## Shader library)

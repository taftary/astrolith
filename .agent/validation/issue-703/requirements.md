# Requirements (CONFIRMED) - issue #703

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/703#issuecomment-6187425087,
owner approval https://github.com/taftary/astrolith/issues/703#issuecomment-6187491002),
with the decisions Q1-Q5. How the lab window looks and whether the colour
picker opens is judged by the main agent's `engine-lab --screens` pictures
and the owner test; the validator takes no pictures (#533).

- [ ] C1: Clicking "Planet squares" under "Shaders" selects it: the left line stays highlighted and the right panel shows its controls (AC1) (verify: contains the_selected_shader_shows_its_controls ... ok in cargo-test.log)
- [ ] C2: Clicking the Colour A box keeps "Planet squares" selected: the left highlight stays, the right panel does not switch to "Nothing selected", and the colour picker opens (AC2) (verify: contains panel_click_does_not_reach_the_view ... ok in cargo-test.log)
- [ ] C3: Picking a colour changes the picture at once and the selection still stays (AC3) (verify: contains live_settings_reach_the_shader_within_limits ... ok in cargo-test.log)
- [ ] C4: The Colour B box shows its value and behaves the same as Colour A (AC4) (verify: contains the_selected_shader_shows_its_controls ... ok in cargo-test.log)
- [ ] C5: The list and the view stay in sync: a list click highlights its line and empty space still clears the selection (owner Q3, T2 no-change rule) (verify: contains selection_is_shared_by_the_list_and_the_view ... ok in cargo-test.log)
- [ ] C6: The game is unchanged: the headless check passes on the same seed (verify: contains VERIFY-OK in verify.log)

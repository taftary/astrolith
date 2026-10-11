# Requirements — #473 Menu button with Guides and Settings screens (correction round 1)

Source: Spec v1 (Issue comment 6103653834, owner-approved 6103665488), Spec v3
(comment 6107960483) and Spec v4 (comment 6110249570, owner `approved`
6110260415). Criteria C1–C29 carry the spec's own numbers: Spec v1 criteria 1
and 4–17 stand (9 now includes the Background dimming row), Spec v3 replaces 2
and 3 and adds 18–27, Spec v4 restates 19 and adds 28–29. C30–C32 are the Spec
v1 owner expectations that need eyes on the real window. Each criterion is in
the owner's words; a `(verify: ...)` probe is given where automation can check
it, and a `... ok` probe only matches a test that passed.

Status: CONFIRMED (derived from the approved Spec v1, v3 and v4).

- [ ] C1: A menu button is visible in the top-right corner of the universe at all times and does not overlap the scale readout. (verify: contains menu_button_shows_in_the_universe_and_hides_behind_screens ... ok in cargo-test.log)
- [ ] C2: Clicking the menu button opens the Menu screen: a dim layer over the whole window with a framed Menu card on it, holding Resume, Guides, Settings and Quit (in that order), each with its icon, and a ✕. The universe is paused behind it. (verify: contains menu_screen_lists_resume_guides_settings_quit_in_order ... ok in cargo-test.log)
- [ ] C3: Resume and ✕ return to the universe, which continues exactly where it stopped. Quit opens the Quit confirmation instead of closing the app. (verify: contains menu_quit_opens_the_popup_and_keeps_the_app_open ... ok in cargo-test.log)
- [ ] C4: Esc opens the Menu from the universe, steps back from Guides and Settings to the Menu, returns from the Menu to the universe, and never quits directly. (verify: contains esc_follows_the_spec_rule_and_never_quits ... ok in cargo-test.log)
- [ ] C5: No click on any screen, button, or blank area of a screen ever selects or targets a marker. (verify: contains keys_and_clicks_do_not_move_the_universe_behind_a_screen ... ok in cargo-test.log)
- [ ] C6: While a screen is open, game keys have no effect. (verify: contains keys_and_clicks_do_not_move_the_universe_behind_a_screen ... ok in cargo-test.log)
- [ ] C7: Guides and Settings share the layout: category list left, content right, title, Back and close; sub-categories expand and collapse; the selected item is shown by more than colour. (verify: contains categories_expand_and_collapse_leaves_select ... ok in cargo-test.log)
- [ ] C8: Under 720 px wide, the list shows alone first, a category opens full width, and Back returns to the list. (verify: contains narrow_flow_lists_first_then_details_with_back ... ok in cargo-test.log)
- [ ] C9: Settings contains exactly the categories, sub-categories, settings, choices, and defaults in the Spec v1 table plus the Background dimming row; Accessibility shows "Coming soon". (verify: contains store_defaults_match_the_spec_table ... ok in cargo-test.log)
- [ ] C10: Every setting change takes effect immediately with no Apply step; Reset to defaults restores that category's defaults. (verify: contains widgets_write_through_the_store ... ok in cargo-test.log)
- [ ] C11: Fullscreen and Vsync show a loader until the change is done and cannot be changed again meanwhile. (verify: contains fullscreen_request_pends_until_the_window_reports_back ... ok in cargo-test.log)
- [ ] C12: Keys and Settings stay in agreement: a change made with a key shows in Settings and is saved. (verify: contains keys_write_through_the_store_and_stay_in_sync ... ok in cargo-test.log)
- [ ] C13: After closing and reopening the app, all settings and the 8 saved views are as you left them. (verify: contains save_then_load_round_trips ... ok in cargo-test.log)
- [ ] C14: A missing or damaged settings file gives a normal start with defaults, no crash. (verify: contains damaged_file_gives_defaults_for_bad_lines ... ok in cargo-test.log)
- [ ] C15: `--verify` and `--capture` ignore the settings file; their output and the golden files are unchanged. (verify: contains verify_stdout_matches_golden ... ok in cargo-test.log)
- [ ] C16: Guides contains About (What is Astrolith, The scale ladder) and Help (Getting started, Navigation, Free flight, Saved views) with the approved text. (verify: contains every_spec_page_exists_with_its_approved_text ... ok in cargo-test.log)
- [ ] C17: All screen buttons are at least 48 px tall; text meets the 4.5:1 contrast floor on the dark background. (verify: contains screen_tokens_meet_the_spec_sizes_and_contrast ... ok in cargo-test.log)
- [ ] C18: The dim layer is 80% dark by default; the paused universe stays visible but dimmed behind it; screen and pop-up cards are fully solid. (verify: contains background_dimming_drives_the_veil_and_resets_with_display ... ok in cargo-test.log)
- [ ] C19: The layers always stack in this order: universe and HUD, then dim layer, then screens, then pop-ups. No HUD text and no label, including the hover label of a portal under the mouse, ever shows on top of a screen or a pop-up. (verify: contains ui_roots_stack_universe_below_screens ... ok in cargo-test.log)
- [ ] C20: Every card has the outer border, the inner frame line, rounded corners and the cyan corner brackets. Guides and Settings have the line under the title bar and the line between the list and the content. (verify: file crates/universe-render/src/chrome.rs contains pub(crate) fn spawn_card_trim)
- [ ] C21: Every screen button has an outer and an inner border, and is still at least 48 px tall. Hover turns the borders cyan; the selected list item shows a bar, brighter text and a cyan border; Quit has a red-tinted border. (verify: contains hover_turns_borders_cyan_and_brightens_pressed_darkens ... ok in cargo-test.log)
- [ ] C22: All screen text is in Inter, at the sizes above (titles 18, buttons and body 14, list items 13, small notes 11). (verify: contains screen_texts_use_bundled_fonts_and_icons ... ok in cargo-test.log)
- [ ] C23: The menu button, close, Back and the four menu entries show Lucide icons. No placeholder letters ("M", "X", "<") remain. (verify: contains icons_are_distinct_private_use_glyphs ... ok in cargo-test.log)
- [ ] C24: The Quit confirmation says "Quit Astrolith?" with Cancel and Quit buttons. Cancel and Esc return to the Menu, and Quit closes the app. Clicks outside the pop-up do nothing. (verify: contains outside_clicks_do_nothing_while_open ... ok in cargo-test.log)
- [ ] C25: The Inter and Lucide licence files ship with the app. (verify: contains bundled_fonts_and_licences_ship_with_the_crate ... ok in cargo-test.log)
- [ ] C26: Settings › Display › Scene has a Background dimming slider from 60% to 95% in 5% steps, default 80%. Moving it changes the dim layer's darkness at once, and Display › Reset to defaults sets it back to 80%. (verify: contains dimming_is_60_to_95_in_fives_default_80 ... ok in cargo-test.log)
- [ ] C27: Background dimming is saved and comes back after restarting the app. A settings file without a dimming value loads normally, with dimming at 80%. (verify: contains dimming_saves_loads_and_falls_back_to_80 ... ok in cargo-test.log)
- [ ] C28: The screenshot mode (`--screens`) also takes a picture of the Quit pop-up (`quit.png`, 1280×720). The run then ends with `SCREENS-OK files=8`. (verify: contains shot_list_matches_the_spec_names_and_sizes ... ok in cargo-test.log)
- [ ] C29: In the `--screens` pictures of the Menu, Settings (wide and narrow), Guides (wide and narrow) and the Quit pop-up, no portal label is visible anywhere on the card. (verify: contains no_label_shows_behind_any_screen_even_when_hovered ... ok in cargo-test.log)
- [ ] C30: The scene is clearly brighter with Exposure at maximum; labels hide and cap as set; flight speed and saved views behave as set. These need eyes on the real window.
- [ ] C31: Real fullscreen switching and the loader timing feel right on the real window. These need eyes on the real window.
- [ ] C32: The Guides texts read as approved, and the scale ladder matches the game's own scale readout. (verify: file crates/universe-core/src/guides.rs contains The 14 levels)

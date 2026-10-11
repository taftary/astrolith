# Requirements — #473 Menu button with Guides and Settings screens

Source: Spec v1 (Issue comment 6103653834), owner-approved (comment 6103665488),
with the owner-test-only waiver for screen visuals (comment 6103923286 discussion).
One criterion per acceptance criterion plus owner-expectation sentences, in the
owner's words; each carries a `(verify: ...)` probe where automation can check
it. Look-and-feel criteria carry no probe: the owner checks them in owner
testing (steps 9–11).

- [ ] C1: A menu button is visible in the top-right corner of the universe at all times and does not overlap the scale readout. (verify: contains menu_button_shows_in_the_universe_and_hides_behind_screens in cargo-test.log)
- [ ] C2: Clicking the menu button opens a full-window Menu screen with Resume, Guides, Settings, Quit in that order and a close button; the universe is paused behind it. (verify: contains menu_screen_lists_resume_guides_settings_quit_in_order in cargo-test.log)
- [ ] C3: Resume and the close button return to the universe, which continues exactly where it stopped. Quit closes the app. (verify: contains keys_and_clicks_do_not_move_the_universe_behind_a_screen in cargo-test.log)
- [ ] C4: Esc opens the Menu from the universe, steps back from Guides and Settings to the Menu, returns from the Menu to the universe, and never quits directly. (verify: contains esc_follows_the_spec_rule_and_never_quits in cargo-test.log)
- [ ] C5: No click on any screen, button, or blank area of a screen ever selects or targets a marker. (verify: contains keys_and_clicks_do_not_move_the_universe_behind_a_screen in cargo-test.log)
- [ ] C6: While a screen is open, game keys have no effect. (verify: contains keys_and_clicks_do_not_move_the_universe_behind_a_screen in cargo-test.log)
- [ ] C7: Guides and Settings share the layout: category list left, content right, title, Back and close; sub-categories expand and collapse; the selected item is shown by more than colour. (verify: contains categories_expand_and_collapse_leaves_select in cargo-test.log)
- [ ] C8: Under 720 px wide, the list shows alone first, a category opens full width, and Back returns to the list. (verify: contains narrow_flow_lists_first_then_details_with_back in cargo-test.log)
- [ ] C9: Settings contains exactly the categories, sub-categories, settings, choices, and defaults in the Spec v1 table; Accessibility shows "Coming soon". (verify: contains store_defaults_match_the_spec_table in cargo-test.log)
- [ ] C10: Every setting change takes effect immediately with no Apply step; Reset to defaults restores that category's defaults. (verify: contains widgets_write_through_the_store in cargo-test.log)
- [ ] C11: Fullscreen and Vsync show a loader until the change is done and cannot be changed again meanwhile. (verify: contains fullscreen_request_pends_until_the_window_reports_back in cargo-test.log)
- [ ] C12: Keys and Settings stay in agreement: a change made with a key shows in Settings and is saved. (verify: contains keys_write_through_the_store_and_stay_in_sync in cargo-test.log)
- [ ] C13: After closing and reopening the app, all settings and the 8 saved views are as you left them. (verify: contains save_then_load_round_trips in cargo-test.log)
- [ ] C14: A missing or damaged settings file gives a normal start with defaults, no crash. (verify: contains damaged_file_gives_defaults_for_bad_lines in cargo-test.log)
- [ ] C15: `--verify` and `--capture` ignore the settings file; their output and the golden files are unchanged. (verify: contains verify_stdout_matches_golden in cargo-test.log)
- [ ] C16: Guides contains About (What is Astrolith, The scale ladder) and Help (Getting started, Navigation, Free flight, Saved views) with the approved text. (verify: contains every_spec_page_exists_with_its_approved_text in cargo-test.log)
- [ ] C17: All screen buttons are at least 48 px tall; text meets the 4.5:1 contrast floor on the dark background. (verify: contains screen_tokens_meet_the_spec_sizes_and_contrast in cargo-test.log)
- [ ] C18: The scene is clearly brighter with Exposure at maximum; labels hide and cap as set; flight speed and saved views behave as set. These need eyes on the real window.
- [ ] C19: Real fullscreen switching and the loader timing feel right on the real window. These need eyes on the real window.
- [ ] C20: The Guides texts read as approved, and the scale ladder matches the game's own scale readout. (verify: file crates/universe-core/src/guides.rs contains The 14 levels)

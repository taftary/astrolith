# Requirements — Issue #956 (Scale HUD)

From Spec v3 (https://github.com/taftary/astrolith/issues/956#issuecomment-6382679155), approved in https://github.com/taftary/astrolith/issues/956#issuecomment-6382682909.

- [ ] C1: Fresh start shows nothing in the bottom-left corner (AC1) (verify: contains scale_hud_starts_hidden_and_the_switch_shows_it ... ok in cargo-test.log)
- [ ] C2: Settings Display Scene has a Scale HUD switch, off by default (AC2) (verify: contains scale_hud_toggle_writes_through_the_store ... ok in cargo-test.log)
- [ ] C3: Turning Scale HUD on shows the level, distance and scale lines (AC3) (verify: contains scale_hud_starts_hidden_and_the_switch_shows_it ... ok in cargo-test.log)
- [ ] C4: Turning Scale HUD off hides all of those lines at once, immediately, no restart (AC4) (verify: contains scale_hud_starts_hidden_and_the_switch_shows_it ... ok in cargo-test.log)
- [ ] C5: The choice is kept after closing and reopening the game; Reset Display puts it back to hidden (AC5) (verify: contains scale_hud_setting_defaults_off_and_saves ... ok in cargo-test.log)
- [ ] C6: Help then Navigation carries the new sentence naming the switch (AC6) (verify: contains guides_name_the_scale_hud_switch ... ok in cargo-test.log)
- [ ] C7: Nothing else on screen changes (AC7) (verify: contains every_spec_page_exists_with_its_approved_text ... ok in cargo-test.log)

Owner-test-only (in the Spec v3 test plan, presented for owner testing after merge, not machine-checked here): fresh start empty, switch on shows lines, dive follows height, switch off empties at once, choice kept across restart, reset hides again, guides sentence present (owner words: "Hideable via Settings. Hidden by default."). Covered mechanically by C1-C6 above.

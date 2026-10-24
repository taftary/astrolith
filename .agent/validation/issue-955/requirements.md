# Requirements — Issue #955 (mode toggle button)

From Spec v1 (https://github.com/taftary/astrolith/issues/955#issuecomment-6383330957), approved in https://github.com/taftary/astrolith/issues/955#issuecomment-6383362234.

- [ ] C1: Closed button shows only the current mode icon, top-left (AC1) (verify: contains mode_toggle_button_shows_current_icon_only ... ok in cargo-test.log)
- [ ] C2: Click opens the three modes with icons and names, current highlighted (AC2) (verify: contains mode_chooser_lists_three_modes_with_highlight ... ok in cargo-test.log)
- [ ] C3: Clicking a mode switches to it and closes the chooser (AC3) (verify: contains choosing_a_mode_switches_and_closes ... ok in cargo-test.log)
- [ ] C4: Clicking outside closes with no switch (AC4) (verify: contains outside_click_closes_without_switching ... ok in cargo-test.log)
- [ ] C5: Keys 1, 2, 3 and F still switch modes, and the button icon follows (AC5) (verify: contains keys_and_f_switch_modes_and_icon_follows ... ok in cargo-test.log)
- [ ] C6: With Mode reminder off in Settings, the button is hidden (AC6) (verify: contains mode_button_follows_the_setting ... ok in cargo-test.log)
- [ ] C7: Touch tap on the button opens the chooser like a click (AC7) (verify: contains touch_tap_opens_the_chooser ... ok in cargo-test.log)

Owner-test-only (in the Spec v1 test plan, presented for owner testing after merge, not machine-checked here): icon-only button top-left, chooser with three highlighted modes, pick-to-switch, outside-click dismiss, keys follow, setting hides, touch opens (owner words: "Button on top left ... shows current mode icon only ... selected mode highlighted ... choose one or click outside to close ... Use icons."). Covered mechanically by C1-C7 above.

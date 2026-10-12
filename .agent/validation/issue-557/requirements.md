# Requirements (CONFIRMED) - issue #557

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/557#issuecomment-6122756933,
owner approval https://github.com/taftary/astrolith/issues/557#issuecomment-6122766140),
with the owner's candidate answers (all yes). One criterion per acceptance
criterion, in the spec's words. The feel on the real window (test plan) is
judged by the main agent's screenshots and the owner test.

- [ ] C1: Key 2 enters free mode and key 3 enters dive mode from either mode; F still toggles between them; key 1 shows "Normal mode comes later" and changes nothing (verify: contains keys_two_and_three_switch_free_and_dive ... ok in cargo-test.log)
- [ ] C2: Ctrl+1-8 saves a view and Alt+1-8 brings it back; plain 1-8 no longer recalls views (verify: contains a_saved_view_brings_its_destination_back ... ok in cargo-test.log)
- [ ] C3: The mode reminder shows the current mode and its keys, follows every mode change, and hides when its setting is off (verify: contains mode_reminder_follows_the_mode_and_the_setting ... ok in cargo-test.log)
- [ ] C4: Look sensitivity and invert look change how looking around turns (verify: contains look_follows_sensitivity_and_invert ... ok in cargo-test.log)
- [ ] C5: With Dive set to toggle, one press keeps diving until the next press (verify: contains dive_toggle_keeps_diving_after_one_press ... ok in cargo-test.log)
- [ ] C6: The new settings sit in Controls > Flight and Display > Scene and change the store from their rows (verify: contains widgets_write_through_the_store ... ok in cargo-test.log)
- [ ] C7: The new settings keep their defaults on first launch and are kept after closing the game (verify: contains round_trip_keeps_every_value ... ok in cargo-test.log)
- [ ] C8: With every new setting at its default, the game behaves as today apart from the new keys (verify: contains VERIFY-OK in verify.log)
- [ ] C9: Help > Navigation names the mode keys as in G1 (verify: file crates/universe-core/src/guides.rs contains 2: free mode. 3: dive mode. 1 is kept for normal mode, coming later.)
- [ ] C10: Help > Saved views reads Alt+1 to Alt+8 as in G2 (verify: file crates/universe-core/src/guides.rs contains Alt+1 to Alt+8 bring you back there.)
- [ ] C11: Help > Free flight names the new keys as in G3 (verify: file crates/universe-core/src/guides.rs contains Press F or 2 to fly freely.)

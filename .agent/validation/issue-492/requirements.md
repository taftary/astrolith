# Requirements — issue #492 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/492#issuecomment-6108082421,
approval: https://github.com/taftary/astrolith/issues/492#issuecomment-6108238193).
Committed in correction round 1 (#508), replacing the unconfirmed draft.

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax. The window runs (C1-C4 in a
real window) are judged by the validator from its own `--screens` runs and the
pictures it views, quoted in the verdict; the probes below cover the code and
tests behind them.

## Layer 1 criteria (AC1-AC9)

- [ ] C1: `universe-app --screens <folder>` opens the real window, shows the six screens, and closes by itself with `SCREENS-OK files=7` (six pictures plus the list file) (verify: contains sequencer_shows_every_shot_in_order_then_finishes ... ok in cargo-test.log)
- [ ] C2: The folder holds one PNG per screen with the spec names, at 1280x720 (wide) and 640x720 (narrow), plus `screens.txt` listing each picture and its size (verify: contains shot_list_matches_the_spec_names_and_sizes ... ok in cargo-test.log)
- [ ] C3: No picture is blank: each holds more than one colour; a blank capture is re-taken, not kept (verify: contains a_blank_picture_is_retaken_not_kept ... ok in cargo-test.log)
- [ ] C4: Each picture shows the screen its name says (checked by eye by the owner and the validator) (verify: contains list_file_names_each_picture_with_its_size ... ok in cargo-test.log)
- [ ] C5: When the folder can't be written, the app prints `SCREENS-FAIL` with a reason, exits non-zero, and does not hang (verify: contains unwritable_folder_fails_before_any_window ... ok in cargo-test.log)
- [ ] C6: The mode neither reads nor writes the settings file (verify: contains driver_never_touches_the_settings_file ... ok in cargo-test.log)
- [ ] C7: Normal play, `--verify` and `--capture` are unchanged; the golden test passes (verify: contains VERIFY-OK in verify.log)
- [ ] C8: The list of screens lives in one place, so #473 can add a screen with one new entry (verify: file crates/universe-render/src/screens.rs contains the single place to add a screen)
- [ ] C9: Once proven, the capability is recorded in `docs/validator-capabilities.md`, and the validator has posted the six pictures on the Issue (verify: file docs/validator-capabilities.md contains run the window screens mode)

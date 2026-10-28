# #1022 T11 evidence (#1068): Settings and Help

Release build, `--screens menus smoke,menus` (the built-in menu shots,
at native). The new shots, judged by eye:

- `settings-performance.png` (wide) and `settings-performance-narrow.png`
  (the narrow layout): Display > Performance shows Quality (S1), Target
  frame rate (S2), Auto quality ceiling (S3), 3D resolution with its
  physical size (S4), the five status lines (S9) and Reset Display. Every
  control is fully visible in both layouts.
- `settings-performance-touch.png`: the same page with the touch controls
  on. The panel looks the same: the touch controls show in the universe
  view only.
- `guides-graphics.png`: Help > Graphics quality shows the G1 text. The
  page list does not scroll down to the selected entry; that is how the
  list already behaves for the last pages.

- `menus2/settings-window.png` (a second run, after the shot was added):
  Display > Window shows the vsync note under Vsync.

Display > Scene gains Atmosphere (S7) and Bloom (S8). These rows sit below
the visible part of the Scene page, so no shot shows them; the test
`scene_window_and_performance_carry_the_quality_controls` checks that
they, the Performance rows and status, and the vsync note are built. In Auto, Planet
detail, Shadows, Marker detail, Atmosphere and Bloom show the effective
value with "(Auto-managed)" and do not change on a click (tests in
`settings_quality_ui`). Display > Window carries the vsync note.

Tests:
- `graphics_quality_is_g1_word_for_word` (core `guides`) checks G1 word
  for word.
- `each_setting_changes_only_in_its_mode`,
  `values_show_the_saved_choice_in_manual_and_the_effective_in_auto` and
  `the_status_names_size_tiers_target_and_limits` cover the settings.

The PNGs stay out of git and go to the `validation-evidence` release
with the pull request.

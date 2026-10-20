# Requirements — Issue #863 (screenshots from a list of camera positions)

From Spec v1 (https://github.com/taftary/astrolith/issues/863#issuecomment-6293656009), approved in https://github.com/taftary/astrolith/issues/863#issuecomment-6293762711.

- [ ] C1: `--screens <dir> --shots <file>` takes exactly the file's shots in order, each saved as `<name>.png` and listed in `screens.txt` (AC1) (verify: contains shot_file_takes_its_shots ... ok in cargo-test.log)
- [ ] C2: A shot in planet terms puts the camera at that latitude, longitude and height above the drawn ground, looking along the heading and tilt (AC2) (verify: contains planet_terms_place_the_camera ... ok in cargo-test.log)
- [ ] C3: A shot in raw coordinates puts the camera there looking along `dir`, at any level L1-L10 (AC3) (verify: contains raw_coordinates_work_at_every_level ... ok in cargo-test.log)
- [ ] C4: A `figure` shot shows the walking figure standing at the spot with the camera behind it (AC4) (verify: contains figure_shot_stands_the_body ... ok in cargo-test.log)
- [ ] C5: A wrong line stops the run before the window opens, naming the line and the problem (AC5) (verify: contains wrong_shot_lines_are_refused ... ok in cargo-test.log)
- [ ] C6: The visual-validation skill explains the file, shows an example, and keeps the file with the evidence (AC6) (verify: file .claude/skills/visual-validation/SKILL.md contains --shots)
- [ ] C7: Everything else behaves as after #847, and the headless game check passes (AC7) (verify: contains VERIFY-OK in verify.log)

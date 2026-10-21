# Requirements — Issue #899 (player second view camera follows behind on turn)

From Spec v3 (https://github.com/taftary/astrolith/issues/899#issuecomment-6320154032), approved in https://github.com/taftary/astrolith/issues/899#issuecomment-6320174422.

- [ ] C1: In the behind view while walking, turning with A/D ends with the camera behind you (AC1) (verify: contains behind_camera_ends_behind_after_turn ... ok in cargo-test.log)
- [ ] C2: The move is smooth with no snap or cut (AC2) (verify: contains behind_camera_moves_smoothly ... ok in cargo-test.log)
- [ ] C3: The mouse still turns the camera freely; after orbiting with the mouse, turning with A/D brings it back behind (AC3) (verify: contains behind_camera_eases_back_after_free_orbit ... ok in cargo-test.log)
- [ ] C4: Same behind-follow while flying; first-person view is unchanged (AC4) (verify: contains behind_follow_walking_and_flying_first_unchanged ... ok in cargo-test.log)
- [ ] C5: Behind camera follow in Settings changes the follow (Off keeps today, Quick/Smooth follow at two speeds) (AC5) (verify: contains behind_follow_setting_round_trips ... ok in cargo-test.log)
- [ ] C6: Help > Normal mode shows the new glide-back sentence word for word (AC6) (verify: contains guides_explain_behind_follow ... ok in cargo-test.log)

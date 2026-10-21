# Requirements — Issue #899 (player second view camera follows behind on turn)

From Spec v5 (https://github.com/taftary/astrolith/issues/899#issuecomment-6321124140), approved in https://github.com/taftary/astrolith/issues/899#issuecomment-6321134263. Replaces the Spec v3 requirements after the PR #905 review (owner answer A).

- [ ] C1: In the behind view while walking, A/D turn the body and the camera comes to rest exactly behind it; holding D for 3 s never makes the camera circle on its own (AC1) (verify: contains behind_follow_settles_behind ... ok in cargo-test.log)
- [ ] C2: The camera's move is smooth with no snap or cut (AC2) (verify: contains behind_camera_moves_smoothly ... ok in cargo-test.log)
- [ ] C3: The mouse still turns the camera freely; W/S move along the body, and turning or walking brings the camera back behind (AC3) (verify: contains behind_camera_eases_back_after_free_orbit ... ok in cargo-test.log)
- [ ] C4: The same while flying in drone style, plane flying keeps A/D roll and bank, and first-person view is unchanged (AC4) (verify: contains behind_follow_walking_and_flying_first_unchanged ... ok in cargo-test.log)
- [ ] C5: Behind camera follow in Settings changes the follow (Off keeps the camera where the mouse leaves it; Quick and Smooth follow at two speeds) (AC5) (verify: contains behind_follow_setting_round_trips ... ok in cargo-test.log)
- [ ] C6: Help > Normal mode shows the Spec v5 G1 text word for word (AC6) (verify: contains guides_explain_behind_follow ... ok in cargo-test.log)
- [ ] C7: In third person A/D turn the body at the body turn rate in place and W walks the way the body faces, not the camera (owner answer A) (verify: contains third_person_moves_with_the_camera ... ok in cargo-test.log)

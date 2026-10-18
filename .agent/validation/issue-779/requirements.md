# Requirements (CONFIRMED) - issue #779

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/779#issuecomment-6256420289,
owner approval https://github.com/taftary/astrolith/issues/779#issuecomment-6256427323),
with Q1-Q7 (all A) and G1 yes, G2 yes. The look on the real window and how
walking feels are judged by the main agent's screenshots and the owner test;
the validator takes no pictures (#533).

- [ ] C1: The ground under the body is the drawn surface (the finest drawn patch blended as the shader does), returned at L10 (AC1, Q1 A) (verify: contains ground_is_the_drawn_surface ... ok in cargo-test.log)
- [ ] C2: Below about 100 m the body's physics runs in a local tangent frame (up, east, north) re-anchored about every 1 km; crossing the boundary or re-anchoring moves the body under 1 mm (AC2, Q2 A) (verify: contains local_frame_switch_moves_under_a_millimetre ... ok in cargo-test.log)
- [ ] C3: Gravity points to the planet's centre, the body stands upright along it, and on any slope the feet follow the ground height (AC3, Q3 A, Q4 A) (verify: contains body_stays_upright_on_slopes ... ok in cargo-test.log)
- [ ] C4: On the scripted 200 m walk and run at 60 fps over a mountain slope and a plain, the feet stay within 1 cm of the drawn ground every frame and a jump lands back within 1 cm (AC4, Q6 A) (verify: contains GROUND in verify.log)
- [ ] C5: Normal mode starts standing on the ground at the seeded spot (AC5, Q7 A) (verify: contains player_starts_standing_on_the_ground ... ok in cargo-test.log)
- [ ] C6: The headless check prints a `GROUND` line with the worst feet gap while walking and running, the landing gap and the frame-switch jump, all within target (AC6) (verify: contains frame_switch_mm= in verify.log)
- [ ] C7: Help > Navigation and About > What is Astrolith show the G1 and G2 walking sentences (AC7, G1, G2) (verify: contains guides_restore_the_walking_sentences ... ok in cargo-test.log)
- [ ] C8: Everything else behaves as after #778 and the headless game check passes (AC8) (verify: contains VERIFY-OK in verify.log)

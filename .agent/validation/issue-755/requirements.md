# Requirements (CONFIRMED) - issue #755

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/755#issuecomment-6245452262,
owner approval https://github.com/taftary/astrolith/issues/755#issuecomment-6245455221),
with the decisions carried from #754 (Q3 A, Q4 A, Q6 A, Q9 A) and S1 yes.
How the figure looks on the real window is judged by the main agent's
`--screens` pictures and the owner test; the validator takes no pictures (#533).

- [ ] C1: The body is a low-poly figure (head, torso, arms, legs), 1.8 m tall at every level, facing the body's heading, built in code with no model files (AC1) (verify: contains the_figure_is_a_true_size_low_poly_person ... ok in cargo-test.log)
- [ ] C2: The figure is shaded by the direction of the nearest star, lit side brighter, dark side above a minimum brightness; nothing else changes how it is drawn (AC2) (verify: contains the_figure_is_shaded_by_its_star ... ok in cargo-test.log)
- [ ] C3: The waiting body and the active body come from the same drawing code (AC3) (verify: contains one_drawing_for_waiting_and_active ... ok in cargo-test.log)
- [ ] C4: Third person shows the whole figure; first person shows torso, arms and legs looking down and never the head (AC4) (verify: contains first_person_shows_the_body_not_the_head ... ok in cargo-test.log)
- [ ] C5: Body colour cycles grey and orange (default), white, blue, red; applied at once; saved, restored and reset with Display (AC6, S1) (verify: contains body_colour_setting_cycles_and_saves ... ok in cargo-test.log)
- [ ] C6: Everything else is unchanged and the headless game check passes (AC5) (verify: contains VERIFY-OK in verify.log)

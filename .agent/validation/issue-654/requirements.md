# Requirements (CONFIRMED) - issue #654

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/654#issuecomment-6163656011,
owner approval https://github.com/taftary/astrolith/issues/654#issuecomment-6163724828),
with the decisions D1-D7. How the lab looks on the real window is judged by
the main agent's `engine-lab --screens` pictures and the owner test; the
validator takes no pictures (#533).

- [ ] C1: The program is engine-lab, titled "Engine lab", its screenshots are named engine-lab-..., and no planet-lab name remains in code or docs outside the history and ADR 0029's rename note (AC1) (verify: contains the_lab_is_named_engine_lab ... ok in cargo-test.log)
- [ ] C2: The engine-lab doc exists (AC1) (verify: file docs/engine-lab.md contains cargo run -p engine-lab)
- [ ] C3: Selecting in 3D or in the Nodes list selects the same object in both, the selected object has an orange outline, and clicking empty space clears it (AC2) (verify: contains selection_is_shared_by_the_list_and_the_view ... ok in cargo-test.log)
- [ ] C4: F or Show selected on a moon makes it the main object at the centre, the list starts from it, its readouts are its own, and Back returns to the planet (AC3) (verify: contains f_shows_the_selected_moon_and_back_returns ... ok in cargo-test.log)
- [ ] C5: G switches Orbit and Free-fly; F no longer does (AC4) (verify: contains g_switches_free_fly_and_f_shows_the_selected ... ok in cargo-test.log)
- [ ] C6: Parent goes from L10 up to L1 one level at a time and Child goes back down into the selected object; the status bar names the level (AC5) (verify: contains parent_and_child_move_through_every_level ... ok in cargo-test.log)
- [ ] C7: Above the planet the Nodes list shows that level's objects with the game's names, and they can be selected (AC6) (verify: contains every_level_lists_its_objects ... ok in cargo-test.log)
- [ ] C8: At L1-L9, Wireframe, Normals and LOD depth view are greyed out and the stats show the level size instead of the height (AC7) (verify: contains planet_views_are_greyed_above_the_planet ... ok in cargo-test.log)
- [ ] C9: The game is unchanged: the same seed gives the same universe on every run, and no game crate depends on the lab (AC8) (verify: contains VERIFY-OK in verify.log)

# Requirements (CONFIRMED) - issue #640

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/640#issuecomment-6157637529,
owner approval https://github.com/taftary/astrolith/issues/640#issuecomment-6157647940),
with the clarification answers Q1-Q10. How the lab looks on the real window
is judged by the main agent's `planet-lab --screens` pictures and the owner
test; the validator takes no pictures (#533).

- [ ] C1: The lab is its own program with the menu bar, nodes tree, tools panel, viewport and status bar, titled "Planet lab" (AC1) (verify: contains the_lab_lays_out_the_editor_panels ... ok in cargo-test.log)
- [ ] C2: The game does not depend on the lab (AC1) (verify: contains the_game_crates_never_depend_on_the_lab ... ok in cargo-test.log)
- [ ] C3: On start the lab shows the game's home planet with its moons from the game's own planet code (AC2) (verify: contains the_lab_opens_on_the_home_planet_with_its_moons ... ok in cargo-test.log)
- [ ] C4: The same seed and path always give the same planet; a new seed rebuilds the universe (AC3) (verify: contains the_same_seed_and_path_give_the_same_planet ... ok in cargo-test.log)
- [ ] C5: The path starts on the home star; a valid path jumps to that star's first planet; a wrong path shows a red message and keeps the planet (AC4) (verify: contains a_wrong_path_keeps_the_planet_and_says_why ... ok in cargo-test.log)
- [ ] C6: Previous and Next step through every planet of the star, "Planet i of N", each with its moons (AC5) (verify: contains previous_and_next_step_through_every_planet ... ok in cargo-test.log)
- [ ] C7: Each node shows its values in the tools panel (AC6) (verify: contains every_node_shows_its_values ... ok in cargo-test.log)
- [ ] C8: The orbit camera orbits, zooms and pans; the seven preset views and Home work (AC7) (verify: contains the_orbit_camera_and_preset_views_frame_the_planet ... ok in cargo-test.log)
- [ ] C9: F switches to free-fly and back; going down, the height and shell level follow L11-L14 (AC8) (verify: contains free_fly_reaches_the_ground_through_every_shell ... ok in cargo-test.log)
- [ ] C10: Wireframe, Normals and LOD depth view toggle; the LOD view counts splits per depth (AC9) (verify: contains the_debug_views_toggle_and_count_lod_splits ... ok in cargo-test.log)
- [ ] C11: A screenshot is named with the date and time in the screenshots folder (AC10) (verify: contains screenshots_are_named_by_date_and_time ... ok in cargo-test.log)
- [ ] C12: The stats overlay reads FPS, frame time, triangles, meshes, position, distance, height and shell, and hides (AC11) (verify: contains the_stats_overlay_reads_the_scene_and_hides ... ok in cargo-test.log)
- [ ] C13: --screens writes every listed picture and screens.txt; a missing folder exits 2 (AC12) (verify: contains the_screens_list_covers_every_view_and_panel ... ok in cargo-test.log)
- [ ] C14: Units are real metres: Earth shows 6,371 km, the grid and readouts in km or m (AC13) (verify: contains readouts_are_real_metres ... ok in cargo-test.log)
- [ ] C15: The lab is a desktop-only program, never part of a mobile or touch build (AC14) (verify: contains the_lab_is_desktop_only ... ok in cargo-test.log)
- [ ] C16: The game is unchanged: the same seed gives the same universe on every run (non-goal 1) (verify: contains VERIFY-OK in verify.log)

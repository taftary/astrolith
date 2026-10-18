# Requirements (CONFIRMED) - issue #776

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/776#issuecomment-6248879855,
owner approval https://github.com/taftary/astrolith/issues/776#issuecomment-6248886771),
with Q1-Q8 (all A) and S1, S2 yes. What the patches look like on the real
window, and the frame time on the dev PC, are judged by the main agent's
screenshots and measurements and by the owner test; the validator takes no
pictures (#533).

- [ ] C1: Each face is a quadtree of 33 × 33-point patches on #775's mapping (AC1, Q1 A) (verify: contains faces_are_quadtrees_of_33_point_patches ... ok in cargo-test.log)
- [ ] C2: A patch splits above 1.5 px and merges only below 0.75 px, neighbours are never more than one level apart, and the deepest level is about 1 m between points for Earth (AC2, Q2 A, Q3 A, Q7 A) (verify: contains patches_split_and_merge_by_screen_error ... ok in cargo-test.log)
- [ ] C3: Patches are built on worker threads, at most the measured number is applied per frame, and a parent stays drawn until its children are ready (AC3, Q4 A) (verify: contains parents_stay_until_children_are_ready ... ok in cargo-test.log)
- [ ] C4: On the drawn patches the shake is under 1 mm within 1 km of the camera at every tested height (AC4, Q5 A) (verify: contains drawn_patches_shake_under_a_millimetre ... ok in cargo-test.log)
- [ ] C5: Patch edges at the same level and face seams match bit for bit, and cracks between levels are counted and reported (AC5) (verify: contains same_level_edges_match_bit_for_bit ... ok in cargo-test.log)
- [ ] C6: F3 toggles the debug overlay with all seven items, off by default (AC6, Q6 A) (verify: contains f3_toggles_the_debug_overlay ... ok in cargo-test.log)
- [ ] C7: The headless check prints an `LOD` line (patch counts per height, worst shake, neighbour rule, same-level mismatches, cross-level cracks) (AC7) (verify: contains LOD patches in verify.log)
- [ ] C8: Everything else behaves as after #775, and the headless game check passes (AC8) (verify: contains VERIFY-OK in verify.log)
- [ ] C9: Planet detail (Low 2 px, Normal 1.5 px, High 1 px; default Normal) sets the thresholds, and Debug overlay (default off) stays in step with F3; both are saved, restored and reset (AC9, S1, S2) (verify: contains planet_detail_and_debug_overlay_settings ... ok in cargo-test.log)

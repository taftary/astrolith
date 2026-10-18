# Requirements (CONFIRMED) - issue #777

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/777#issuecomment-6250561240,
owner approval https://github.com/taftary/astrolith/issues/777#issuecomment-6250577575),
with Q1-Q7 (all A). The smoothness on the real window and the frame time on
the dev PC are judged by the main agent's screenshots and the owner test;
the validator takes no pictures (#533).

- [ ] C1: Merges use the relief-aware error; over a mountain the tree reaches deeper than over a plain within a few frames and stays, and a still camera causes 0 splits and 0 merges (AC1, Q1 A) (verify: contains mountain_detail_builds_up_and_stays ... ok in cargo-test.log)
- [ ] C2: Every point blends between its own and its parent position and normal over the last 30% of its band, with no per-frame mesh rebuild (AC2, Q2 A, Q6 A, Q7 A) (verify: contains fully_blended_patch_is_its_parent ... ok in cargo-test.log)
- [ ] C3: On edges touching a coarser neighbour the in-between points lie exactly on the coarse edge (AC3, Q3 A) (verify: contains edges_pin_exactly_to_the_coarse_edge ... ok in cargo-test.log)
- [ ] C4: Every patch edge has a skirt below the ground (AC4, Q4 A) (verify: contains every_edge_has_a_skirt ... ok in cargo-test.log)
- [ ] C5: 0 cracks at every edge, equal and different levels, over the scripted path (AC5) (verify: contains scripted_path_has_no_cracks_and_smooth_morphs ... ok in cargo-test.log)
- [ ] C6: On the scripted path at the default flight speed, 60 fps, below 3 km the worst on-screen jump is under 1 px at 1600×900 and normals change under 2° per frame; at or above 3 km popping stays under 2.5 px and normals are reported only (AC6, Q5 A, owner decision option 2 with the 3 km line, Telegram 2026-10-18; the rest moves to #817) (verify: contains scripted_path_has_no_cracks_and_smooth_morphs ... ok in cargo-test.log)
- [ ] C7: The headless check prints a `MORPH` line (popping px and normal change below and above 3 km, cracks and worst gap, mountain vs plain depth, still-camera splits and merges) (AC7) (verify: contains MORPH in verify.log)
- [ ] C8: Everything else behaves as after #799, and the headless game check passes (AC8) (verify: contains VERIFY-OK in verify.log)

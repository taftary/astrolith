# Requirements (CONFIRMED) - issue #817

Derived by the main agent from approved Spec v4
(https://github.com/taftary/astrolith/issues/817#issuecomment-6262578086,
owner approval https://github.com/taftary/astrolith/issues/817#issuecomment-6262667743;
the full spec with the F1 range base, the roughness measure, owner F2 A
pixels per quad, the level ramp of the scale and edge masks against the
whole tree), with Q1-Q6 (all A). The frame time on the dev PC and the look on the real window are
judged by the main agent's flight bench and screenshots and by the owner
test; the validator takes no pictures (#533).

- [ ] C1: The selection measures each patch at its real height range; near the ground at the mountain spot the tree reaches its deepest level (AC1, Q1 A, Q5 A) (verify: contains tree_reaches_the_deepest_level_near_the_ground ... ok in cargo-test.log)
- [ ] C2: Each level owns a fixed range proportional to its patch size, doubling per coarser level, so a quad covers 4 / 8 / 16 px at the range end (High / Normal / Low); a patch splits when its box enters the next finer range (AC2, F1 A, F2 A) (verify: contains ranges_follow_patch_size_and_detail ... ok in cargo-test.log)
- [ ] C3: Ranges are scaled 1x to 4x by a smooth relief roughness field, the same on both sides of every edge, capped by level (1x up to level 6, rising to 4x from level 10); over a mountain the tree reaches deeper than over a plain, and neighbours stay within one level (AC3, Q3 A) (verify: contains mountain_depth= in verify.log)
- [ ] C4: Each vertex blends over the last 30% of its own level's range, so a patch is fully its parent's shape when it enters the parent's range (AC4, Q4 A) (verify: contains children_appear_fully_blended ... ok in cargo-test.log)
- [ ] C5: On the whole path popping stays under 1 px and normals under 2 degrees, cracks are 0, and a still camera causes 0 splits and 0 merges (AC5, Q6 A) (verify: contains still_splits=0 still_merges=0 PASS in verify.log)
- [ ] C6: The MORPH line has no 3 km split and passes on 1 px and 2 degrees over the whole path (AC6) (verify: contains scripted_path_has_no_cracks_and_smooth_morphs ... ok in cargo-test.log)
- [ ] C7: The CULL budgets of #778 hold at Normal, the LOD and DEPTH lines pass, and the headless game check passes (AC7) (verify: contains drawn_hidden=0 visible_dropped=0 PASS in verify.log)

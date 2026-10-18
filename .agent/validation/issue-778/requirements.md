# Requirements (CONFIRMED) - issue #778

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/778#issuecomment-6254908810,
owner approval https://github.com/taftary/astrolith/issues/778#issuecomment-6254923875),
with Q1-Q9 (all A). AC10 (frame time on the dev PC not above `main`'s) and
the look on the real window are judged by the main agent's flight bench and
screenshots and by the owner test; the validator takes no pictures (#533).

- [ ] C1: At L10, in every mode, the near plane follows the height above the drawn terrain (a small fraction of it, floor ~1 mm), and on the scripted descent it always stays closer than the nearest drawn ground (AC1, Q1 A) (verify: contains near_plane_follows_height_above_terrain ... ok in cargo-test.log)
- [ ] C2: The projection stays infinite reverse-Z, with no far plane (AC2, Q2 A) (verify: contains projection_is_infinite_reverse_z ... ok in cargo-test.log)
- [ ] C3: The selection drops patches outside the view (neither split nor drawn); a fast turn refines in over a few frames with no holes; patch boxes cover the parent shape (AC3, Q3 A) (verify: contains frustum_culled_patches_are_not_split ... ok in cargo-test.log)
- [ ] C4: The selection drops patches fully below the horizon, tested with relief, so mountain tops beyond the horizon stay (AC4, Q4 A) (verify: contains horizon_keeps_mountain_tops ... ok in cargo-test.log)
- [ ] C5: Patch triangles are one-sided, and skirts face out (AC5, Q5 A) (verify: contains skirts_face_out ... ok in cargo-test.log)
- [ ] C6: At 1 cm and 1 m, surfaces 1 mm apart get distinct depth values; at 1,000 km, surfaces one pixel apart on the ground get distinct depth values (AC6, Q8 A) (verify: contains DEPTH in verify.log)
- [ ] C7: On the scripted path: at most 2,000 visible patches, at most 3 million terrain triangles, at most 100 builds applied per frame; 0 drawn patches outside the view or below the horizon, and no visible patch dropped (AC7, Q7 A) (verify: contains CULL in verify.log)
- [ ] C8: The headless check prints a `CULL` line and a `DEPTH` line (AC8) (verify: contains VERIFY-OK in verify.log)
- [ ] C9: Cracks 0, popping and normals within #777 Spec v2, a still camera 0 splits and 0 merges; everything else behaves as after #777 (AC9) (verify: contains MORPH in verify.log)

# Requirements (CONFIRMED) - issue #593

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/593#issuecomment-6139889943,
owner approval https://github.com/taftary/astrolith/issues/593#issuecomment-6139897357),
with answers Q1-Q5 and design decision A
(https://github.com/taftary/astrolith/issues/593#issuecomment-6139940858,
confirmed in https://github.com/taftary/astrolith/issues/593#issuecomment-6139963980):
the planet is the icosphere split once, 80 flat faces with corners on the
sphere, and finer splits lie flat inside those faces. One or more criteria
per acceptance criterion, in the spec's words. The look of the faces and
edge lines on the real window is judged by the main agent's screenshots and
the owner test.

- [ ] C1: At L10-L13 the surface's face corners lie on the planet's sphere, with no relief (AC1) (verify: contains face_corners_sit_on_the_sphere ... ok in cargo-test.log)
- [ ] C2: The surface between corners is flat triangles; finer splits lie flat inside the faces (AC1, decision A) (verify: contains the_surface_is_flat_inside_each_face ... ok in cargo-test.log)
- [ ] C3: Every drawn mesh vertex lies on the flat-faced surface at L10-L13 (AC1) (verify: contains vertices_sit_on_the_flat_faces ... ok in cargo-test.log)
- [ ] C4: The drawn triangles are flat, one normal per triangle (AC1) (verify: contains mesh_normals_are_flat_per_triangle ... ok in cargo-test.log)
- [ ] C5: Every triangle edge the surface mesh draws shows as a marker line, and the lines get finer near the camera with the mesh's splits (AC2) (verify: contains every_drawn_edge_shows_once_and_follows_the_splits ... ok in cargo-test.log)
- [ ] C6: The edge lines appear with the surface at L10-L13 and go away off the surface levels (AC2) (verify: contains l11_open_cell_spawns_one_surface_and_leaving_despawns_it ... ok in cargo-test.log)
- [ ] C7: In normal mode the body stands on the flat face under it, neither sinking nor floating, including while walking across a face edge (AC3) (verify: contains a_body_walks_on_the_flat_faces_across_their_edges ... ok in cargo-test.log)
- [ ] C8: Neighbouring faces meet with no step at their edges (AC3) (verify: contains neighbouring_faces_meet_without_a_step ... ok in cargo-test.log)
- [ ] C9: Region portals sit on the flat faces of the drawn surface (AC4) (verify: contains region_portals_ride_the_icosphere_surface ... ok in cargo-test.log)
- [ ] C10: The L11-L13 ground is the same flat-faced surface (AC4) (verify: contains ground_height_sits_on_the_flat_surface ... ok in cargo-test.log)
- [ ] C11: The dive's ground stop follows the same flat faces (AC4) (verify: contains clearance_follows_the_icosphere_surface ... ok in cargo-test.log)
- [ ] C12: Portals at L10-L13 sit on their structure in the headless check (AC4) (verify: contains PORTAL-ON-STRUCTURE L10=PASS L11=PASS L12=PASS L13=PASS PASS in verify.log)
- [ ] C13: The relief code is gone: the planet record carries no height sample (AC5) (verify: contains F dot T 23.400000 23.900000 1.000000 in verify.log)
- [ ] C14: The same seed still gives the same universe on every run (AC6) (verify: contains VERIFY-OK in verify.log)

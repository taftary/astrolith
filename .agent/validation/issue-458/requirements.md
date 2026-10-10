# Requirements (CONFIRMED) - issue #458

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/458#issuecomment-6099441953,
owner approval https://github.com/taftary/astrolith/issues/458#issuecomment-6099500901).
One criterion per acceptance criterion, in owner words. Window feel is
judged by the owner test; the script marks a criterion without a probe
UNVERIFIABLE and it never counts as MET from a summary claim. Test names
below are the planned probes; they are fixed when the tests land. No
criterion asserts a pull-request run observation.

- [ ] C1: The planet, its regions, cities and plots are one continuous icosphere surface; no separate flat rectangle exists from L10 to L13 (verify: contains tail_levels_wear_no_rects_or_patches in cargo-test.log)
- [ ] C2: The surface height at every level comes from the planet's own height field; the same point has the same height from orbit and from the ground (verify: contains surface_height_agrees_from_orbit_to_ground in cargo-test.log)
- [ ] C3: The triangles entered at one level are the same triangles split further at the next; nothing jumps, appears or disappears when crossing a level (verify: contains entered_tile_refines_without_seams_or_pops in cargo-test.log)
- [ ] C4: Region, city and plot clusters nest exactly: every city lies inside its region and every plot inside its city (verify: contains clusters_nest_inside_their_parent in cargo-test.log)
- [ ] C5: The whole surface is divided into clusters; only the enterable ones are highlighted, in today's counts (verify: contains clusters_cover_the_surface_and_portal_counts_hold in cargo-test.log)
- [ ] C6: Every enterable building plot shows one building box standing on the slope, and entering it opens the room (verify: contains plots_carry_one_building_box_on_the_slope in cargo-test.log)
- [ ] C7: Ground clearance, landing and local up follow the new surface; diving never goes under it (verify: contains clearance_follows_the_icosphere_surface in cargo-test.log)
- [ ] C8: The same seed always gives the same clusters and surface, on every platform (verify: contains VERIFY-OK in verify.log)
- [ ] C9: One new decision record extends meshes to L10-L13 and supersedes the relevant parts of ADR 0016 and ADR 0017 (verify: file docs/decisions/0020-icosphere-surface-planet-to-building.md contains supersedes)

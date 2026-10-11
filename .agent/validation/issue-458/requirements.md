# Requirements (CONFIRMED) - issue #458, correction round 1

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/458#issuecomment-6099441953,
owner approval https://github.com/taftary/astrolith/issues/458#issuecomment-6099500901)
and the round 1 owner answers: Q1-Q5 as proposed
(https://github.com/taftary/astrolith/issues/458#issuecomment-6113516022)
and option A
(https://github.com/taftary/astrolith/issues/458#issuecomment-6113554530).
One criterion per acceptance criterion or answer, in owner words. Window
feel is judged by the owner test; the script marks a criterion without a
probe UNVERIFIABLE and it never counts as MET from a summary claim. A
`... ok` probe only matches a test that passed.

- [ ] C1: The planet, its regions, cities and plots are one continuous icosphere surface; no separate flat rectangle exists from L10 to L13 (verify: contains tail_levels_wear_no_rects_or_patches ... ok in cargo-test.log)
- [ ] C2: The surface height at every level comes from the planet's own height field; the same point has the same height from orbit and from the ground (verify: contains ground_height_sits_on_the_relief_surface ... ok in cargo-test.log)
- [ ] C3: The triangles entered at one level are the same triangles split further at the next; nothing jumps, appears or disappears when crossing a level (verify: contains entry_keeps_the_same_triangles_on_both_sides ... ok in cargo-test.log)
- [ ] C4: Region, city and plot clusters nest exactly: every city lies inside its region and every plot inside its city (verify: contains regions_and_cities_nest_down_the_journey ... ok in cargo-test.log)
- [ ] C5: The whole surface is divided into clusters; only the enterable ones are highlighted, in today's counts (verify: contains tail_cells_hold_windowed_nested_clusters ... ok in cargo-test.log)
- [ ] C6: Every enterable building plot shows one building box standing on the slope, and entering it opens the room (verify: contains tail_points_are_cluster_anchors_on_the_surface ... ok in cargo-test.log)
- [ ] C7: Ground clearance, landing and local up follow the new surface; diving never goes under it (verify: contains landing_cap_never_drops_the_floor_under_the_surface ... ok in cargo-test.log)
- [ ] C8: The same seed always gives the same clusters and surface, on every platform (verify: contains VERIFY-OK in verify.log)
- [ ] C9: One new decision record covers the one icosphere mesh and supersedes ADR 0020 (verify: file docs/decisions/0024-one-icosphere-mesh-planet-to-building.md contains supersedes ADR 0020)
- [ ] R1: No new meshes: the icosphere is the main and only ground mesh from planet to plot (verify: contains l11_open_cell_spawns_one_surface_and_leaving_despawns_it ... ok in cargo-test.log)
- [ ] R2: By default the planet is the icosphere split once (verify: contains far_planet_is_the_icosphere_split_once ... ok in cargo-test.log)
- [ ] R3: Each level splits the entered triangles as often as its ladder size needs, then scales them to fill the view (verify: contains tail_depths_follow_the_ladder_sizes ... ok in cargo-test.log)
- [ ] R4: Enterable spots are their own triangles in the brighter colour, with no extra shapes (verify: contains enterable_clusters_draw_brighter ... ok in cargo-test.log)
- [ ] R5: On the real window, the planet shows as a faceted icosphere, regions, cities and plots show the same ground split finer with brighter enterable clusters, and diving in and out never pops or goes under the ground. These need eyes on the real window.

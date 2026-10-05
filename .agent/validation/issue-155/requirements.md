# Requirements (CONFIRMED) — issue #155

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/155#issuecomment-5990401683,
owner approval https://github.com/taftary/astrolith/issues/155#issuecomment-5990408113).
One criterion per acceptance criterion, in owner words. Criteria about
variety, home fixtures, and crowding are judged by the
validator directly (headless output plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: L6 cells hold usually one system portal instead of a fixed handful, with types varied (mostly small red dwarfs, rarely big blue ones) instead of identical markers (verify: file crates/universe-core/src/system.rs contains STAR_MIX)
- [ ] C2: about the observed share of systems are multiple (companions shown beside their star, never opening); the Alpha Centauri home portal is a fixed triple (verify: file crates/universe-core/src/system.rs contains ALPHA_CEN_TRIPLE)
- [ ] C3: stars carry at least one planet on average with the size gap visible in the data; L8 shows one star with its planets ordered outward, L7 shows one star with Oort shells around it that never open (verify: file crates/universe-core/src/system.rs contains SOLAR_AXES_AU)
- [ ] C4: L5 clouds sit along the arms with the observed size and mass spread instead of uniform scatter (verify: file crates/universe-core/src/sysgen.rs contains attach_l5_clouds)
- [ ] C5: the home path runs Alpha Centauri triple, then Solar Oort shells, Solar 8 planets, Sun, Earth-Moon; procedural neighbours never move or cover them; the dive never strands and portal order from #151/#153 is unchanged (verify: file crates/universe-core/src/nest.rs contains is_home_system_cell)
- [ ] C6: same seed gives the same systems on rerun and across Windows/Linux; a regenerated cell byte-matches; --verify ends in VERIFY-OK; --capture succeeds; the golden rewrite is explained line by line in the PR (verify: contains VERIFY-OK in verify.log)
- [ ] C7: window rendering is unchanged in style (gizmo indicators, single group color, portal-prominent versus population-faint split); no new crate, no ladder-row change, no ADR in this Issue (verify: file crates/universe-render/src/style.rs contains cyan)

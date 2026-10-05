# Requirements (CONFIRMED) — issue #154

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/154#issuecomment-5989687115,
owner approval https://github.com/taftary/astrolith/issues/154#issuecomment-5989691643).
One criterion per acceptance criterion, in owner words. Criteria about
shape variety, density linkage, and crowding are judged by the
validator directly (headless output plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: L4 cells hold all three shapes (spiral, elliptical, irregular) with varied sizes and tilts; spirals show arms plus bar plus bulge in their layout data (verify: file crates/universe-core/src/astro.rs contains barred)
- [ ] C2: dense cells hold clearly more ellipticals than sparse cells per the kept bias; the Milky Way home portal is a barred spiral of Milky-Way size (verify: file crates/universe-core/src/astro.rs contains MILKY_WAY_ORIENTATION)
- [ ] C3: disk/bulge/halo proportions read in the layout data (thin disk, central bulge share, edge fade); no bulge or halo appears as its own marker (verify: file crates/universe-core/src/astro.rs contains BAR_DISK_SHARE)
- [ ] C4: distant galaxies past the gate carry the simplified record; generation stays inside the frame budget on the dev PC; the dive never strands and portal order is unchanged from #151/#153 (verify: file crates/universe-core/src/astro.rs contains FAR_VIEW_SIZE)
- [ ] C5: same seed gives the same galaxies on rerun and across Windows/Linux; a regenerated cell byte-matches; counts and portal mapping are unchanged (verify: contains VERIFY-OK in verify.log)
- [ ] C6: --verify ends in VERIFY-OK; --capture succeeds; the golden rewrite is explained line by line in the PR (verify: contains VERIFY-OK in verify.log)
- [ ] C7: window rendering is unchanged in style (gizmo indicators, single group color, portal-prominent versus population-faint split); no new crate, no ladder-row change, no ADR in this Issue (verify: file crates/universe-render/src/style.rs contains cyan)

# Requirements (CONFIRMED) — issue #157

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/157#issuecomment-5994157367,
owner approval https://github.com/taftary/astrolith/issues/157#issuecomment-5994168115).
One criterion per acceptance criterion, in owner words. Criteria about
glow feel, readability, and crowding are judged by the
validator directly (headless output plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: Stars show per-kind colors with a white Sun (blue giants read blue-white, red dwarfs read dim red); galaxies show per-kind colors by density (red ellipticals, blue spirals, green-valley Milky Way) (verify: file crates/universe-core/src/system.rs contains BLACKBODY)
- [ ] C2: Bright sources glow through bloom and a filmic tonemap; nothing blows out to pure white where color should read; planets keep their #156 climate colors (verify: file crates/universe-render/src/camera.rs contains Filmic)
- [ ] C3: The exposure keys brighten and dim the whole view and reopen at the look of today (1.0); no other key binding changes (verify: file crates/universe-render/src/input.rs contains KeyE)
- [ ] C4: A persistent readout shows a scale bar plus the level name plus the distance; the dive route and timing are unchanged (verify: file crates/universe-render/src/hud.rs contains scale_bar)
- [ ] C5: Nothing moves that did not move before (no orbit, spin, twinkle, or time rate); L10 stays terminal; ladder rows and anchors unchanged (verify: file crates/universe-core/src/nest.rs contains MAX_OPEN_LEVEL)
- [ ] C6: Same seed gives the same colors on rerun and across Windows/Linux; a regenerated cell byte-matches; counts and portal mapping unchanged (verify: contains VERIFY-OK in verify.log)
- [ ] C7: --verify ends in VERIFY-OK; --capture succeeds; the golden rewrite is explained line by line in the PR; the render-permission update names this Issue; 60 fps and the 32-cell / 2 ms budgets hold on the dev PC (verify: file docs/universe/stack.md contains bloom)

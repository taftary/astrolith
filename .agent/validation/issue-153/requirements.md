# Requirements (CONFIRMED) — issue #153

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/153#issuecomment-5985520234,
owner approval https://github.com/taftary/astrolith/issues/153#issuecomment-5985523508).
One criterion per acceptance criterion, in owner words. Criteria about
environment proportions, portal behavior, and crowding are judged by the
validator directly (headless output plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: L1-L3 cells carry all four environment tags, with volume shares near 77 / 20 / 2 / under-1 over the journey sample (verify: file crates/universe-core/src/density.rs contains filament)
- [ ] C2: the densest markers open as portals in density order; void/population markers never open however close you fly; hover and click only ever take portals
- [ ] C3: crowded cells hold clearly more markers than empty ones; every visited cell on the journey has at least one portal; no cell exceeds its overlap cap; rich-cluster cells read in the thousands as in #151
- [ ] C4: voids read up to about 1 Gly across; nothing structures above about 200 Mpc; every anchor and level number is unchanged (verify: file docs/universe/ladder.md contains Sloan Great Wall)
- [ ] C5: same seed gives the same field on rerun and across Windows/Linux; shared faces agree; a regenerated cell byte-matches (verify: contains VERIFY-OK in verify.log)
- [ ] C6: --verify ends in VERIFY-OK; --capture succeeds; the golden rewrite is explained line by line in the PR (verify: contains VERIFY-OK in verify.log)
- [ ] C7: window rendering is unchanged in style (gizmo indicators, cyan group color, portal-prominent versus population-faint split from #151); no new crate, no ladder-row change (verify: file crates/universe-render/src/style.rs contains cyan)

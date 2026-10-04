# Requirements (CONFIRMED) — issue #151

Derived by the main agent from approved Spec v3
(https://github.com/taftary/astrolith/issues/151#issuecomment-5982815523,
owner approval https://github.com/taftary/astrolith/issues/151#issuecomment-5982820796;
supersedes Spec v1 5980590918 / approval 5980645257 and Spec v2 5982779852 /
approval 5982790122).
One criterion per acceptance criterion, in owner words. Criteria about
window behavior, count variability, and the golden rewrite are judged by
the validator directly; the script marks those UNVERIFIABLE and they
never count as MET from a log line.

- [ ] C1: ten named levels L1-L10; L2 is the cosmic web containing superclusters; Earth is at L10; ladder.md shows this table (verify: file docs/universe/ladder.md contains L10)
- [ ] C2: hover highlight, targeting and clicking work on portals only; clicking a population point does nothing; populations are visibly smaller and dimmer
- [ ] C3: no cell shows the old nonsense: no system holds two stars except flagged multiples, no Oort cloud holds systems, no star holds planets as markers (companions and moons are populations)
- [ ] C4: anonymous cells are invisible (no labels, no ladder rows) and the zoom runs continuously across every span in the spec table
- [ ] C5: counts vary between cells of the same level from the same seed run (the Virgo-member L4 cell rich versus a poor cluster's or group's L4 cell; L6 usually 1 system) and are byte-identical across reruns and platforms
- [ ] C6: the autopilot visits all 10 levels and ends at Earth; --verify ends in VERIFY-OK; the golden rewrite is explained line by line in the PR (verify: contains VERIFY-OK in verify.log)
- [ ] C7: ADRs 0010 and 0011 exist, each naming this Issue, each marked proposed-or-accepted per E-ADR-ONE
- [ ] C8: rendering is otherwise unchanged: same glyphs, colors, brightness curves, and camera behavior; --capture frames differ only by counts, sizes and positions, confirmed by the frame proof

# Requirements (CONFIRMED) — issue #152

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/152#issuecomment-5984020182,
owner approval https://github.com/taftary/astrolith/issues/152#issuecomment-5984025604).
One criterion per acceptance criterion, in owner words. Criteria about
window flight, saves, and the autopilot switch are judged by the
validator directly (headless proxies plus code reads where stated); the
script marks those UNVERIFIABLE and they never count as MET from a log
line.

- [ ] C1: diving at a target through other markers opens each crossed portal in ray order; the stored target never changes; crossed populations never open; exited cells close past the horizon
- [ ] C2: in free flight WASD moves, mouse-drag looks, speed follows surface distance with keys 1-9 stepping it; F toggles both ways; leaving targets the nearest portal
- [ ] C3: at most 32 live cells ever; everything within 5 s of travel is generated before it is needed; overload unloads longest-unseen-first; the headless output proves the bound on the whole journey (verify: contains VERIFY-OK in verify.log)
- [ ] C4: Ctrl+3 then 3 (any slot) returns to the saved camera and target; slots die with the session
- [ ] C5: Spacebar in free flight returns to the dive first, then the autopilot flies the fixed journey unchanged
- [ ] C6: the replay's scripted steering leg is fixed-seed and byte-identical across reruns and platforms; --verify ends in VERIFY-OK; the golden rewrite is explained line by line in the PR (verify: contains VERIFY-OK in verify.log)
- [ ] C7: both ADRs exist, each naming this Issue, each marked proposed-or-accepted per E-ADR-ONE
- [ ] C8: determinism kept (E-DET-TIERS); same seed, same universe, on Windows and Linux; no per-frame allocation on the new paths (E-HOT-NOALLOC); still gizmos only (E-RENDER-NO-MESH)

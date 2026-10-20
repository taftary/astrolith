# Requirements — Issue #819 (steady, faster frames)

From Spec v2 (https://github.com/taftary/astrolith/issues/819#issuecomment-6299856305), approved in https://github.com/taftary/astrolith/issues/819#issuecomment-6299868119.

- [ ] C1: A shot-file run with `--timing` writes per shot the mean, 95th percentile, worst and short-to-long alternation of its frame times (AC1) (verify: contains timing_summarises_frame_times ... ok in cargo-test.log)
- [ ] C2: A fixed timing file covers orbit, 100 km, 1 km, ground, walking and the L1-L9 dive views, and the release-build baseline is recorded (AC2) (verify: file docs/decisions/0044-frame-time.md contains Baseline)
- [ ] C3: The short/long alternation is found and fixed or shown gone: the 95th percentile within 1.5 times the mean on every view (AC3) (verify: file docs/decisions/0044-frame-time.md contains Alternation)
- [ ] C4: The views limited by the work (orbit, 1 km, walking) get faster against the baseline, against the 16.7 ms target, no view gets slower beyond measuring noise, and the vsync floor and the remaining cost per part are recorded (AC4) (verify: file docs/decisions/0044-frame-time.md contains 16.7 ms)
- [ ] C5: The same views shot before and after match by eye (AC5) (verify: file docs/decisions/0044-frame-time.md contains Same picture)
- [ ] C6: Everything else behaves as after #863, and the headless game check passes (AC6) (verify: contains VERIFY-OK in verify.log)

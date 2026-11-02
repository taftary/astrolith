# Requirements — Issue #1130 (planets keep their own shape after visiting others)

From Spec v2 (https://github.com/taftary/astrolith/issues/1130#issuecomment-6572902592), approved in https://github.com/taftary/astrolith/issues/1130#issuecomment-6575061470.

- [ ] C1: After visiting other planets, any planet opened next shows as a round sphere at its own size, with its six faces in place; no grey rounded cube and no grey band (AC1, test plan steps 1 and 3) (judged by the validator from the shot evidence comment on the PR: the planet tour `.agent/validation/issue-1130/p1130e.shots` and the second order `p1130k.shots`; no script probe)
- [ ] C2: A ground tile made for one planet is never used on another (AC2) (verify: contains patch_grid::tests::a_tile_packed_for_one_planet_is_never_taken_by_another ... ok in cargo-test.log)
- [ ] C3: The first planet visited looks the same as on main: a tile made for the planet being drawn is still used (AC3) (verify: contains patch_grid::tests::a_tile_packed_for_the_same_planet_is_still_used ... ok in cargo-test.log)
- [ ] C4: Every patch drawn after a planet change still has its tile (AC1, AC3) (verify: contains patch_grid::tests::every_cached_patch_keeps_its_tile_across_a_cache_clear ... ok in cargo-test.log)
- [ ] C5: No frame is slower than on main when changing planets (AC4) (judged by the validator: the change adds one comparison per tile taken and packs nothing twice for the same planet, per C3; no script probe)
- [ ] C6: Going back to a planet visited before shows it round, as the first time (test plan step 2) (judged by the validator from the shot evidence comment on the PR: `p1130k.shots` comes back to planets 2 and 3 after visiting others; no script probe)
- [ ] C7: The game's headless check still passes (verify: contains VERIFY-OK in verify.log)

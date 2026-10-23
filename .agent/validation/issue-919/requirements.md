# Requirements — Issue #919 (player never under the ground)

From Spec v2 (https://github.com/taftary/astrolith/issues/919#issuecomment-6372187568), approved in https://github.com/taftary/astrolith/issues/919#issuecomment-6372194629.

- [ ] C1: On arrival on foot (cold start, Go-to-player, Reset, dive handover), the player figure is hidden until the drawn ground under the player is ready, and is never rendered below it (AC1, AC2, AC3) (verify: contains player_hidden_until_ground_ready ... ok in cargo-test.log)
- [ ] C2: The ground patch containing the player builds first, so the hidden wait is short (AC4) (verify: contains player_patch_builds_first ... ok in cargo-test.log)
- [ ] C3: When shown, the player sits exactly on the drawn ground: feet on the surface, no hover, no sink, no pop from below (AC5) (verify: contains player_sits_exactly_on_drawn_ground ... ok in cargo-test.log)
- [ ] C4: Free flight still passes through the ground and dive still keeps its height floor; neither behaviour changes (AC6) (verify: contains window_free_flight_passes_through_the_ground ... ok in cargo-test.log)
- [ ] C5: No crash or regression in the headless journey: the deterministic check passes (AC1-AC6 guard) (verify: contains VERIFY-OK in verify.log)

Owner-test-only (in the Spec v2 test plan, presented for owner testing after merge, not machine-checked here): the walking player is never below the drawn ground at cold start, Go-to-player/Reset, first/third person, and dive handover — hidden until ready, then exactly placed — while free flight still passes through (owner words: "the player should never go under the ground"). Covered mechanically by C1-C4 above.

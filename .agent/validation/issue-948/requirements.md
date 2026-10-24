# Requirements — Issue #948 (quick turns find soft ground, never black)

From Spec v2 (https://github.com/taftary/astrolith/issues/948#issuecomment-6379245891), approved in https://github.com/taftary/astrolith/issues/948#issuecomment-6379256008.

- [ ] C1: A hard third-person A/D flick finds no bare ground: every visible leaf keeps a built ancestor, so the worst case is brief softness, never black (AC1, AC2) (verify: contains fast_yaw_keeps_side_ground_built ... ok in cargo-test.log)
- [ ] C2: A second of first-person run-speed A/D strafe finds no bare ground either (AC1) (verify: contains fast_strafe_keeps_side_ground_built ... ok in cargo-test.log)
- [ ] C3: The background cover queue stays dev-PC scale and drains at rest (verify: contains probe_models_a_real_frame_budget ... ok in cargo-test.log)
- [ ] C4: Nothing else breaks: slow turns still need no builds, the headless game check passes, goldens unchanged (AC3, AC4) (verify: contains VERIFY-OK in verify.log)
- [ ] C5: Ground memory stays under the cap through arrival, turns and travel (AC4) (verify: contains ground_memory_stays_under_the_cap ... ok in cargo-test.log)
- [ ] C6: A hard A/D flick shows no bare ground where the owner saw black (owner notion, AC1/AC2; the eye check stays owner-tested in Spec v2 T1/T2 and gates Done, not the merge) (verify: contains fast_yaw_keeps_side_ground_built ... ok in cargo-test.log)

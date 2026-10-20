# Requirements — Issue #851 (crash on a planet rebuild)

From Spec v1 (https://github.com/taftary/astrolith/issues/851#issuecomment-6287467917), approved in https://github.com/taftary/astrolith/issues/851#issuecomment-6287562619.

- [ ] C1: When the ground is rebuilt for a planet, no piece that is being deleted is touched again in the same frame (AC1) (verify: contains rebuild_never_touches_despawned_patches ... ok in cargo-test.log)
- [ ] C2: An automated test without a window does a planet rebuild and a fresh ground in the same frame and passes with no "Entity despawned" error (AC2) (verify: contains rebuild_never_touches_despawned_patches ... ok in cargo-test.log)
- [ ] C3: Everything else behaves as after #780, and the headless game check passes (AC3) (verify: contains VERIFY-OK in verify.log)

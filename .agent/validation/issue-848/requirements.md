# Requirements — Issue #848 (graphics backend policy)

From Spec v2 (https://github.com/taftary/astrolith/issues/848#issuecomment-6288807184), approved in https://github.com/taftary/astrolith/issues/848#issuecomment-6289043992.

- [ ] C1: Window builds no longer force a backend: `WGPU_BACKEND` when set, otherwise the engine's automatic choice (AC1) (verify: contains window_builds_use_the_automatic_choice ... ok in cargo-test.log)
- [ ] C2: `WGPU_BACKEND=dx12` still starts the game on DirectX 12 (AC2) (verify: contains override_wins_on_windows ... ok in cargo-test.log)
- [ ] C3: On Vulkan the render thread still waits for the GPU each frame (AC3) (verify: contains only_vulkan_waits_for_the_gpu ... ok in cargo-test.log)
- [ ] C4: The repo's cargo config sets `VK_LOADER_LAYERS_DISABLE=~implicit~` (AC4) (verify: file .cargo/config.toml contains VK_LOADER_LAYERS_DISABLE)
- [ ] C5: A new ADR records the policy and notes the newer driver as untested (AC5) (verify: file docs/decisions/0042-automatic-graphics-backend.md contains untested)
- [ ] C6: stack.md, ARCHITECTURE.md and validator-capabilities.md match the new ADR (AC5) (verify: file docs/universes/stack.md contains 0042)
- [ ] C7: Everything else behaves as after #851, and the headless game check passes (AC6) (verify: contains VERIFY-OK in verify.log)

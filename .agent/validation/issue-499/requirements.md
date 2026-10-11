# Requirements — issue #499 (CONFIRMED)

Derived by the specification skill from approved Spec v1
(https://github.com/taftary/astrolith/issues/499#issuecomment-6108905446,
approval: https://github.com/taftary/astrolith/issues/499#issuecomment-6108909752).

Owner approval of the spec confirms the criteria text below. Probes are the
AI's addition; the owner never sees this syntax. The window runs (AC1-AC4
in a real window) are judged by the validator from its own bounded runs and
quoted in the verdict; the probes below cover the code and tests behind them.

## Layer 1 criteria (AC1-AC6)

- [ ] C1: On Windows, a normal window run uses DirectX 12: its log names `Dx12` as the graphics system (verify: contains windows_without_override_uses_dx12 ... ok in cargo-test.log)
- [ ] C2: On your PC, the log of a normal window run has no `ERROR` lines (verify: file crates/universe-app/src/main.rs contains .set(render_plugin())
- [ ] C3: On your PC, the log of a `--screens` run has no `ERROR` lines, it ends with `SCREENS-OK files=7`, and every picture still shows the screen its name says (verify: contains both_window_builds_use_this_render_plugin ... ok in cargo-test.log)
- [ ] C4: With `WGPU_BACKEND=vulkan` set, the window runs on Vulkan again (the developer override still works) (verify: contains override_wins_on_windows ... ok in cargo-test.log)
- [ ] C5: On Linux the default stays Vulkan (checked in the code and by its test; there's no Linux PC to run it on) (verify: contains other_platforms_keep_the_default ... ok in cargo-test.log)
- [ ] C6: `--verify`, `--capture` and the golden files are unchanged; the golden test passes (verify: contains VERIFY-OK in verify.log)

## Owner expectations

- [ ] C7: The picture stays the same: same screens, same colours, same layout (verify: file docs/decisions/0022-dx12-on-windows.md contains drew the screens correctly)
- [ ] C8: A developer who wants to test Vulkan can still choose it with the `WGPU_BACKEND` setting (verify: file docs/universes/stack.md contains `WGPU_BACKEND` (for example `vulkan`) always overrides)

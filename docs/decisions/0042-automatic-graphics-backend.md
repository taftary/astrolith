# 0042 — Automatic graphics backend

- Status: accepted
- Date: 2026-10-20
- Issue: #848 (Spec v2)
- Follows up: ADR 0041 (Vulkan on Windows, with frame pacing) and ADR 0022 (DirectX 12 on Windows)

## Context

ADR 0041 forced Vulkan on Windows, because DirectX 12 corrupts the engine's atmosphere tables on the dev PC (Intel UHD 620, driver 31.0.101.2130). The research on #848 points the other way:
- Bevy and wgpu recommend the automatic choice. Maintainers advise against hardcoding a backend, and wgpu ranks Vulkan ahead of DirectX 12 on a GPU that offers both.
- Shipped wgpu games keep the automatic choice and let a launcher or player override it.
- Older Intel chips like the dev PC's have weak Vulkan drivers, so some wgpu apps default those chips to DirectX 12.

The owner chose the automatic choice (Q1 B), no player-facing override (Q4 A), and hiding the Steam loader lines in cargo runs (Q3 A1). The driver update re-test was dropped from this Issue (Spec v2).

## Decision

- **Backend.** `gpu::backends_for` returns `WGPU_BACKEND` when it is set, otherwise the engine's default (every backend it supports), on every platform. There is no Windows special case.
  - On the dev PC the automatic choice is Vulkan: a run offering Vulkan, DirectX 12 and GL logged `backend: Vulkan`, and a default `--screens` run logs `backend: Vulkan`.
  - `WGPU_BACKEND=dx12` forces DirectX 12.
- **Frame pacing.** ADR 0041's frame pacing stays: on Vulkan the render thread waits for the GPU each frame.
- **Steam loader lines.** `.cargo/config.toml` sets two variables for every cargo-launched run:
  - `VK_IMPLICIT_LAYER_PATH`, pointing at the `.cargo` folder, which holds no layer files;
  - `VK_LOADER_LAYERS_DISABLE=~implicit~`.

  The disable rule alone does not hide the lines: the loader fails to open the registry's missing Steam layer files before any disable rule applies, so the search path is replaced too. Implicit layers (overlays such as Steam's or OBS's) do not load in these runs. The game cannot set these itself, because `std::env::set_var` is `unsafe` in edition 2024 and `E-NO-UNSAFE` forbids it.
- **No player choice** (Q4 A): `WGPU_BACKEND` is enough.

## Consequences

- On the dev PC nothing changes for the player: still Vulkan, still paced, with a correct sky.
- On another PC the engine may pick DirectX 12, where the dev PC showed a corrupted sky. That is the accepted trade-off of Q1 B, and `WGPU_BACKEND=vulkan` forces Vulkan there.
- The two accepted Vulkan swapchain validation lines still print in debug window runs. They come from a wgpu bug that is fixed in wgpu 30 (Bevy 0.20), not in the pinned wgpu 29.
- A future packaged game needs the two loader variables in its launcher to hide the Steam lines.
- The newer Intel driver for this chip (31.0.101.2145, September 2026) is untested: the re-test was dropped from #848 (Spec v2) and stays a later check.

# 0022 — DirectX 12 on Windows

- Status: proposed
- Date: 2026-10-11
- Issue: #499 (Spec v1 approved)

## Context

Bevy's default lets wgpu pick from all backends, and on Windows it picks
Vulkan. On the dev PC (Intel UHD Graphics 620, Windows 11) every window run
then logs `ERROR wgpu_hal::vulkan::instance` lines: Vulkan validation
messages (`VUID-vkAcquireNextImageKHR-semaphore-01286`,
`VUID-VkPresentInfoKHR-pImageIndices-01430`) from the debug-build checks,
plus loader messages from stale Steam Vulkan layer entries. The validator
fails any run whose log has an `ERROR` line, so window runs (normal play,
`--screens` from ADR 0021) can't be validated with a clean log. A probe
run with DirectX 12 logged no `ERROR` and no `WARN` line and drew the
screens correctly.

## Decision

- `universe-app` builds the `RenderPlugin` for both window builds in one
  place (`gpu.rs`), with all `WgpuSettings` at Bevy's defaults except the
  backends.
- Backends: the `WGPU_BACKEND` environment variable when set; otherwise
  DirectX 12 on Windows; otherwise Bevy's default (Vulkan on Linux).
- `--verify` and `--capture` open no window and are unchanged.

## Consequences

- Window runs on Windows log `backend: Dx12` and no graphics `ERROR` lines
  on the dev PC.
- No new Bevy feature and no pin change (`E-PIN-BEVY`).
- A developer can still test Vulkan with `WGPU_BACKEND=vulkan`, where the old
  lines are expected.
- A Windows PC without DirectX 12 support would need that override (very
  unlikely on Windows 10/11).

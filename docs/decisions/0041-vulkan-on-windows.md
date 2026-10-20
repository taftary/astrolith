# 0041 — Vulkan on Windows, with frame pacing

- Status: accepted; the forced Vulkan default is replaced by the automatic choice in ADR 0042 (#848), and the frame pacing stays
- Date: 2026-10-20
- Issue: #780 (F3 A)
- Follows up: ADR 0022 (DirectX 12 on Windows)

## Context

ADR 0022 chose DirectX 12 on Windows because Vulkan logs validation and loader `ERROR` lines on every window run on the dev PC (Intel UHD 620).

With the planet's atmosphere (#780, ADR 0040), DirectX 12 corrupts the engine's atmosphere lookup tables on that PC:
- white, black and magenta bands across the sky and the haze;
- in about half the runs at 1280x720, and in almost every view at 1600x900, orbit included.

The corruption also shows with the engine's own Earth medium and with shadows off, and with both DirectX 12 shader compilers. It never showed on Vulkan.

Vulkan had a second problem on that PC: the CPU queued frames far ahead of a slower GPU, so while the ground streamed in on arrival the picture lagged about 50 seconds behind.

## Decision

- Windows uses Vulkan (owner answer F3 A). `WGPU_BACKEND` still overrides it, so DirectX 12 stays a manual fallback. Other platforms keep Bevy's default.
- The owner accepts Vulkan's known lines as harmless:
  - `VUID-vkAcquireNextImageKHR-semaphore-01286`;
  - `VUID-VkPresentInfoKHR-pImageIndices-01430`;
  - the stale Steam layer loader messages.

  The headless `--verify` and the validator's checks open no window and log none of them.
- `gpu::FramePacingPlugin`: on Vulkan the picture never runs ahead of the GPU. DirectX 12 and the other backends pace themselves and are left alone. (#819 replaced the wait at the end of each frame, `RenderSystems::Cleanup`, with two frames in flight: before a frame takes its window image it waits for the frame before the previous one, so the GPU draws while the CPU prepares the next frame: ADR 0044.)

## Consequences

- The atmosphere draws correctly on the dev PC.
- On the dev PC, frame time on Vulkan is the full CPU-plus-GPU time, about 95-135 ms at 1600x900 at the planet (ADR 0040, Frame time). That is slower than DirectX 12's 55-85 ms, which overlaps CPU and GPU work but draws a corrupted sky.
- Window runs log the accepted Vulkan lines; they are not failures.
- ADR 0022's reason (a clean window log) gives way to a correct picture.

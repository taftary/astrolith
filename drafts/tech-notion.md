# Game Technical Specification

**Targets:** Windows, macOS, Linux, Android, iOS, consoles (to be defined)
**Priorities:** performance, full control, AI-driven development

## Stack

- **Language:** Rust stable, edition 2024, toolchain pinned in the repo.
- **Engine:** Bevy 0.19, pinned to an exact version and upgraded deliberately.
- **Features:** enable only the Bevy features the game uses.
- **Dependencies:** each crate is added only after confirming it supports the pinned Bevy version.

## Architecture

- Cargo workspace with three crates: core logic, rendering, and app entry point.
- Core logic has no rendering dependency, so it runs and tests headless.
- One plugin per feature.
- Content and balance are data-driven, in text-based files.
- Simulation runs on a fixed timestep.

## Platforms

- Mobile builds use community tooling (cargo-mobile2 or equivalent). Compatibility with Bevy 0.19 is unverified.
- Development and testing happen on desktop until the MVP is ready.
- Real-device testing on Android and iOS starts once the MVP is complete.
- Design constraints applied from the start, verified on devices at MVP: app suspend/resume handling, touch input, safe-area layout.
- iOS builds require macOS and Xcode.
- Console targets are in scope. Bevy has no official console support, so each port is custom work.
- Console prerequisites: platform-holder developer program access and SDKs (typically under NDA), plus dev kits for on-device testing.
- Input goes through one action-mapping layer shared by touch, keyboard, and gamepad.

## Performance

- Profile on desktop during development; profile on the weakest supported phone once the MVP is ready.
- No per-frame allocations in hot systems.
- Use change detection to skip unchanged work.
- Compress textures (KTX2/Basis).
- Set draw-call and texture-memory budgets before building content.
- Release profile uses LTO and a single codegen unit.
- Optimize dependencies in dev builds.

## Tooling

- **Profiling:** Tracy, Android Studio profiler, Xcode Instruments.
- **Quality:** rustfmt, clippy, cargo nextest.
- **CI:** on every commit, build and test desktop; build Android and iOS (iOS on macOS runners); build consoles on self-hosted runners once SDK access is secured.
- **Dev speed:** fast linker (mold/lld), dynamic linking in dev builds only.

## AI-Driven Development Rules

- A `CLAUDE.md` at the repo root states the pinned versions, architecture rules, and conventions.
- Agents read the real crate source or generated docs instead of guessing Bevy APIs, since model knowledge of Bevy is often outdated.
- Every change passes `cargo check` and `clippy`.
- Features ship with headless tests.

## Open Decisions

- [ ] 2D or 3D
- [ ] Single-player or multiplayer (and networking crate)
- [ ] Physics crate
- [ ] Input crate
- [ ] Which consoles, and who obtains platform SDK access
- [ ] Minimum supported device spec
- [ ] Target frame rate

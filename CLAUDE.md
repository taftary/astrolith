# CLAUDE.md — universe workspace conventions

Applies to the `universe-*` crates and `docs/universe/`.
Parent Issue #35 (spec v4 approved). Sub-issues #37–#43.

## Pins

- Rust stable `1.97.1` exact (`rust-toolchain.toml`), edition 2024.
- Bevy exact `=0.19.1` (newest 0.19 patch, not 0.20-rc),
  `default-features = false`, features listed in the root `Cargo.toml`
  and justified in `docs/universe/stack.md`.
- Full tree in root `Cargo.lock`; CI and local builds use `--locked`.

## Architecture rules

- `universe-core`: pure logic, no Bevy, no render deps, headless-testable.
- `universe-render`: Bevy indicators only (grids, vectors, points), no meshes.
- `universe-app`: window plus `--verify` headless check (replays the journey L1-L11, prints PASS per level, exits 0 on VERIFY-OK).
- Allowed dependency direction: core <- render <- app.
- One plugin per feature; DRY; SOLID; public items documented.
- Frozen docs: `docs/universe/ladder.md` (M1) changes only with owner approval.

## Conventions

- Read crate source and docs before using an API; never guess Bevy APIs.
  Pinned sources of truth: the `bevy 0.19.1` manifest `[features]` table
  and `bevy_internal 0.19.1` (`src/default_plugins.rs`, `src/lib.rs`,
  `src/prelude.rs`).
- Every change passes `cargo check --locked --workspace` and
  `cargo clippy --locked --workspace` with zero warnings.
- Every feature ships headless tests; `cargo test --locked --workspace`
  stays green.
- Markdown: no trailing whitespace, no tabs, exactly one trailing newline
  (enforced by the CI markdown lint).
- Do not touch `README.md`.
- Changes to workflow files, skills, agents, and board docs follow
  `docs/workflow.md` (Sync and Governance), not this file. This file governs
  only the `universe-*` crates and `docs/universe/`.

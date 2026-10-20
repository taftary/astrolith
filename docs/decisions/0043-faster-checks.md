# 0043 — Faster checks: test tiers, a validator that reuses CI, screenshot groups

- Status: accepted
- Date: 2026-10-20
- Issue: #847 (Spec v1)

## Context

Around every change the waits had grown long on the dev PC (Intel UHD 620): about 9 minutes per validation and about 8 minutes per screenshot run. Plain local `cargo test` took over two minutes. The owner asked for faster screenshots, tests and validator, as fast as we can (Q1 A, Q2 C, Q3 B, Q4).

## Decision

- **Test tiers (Q2 C, rule `E-TEST-TIER`).**
  - The 18 tests over about 10 s carry `#[ignore = "slow ..."]`: plain `cargo test` skips them.
  - CI runs `cargo test --locked --workspace -- --include-ignored`, so every test still runs on every push.
- **Faster slow tests (Q2 C).**
  - The ground and LOD tests build their patches on every core (the headless `par_chunks`), through a per-binary cache for the one relief they share. The cull test builds its 384 patches once for both views.
  - The assertions are unchanged.
- **Validator reuses CI (Q3 B).**
  - `validate.py` takes the workspace test output from CI's passed `ci` run on the exact head commit. It finds the step by name or by its `##[group]Run cargo test` marker.
  - Colour codes and cargo's build progress are dropped before the error scan.
  - With no passed run for that commit, it runs the tests itself, slow tier included.
  - The headless `--verify` and its repeat start at once, side by side with the tests.
- **Screenshot groups (Q1 A).**
  - `--screens <dir> <groups>` takes a comma list of `smoke`, `menus`, `planet`, `sky` and `walking`. With no list, every shot runs.
  - The visual-validation skill says which groups a change runs.
  - Planet, sky and walking shots are taken once the drawn ground is built and the body has landed, after at least 60 frames, instead of after their full settle time (kept as the cap).
  - The usage line becomes `usage: universe-app [--verify] [--capture <dir>] [--screens <dir> [groups]]`.

## Times

Measured on the dev PC. Goals (Q4): a validation run under 180 s and a screenshot run under 2 min.

| Check | Before | After |
|---|---|---|
| Validation run (`validate.py`, warm build) | 535 s | 164 s |
| Full screenshot run (26 shots, Vulkan) | about 480 s | 155 s |
| Smoke screenshot run (4 shots) | — | 61 s |
| Plain local `cargo test` (warm build) | about 130 s | 15 s |
| `ground_is_the_drawn_surface`, run alone | 47.8 s | 17.9 s |
| `local_frame_switch_moves_under_a_millimetre`, run alone | 47.8 s | 17.8 s |
| `hidden_patches_have_no_visible_point`, run alone | 11.6 s | 2.0 s |
| `tree_reaches_the_deepest_level_near_the_ground`, run alone | 12.2 s | 5.6 s |
| The whole slow tier, all at once (CI) | about 126 s | about 127 s |

- The validation goal is met (164 s < 180 s).
- The smoke set meets the screenshot goal (61 s < 120 s); a full run (155 s) does not.
- Run together, the slow tier stays CPU-bound on four cores, so CI's test step is no shorter. Run alone, those tests are 2.2 to 5.8 times faster.
- A cold build after a code change still adds several minutes to any check.

## Consequences

- A local `cargo test` no longer runs the slow tier. A break there shows first in CI, on the same push, and the validator reads that same CI run.
- The validator's verdict names its source: `cargo test from CI run <id>`, or `run here`.
- A screenshot run that skips groups writes fewer pictures, and its `screens.txt` lists only those.

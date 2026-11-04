# #1085 T6 (#1185): adaptive frames in flight

**Built:** commit d852224.
- **The part:** prototype part `adaptive` (`crates/universe-app/src/adaptive.rs`), a pure controller with 8 unit tests.
- **Behaviour:** VSync off keeps two frames in flight. VSync on starts at two and tries one for about 45 frames every 20 s, keeping the faster by at least 3%. Each change prints `ADAPTIVE limit=N`.
- **Checks:** clippy clean; 46 + 1 + 285 tests pass; `VERIFY-OK`. `ASTROLITH_PROTOTYPE=adaptive+bogus` is refused.
- **Executable:** release SHA-256 `04c0ab17b37594212ccdceb19f4e1e63d27010c994c05c4dfd75ab7ca8c6bf0e`.

**Done criteria:**
- Met: `rank-low-adaptive/` completed, with `prototype: adaptive` and `settings_unchanged: true` in its manifest.
- Met: `normal-launch-baseline/` printed no prototype line and left settings unchanged. It is partial by design: the 60 s timeout ends a normal launch.

## Result: it does not work

| run | what | result (mean frame interval, ms) |
|---|---|---|
| `rank-low-adaptive/` | Low, VSync on, all 8 scenes | every scene 39.79-40.05 (T4 `inflight1`: ground 24.99, turns 23.87); 90 limit changes |
| `explore/nudge-*` | Normal ground (T3 fixture), plus a one-off 8/16/24 ms delay when switching to one | ground 39.30-39.33 in all four; 14 changes each |
| `explore/inflight1-check/` | `inflight1` from the start, same exe | warm-up 26.89, ground 28.72 |
| `explore/start1-ground/` | `adaptive` starting at one (exploration aid) | warm-up 26.18, then ground 36.68 |
| `explore/start1-rank-normal/` | the same, Normal, 8 scenes | turns, ground, ground-night and world-two 39.75-39.83 (T4: baseline 24.96-33.14, `inflight1` 28.41-37.10); 89 changes |

**Reading:**
- The VSync-on 40 ms lock is **one-way**. It forms easily: at start-up with two frames in flight, or during a probe at two.
- **Changing the limit afterwards does not break it.** In the 10-frame averages of `frames.txt`, the one-frame probes show no change in wait at all (about 34 ms). In the lock, the GPU has already finished by the time the frame waits, so the wait is for the screen. One frame in flight only helps by never letting the lock form.
- A one-off delay to shift the frame's timing did not break it either.
- So no controller that switches between one and two can reach the `inflight1` numbers. Starting at one and probing two is worse than both fixed settings.

The exploration aids (start limit, delay) were environment variables, kept only as `explore/exploration-aids.diff` and not committed to code.

**Consequence:** the agreed bounded experiment (A) fails before the full T7 comparison. The closest working form, `inflight1` only with VSync on, is already measured: it is the VSync-on rows of `../t4/README.md`, where the rollout numbers are not met.

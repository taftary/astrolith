# #1085 T3 (#1150): frame-delivery prototype switch

**What T3 built** (commit 28f28da, on `task/1150-frame-delivery-prototype`):

- **The switch:** `ASTROLITH_PROTOTYPE=<name>`, or a route file's `prototype <name>` line (`crates/universe-app/src/prototype.rs`).
- **Its parts**, joined with `+`:
  - `latency1-3`: the window's frame latency hint;
  - `inflight1-3`: frames in flight before the #819 pacing waits;
  - `nopace`: no pacing wait.
- **Errors:** unknown, repeated or out-of-range parts are refused. A route line and the environment that disagree end the run before the window opens.
- **Baseline:** no switch is exactly the #819 pacing (two frames in flight, Bevy's default latency).
- **Records:** the run line (`ROUTE-RUN` / `TIMING-RUN`) ends with `prototype=<name>`, and `profile_run.py` records `prototype_requested` (environment) and `prototype` (what the run reported) in `manifest.json`.
- **Not a setting:** the switch is never written to `settings.toml`.

**Build:** release `universe-app.exe` SHA-256 `e5447043bd1ff0f2607c7a20f5a90996f7f6d860b5a7127acc3572be7bdcdc77` (in every manifest here). Same PC and conditions as T2: Intel UHD 620, Vulkan, 1600x900, 60 Hz, Balanced on AC.

**Checks:**

- `cargo clippy --workspace --all-targets -D warnings` is clean.
- `cargo test -p universe-app -p universe-render`: 37 + 1 + 285 tests pass.
- Python tests `test_profile_process` and `test_profile_report`: 16 pass.
- `--verify`: `VERIFY-OK`.
- `ASTROLITH_PROTOTYPE=bogus` exits 2 with "unknown prototype part 'bogus'".

**Present modes on this driver** (`vulkaninfo`): only `FIFO` and `IMMEDIATE`. There is no mailbox and no relaxed FIFO, so a tear-free present-mode change is not available. VSync on means FIFO.

## Exploration: ground, Normal, VSync on (`explore/<variant>/`)

Fixture `explore-ground-capped.route` (the T2 `tg-capped` fixture): 30 s warm-up plus 120 s ground. One run per variant, in this order, GPU timers off. Ground row of each `summary.json`, ms:

| variant | manifest prototype | interval | p95 | worst | main | render | wait | mode | >100ms | settings unchanged |
|---|---|---:|---:|---:|---:|---:|---:|---|---:|---|
| none | none | 39.86 | 48.96 | 52.52 | 11.52 | 4.57 | 34.38 | slow | 0 | True |
| latency1 | latency1 | 39.87 | 48.90 | 51.97 | 11.54 | 5.11 | 33.70 | slow | 0 | True |
| latency3 | latency3 | 39.87 | 48.85 | 52.89 | 11.56 | 5.16 | 33.64 | slow | 0 | True |
| nopace | nopace | 39.86 | 48.94 | 54.11 | 11.65 | 5.24 | 33.65 | slow | 0 | True |
| inflight1 | inflight1 | 27.07 | 28.84 | 35.97 | 9.67 | 3.81 | 22.38 | slow | 0 | True |
| inflight3 | inflight3 | 39.88 | 48.96 | 53.83 | 11.49 | 5.07 | 33.75 | slow | 0 | True |
| latency3_inflight3 | latency3+inflight3 | 39.87 | 48.95 | 54.33 | 11.52 | 5.07 | 33.74 | slow | 0 | True |
| latency3_nopace | latency3+nopace | 39.88 | 49.00 | 56.08 | 11.69 | 5.21 | 33.71 | slow | 0 | True |

Ground frame intervals in 4.17 ms bins (share of frames), from `frames.txt`:

| variant | frames | 25-29 | 29-33 | 33-38 | 42-46 | 46-50 | 50-54 |
|---|---:|---:|---:|---:|---:|---:|---:|
| none | 3010 | | 45% | 4% | 8% | 40% | 1% |
| inflight1 | 4432 | 97% | 2% | | | | |

### Reading the exploration

**One frame in flight is the only variant that changes delivery.**

- Ground with VSync on goes from frames alternating between two and three refreshes (about 32 and 48 ms) to a steady 25-29 ms.
- The latency hint changes nothing, so the driver does not honour it.
- More frames in flight, or no pacing at all, behave like the baseline.

**ADR 0044 tried this before.** It tried one frame in flight ("Wait for the GPU just before submitting"), and saw "orbit and walking faster". It dropped the idea only because a screenshot run timed out. That cause was found and fixed separately (`max_builds`) in the same ADR.

## Full route with `inflight1` (`full-inflight1-normal/`)

The T2 `rank-normal.route` (Normal, VSync on, eight scenarios), one run. Its manifest has `prototype: "inflight1"`, `settings_unchanged: true` and `complete: true`. Compared against the T2 baseline (median of `t2/rank-normal-r1..r3`), ms:

| scenario | baseline interval | inflight1 | change | baseline p95 | inflight1 p95 | inflight1 main | inflight1 wait | mode |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| universe | 39.92 | 30.49 | -23.6% | 50.53 | 47.08 | 9.88 | 24.42 | slow |
| system | 40.06 | 40.04 | -0.0% | 51.08 | 49.56 | 9.94 | 30.34 | slow |
| orbit | 28.91 | 33.55 | +16.0% | 32.71 | 37.72 | 7.60 | 27.05 | slow |
| descent | 35.17 | 39.12 | +11.2% | 42.63 | 47.87 | 11.14 | 33.67 | slow |
| turns | 28.64 | 30.32 | +5.9% | 46.52 | 31.74 | 10.98 | 26.47 | slow |
| ground | 39.84 | 28.75 | -27.8% | 48.85 | 29.85 | 9.34 | 24.65 | slow |
| ground-night | 39.87 | 28.16 | -29.4% | 48.87 | 29.20 | 9.33 | 24.08 | slow |
| world-two | 39.76 | 36.66 | -7.8% | 48.39 | 39.39 | 10.67 | 31.57 | slow |

This is one run against three, so it is a direction, not a classification. T4 does that with alternating pairs.

- **Faster:** the ground scenes and universe.
- **Slower:** orbit and descent, which are among the heavier GPU scenes (T2: opaque pass 7.8 and 17.5 ms). One frame in flight lets the CPU and GPU overlap less.
- **Turns:** mean +5.9%, but p95 is much better (46.5 → 31.7 ms).

## Normal launch without the switch (`normal-launch-baseline/`)

`profile_run.py --normal` with a disposable profile and no `ASTROLITH_PROTOTYPE`. A normal launch only ends when a person closes the window, so the 60 s timeout ends it and the run is marked partial (`complete: false`, `process: timeout`), as `README-profile.md` describes.

- The window opened on the UHD 620 (`stderr.txt`).
- No "prototype … in force" line was printed: the switch is unset, so the pacing is the #819 baseline.
- `settings_unchanged: true`.

The real-window check with disposable settings (restart, Reset, no prototype persisted) is T5's.

## Choice for T4

**`inflight1`** (one frame in flight on Vulkan). It is the only variant that changes VSync-on delivery. It keeps everything drawn and every player setting.

T4 compares it against the baseline in alternating pairs, per scenario. Orbit and descent are expected to be the regressions to judge, and they are reported as they come.

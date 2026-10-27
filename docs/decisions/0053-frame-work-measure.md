# 0053 - Measuring a frame's work apart from its waits, and route runs

- Status: accepted
- Date: 2026-10-27
- Issue: #1022 (Spec v3, tasks T1 #1058 and T2 #1059)

## Context

Adaptive quality (#1022) changes the picture only when the frame's work is too much for the target, never because the frame waited (Spec v3 R1: "exclude intentional cap/VSync waits"; "do not add overlapping CPU/GPU times together"). The timing runs (#819, ADR 0044) knew only the frame interval. The R4 runs need a 1600 x 900 window outside the fixed timing shots, and a DPI case, and the preflight found that a timing file's header had been written by hand with the wrong size (#998 said 1280 x 720; `--timing` opens 1600 x 900).

While building the measure, the first runs showed the main world taking the whole 40 ms frame in space views. A probe placed the time inside the fixed-step loop, which holds no systems of ours. The pool sizes explained it: the dev PC's eight threads gave one to file I/O, six (three quarters) to the ground builds (#918 T8), and one to the frame. The render world's blocking wait for the window image held that one frame thread, so the main world's systems queued behind it. With two frame threads the main schedules took about 7 ms in the same views.

## Decision

- **Work without waits.** `frame_work::FrameWorkPlugin`, part of `UniverseRenderPlugin`, measures every frame:
  - the main world from `First` to `Last`;
  - the render schedule from `ExtractCommands` to `PostCleanup`, minus a wait region. The region opens at `FrameWorkSet::WaitBegins`, which the Vulkan pacing wait (#819) now follows, and closes after `prepare_windows`, which takes the window image.
  The work is the longer of the two threads, never their sum. The render thread hands its numbers to the main world through atomics, and no allocation is made per frame.
- **Two threads for the frame.** `gpu::task_pool_plugin` caps the ground build threads at the cores minus file I/O minus `FRAME_THREADS` (2). On the dev PC that is five build threads instead of six. `--build-workers <n>` still sets an exact count.
- **Window options.** Trailing `--size <w>x<h>` and `--scale <f>` join `--build-workers <n>` in any order:
  - `--size` sets the physical size of the plain window or of a `--timing` run, whose default stays 1600 x 900. A plain `--screens` run refuses it, since its shots set their own sizes.
  - `--scale` overrides the desktop scale factor.
- **Self-describing timing files.** `timing.txt` opens with a `TIMING-RUN` line, read from the window and the adapter: size, scale, adapter, backend, present mode, build threads and timer. Each `TIMING` line adds `work_mean_ms`, `work_p95_ms`, `main_mean_ms`, `render_mean_ms` and `wait_mean_ms`.

- **Route runs (T2).** `--route <file> <dir>` drives a real window along a route file and records every frame.
  - A route is shot lines (#863) plus `for=<seconds>` and `scenario=<name>`. The camera moves from each leg's pose toward the next leg's in real time: heights by a constant factor, the sky's time along with it. A `journey` leg flies the autopilot from L1. `target <fps>` and `set <key> = <value>` lines set the judged rate and the run's settings.
  - It writes `frames.txt` (one line per frame, timed in real time so a stall over 250 ms is not cut short) and `route.txt`:
    - the `ROUTE-RUN` line;
    - `ROUTE-LAUNCH`;
    - a `ROUTE-WARMUP` line, one `ROUTE-SCENARIO` per scenario, `ROUTE-ALL`, `ROUTE-FIRST5` and `ROUTE-FINAL5`, each with the share within 110 % of the target, p95, worst, frames over 100 ms and work p95;
    - one `ROUTE-LEG` per planet leg, with the ring and readiness times.
  - It prints them and `ROUTE-OK`, or `ROUTE-FAIL <reason>`. Process memory is sampled from outside the run, since the game reads no OS memory counters (no new dependency).

## Rules amended

- ADR 0044 and #918 T8: the build pool's three quarters stop two threads short of all the cores.
- `E-STDOUT-PROTOCOL`: the usage line names `--size <w>x<h>`, `--scale <f>` and `--route <file> <dir>`, and `ROUTE-OK` / `ROUTE-FAIL` join the protocol lines. The `TIMING` line format grows; it is a measuring aid, not a golden file.

## Consequences

- The quality controller (#1022 T5) can tell work from waiting, and a pacing wait is never read as load.
- The ground builds lose one thread on the dev PC. Ground readiness is measured in #1022 T3 against both counts (`--build-workers 6` keeps the old one).
- On the dev PC the space views do about 4 ms of work and wait about 35 ms at vsync (about 10 ms frames with vsync off). The 40 ms there is a pacing limit, which the frame cap (#1022 T7) addresses. The ground does about 37 ms of work, which the ground tiers address (T8, T9). Evidence is in `.agent/validation/issue-1022/t1/`.
- The present call stays inside the render work. It does not block on this Vulkan driver, but another driver might block there; the GPU pass timers (`ASTROLITH_GPU_TIMERS=1`) remain the cross-check.

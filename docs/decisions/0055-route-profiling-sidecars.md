# ADR 0055: Opt-in route profiling sidecars

- Status: accepted
- Issue: #1087, approved Spec v1

## Context

The #1085 experiment requires process-memory samples and live resource/quality
observations in addition to existing route interval, CPU and readiness evidence.
The normal application must retain its behavior and existing machine protocol.

## Decision

`scripts/validation/profile_run.py` launches the release app directly without a
shell, observes that PID's Windows working set once per second, and records a
versioned manifest and partial process output. It redirects only the child's
configuration environment and refuses existing evidence directories. An OS file
lock prevents two instances of this runner from measuring at once; unrelated
applications still need to be controlled when benchmarking.

`ASTROLITH_PROFILE=1` opts a **route run** into extra telemetry. Normal startup
does not inspect this switch or register the collector. `telemetry-v1.csv` is a
streaming sidecar, flushed once per sample. Existing `frames.txt`, `route.txt`,
`ROUTE-OK` and `ROUTE-FAIL` semantics stay unchanged. Samples occur once per
second, on leg changes, and at route completion; cadence gaps remain visible.

Resource counts identify the main-world Mesh, Image, StandardMaterial and
PatchGridMaterial asset stores. Missing stores are `none`, distinct from empty
stores. Render-world-only data can leave main-world stores after extraction;
these counts are neither GPU allocation counts nor proof of GPU memory release.
Ground accounting uses PatchCache's existing byte estimate and PatchTasks' cap;
neither is total process/device memory. Ground readiness is not applicable
outside L10. A missing per-leg completion remains `none`, not zero latency.

The cap-wait observation is the last completed CPU sample at that observation,
not a GPU timer or cumulative cap time. Main and render CPU work overlap; route
work is their maximum. Presentation work not separately observable remains in
render work and must not be attributed to GPU execution.

## Consequences

Instrumentation adds measurable cost and uses allocations only at observation
cadence. Collection-on/off comparisons report this cost rather than claiming
zero overhead. A one-second working-set peak can miss short-lived allocations.
Any unsupported/missing required observation makes comparison incomplete.
No player settings, in-game guides, engine pins or headless goldens change.

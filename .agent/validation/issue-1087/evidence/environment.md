# Reference desktop observations

Observed by the main agent on 2026-10-29 before the full-duration evidence runs:

- `Get-CimInstance Win32_VideoController`: Intel(R) UHD Graphics 620,
  driver 31.0.101.2130, desktop 1920×1080, reported refresh 60 Hz.
- `powercfg /getactivescheme`: 381b4222-f694-41f0-9685-ff5bb260df2e,
  Windows Balanced (localized output: Utilisation normale).
- `Get-CimInstance Win32_Processor`: Intel(R) Core(TM) i5-8350U CPU @ 1.70GHz,
  four cores, eight logical processors.
- `Get-CimInstance Win32_Battery`: BatteryStatus 2 (AC present; no discharge),
  EstimatedChargeRemaining 100%. This is a point observation, not a continuous
  power log. Actual package power and thermal throttling remain unobserved.
- Measurement windows request 1600×900; per-sample output and 3D camera target
  dimensions are in `telemetry-v1.csv`. Desktop dimensions are not app dimensions.
- Release build completed successfully with
  `cargo build --locked --release -p universe-app` in 6m45s. Its renderer sources
  correspond to commit `4aa0b44`; build occurred before that commit was recorded.
  The executable SHA-256 is recorded separately in each manifest. The dirty
  source patch at build time is retained in `telemetry-smoke/source-changes.patch`.
- These runs prove tooling capability, not a baseline/prototype improvement.
  Missing environment conditions must not be inferred as matched.

The smoke run completed with actual mesh/image/standard/custom-ground material
counts, terrain bytes/cap, readiness, quality, world identity and dimensions.
Full short, sustained, transition, overhead and visual evidence is recorded
separately as it completes. Interactive normal-launch save/restart/Reset remains
pending owner assistance because no desktop-control tool is exposed in this
session.

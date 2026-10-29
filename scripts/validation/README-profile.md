# Desktop route measurements (#1087)

Build the intended commit with `cargo build --locked --release -p universe-app`.
Record that build command and commit with the evidence. The runner records the
executable SHA-256 and timestamp; the checkout SHA alone does not prove which
source built an existing executable.

```powershell
python scripts/validation/profile_run.py --route .agent/validation/issue-1087/runner-smoke.route --out .agent/validation/issue-1087/evidence/my-new-run --timeout 90
```

The smoke fixture is only seven seconds. It establishes instrumentation access,
not the full #1085 performance protocol. Use approved full-duration fixtures for
acceptance. Evidence directories must be new; existing files are never replaced.

## Isolation and lifetime

The runner invokes only `target/release/universe-app.exe`, without a shell. It
copies the requested route into the evidence directory, gives the child a local
APPDATA root, observes its actual PID, and waits for automatic route completion.
The route itself uses in-memory settings. Real settings are checked by hash and
existence only; their contents are not published. An OS lock prevents concurrent
instances of this runner. Other apps and power/thermal changes still matter.

`--timeout` defaults to 1500 seconds; failures and timeouts return nonzero.
Partial stdout, stderr and memory samples remain. Only the owned child is
terminated/reaped. This is not a general subprocess wrapper or a permission
workaround. Validator use relies on the repository's existing approved script
permission and the separately proven authorized app launch.

## Files and units

- `manifest.json`: version, UTC start, executable/fixture identities, checkout
  state, explicit invocation, environment observations, settings isolation and
  completion status. Unknown refresh/power facts prevent a matched claim; supply
  `--refresh-hz` and `--power-note` only from actual observations.
- `process.json`: PID, elapsed monotonic seconds, exit/timeout/sampling/cleanup
  result and observed memory peak. A run with missing required samples fails.
- `memory.jsonl`: actual elapsed monotonic seconds, PID and working-set **bytes**
  at approximately one-second cadence, starting immediately after launch.
  Gaps above 1.5 seconds are recorded. Short-lived peaks between samples can be
  missed. This is Windows process-resident memory, not total GPU/device memory.
- `stdout.txt`, `stderr.txt`: unmodified app output retained even on failure.
- `route/frames.txt`: existing timestamp, scenario, interval, work, main, render,
  combined wait. Time is seconds; measurements are milliseconds. Work is the
  maximum of overlapping main/render work, never their sum. These CPU measures
  are not GPU execution. Present cost remains inside render work if unresolved.
- `route/route.txt`: actual adapter/configuration and legacy readiness summaries.
  `none` readiness means not completed during the leg; no zero latency is implied.
- `summary.json`: recomputed mean/p95/worst and stalls above 100 ms, separate
  warm-up/scenarios/first-and-final-five-minute summaries and incomplete loads.
  Interval coverage sums measured intervals; it is not physical scan-out timing.
- `route/telemetry-v1.csv`: opt-in live main-world stores, ground accounting,
  quality, camera target/output dimensions, world identity and last cap sleep.
  `none` is unavailable/not applicable, never an invented zero. Samples are
  approximately once per second, with extra leg-change and final observations.
  A camera target may be unavailable before the camera system first updates.

`--collection off` disables both working-set sampling and the new in-app
collector. Base route timing continues unchanged, so on/off overhead pairs can
measure the extra instrumentation. Ordinary game startup does not register the
new collector, even if the profiling environment variable is set.

## Asset-count limitations

Counts cover Mesh, Image, StandardMaterial and PatchGridMaterial main-world asset
stores separately. Render-world-only assets may be extracted out of these stores;
the counts do not cover all GPU allocations or prove their release. Ground byte
estimates include retained CPU mesh data and shape tiles under the existing
budget, not all graphics memory. Compare identical scenes and documented cache
bounds over repeated transitions. Do not hide monotonically increasing counts.

## Independent execution on OpenCode/PowerShell

The legacy shell scanner in OpenCode v2.0.18 misparsed an unquoted route command.
The validator independently proved this invocation with unchanged permissions:

```powershell
cargo run '--locked' '--release' '-p' 'universe-app' '--' '--route' '.agent/validation/issue-1022/t12/short-native.route' 'C:/Users/PERSO/AppData/Local/Temp/opencode/preflight-1087-native' '--size' '1600x900' '--build-workers' '5'
```

The quoted arguments are the same argv; no alternate executable, wrapper or
permission expansion is involved. New output directories are needed for new
evidence. See `docs/validator-capabilities.md` for the actual proof and limits.

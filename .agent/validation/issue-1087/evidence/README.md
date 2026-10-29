# #1087 capability evidence

These are tooling proof runs of the existing renderer. They are not a
baseline/prototype experiment, an optimization recommendation or an independent
validator verdict. Environment observations are in `environment.md`; each run
retains executable/fixture hashes, raw app output and memory samples.

## Completed short runs

Both use 30 seconds warm-up followed by 120 seconds measured at a fixed ground
view. Warm-up frames are excluded from the figures below.

| Run | Scored frames | Interval mean / p95 / worst (ms) | >100 ms | Observed process peak (bytes) |
| --- | ---: | --- | ---: | ---: |
| `short-normal` | 4530 | 26.49 / 34.01 / 53.13 | 0 | 866131968 |
| `short-minimum` | 4684 | 25.62 / 30.15 / 57.53 | 0 | 870055936 |

Raw summed interval coverage is 119.99421 and 120.00090 seconds respectively;
the route duration itself is fixed at 150 seconds including warm-up. Differences
of a fraction of a frame reflect boundary accounting and raw timing rounding.
Both completed successfully, with unchanged real player-settings hashes.

The minimum run's settled target is 1280×720 at 0.8 scale inside the 1600×900
output; terrain and markers are Low, atmosphere Normal, shadows/bloom off.
Its initial target-size sample still reflects camera setup before the scale
change. Neither the initial sample nor warm-up is used as scored resolution.
Live counts include custom ground materials, not only StandardMaterial.

Each is one run, without repeated alternating configuration trials or complete
thermal observations. The numbers therefore do not establish an improvement.
`resources.json` was audited from the raw telemetry after completion; generating
that report does not modify the original samples or process manifest.

## Transitions and sustained run

`transitions` completed all ten world/quality changes in one process after
30 seconds warm-up. All ten reached full-ring and selected-detail readiness.
Settled counts repeated by world: planet 0 used 6 meshes, 13 images, 1 standard
material and 32 ground materials; planet 1 used 8 meshes, 14 images, 2 standard
materials and 32 ground materials. Targets alternated 1600×900 and 1280×720.
After the first return, planet 0's terrain estimate stabilized at 7,922,520 bytes;
planet 1 repeated 9,231,360 bytes, both below the 67,108,864-byte terrain cap.
This is bounded observed main-world retention over these ten transitions,
not proof about all GPU allocations or indefinite runtimes. There were 10 scored
stalls over 100 ms (worst 258.93 ms); none is omitted from the raw evidence.

`sustained-normal` ran for 1261.062 process seconds, capturing 60 seconds warm-up
and all ten 120-second scenario blocks. Scored interval coverage was
1200.00572 seconds across 32,593 frames. Interval mean/p95/worst was
36.82/48.85/342.81 ms, with 10 stalls above 100 ms. The one-second process sampler
recorded 1,256 samples, peak 911,769,600 bytes and no gaps over 1.5 seconds.
The asset collector recorded 1,242 samples with no gaps over 1.5 seconds.
All reported planet-leg readiness observations completed. Terrain accounting
peaked at 35,504,040 bytes within its 67,108,864-byte cap. StandardMaterial count
temporarily reached 146 during navigation and returned to 2; ground material
count reached 32 and remained bounded in these observations.

The second orbit/descent blocks were slower than the first (orbit p95
37.57 → 47.16 ms, descent 47.95 → 57.46 ms). The fixture changes clock time
between repetitions, and thermals were not measured: do not attribute the
difference solely to heating or a leak. It remains visible for later investigation.

## Disposable normal startup (main agent, real window)

Four normal launches through `profile_run.py --normal`, all with
`--profile disposable-settings`, `--width 1000 --height 560` (1500×840 physical
at 150% display scale). Input was real OS mouse clicks (user32 SetCursorPos /
mouse_event), and each app was closed with a window close message (WM_CLOSE).
For screen capture only, these launches used `WGPU_BACKEND=dx12` and
`WGPU_DX12_COMPILER=dxc`, with the Windows Kits x64 folder on PATH. GDI capture
shows a Vulkan window as blank. The settings path does not depend on the backend.

| Launch | Action in the real UI | Disposable `settings.toml` afterwards |
| --- | --- | --- |
| `settings-1` | none; created defaults | `target_fps = 60` |
| `settings-2` | Settings > Performance: Target frame rate 60 → 30 | `target_fps = 30` |
| `settings-3` | restart showed 30 FPS; Reset Display showed 60 FPS | identical to launch 1 except `game_time` |
| `settings-4` | restart showed 60 FPS | identical to launch 1 except `game_time` |

All four manifests: `complete: true`, exit code 0, `settings_unchanged: true`.
The real player file existed with the same SHA-256 before and after every
launch (`1aaa1267…ba0f71`). Its contents are not published. Pictures and the
profile file after each launch are in `settings-check/`. This shows one choice
persisting and being reset. It does not show every Display setting.

## First-person body: existing defect, not from #1087

The fixture body capture (`scenes/pictures/body.png`), `body-check` and the
built-in `builtin-body/pictures/player-first.png` all show a flat ground-colored
view. Older built-in `player-first.png` captures look the same: #998
(`.agent/validation/issue-998/b-t2/`, 2026-10-27) and #919
(`.agent/validation/issue-919/screens-20261023T203018Z/`, 2026-10-23). So the
problem was already there before #1087. It matches open bug #919 (the player
starts under the ground until the patch under them is built). The first-person
state is reached; whether the body shows is that renderer bug, not something
this tooling can see or fix. No renderer repair is included (Spec non-goal).

## Executable provenance

Every run from `telemetry-smoke` onwards, and the validator runs, used
`target/release/universe-app.exe` with SHA-256 `f0a12fe0…` (manifest
`executable_sha256`). It was built at 2026-10-29T12:02:54Z from the working tree
before 4aa0b44 was committed. At 10b8e94, which has the same crate sources as the
parent merge dae66a7, `cargo build --locked --release -p universe-app` finished in
1.59 s with nothing recompiled. That is Cargo's freshness check, which goes by
file times, not a hash proof that this exact source built this exact executable.

## Independent validation (validator, 2026-10-29, dae66a7)

Raw output is in `../validator/`. The validator ran `short-normal` itself
(exit 0, 3131 scored frames, 151 one-second memory samples, no gaps), a malformed
fixture (`process-failed`, partial output kept), a repeated `--out` (refused) and
a 45-second `--normal` launch (`timeout`, real settings unchanged). It compared
every number in this README with the evidence files and found no mismatch.

Its run fell into the slow wait mode: interval mean/p95 38.33/48.32 ms, combined
wait mean 33.005 ms, against 26.49/34.01 ms for the main-agent `short-normal` run
with the same executable and fixture content. So the two-mode behaviour seen in
overhead pairs 1 and 3 is not unique to those pairs. See README-profile.md.

Three tooling findings were fixed after that run: profile fixtures are now LF on
every checkout (the validator's CRLF copy hashed `1ec1dcf5…` while the evidence
copy hashed `2ed7ab12…`); `--normal` manifests now name stderr as the adapter
source; and the overwrite refusal now gives its own message. An empty
`world_path` (L1 root) is now documented as a real value.

The #1085 preflight rerun returned PASS, with the limits it lists.

## Pending

- Pull request, technical validation, and owner test.

No pending check is treated as passed by compilation, fixture tests or these
short runs. Owner acceptance and parent Issue completion remain pending.

## Collection overhead observations

Six full fixed-Normal short runs completed in order on/off, on/off, on/off.
All pairs match executable SHA-256, fixture SHA-256, output size, worker count,
reported refresh and pre-run power note. Collection-off omits the new working-set
sampler and asset telemetry while retaining the legacy per-frame measurements.

| Pair | Mean interval on / off (ms) | Mean delta | p95 on / off (ms) |
| --- | --- | --- | --- |
| 1 | 38.888 / 28.387 | +36.99% | 48.21 / 31.69 |
| 2 | 28.308 / 28.302 | +0.02% | 30.36 / 30.45 |
| 3 | 38.348 / 28.504 | +34.54% | 48.33 / 31.57 |

The results do not support a stable overhead correction. Pairs 1 and 3 have
large combined synchronization/acquisition waits in the on run (means 33.07
and 33.41 ms versus 0.18 and 0.19 ms off), while main-thread work is lower.
Pair 2's waits are about 0.18 ms in both runs. Requested presentation is
AutoVsync; physical presentation behavior, focus state and thermals were not
continuously captured. Thus the wait-mode difference remains unexplained and
must not be silently attributed entirely to the collector or subtracted from
future timings. All pairs, including the outliers, remain in the evidence.
Instrumented/uninstrumented results are not interchangeable matched-performance
evidence until this variability is controlled or explicitly allowed for.

`gpu-probe` requested `ASTROLITH_GPU_TIMERS=1` on a route run and completed,
but emitted no GPU execution timings. This only proves the request did not
prevent launch; route mode does not install the existing screenshot-timing
diagnostics collector. The supported-mode probe below supplies the actual proof.

`gpu-timing` subsequently completed the supported `--screens ... --shots
scripts/validation/fixtures/profile/gpu.shots --timing` invocation with
`ASTROLITH_GPU_TIMERS=1`. Actual stderr GPU diagnostics include opaque pass
12.36 ms, bloom 3.20 ms, tonemapping 1.30 ms, and UI 0.12 ms. These are
diagnostic-history averages for individual passes, not summed frame time or
CPU work. This screenshot-timing run uses its own defaults/instrumentation and
must not be compared directly with the fixed route runs. Raw diagnostics and
the resulting picture are preserved.

## Main-agent visual review

`scenes/pictures` contains all seven requested fixture captures. Day has a blue
sky and visible ground; both night scenes are very dark, with visibly different
sky/marker arrangements. The light-model test establishes positive moon share
at the moonlit position and zero at the moonless position. Distinct-world
captures have different sky/ground appearance; runtime transition telemetry
separately proves different planet paths. The selection capture shows the
marker field, but the highlight is not sufficiently clear to claim readability.

The body fixture does **not** show a recognizable first-person body. A second
capture at height 0 and tilt -86 (`body-check`) also fails that visual check.
The existing built-in `--screens ... walking` capture (`builtin-body`) shows
the same absence in `player-first.png`. This identifies a reproducible gap;
it does not yet identify its cause or prove the baseline was already affected.
No renderer repair or relaxed acceptance criterion is included silently.

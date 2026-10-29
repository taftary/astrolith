# #1087 capability evidence (in progress)

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

## Pending

- Three alternating collection-on/off pairs and optional GPU probe.
- Rendered scene review, including first-person body and distinct worlds.
- Interactive disposable normal-launch save/restart/Display Reset.
- Independent capability validation and the #1085 preflight rerun.

No pending check is treated as passed by compilation, fixture tests or these
short runs. Owner acceptance and parent Issue completion remain pending.

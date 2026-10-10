# Motion: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-MOT-1: gate every representation change with hysteresis

- Give each threshold (`FORM_ANGLE`, preview angle, open angle) an enter
  value and a smaller exit value; retain state inside the band
  [MOT-S006] [MOT-S008] [MOT-S010].
- Drive opacity and brightness inside the band from angular size through
  the same curves the open cell uses, so entries and exits change nothing
  on screen [MOT-S009].

## R-MOT-2: ease flights in-out, never linear or overshooting

- Default to quadratic in-out for flights and fades; reserve cubic
  in-out for long cross-field legs [MOT-S002] [MOT-S005].
- Forbid back, bounce, elastic, or any overshooting curve on camera and
  marker motion; overshoot breaks R7 by construction.
- Shape long dives as zoom-out, pan, zoom-in per van Wijk and Nuij with
  the standard 1.42 trade-off until playtesting says otherwise [MOT-S001].

## R-MOT-3: budget durations by kind and distance

- Fades and selection feedback: 100 to 250 ms.
- Level flights: 250 to 600 ms scaled by distance travelled (Cesium
  auto-duration precedent), one rate per decade on the tail [MOT-S003]
  [MOT-S005].
- Exits and dismissals shorter than entrances; ambient motion only above
  700 ms and never blocking [MOT-S003].
- Do not adopt Powers of Ten film pacing as a game rate (see C8 in the
  [topic critique](../../documents/critique.md)).

## R-MOT-4: keep every flight interruptible and continuous

- Every flight is cancellable; new input retargets from the current pose,
  never from the planned endpoint [MOT-S004] [MOT-S005].
- Hold R7 across markers, shells, limb, air, horizon, ground, and grid:
  fade by size, blend up by altitude, bend landings to the normal, clamp
  clearance, and mask mesh despawn with the rim fade-out [MOT-S009].
- Provide reduced-motion substitutes per
  [accessibility](../../accessibility/documents/README.md): shortened or
  instant transitions with context retained.

## Validation

- Slow-zoom each gate both directions and confirm at most one switch per
  crossing [MOT-S008].
- Screenshot the frame before and after every open and close; pixels
  outside the entered marker must match within the fade tolerance
  [MOT-S009].
- Time flights on a phone at 60 fps; any flight that blocks input fails.

## References

- [MOT-S001](./sources.md#mot-s001--van-wijk-and-nuij-zoom-pan) through
  [MOT-S010](./sources.md#mot-s010--navigation-topic-baseline)

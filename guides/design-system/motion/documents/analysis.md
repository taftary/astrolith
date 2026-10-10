# Motion: analysis

Interpretation for Astrolith. Facts in [research](./research.md).

## Gates

### Gates

Markers below `FORM_ANGLE` (0.006 rad) draw dots at their own size, above
it their form, above preview angle their preview, and at open angle they
open (ADR 0017). The research says a single comparison per gate will
flicker during slow zooms: the gate needs an enter angle and a smaller
exit angle with retained state between them [MOT-S006] [MOT-S008]
[MOT-S010]. Fades run inside the band by mapping angular size to the
brightness curves from
[foundations](../../foundations/documents/README.md), the same level
rules the open cell uses (ADR 0019 context previews).

![Fade gates](../images/fade-gates.svg)

*Figure 1. Hysteresis bands on the angular-size axis. State is retained
inside the band.*

## Easing

Quadratic or cubic in-out is the flight curve: it starts and ends at rest,
which is what preserves the R7 no-jump rule across a frame change
[MOT-S002] [MOT-S005]. Linear would arrive with residual velocity and
read as a step; back, bounce, or any overshoot would break continuity by
construction. The van Wijk zoom-out, pan, zoom-in shape [MOT-S001] is the
model for long cross-field dives; short dives stay near-direct with the
same in-out time shaping.

## Timing

Material bands [MOT-S003] give the budget shape: fades and selection
feedback in the short band (about 100 to 250 ms), level flights in the
medium to long band (about 250 to 600 ms scaled by distance, Cesium style
[MOT-S005]), ambient motion only in the extra-long band. Exits run shorter
than entrances so dismissals and retreats feel responsive. Powers of Ten
pacing (one decade per 10 seconds out, faster back) is a film rate for a
narrated 9 minutes, not a game rate; copying it would make long legs
tedious and short legs rushed (see C8 in the
[topic critique](../../documents/critique.md)).

## Continuity

R7 as extended by ADR 0019 covers everything drawn on the tail, not only
markers: up blends by altitude without steps, landing bends toward the
surface normal and ends on it, clearance never lets the camera sink, kept
parent context follows the entered shell curve, and limb, air, horizon,
and ground previews fade by size rather than switching at the open event
[MOT-S009] [MOT-S010]. Apple motion rules add the interaction contract:
flights are brief, cancellable, and never block input, with a stationary
frame of reference through the transition [MOT-S004].

## Open questions

- Exact gate angles and band widths per level are specification values
  with owner-feel tuning (ADR 0019 anticipates a correction round).
- Per-decade pacing on the tail keeps one rate per decade today; whether
  the van Wijk trade-off parameter needs tuning for the L10 to L14 leg is
  untested.
- Reduced-motion substitutes (instant cuts with retained context versus
  shortened fades) are defined in
  [accessibility](../../accessibility/documents/README.md).

## References

- [MOT-S001](./sources.md#mot-s001--van-wijk-and-nuij-zoom-pan) through
  [MOT-S010](./sources.md#mot-s010--navigation-topic-baseline)

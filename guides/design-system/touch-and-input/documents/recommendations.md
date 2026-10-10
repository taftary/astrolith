# Touch and input: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-TCH-1: pad every target, state the overlap rule

- Ship 48 by 48 dp hit areas (44 by 44 pt on iOS); treat 24 px as the
  floor, not the goal [TCH-S001] [TCH-S002] [TCH-S003].
- Keep at least 8 dp separation between neighbouring hit areas where the
  field allows it; where it does not, specify ranking or tap-to-cycle so
  24 px circles never ambiguously overlap [TCH-S002] [TCH-S003].
- Raise `PICK_PIXELS` handling to the padded rule on touch; keep the
  24.0 value as the desktop floor only.

## R-TCH-2: give every gesture an alternative

- Pinch to dive stays primary; add a single-pointer dive control
  (on-screen button or double-tap-hold) that alone suffices [TCH-S006]
  [TCH-S007].
- Tilt and virtual stick are full alternatives: either alone evades;
  neither is required; expose gain and dead-zone settings [TCH-S007].
- Support both portrait and landscape unless a later specification
  proves an essential lock [TCH-S007].

## R-TCH-3: lay out for one thumb inside the safe area

- Primary input in the bottom thumb arc inside safe-area and
  safe-gesture insets; status top; nothing anchored to physical edges
  [TCH-S004] [TCH-S005] [TCH-S008].
- Test on notched and folded devices in both orientations; any clipped
  or system-shadowed control fails.
- Honour Reduce Motion and Dynamic Type from the operating system plus
  in-game equivalents (see
  [accessibility](../../accessibility/documents/README.md)) [TCH-S009].

## Validation

- Finger test on a large phone one-handed: reach every primary control
  without grip shift; time a target-dive-evade loop.
- Overlap test in the densest field: no tap resolves ambiguously.
- System-gesture test: dive and evade never trigger home, back, or
  notification gestures.

## References

- [TCH-S001](./sources.md#tch-s001--apple-hit-regions) through
  [TCH-S009](./sources.md#tch-s009--apple-accessibility-motion-and-type)

# Touch and input: analysis

Interpretation for Astrolith. Facts in [research](./research.md).

## Targets

The spine gesture is tap a marker to target it. Visible markers are often
far smaller than a finger, so hit areas pad beyond the visible dot: 48 dp
baseline (44 pt on iOS), 24 px as the absolute floor with a spacing
fallback, never as the shipped goal [TCH-S001] [TCH-S002] [TCH-S003]. The
current `PICK_PIXELS = 24.0` meets only the floor and is recorded as C5 in
the [topic critique](../../documents/critique.md). Dense fields need a
stated overlap rule (spacing, ranking, or cycling) because neighbouring
24 px circles otherwise intersect and fail the spacing exception.

## Gestures

Pinch to dive matches platform expectation and stays [TCH-S006]. It still
needs a single-pointer dive alternative (button, double-tap-hold, or
equivalent) to satisfy SC 2.5.1 [TCH-S007]; this is C4 in the
[topic critique](../../documents/critique.md). Tilt to evade needs its
virtual-stick alternative with either input alone sufficient to satisfy
SC 2.5.4; the draft already names both, which passes if neither is
required [TCH-S007]. Dragging the stick needs no further alternative
beyond the tilt path itself, since each is the other's single-pointer
cover. No orientation lock: both portrait and landscape play unless a
later specification justifies an essential lock [TCH-S007].

## Layout

### Layout

One thumb must be enough for the spine. Frequent dive and evade input
lives in the bottom thumb arc inside the safe area; status (level, scale,
distance) lives top; meters sit compact between them without leaving the
safe area [TCH-S004] [TCH-S005] [TCH-S008]. The bottom strip stays clear
of system-gesture insets so dive handles never fight the home or back
gesture [TCH-S005].

![Touch zones](../images/touch-zones.svg)

*Figure 1. Thumb arc inside the safe area; system edges stay clear.*

## Desktop versus touch

Desktop mouse and keyboard controls remain for development and testing;
touch is the design target (draft, Controls and sessions, bullet 3).
Hover and focus states exist on desktop but must not shrink touch minima,
and pointer-specific affordances must not become required paths on phones
[TCH-S001] [TCH-S006].

## Open questions

- Pinch sensitivity, stick gain, and tilt dead zone need device tuning.
- The dense-field overlap rule (rank by interest, cycle on repeated tap,
  or magnify) is unspecified.
- Whether landscape mirrors portrait or reflows meters to the side is a
  layout specification task.

## References

- [TCH-S001](./sources.md#tch-s001--apple-hit-regions) through
  [TCH-S009](./sources.md#tch-s009--apple-accessibility-motion-and-type)

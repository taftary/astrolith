# Motion: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. Optimal zoom and pan moves out, across, then in

Van Wijk and Nuij define a metric on combined zoom and pan from root-mean
square perceived velocity and solve for the smooth and efficient path:
zoom out, pan across, zoom back in, with two free parameters for animation
speed and the zoom-pan trade-off. Their user-study default weights zoom
less than pan (curve factor 1.42, root-mean-square factor one sixth)
[MOT-S001]. MapLibre and Mapbox implement the same curve verbatim in
`flyTo` [MOT-S001]. This is the standard answer for distance-scaled,
non-jarring multi-scale flights.

## 2. Easing shapes time, not path

Penner defines the canonical tween set over normalised time: linear,
quadratic, cubic, quartic, quintic, sine, exponential, and circular, each
in entering (accelerating from rest), leaving (decelerating to rest), and
in-out (accelerating to halfway, then decelerating) variants
[MOT-S002]. Tween libraries and Cesium easing functions port the same
set. In-out is the continuity-preserving choice; linear looks mechanical
and back or bounce overshoots the endpoint.

## 3. Material duration and easing bands

Material Design 3 specifies a standard easing cubic bezier (0.2, 0, 0, 1)
for motion that begins and ends on screen, decelerate (0, 0, 0, 1) for
entering, and accelerate (0.3, 0, 1, 1) for exiting [MOT-S003]. Duration
bands are short 50 to 200 ms (selection controls 100 to 200 ms), medium
250 to 400 ms, long 450 to 600 ms, and extra-long 700 to 1000 ms for
ambient motion only [MOT-S003]. Exits run shorter than entrances (about
200 ms versus 250 to 400 ms) so dismissals feel responsive [MOT-S003].

## 4. Apple motion: brief, interruptible, stationary frame

Apple Human Interface Guidelines for motion direct designers to add motion
purposefully, keep feedback brief and precise, let people cancel or
interrupt motion rather than blocking on animation, use built-in
ease-in-out for layout and appearance changes, and avoid sustained
oscillation (especially near 0.2 Hz) and whole-frame rotation, preferring
fades with an instantaneous directional change against a stationary frame
of reference [MOT-S004].

## 5. Fly-to auto-duration with cancellation

Cesium `flyTo` accepts destination, orientation, duration, easing
function, and completion or cancellation callbacks; when duration is
omitted, the engine calculates an ideal duration from distance travelled
[MOT-S005]. The worked example flies Tokyo to Seattle with quadratic
in-out easing [MOT-S005]. Flights are cancellable by design.

## 6. Fade by size with a band, not a line

Unity selects level of detail by screen-size ratio and smooths the switch
with a fade transition width (a fraction of the range before the
threshold) or a fixed-time cross-fading animation after it [MOT-S006].
Cesium 3D Tiles refines while screen-space error exceeds a per-tile
budget, progressing from coarse roots to fine leaves [MOT-S007]. A
Schmitt trigger switches high at an upper threshold and low at a lower
one; the gap between them rejects noise and slow-input oscillation
[MOT-S008]. The navigation topic records the same pattern as
Schmitt-trigger gates for Astrolith open, close, and preview thresholds
[MOT-S010].

## 7. Continuity invariants

The navigation analysis defines continuity at a frame change: opening a
marker is a pure origin shift and closing is its exact inverse, so the
picture must not jump [MOT-S010]. ADR 0019 extends the R7 rule from marker
points to everything drawn on the tail: kept parent context follows the
entered marker shell curve, the L10 limb and air preview fade in before
entry, and the air rim fades out to mask mesh despawn [MOT-S009].

## References

- [MOT-S001](./sources.md#mot-s001--van-wijk-and-nuij-zoom-pan) through
  [MOT-S010](./sources.md#mot-s010--navigation-topic-baseline)

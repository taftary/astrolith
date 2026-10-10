# Touch and input: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. Target sizes

Apple requires hit regions of at least 44 by 44 pt (60 by 60 pt in
visionOS) so people can select easily by touch, pointer, or assistive
input; the visible glyph may be smaller if the hit area is padded
[TCH-S001]. Material recommends at least 48 by 48 dp (about 9 mm) with at
least 8 dp separation, the icon glyph allowed smaller than the padded
target [TCH-S002]. WCAG SC 2.5.8 sets the legal floor: pointer targets
must fit a 24 by 24 CSS px square or carry spacing so that 24 px circles
centred on undersized targets do not intersect [TCH-S003]. Best practice
meets the size regardless of spacing; important controls aim for the older
44 by 44 enhanced value [TCH-S003].

## 2. Safe areas and system insets

Apple directs designers to lay out full-screen, adapt across sizes and
orientations, and respect safe areas and layout margins so content is not
clipped by rounded corners, sensor housing, or the home indicator; no
fixed margin is prescribed, the system insets govern [TCH-S004]. Android
requires drawing behind system bars and then insetting critical content
via window insets (status, navigation, safe gestures, display cutout);
system-gesture insets mark where operating-system gestures take priority
over application gestures [TCH-S005]. Both platforms agree: critical
interface and one-thumb controls live inside the safe area, never under
it or on physical edges.

## 3. Pinch is the expected zoom gesture

Apple and Material both document pinch open and close as the system-wide
zoom in and out gesture [TCH-S006]. Astrolith pinch to dive in and out
therefore needs no retraining. Neither platform wants complex
multi-finger gestures as the only path; games should not require force or
multi-finger input without an alternative [TCH-S006].

## 4. Pointer gestures, tilt, and dragging need alternatives

WCAG SC 2.5.1 requires a single-pointer alternative for every multipoint
or path-based gesture unless the gesture is essential [TCH-S007]. SC 2.5.4
requires a user-interface alternative for motion actuation such as tilt:
tilt must not be required [TCH-S007]. SC 2.5.7 requires a single-pointer
alternative for dragging movements [TCH-S007]. SC 1.3.4 adds that the
interface must not lock to a single display orientation unless essential
[TCH-S007].

## 5. Reach and one-handed use

Apple publishes no one-thumb zone specification, but the layout guidance
implies it: keep primary and frequent actions reachable, centred or in
the lower half, adaptable to one-handed thumb use on large phones
[TCH-S008]. Industry synthesis places frequent actions in the
bottom-third thumb arc and reserves top corners for read-only status,
with separate portrait and landscape layouts [HUD lineage, TCH-S008].
Putting critical input at the top edge forces a grip shift.

## 6. Motion and text scaling

Apple requires honouring Reduce Motion (disable or replace parallax,
zoom, and non-essential animation when active), supporting Dynamic Type
and larger text sizes, and never conveying information by colour alone
[TCH-S009]. The `isReduceMotionEnabled` flag reflects the system setting
and must be queried [TCH-S009]. Dynamic Type reaches five larger
accessibility sizes with layout required to survive 200 percent scaling
[TCH-S009].

## References

- [TCH-S001](./sources.md#tch-s001--apple-hit-regions) through
  [TCH-S009](./sources.md#tch-s009--apple-accessibility-motion-and-type)

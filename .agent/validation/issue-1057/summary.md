# #1057 evidence: big circles fade when seen edge-on (Spec v2)

Release builds, real window, `--screens <dir> --shots edge-on.shots`
(1280 x 720, the clock held). `before/` is `main` at db08eff (after
#1056); `after/` is `fix/1057-edge-on-circles` with Spec v2's rule. The
PNGs stay out of git and go to the `validation-evidence` release with the
pull request.

| shot | before | after |
|---|---|---|
| L7-gate (the #998 view where the bug was seen) | light vertical line through the Sun; dark bands and a light arc at the top | no line, no bands; the Sun, its label and the small circles round it stay |
| L7-far | the same line and bands | no line; two curved arcs remain near the top (seen well out of their planes, they read as circles) |
| L7-above (over the Sun, looking down) | a cross of two straight lines through the Sun | no cross; the rest as before |
| space-l4 | markers and rings | identical |
| orbit (planet, 300 km) | the moon's path as a straight green line across the sky (#1056's colour) | gone: that path lay edge-on in this view |

- **What the line was.** A huge sphere marker round the Sun. A gizmo
  sphere is three axis circles, and from inside it one of them lay
  edge-on. Big spheres (within four radii) now fade each outline circle.
- **What the bands were.** Circles of spheres round the camera. Measured
  from their centre the eye was about 30° out of their plane, but their
  nearest part swept across the screen. Spec v2 (owner Q2) also measures
  the angle from the nearest point, and the smaller of the two decides.
- **For the #1056 test.** Seen from orbit, a moon's path shows in its
  lighter colour unless the view lies within 10° of its plane. Then it
  fades, as this rule asks.

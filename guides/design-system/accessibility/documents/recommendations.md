# Accessibility: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-ACC-1: hold contrast floors on black

- Body and meter numerals at 4.5:1 minimum against the background; aim
  7:1 for small custom text [ACC-S001].
- Icons, fills, reticles, sticks, focus indicators, and required graphics
  at 3:1 against adjacent colours [ACC-S002] [ACC-S003].
- No meaning-carrying 1 px lines; validate every tint at its floor weight
  with an analyser, never by eye [ACC-S002].

## R-ACC-2: adopt a colour-blind-safe categorical set

- Base categorical tints on blue-vermilion (Wong `#0072B2` plus
  `#D55E00` anchor) or Tol bright and vibrant families; forbid
  red-green-only coding [ACC-S008] [ACC-S009] [ACC-S010].
- Replace pure or dark red with vermilion or magenta where a warm alert
  is needed; never set red text or dim red markers on black [ACC-S008].
- Proof every release pair in Coblis or Stark simulation plus a
  monochrome proof; confirm with colour-blind players before freezing
  [ACC-S010].

## R-ACC-3: code redundantly, always

- Every state carries shape plus text or icon plus brightness; colour is
  never the only code [ACC-S006].
- Meters add alert glyphs or words; portals add marks and labels; hazards
  differ by silhouette first (see
  [markers](../../markers-and-forms/documents/README.md)).

## R-ACC-4: ship motion controls from version one

- Global reduced-motion toggle in game that honours the operating-system
  setting; disable or replace parallax, bob, shake, blur, and
  non-essential zoom animation when active [ACC-S011].
- Opaque backing option behind text over moving scenes; field-of-view,
  sensitivity, bob, shake, and blur settings; player-controlled camera
  with maintained horizon as the comfortable default [ACC-S011].
- Keep focus indicators visible at 3:1 with a 2 px perimeter where
  keyboard or controller focus exists [ACC-S005].

## Validation

- Analyser pass on every tint, text size, and weight before any freeze.
- Simulation pass plus colour-blind participant pass on meters, hazards,
  and portal states.
- Motion pass with Reduce Motion on: complete a dive, an evade, and a
  retreat without vestibular triggers and without losing place.

## References

- [ACC-S001](./sources.md#acc-s001--contrast-minimum) through
  [ACC-S011](./sources.md#acc-s011--motion-sensitivity)

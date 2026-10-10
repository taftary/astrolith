# Markers and forms: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-MRK-1: keep form nominal, emphasis quantitative

- Assign each rung one primary form family and keep it stable so players
  learn the code once (for example arms and threads for cosmic web,
  disks and rings for galaxies, shells and orbits for systems).
- Never encode importance by form; encode it by size and brightness step
  from [foundations](../../foundations/documents/README.md)
  [MRK-S001] [MRK-S002].

## R-MRK-2: mark portals redundantly

- Every portal carries three cues: its body form, a portal mark on or in
  the body at the true child offset, and a text label. Populations carry
  dots only [MRK-S004] [MRK-S005].
- Default centred portals still draw the mark; off-centre portals (for
  example the Milky Way lane) draw it at the measured offset so opening
  position matches the picture.

## R-MRK-3: select with outline plus halo, never hue alone

- Selection is a thin outline ring at the line-weight floor plus a label
  halo; hover is a brighter variant of the same shape. Do not restyle by
  hue change only [MRK-S003] [MRK-S004].
- Keep halo widths effective but invisible: wide enough to separate the
  mark from the field, narrow enough not to read as a second ring.

## R-MRK-4: draw silhouettes as opaque flat two-tone fills

- Surfaces, vessel, and hazards use opaque unlit fills in two tones, no
  lights, no shadows, minimal transparency to limit overdraw
  [MRK-S011] [MRK-S012] [MRK-S014].
- Distinguish hazard kinds by silhouette shape first and tint second;
  test every pair under protanopia simulation (see
  [accessibility](../../accessibility/documents/README.md)).

## Validation

- Walk the dive path on a space-scale sketch and confirm each semantic
  boundary is crossed once per dive [MRK-S007].
- Proof every portal versus population pair in monochrome; if the
  distinction vanishes, the shape code is insufficient [MRK-S004].

## References

- [MRK-S001](./sources.md#mrk-s001--bertin-visual-variables) through
  [MRK-S014](./sources.md#mrk-s014--unity-unlit-shader)

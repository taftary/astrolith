# HUD and typography: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-HUD-1: lay out status top, meters compact, input bottom

- Status cluster top inside the safe area: level label, dynamic scale
  bar with units, distance readout. Read-only, outside the input arc
  [HUD-S006] [HUD-S010].
- Meters compact and persistent; one primary input affordance visible;
  all else transient [HUD-S004].
- Ship separate portrait and landscape layouts; never stretch one layout
  across orientations [HUD-S010].

## R-HUD-2: build every meter as icon plus number plus bar

- Icon identifies the system at a glance and under monochromacy; number
  gives the decision value; bar gives the fraction [HUD-S001]
  [HUD-S005].
- Alert adds a glyph or word (LOW, a warning triangle); never a hue
  shift alone. Meter fills hold 3:1 against their track; numerals hold
  4.5:1 against the background [HUD-S005].
- Playtest the four meters against the experience goal "read scale plus
  survival state at a glance on a phone" [HUD-S003].

## R-HUD-3: pair the scale bar with label and distance, always dynamic

- Recompute the bar every level with labelled units and two to four
  divisions; keep level label plus distance readout adjacent [HUD-S006].
- Forbid static bars and unlabelled log-tick decoration.

## R-HUD-4: set a small type scale with tabular numerals

- Three to four sizes only, mapped to roles (level, distance, meter
  number, caption); minimum at or above the 11 pt equivalent; no thin
  weights at small sizes [HUD-S007] [HUD-S008].
- Tabular figures on every changing number (`monospacedDigit` or
  `tabular-nums` semantics); never proportional figures for readouts,
  never a full monospace face for labels [HUD-S009].
- Semibold or medium white numerals on near-black with analyser-verified
  ratios; keep the Bevy atlas small by freezing the token set
  [HUD-S005].

## Validation

- Glance test on a phone at arm's length: scale, level, distance, and
  all four values readable in under two seconds.
- Monochrome proof: if any meter state vanishes, the icon or text code
  is insufficient [HUD-S005].
- Largest-accessibility-text test: layout must survive 200 percent
  scaling inside the safe area without overlap.

## References

- [HUD-S001](./sources.md#hud-s001--hud-taxonomy) through
  [HUD-S010](./sources.md#hud-s010--mobile-layout-patterns)

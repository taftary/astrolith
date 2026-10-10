# HUD and typography

Scope: scale bar, level label, distance readout, the four meters, type
and numerals readable on a phone, and minimal chrome. This subtopic
defines the overlay that makes scale and survival state glanceable
without covering the universe.

Overview: the HUD is non-diegetic overlay, always inside the safe area:
status (level label, dynamic scale bar, distance) top, meters compact and
persistent, every changing number in tabular figures at or above the
11 pt equivalent, every meter as icon plus number plus bar, never colour
alone. Chrome stays near zero; the universe stays the picture. See
[research](./research.md), [analysis](./analysis.md),
[recommendations](./recommendations.md), and [sources](./sources.md).

## Documents

| Document | Type | What it covers |
|---|---|---|
| [Research](./research.md) | research | HUD taxonomy and meter precedent, scale-bar convention, mobile type floors, tabular numerals, contrast on black |
| [Analysis](./analysis.md) | analysis | Four-meter reads, scale and distance pairing, type scale, chrome budget |
| [Recommendations](./recommendations.md) | recommendations | Layout, meter, type, and chrome rules with validation |
| [Sources](./sources.md) | sources | `HUD-S001` to `HUD-S010` with claims and limits |
| [Critique](../../documents/critique.md) | critique | Topic-level contradictions with the draft and ADRs |

## Images

| Image | Shows | Linked from |
|---|---|---|
| [HUD layout](../images/hud-layout.svg) | Safe-area HUD with top status and compact meters | [Analysis](./analysis.md#layout) |

![HUD layout inside the safe area: status top, meters compact, primary input bottom](../images/hud-layout.svg)

*Figure 1. Portrait-first HUD. Status reads top; meters stay compact;
primary input lives in the thumb arc (see
[touch](../../touch-and-input/documents/README.md)).*

## Related topics

- Parent topic: [Design system](../../documents/README.md).
- Siblings: [Touch and input](../../touch-and-input/documents/README.md)
  for safe areas and reach;
  [Accessibility](../../accessibility/documents/README.md) for contrast
  and redundant coding;
  [References](../../references/documents/README.md) for Out There, FTL,
  and Kingdom HUD precedent.
- [Navigation](../../../navigation/documents/README.md): level names and
  distance orders quoted by the HUD; camera research is not restated.
- Read-only draft line: "scale bar, level label, distance, the four
  meters. Strong typography, minimal chrome."

## Documentation status

Researched October 2026. Exact meter order, portrait versus landscape
variants, and numeric type tokens await the specification.

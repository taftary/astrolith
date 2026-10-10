# Accessibility

Scope: contrast on black, colour-blind-safe tints, and
motion-sensitivity options. This subtopic states the minima every other
subtopic must meet: which ratios govern text and graphics, which palettes
survive protanopia and deuteranopia, and which motion controls ship.

Overview: text holds 4.5:1 (3:1 large), graphics and controls hold 3:1
against adjacent colours, targets meet 48 dp with a 24 px floor, colour
is never the only code, categorical tints come from a colour-blind-safe
set (blue-vermilion or Tol bright and vibrant families), and a global
reduced-motion path plus field-of-view and sensitivity controls ship from
version one. See [research](./research.md),
[analysis](./analysis.md), [recommendations](./recommendations.md), and
[sources](./sources.md).

## Documents

| Document | Type | What it covers |
|---|---|---|
| [Research](./research.md) | research | WCAG contrast, target, and motion criteria; CVD prevalence and safe palettes; game motion guidance |
| [Analysis](./analysis.md) | analysis | What the minima mean for tints, meters, markers, and flights on black |
| [Recommendations](./recommendations.md) | recommendations | Contrast, palette, redundancy, and motion rules with validation |
| [Sources](./sources.md) | sources | `ACC-S001` to `ACC-S011` with claims and limits |
| [Critique](../../documents/critique.md) | critique | Topic-level contradictions with the draft and ADRs |

## Images

| Image | Shows | Linked from |
|---|---|---|
| [Contrast pairs](../images/contrast-pairs.svg) | Safe versus unsafe tint pairs on black | [Analysis](./analysis.md#contrast-on-black) |

![Contrast pairs on black: vermilion and blue pass, dark red fails](../images/contrast-pairs.svg)

*Figure 1. Safe pairs keep luminance and shape redundancy; dark red on
black fails twice.*

## Related topics

- Parent topic: [Design system](../../documents/README.md).
- Siblings: [Foundations](../../foundations/documents/README.md) for the
  tints under test; [Markers and forms](../../markers-and-forms/documents/README.md)
  for shape redundancy; [HUD and typography](../../hud-and-typography/documents/README.md)
  for meter text; [Touch and input](../../touch-and-input/documents/README.md)
  for targets; [Motion](../../motion/documents/README.md) for flight
  timing.
- Read-only norms: WCAG 2.2 (W3C Recommendation 2024-12-12), Apple HIG,
  Material 3.

## Documentation status

Researched October 2026. Device-measured ratios and CVD participant
proofing are specification tasks, not yet done.

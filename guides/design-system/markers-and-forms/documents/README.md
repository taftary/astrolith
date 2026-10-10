# Markers and forms

Scope: points, rings, outlines, the portal versus population split, and
flat two-tone silhouettes for surfaces, vessel, and hazards. This subtopic
defines the drawn vocabulary of the game: which shape means what, how a
body differs from its portal, and how flat fills stay readable without a
lighting model.

Overview: cosmic markers use glowing points, lines, and outlines only
(ADR 0002). Every marker has a body drawn at its own size and form and,
for portals, a portal position where the child cell opens (ADR 0017).
Populations show only; portals open. Surfaces, vessel, and hazards are
flat two-tone silhouettes with unlit fills. Redundant coding (shape plus
label plus brightness, never hue alone) keeps the vocabulary legible and
colour-blind safe. See [research](./research.md),
[analysis](./analysis.md), [recommendations](./recommendations.md), and
[sources](./sources.md).

## Documents

| Document | Type | What it covers |
|---|---|---|
| [Research](./research.md) | research | Map symbology, point versus line versus area, portal and semantic zoom, halo legibility, flat-silhouette precedent |
| [Analysis](./analysis.md) | analysis | Body and portal forms against ADR 0017, portal versus population reads, silhouette rules |
| [Recommendations](./recommendations.md) | recommendations | Form assignments, selection affordance, halo and label rules |
| [Sources](./sources.md) | sources | `MRK-S001` to `MRK-S010` with claims and limits |
| [Critique](../../documents/critique.md) | critique | Topic-level contradictions with the draft and ADRs |

## Images

| Image | Shows | Linked from |
|---|---|---|
| [Marker forms](../images/marker-forms.svg) | Dot, ring, outline, disk, shell, and patch forms with portal marks | [Analysis](./analysis.md#form-vocabulary) |

![Marker forms: dot, ring, outline, disk, shell, and patch, each with its portal mark](../images/marker-forms.svg)

*Figure 1. The six representative forms. Portal marks sit on or in the
body; dots carry no portal detail until they grow.*

## Related topics

- Parent topic: [Design system](../../documents/README.md).
- Siblings: [Foundations](../../foundations/documents/README.md) for
  colour and brightness tokens;
  [Accessibility](../../accessibility/documents/README.md) for redundant
  coding rules.
- [Universe](../../../universe/documents/README.md): per-level bodies and
  parts; this subtopic defines the vocabulary, not any level.
- [Navigation](../../../navigation/documents/README.md): angular-size
  gates for dot, form, and preview; timing lives in
  [Motion](../../motion/documents/README.md).
- Read-only ADRs: `0002` (indicators only), `0010` (portal and population
  split), `0017` (body and portal forms).

## Documentation status

Researched October 2026. Form-to-gizmo mapping per level is a
specification task; hazard silhouette list awaits the gameplay notion.

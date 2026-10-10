# Tokens

Scope: how the design system is written down and used in code (named
colour, size, and timing values as Rust constants used by Bevy) and how
the tokens stay in sync with `crates/universe-render/src/style.rs`. This
subtopic is the bridge between the design reference and the renderer: it
names the token tiers, the naming rules, the Bevy mapping, and the sync
discipline. It defines no visual values itself; values live as
recommendations in the sibling subtopics.

Overview: one dictionary of named constants in three tiers (reference
palette, semantic roles, component assignments), role-first stable names,
sRGB authoring with linear conversion at use, small frozen sets for fonts
and timings, and a single-direction sync rule: tokens are specified here
in `guides/`, implemented once in `style.rs` (read-only from here), and
verified by tests and screenshots. See [research](./research.md),
[analysis](./analysis.md), [recommendations](./recommendations.md), and
[sources](./sources.md).

## Documents

| Document | Type | What it covers |
|---|---|---|
| [Research](./research.md) | research | W3C token format, Style Dictionary, Material tiers, Apple semantic colours, Bevy constant mapping |
| [Analysis](./analysis.md) | analysis | Token tiers for Astrolith, naming rules, Bevy mapping, sync direction |
| [Recommendations](./recommendations.md) | recommendations | Token list shape, naming conventions, sync and validation rules |
| [Sources](./sources.md) | sources | `TOK-S001` to `TOK-S009` with claims and limits |
| [Critique](../../documents/critique.md) | critique | Topic-level contradictions with the draft and ADRs |

## Images

| Image | Shows | Linked from |
|---|---|---|
| [Token flow](../images/token-flow.svg) | Guide specification to Rust constants to Bevy usage to verification | [Analysis](./analysis.md#sync-direction) |

![Token flow from guides specification to Rust constants to Bevy to verification](../images/token-flow.svg)

*Figure 1. One direction only: guides specify, `style.rs` implements,
tests and screenshots verify.*

## Related topics

- Parent topic: [Design system](../../documents/README.md).
- Siblings: [Foundations](../../foundations/documents/README.md) for
  colour, glow, and ladder values;
  [Markers and forms](../../markers-and-forms/documents/README.md),
  [Motion](../../motion/documents/README.md),
  [HUD and typography](../../hud-and-typography/documents/README.md),
  [Touch and input](../../touch-and-input/documents/README.md), and
  [Accessibility](../../accessibility/documents/README.md) for the
  values these tokens carry.
- Read-only code: `crates/universe-render/src/style.rs` (current
  constants; never edited from `guides/`).

## Documentation status

Researched October 2026. The token list shape is proposed; exact names
and values await the specification and the owner.

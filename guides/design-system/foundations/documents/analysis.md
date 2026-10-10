# Foundations: analysis

Interpretation for Astrolith. Facts live in [research](./research.md);
proposals live in [recommendations](./recommendations.md).

## Current baseline

`crates/universe-render/src/style.rs` (read-only) defines per-era point
colours for L1 to L3 cyan, L4 to L9 warm, L10 green, L11 violet, L12
orange, L13 teal, L14 white, plus dimmed sibling colours per era, a hue
passthrough `tint_color`, a linear `scaled` helper, and `PICK_PIXELS =
24.0`. ADR 0015 adds per-object tints from core `ColorInfo`, one emissive
billboard per bright portal at or above Sun brightness, bloom plus
TonyMcMapface with a manual exposure resource, and a scale HUD. ADR 0016
narrows planet colour to one flat-tone unlit body. ADR 0017 maps body
forms to gizmo primitives with dot, form, preview, and open angular
thresholds.

## Colour

Per-class tints identify content; per-era colours identify depth. The two
must not collide: a warm star tint and the warm L4 to L9 era colour can
agree, but a cyan galaxy kind tint on a cyan L1 to L3 background loses its
nominal category. The analysis rule is that hue plus shape carries the
nominal code (see [markers and forms](../../markers-and-forms/documents/README.md)),
and value or lightness carries emphasis. Storing tokens as sRGB and
converting to linear before scaling follows [FOU-S006] and [FOU-S007] and
matches the existing `scaled` helper.

## Light and glow

The fixed-bloom plus emissive-ladder pattern from the research fits the
prototype exactly: leave bloom settings and prefilter defaults alone and
drive glow through HDR emissive values [FOU-S001] [FOU-S003]. Only bright
portals cross the bloom threshold today; the ladder should keep it that
way so bloom marks importance rather than washing the field. Keeping
emissive exposure weight at zero (the Bevy default) stops manual exposure
changes from shifting glow tokens [FOU-S005].

TonyMcMapface as the single fixed operator is the right call: it is
neutral, desaturates brights gracefully, and pushes artistic grading into
tokens where it can be named and tested [FOU-S004] [FOU-S011]. Manual
exposure stays a single `ev100` token with named presets; one stop equals
a doubling, which gives the ladder its step math [FOU-S009] [FOU-S012].

## Line weight and point size

Gizmo width in pixels with a 2.0 default and no engine point size means
Astrolith needs two explicit tokens: a line-width floor and a point-size
rule [FOU-S008]. The WCAG thin-line warning is the binding constraint:
thin plus low-contrast fails twice, once for weight and once for ratio.
The current look has no stated minimum, which is recorded as C3 in the
[topic critique](../../documents/critique.md).

## Brightness ladder

### Brightness ladder

The ladder has three rungs: shell (the open cell, brightest, crosses the
bloom threshold), children (portals and populations inside it, near the
threshold), and context floor (kept parent siblings, faint but present).
Spacing the rungs in stops (factors of two) makes the hierarchy survive
exposure shifts: moving exposure up or down moves all rungs together. The
existing `sibling_color_for_level` dimming is the context-floor
precedent; the shell-versus-children split is the emissive-billboard
precedent from ADR 0015.

![Brightness ladder](../images/brightness-ladder.svg)

*Figure 1. Shell above the bloom threshold, children near it, context
floor below it, spaced in stops.*

## Open questions

- Measured contrast of each tint at its drawn weight on a real OLED
  panel is missing; nominal sRGB values are not proof.
- Whether perspective line width is wanted for 3D views or a constant
  pixel width everywhere is undecided.
- The context floor value that stays visible without competing with
  children needs a device test.

## References

- [FOU-S001](./sources.md#fou-s001--bevy-bloom) through
  [FOU-S013](./sources.md#fou-s013--accepted-project-baseline)

# Foundations: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-FOU-1: fix the operator and exposure tokens

- Keep one tonemap token: TonyMcMapface, with the `tonemapping_luts`
  feature pinned and justified. Do not expose ACES or Reinhard as runtime
  options; grade through tokens instead [FOU-S004] [FOU-S011].
- Keep one manual exposure token (`ev100`) with named presets
  (sunlight, overcast, indoor equivalents). No auto-exposure in version
  one [FOU-S009] [FOU-S012].
- Keep emissive exposure weight at zero so exposure moves all rungs
  together without regrading glow [FOU-S005].

## R-FOU-2: name colour tokens by role, not by value

- Reference tier: raw per-class tints (O to M stars, galaxy kinds, tail
  rungs) as sRGB authoring values.
- Semantic tier: `shell`, `child`, `context-floor`, `outline`, `focus`
  assignments that alias reference tints. Names stay stable while values
  change [TOK lineage; FOU-S007].
- Never reuse a semantic colour for an unrelated role; add light, dark,
  and high-contrast discipline even though the game ships on black first.

## R-FOU-3: set a line-width floor and a point-size rule

- Minimum gizmo width token at or above the 2.0 px default; no 1 px
  meaning-carrying lines on phones.
- Point-size token as an explicit world-size or pixel-derived rule with a
  minimum on-screen diameter at the form threshold, because Bevy provides
  no engine point size [FOU-S008].
- Validate every tint at its floor weight against black for 3:1 non-text
  contrast; thin plus faint is rejected (see C3 in the
  [topic critique](../../documents/critique.md)).

## R-FOU-4: space the brightness ladder in stops

- Shell crosses the bloom threshold; children sit near it; context floor
  rests at least two stops below children so it never competes.
- Drive the steps through emissive values, not bloom intensity; leave
  prefilter at defaults [FOU-S001] [FOU-S003] [FOU-S010].
- Record the ladder as named constants in the tokens file (see
  [tokens](../../tokens/documents/README.md)) so `style.rs` stays the
  single consumer, never the definition site.

## Validation

- Analyse each tint with a contrast analyser in linear luminance, not by
  eye [FOU-S006].
- Screenshot each rung on a phone OLED at minimum and maximum exposure
  before freezing values.
- Proof dim reds under protanopia simulation; reject any red that reads
  as black (see [accessibility](../../accessibility/documents/README.md)).

## References

- [FOU-S001](./sources.md#fou-s001--bevy-bloom) through
  [FOU-S013](./sources.md#fou-s013--accepted-project-baseline)

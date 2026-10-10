# HUD and typography: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. HUD taxonomy and meter precedent

Game interfaces split into diegetic (in-world objects), non-diegetic
(screen overlay), and spatial or meta hybrids [HUD-S001]. Astrolith scale
and survival state need the immediacy of non-diegetic overlay. The Game
UI Database catalogues over 1,300 games and 55,000 screenshots by screen
type, with a Player Vitals category isolating health, magic, stamina, and
special bars: the direct precedent library for multi-meter treatments,
filterable to touch-screen examples [HUD-S002]. Fullerton treats the HUD
as part of the player-experience loop to be playtested against experience
goals, not as an art layer [HUD-S003]. Robertson argues for maximal
readability from minimal visual elements through hierarchy and motion
[HUD-S004].

## 2. Meters must not rely on colour alone

WCAG SC 1.4.1 forbids colour as the sole means of conveying information
or distinguishing an element; the fix adds text, shape, icons, or a
lightness difference [HUD-S005]. Body text needs 4.5:1 and large text
(at least 18 pt regular or 14 pt bold) needs 3:1 [HUD-S005]. UI
components and meaning-carrying graphics need 3:1 against adjacent
colours [HUD-S005]. For meters this means each bar carries an icon, a
number, and a fill, with alert states adding a glyph or word rather than
a hue shift alone.

## 3. Scale bars are dynamic and labelled

Cartographic convention divides the bar into labelled parts showing
ground units for the current view extent; the bar recomputes as the view
changes because zoom alone does not convey scale [HUD-S006]. Reference
implementations pair the bar with extent readouts. A static bar or an
unlabelled logarithmic decoration does not meet the convention.

## 4. Mobile type floors

Apple sets the iOS and iPadOS default body at 17 pt with a minimum of
11 pt, warns against thin weights at small sizes, and directs designers
to test legibility per device and raise size, contrast, or weight when it
fails [HUD-S007]. Material defines a systematic type scale (display,
headline, title, body, label, with body at 16 sp, caption at 12 sp) with
role-based line heights and weights rather than ad-hoc sizes [HUD-S008].
Both systems ship scalable text that reaches 200 percent and more without
layout loss.

## 5. Tabular numerals stop jitter

Proportional figures change width per digit, so rapidly changing numbers
dance sideways. Tabular figures (fixed-width digits, exposed in SwiftUI
as `monospacedDigit` and in CSS as `font-variant-numeric: tabular-nums`)
give every digit the same advance width and hold columns still
[HUD-S009]. They differ from a full monospace face: only the digits are
fixed, labels keep their designed proportions.

## 6. Contrast on black and thin-text risk

The same ratios apply on black: 4.5:1 body, 3:1 large and UI graphics
[HUD-S005]. Thin or anti-aliased light-on-black text can measure a pass
while rendering faint, so semibold or medium weights with margin above
the minimum are preferred [HUD-S007 lineage]. Pure red on black is
additionally risky under protanopia (see
[accessibility](../../accessibility/documents/README.md)).

## 7. Minimal chrome and thumb reach

Frequent actions belong in the bottom-third thumb arc on large phones;
top corners require a grip shift and suit read-only status [HUD-S010].
Portrait and landscape need separate layouts; one stretched layout breaks
reach or density [HUD-S010]. Touch targets stay at 44 to 48 px minimum
with status outside the input arc [HUD-S010].

## References

- [HUD-S001](./sources.md#hud-s001--hud-taxonomy) through
  [HUD-S010](./sources.md#hud-s010--mobile-layout-patterns)

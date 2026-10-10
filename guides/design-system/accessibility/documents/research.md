# Accessibility: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. Contrast minima

Text and images of text need 4.5:1, except large text (at least 18 pt
regular or 14 pt bold, about 24 px or 18.5 px) which needs 3:1;
decorative, inactive, and logotype text are exempt [ACC-S001]. The
rationale compensates roughly 20/40 vision and ageing contrast loss; hue
alone is not relied on [ACC-S001]. User-interface components and
graphical objects required to identify controls and states need 3:1
against adjacent colours, including borders, icons, focus indicators, and
chart lines required for understanding [ACC-S002]. Thin lines and shapes
render fainter than nominal colour through anti-aliasing, so particularly
thin lines should be avoided or held above the minimum [ACC-S002].
Material mirrors the same numbers for small text, large text, icons, and
interactive states [ACC-S003].

## 2. Target and focus minima

Pointer targets must fit 24 by 24 CSS px or carry spacing so that 24 px
circles centred on undersized targets do not intersect; best practice
meets the size regardless, and important controls aim for 44 by 44
[ACC-S004]. A visible focus indicator is required wherever keyboard or
controller focus exists; the indicator must contrast at 3:1 adjacent, and
the strict appearance level asks for at least a 2 px perimeter with a 3:1
change between focused and unfocused pixels, never obscured [ACC-S005].

## 3. Colour must not be the only code

Colour must not be the only visual means of conveying information,
indicating an action, or distinguishing an element; compliant designs add
shape, text, pattern, or a sufficient lightness difference [ACC-S006].
The rule benefits colour-blind users, ageing users, and monochrome
displays alike.

## 4. Colour-vision deficiency prevalence and safe sets

About 1 in 12 men have colour-vision deficiency; the most common type is
red-green, blue-yellow is rare, and total colour blindness is rarest
[ACC-S007]. Red-green pairs of similar lightness are indistinguishable to
protanopes and deuteranopes; protanopes see dark red as near-black, so
red on black fails [ACC-S008]. The remedy replaces red with vermilion
(yellowish red) or magenta (red plus blue, whose blue channel survives),
pairs magenta with green or blue with vermilion, uses thick lines and
bold, codes redundantly with shape, line type, and labels, and labels
directly rather than through a distant key [ACC-S008]. Wong popularises a
seven-colour subset (blue, vermilion, bluish-green, yellow, sky blue,
reddish-purple, plus black excluded) with blue `#0072B2` plus vermilion
`#D55E00` as the anchor pair, encoded on the intact blue-yellow axis
[ACC-S009]. Tol optimises categorical schemes by CIEDE2000 distances
under simulated protan and deutan vision, publishing bright, vibrant,
muted, and high-contrast families with hex values and simulation proof,
plus a dark scheme holding 4.5:1 on white [ACC-S010].

## 5. Motion sensitivity

Motion animation triggered by interaction must be disableable unless
essential to function or information; parallax and non-essential
scroll-linked motion are named examples, and vestibular reactions include
nausea, headache, and need for bed rest [ACC-S001-motion lineage;
ACC-S011]. Auto-starting motion needs pause, stop, or hide [ACC-S011].
Apple requires honouring the system Reduce Motion setting (disabling or
replacing parallax, zoom, and non-essential animation) and exposes it
through `isReduceMotionEnabled` [ACC-S011]. Game Accessibility Guidelines
ask for an option to turn off or hide background movement, to decouple or
disable camera bob, sway, blur, and smoothing differences, to choose a
field of view appropriate to the viewing distance with adjustment, and to
keep the camera player-controlled with a maintained horizon [ACC-S011].
Xbox guideline 117 requires pause, stop, or hide for co-present moving
content with frequency control, an opaque background option behind text
over moving scenes, and settings for field of view, bob, shake, blur,
additive motions, auto-centring, and sensitivity [ACC-S011].

## 6. Proofing tools

Coblis simulates protanopia, deuteranopia, tritanopia, and monochromacy
from uploaded images; Stark adds contrast checking with CVD simulation in
design tools and browsers [ACC-S010-tools lineage]. Both find
indistinguishable pairs; neither replaces testing with colour-blind
players. Proof images judge pairs, not aptitude.

## References

- [ACC-S001](./sources.md#acc-s001--contrast-minimum) through
  [ACC-S011](./sources.md#acc-s011--motion-sensitivity)

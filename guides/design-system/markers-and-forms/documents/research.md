# Markers and forms: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. Point, line, and area symbols

Bertin defines retinal variables (position, size, shape, value, hue,
orientation, texture) applied to three symbol dimensionalities: point,
line, and area. Size and value encode order and quantity; shape and hue
encode nominal category only [MRK-S001]. Esri practice agrees: shape
implies qualitative difference, size implies quantitative difference, with
hue plus shape preferred for qualitative codes and size, value, or
lightness for emphasis [MRK-S002]. USGS cartographic specifications define
every feature as a point, linear, or polygon depiction with explicit
stroke and fill rules, including outline-only polygons [MRK-S003].

## 2. Halos keep symbols legible over dense linework

Esri halo guidance states that labels and symbols over dense linework need
a halo whose colour, width, opacity, and blur stay effective but
invisible: present enough to separate the mark from its background,
restrained enough not to read as a shape of its own [MRK-S003]. The same
principle covers selection rings and portal marks over starfields.

## 3. Colour must never be the only code

WCAG SC 1.4.1 requires that colour not be the only visual means of
conveying information, indicating an action, prompting a response, or
distinguishing a visual element. An acceptable fix adds shape, text,
pattern, or a lightness difference of at least 3:1 [MRK-S004]. TheBarcode
beneficiaries named include colour-blind users, ageing users, and
monochrome displays. For markers this mandates shape plus label plus
brightness redundancy for every nominal distinction.

## 4. Semantic zoom and portals

Pad and Pad++ define semantic zooming: objects change how they look with
size, view complexity, and task, not just geometric magnification, with
simplified rendering far away and detail nearby [MRK-S005]. Portals are
objects that see elsewhere on the surface and filter or re-represent data
differently, the lens metaphor [MRK-S005]. Furnas generalises the
selection rule as degree of interest: display items whose a priori
importance minus distance to focus exceeds a threshold, which explains why
far views show only landmarks [MRK-S006]. Furnas and Bederson make scale
an explicit axis on which pan-zoom trajectories and semantic boundaries
can be drawn and reasoned about [MRK-S007].

## 5. LOD cross-fade by screen size and hysteresis

Unity selects level of detail by screen-size ratio and smooths the switch
with a cross-fade band (a fraction of the range before the threshold) or
a fixed-time animation after it [MRK-S008]. Cesium 3D Tiles refines while
screen-space error exceeds a budget derived from geometric error,
distance, and resolution, with larger errors at the root refining toward
zero at leaves [MRK-S009]. A single-threshold comparator oscillates on
slow or noisy input; a Schmitt trigger switches high at an upper
threshold and low at a lower one, and the gap guarantees clean edges
[MRK-S010]. Together these state the standard answer: fade by size,
hysterese the gate.

## 6. Flat two-tone silhouettes read small and cost little

Thomas Was Alone builds every character as a simple coloured rectangle
with distinct size and ability, carrying personality through narration and
motion rather than modelling, in a Bauhaus-derived minimalism
[MRK-S011]. Alto's Adventure builds assets as flat-shaded geometry that
scales crisply to any resolution, saves texture memory, and reads on
mobile with subtle parallax [MRK-S012]. Kingdom Two Crowns keeps a
minimalist feel in a modern pixel-art silhouette where the viewer fills
detail, chosen as the economical path from sketch to engine [MRK-S013].
Unity documents the unlit shader as the simplest shading model with no
lighting lookups, optimal for lower-end hardware, with opaque preferred
over transparent to limit overdraw [MRK-S014].

## References

- [MRK-S001](./sources.md#mrk-s001--bertin-visual-variables) through
  [MRK-S014](./sources.md#mrk-s014--unity-unlit-shader)

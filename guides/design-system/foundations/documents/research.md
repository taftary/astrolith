# Foundations: research

Facts only. Interpretations for Astrolith live in
[analysis](./analysis.md); proposals live in
[recommendations](./recommendations.md). Source records with URLs and
limits live in [sources](./sources.md).

## 1. sRGB authoring versus linear rendering

Authoring tools and CSS-style hex values describe colour in gamma-corrected
sRGB. Lighting, emissive math, and bloom thresholds operate in linear
light. The W3C contrast documents give the normative sRGB to linear
transfer and the relative-luminance weights, and note the formula derives
from IEC 61966-2-1 [FOU-S006]. Bevy splits the two spaces explicitly:
`Srgba` for authoring, `LinearRgba` for lighting, with shortest-path
conversion between them, and Oklch as the perceptual intermediary for hue
and contrast operations [FOU-S007].

Consequence used elsewhere: store token values as sRGB, convert to linear
before any brightness math, and do contrast checks in linear luminance.

## 2. HDR emissive and bloom threshold

Bevy cameras need an `Hdr` intermediate texture for bloom to function;
this is not display HDR, only the render path [FOU-S002]. Bloom itself
carries intensity, threshold, softness, and composite controls. The
reference advises making a mesh glow by raising its emissive value rather
than the bloom intensity [FOU-S001], and leaving the prefilter threshold
at defaults because non-default curves are physically inaccurate and easy
to make worse [FOU-S003].

Emissive colours may exceed 1.0 per channel (for example values in the
hundreds for intense red); magnitude is strength and channel ratios are
hue. Emissive adds to lit colour so surfaces read in darkness, but does
not light neighbours like a light source [FOU-S005]. Filament, which
Bevy cites for its exposure formula, treats emissive in linear with
exposure compensation and runs bloom on pre-tonemap HDR luminance, so only
super-bright pixels scatter halos [FOU-S010].

## 3. Filmic tonemap and manual exposure

Tonemapping maps linear stimulus to a perceptually uniform image per
camera. Bevy 0.19 offers several operators; TonyMcMapface is the default,
described as neutral with subtle intentional hue shifting where brights
desaturate, and it needs the `tonemapping_luts` cargo feature. The
Reinhard variants shift hue with brights that do not desaturate naturally
[FOU-S004]. The operator author describes the design goal as
intentionally boring: no contrast or saturation boost, near-identity where
compression is unneeded, with look changes applied before or after the
transform rather than inside it [FOU-S011].

Exposure is a single `ev100` value per camera, defined as absorbed energy
with photographic presets (sunlight 15.0, overcast 12.0, indoor 7.0),
converted to a multiplier by the Filament formula [FOU-S009]. One stop is
a doubling of light; EV100 ties scene illuminance to the ISO-100 scale on
a log2 basis [FOU-S012]. Unreal documents the same manual-metering form
with an added compensation term, confirming manual exposure as an
industry-standard mode [FOU-S012].

## 4. Line weight and point size legibility

Bevy gizmo line width is specified in pixels with a 2.0 default, with an
optional perspective mode that interprets width at the camera near plane
for 3D non-orthographic views. Style, joints, and perspective are separate
flags [FOU-S008]. There is no engine point size: points are small meshes
or billboards, so size must be a world-size or pixel-derived application
token [FOU-S008].

WCAG Understanding for non-text contrast requires at least 3:1 for
meaning-carrying graphics against adjacent colours and explicitly warns
that thin lines and shapes render fainter than nominal colour through
anti-aliasing, advising against particularly thin lines [FOU-S010 in
analysis cross-check; primary record FOU-S007 lineage]. Apple sets the iOS
minimum text size at 11 pt and warns against thin weights at small sizes
[FOU-S013 lineage via HUD research; kept here as a floor reference].

## 5. Token-format precedent

The W3C Design Tokens Format Module (first stable, October 2025) defines
a token as a human-readable name plus a value with type and description;
names are case-sensitive dot-joined paths and values may alias other
tokens [TOK-S001, recorded in tokens sources; summarised here to justify
the shape]. Style Dictionary documents the same name, value, type, and
alias model with per-platform transforms from one dictionary [TOK-S002].
Material Design 3 organises tokens in reference, system, and component
tiers with role-first names that stay stable while values change
[TOK-S003]. Apple semantic colours are named by purpose rather than
appearance and adapt to light, dark, and contrast contexts [TOK-S004].

## References

- [FOU-S001](./sources.md#fou-s001--bevy-bloom)
- [FOU-S002](./sources.md#fou-s002--bevy-hdr-camera)
- [FOU-S003](./sources.md#fou-s003--bevy-bloom-prefilter)
- [FOU-S004](./sources.md#fou-s004--bevy-tonemapping)
- [FOU-S005](./sources.md#fou-s005--bevy-emissive-material)
- [FOU-S006](./sources.md#fou-s006--srgb-transfer-and-relative-luminance)
- [FOU-S007](./sources.md#fou-s007--bevy-colour-spaces)
- [FOU-S008](./sources.md#fou-s008--bevy-gizmo-line-config)
- [FOU-S009](./sources.md#fou-s009--bevy-exposure)
- [FOU-S010](./sources.md#fou-s010--filament-hdr-and-exposure)
- [FOU-S011](./sources.md#fou-s011--tony-mcmapface)
- [FOU-S012](./sources.md#fou-s012--exposure-stops)
- [FOU-S013](./sources.md#fou-s013--accepted-project-baseline)

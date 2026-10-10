# Tokens: research

Facts only. Interpretation in [analysis](./analysis.md); proposals in
[recommendations](./recommendations.md). Records in
[sources](./sources.md).

## 1. W3C token format

The W3C Design Tokens Format Module (first stable, October 2025) defines
a token as a human-readable name plus a value with type and description.
Names are case-sensitive dot-joined paths; values may alias other tokens
(for example an accent token aliasing a palette token); types
disambiguate colour, dimension, and other kinds; translation tooling
generates platform code from the single source [TOK-S001].

## 2. Style Dictionary translation precedent

Style Dictionary treats tokens as platform-agnostic name and value
records (value required; type, comment, themeable flags, and name metadata
optional) organised in nested category, type, and item trees. References
alias values by path; the build resolves and transforms one dictionary
into per-platform outputs while preserving pristine originals plus path
and file metadata [TOK-S002].

## 3. Material three-tier naming

Material Design 3 organises tokens in reference (palette primitives),
system (role decisions), and component (element assignments) tiers. Token
names are code-like, encode system, class, and role, may hold a literal
or another token as value, and stay stable while values change [TOK-S003].

## 4. Apple semantic colours

Apple dynamic and semantic colours are named by purpose (label, system
background, separator, tint), not appearance, and auto-adapt to light,
dark, contrast, and vibrancy contexts. Guidance forbids repurposing a
semantic colour and asks custom colours to ship light, dark, and
high-contrast variants holding 4.5:1 (7:1 for small custom text)
[TOK-S004].

## 5. Bevy constant mapping

Bloom settings (intensity, threshold curve, composite mode) apply per
camera on the HDR path; emissive strength lives in per-material linear
values that may exceed 1.0; tonemapping is a per-camera operator with
TonyMcMapface as default needing the LUT feature; exposure is a per-camera
`ev100` value; gizmo line width is pixels with a 2.0 default; text size is
pixels per face-plus-size atlas handle [TOK-S005] [TOK-S006] [TOK-S007]
[TOK-S008] [TOK-S009]. Each maps to a typed Rust constant family rather
than a stylesheet rule.

## References

- [TOK-S001](./sources.md#tok-s001--w3c-design-tokens-format) through
  [TOK-S009](./sources.md#tok-s009--bevy-text)

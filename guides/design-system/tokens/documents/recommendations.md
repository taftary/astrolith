# Tokens: recommendations

Proposals for a future specification and the owner. Facts in
[research](./research.md); interpretation in [analysis](./analysis.md).

## R-TOK-1: keep three tiers with role-first stable names

- Reference: `colour.<class>` palette primitives as sRGB values with
  descriptions and CVD notes [TOK-S001] [TOK-S003].
- Semantic: `colour.<role>` assignments (`shell`, `child`,
  `context-floor`, `outline`, `focus`, meter roles) aliasing reference
  entries, never repurposed across roles [TOK-S003] [TOK-S004].
- Component: `size.*`, `time.*`, `type.*` assignments per element
  (marker width, point size, meter bar, HUD font size, fade and flight
  durations) [TOK-S003].
- Every token carries name, typed value, and description; aliases point
  at paths, never duplicate literals [TOK-S001] [TOK-S002].

## R-TOK-2: map each family to its Bevy type once

- Colours as sRGB constants converted to linear at use; emissive ladder
  as linear strengths above 1.0; exposure as `ev100` with presets;
  tonemap fixed to TonyMcMapface; widths in pixels; font sizes in pixels
  in a frozen set of three to four; durations in milliseconds per kind
  [TOK-S005] [TOK-S006] [TOK-S007] [TOK-S008] [TOK-S009].
- Never expose two constants for one role; never read a literal where a
  token exists.

## R-TOK-3: sync one way with tests and screenshots

- Guides specify; `style.rs` implements; nothing else defines tokens.
  Record the mapping table in the specification so reviewers can check
  every token has exactly one constant.
- Add naming tests (every semantic role resolves to a reference entry),
  ordering tests (shell above children above floor; exits shorter than
  entrances), and floor tests (minimum widths, sizes, ratios held).
- Screenshot every rung and meter at minimum and maximum exposure plus a
  CVD simulation before freezing values (see
  [accessibility](../../accessibility/documents/README.md)).

## Validation

- Grep the renderer for colour, size, and duration literals outside the
  token file; any hit is drift.
- Re-run ordering and floor tests on every visual pull request.
- Confirm the font set builds within atlas budget on device.

## References

- [TOK-S001](./sources.md#tok-s001--w3c-design-tokens-format) through
  [TOK-S009](./sources.md#tok-s009--bevy-text)

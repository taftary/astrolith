# Critique: where research questions the draft and the ADRs

Scope: every place where the research in this topic contradicts or
questions a line of `drafts/game-style.md` or an accepted ADR. Each entry
quotes the exact draft line or ADR clause concerned, states the research
finding, and names the subtopic that holds the evidence. Nothing here
edits the draft or any ADR. The owner decides whether a recheck or an
amendment follows.

Read `drafts/game-style.md` (2026-10-09) and ADRs `0002`, `0015`, `0016`,
`0017`, `0019` as the baseline. Evidence lives in the subtopic
`research.md` and `sources.md` files cited per entry.

## C1 — Red-green-only meter coding fails colour-blind users

- **Draft line:** "One vessel, four meters: fuel, oxygen, hull, heat.
  Each drains with distance travelled and with each level crossed; hazards
  drain them faster." (`drafts/game-style.md`, Gameplay, Spine, bullet 1).
- **Concerned style line:** "HUD is part of the look: scale bar, level
  label, distance, the four meters. Strong typography, minimal chrome."
  (Visual style, bullet 3).
- **Finding:** about 1 in 12 men have colour-vision deficiency, most
  commonly red-green (National Eye Institute). WCAG SC 1.4.1 forbids colour
  as the only visual means. A heat meter that shifts green to red, or a
  hull meter that relies on red versus green state, is unreadable to
  protanopes and deuteranopes, and dark red on black collapses to
  near-black (Okabe and Ito).
- **Evidence:** [Accessibility research](../accessibility/documents/research.md),
  sources `ACC-S001`, `ACC-S002`, `ACC-S003`; also
  [HUD research](../hud-and-typography/documents/research.md) `HUD-S005`.
- **Question for recheck:** confirm each meter carries icon plus number
  plus bar, and that alert states add shape or text (for example an
  exclamation glyph or the word LOW), never a hue shift alone.

## C2 — Pure red on black is a double failure

- **Draft line:** "The game looks the way the prototype already looks: a
  luminous abstract instrument, glowing points and lines for everything
  cosmic" (`drafts/game-style.md`, Resume).
- **ADR clause:** ADR 0015 tints "every gizmo marker from its core
  `ColorInfo` (stars by O-M class, galaxies by kind and density; planets
  keep their biome colors)".
- **Finding:** protanopes see dark red as near-black, so dim red markers
  or red text on a black sky vanish even when the nominal colour looks
  distinct to trichromats (Okabe and Ito). WCAG non-text contrast further
  warns that thin anti-aliased lines render fainter than their nominal
  colour. A dim M-class red star drawn as a 1 px gizmo line at low
  brightness can therefore fail twice.
- **Evidence:** [Foundations](../foundations/documents/research.md)
  `FOU-S010`; [Accessibility](../accessibility/documents/research.md)
  `ACC-S004`, `ACC-S005`.
- **Question for recheck:** confirm the per-class tint table avoids pure
  or dark red at low brightness steps, and that the brightness floor keeps
  every tint above 3:1 against black at its drawn weight.

## C3 — Thin lines need a minimum weight and a contrast floor

- **Draft line:** "Cosmic levels (L1 to L9): glowing points, lines and
  outlines only, with the per-class tints, emissive billboards, bloom and
  filmic tonemap already in place. No meshes." (Visual style, bullet 1).
- **ADR clause:** ADR 0002, "draws indicators only (axes, points, lines
  via gizmos)"; ADR 0017, "Rendering maps forms to gizmo primitives".
- **Finding:** WCAG Understanding for SC 1.4.11 explicitly warns that thin
  lines and shapes appear fainter than their nominal colour because of
  anti-aliasing and advises avoiding particularly thin lines. Bevy gizmos
  default to 2.0 px width with an optional perspective mode; there is no
  engine point size, so dots need an explicit world-size or pixel-derived
  token. The draft and the ADRs name no minimum weight.
- **Evidence:** [Foundations](../foundations/documents/research.md)
  `FOU-S007`, `FOU-S008`.
- **Question for recheck:** confirm a minimum line-width token and a
  minimum point-size token exist, with the context floor still legible at
  that weight.

## C4 — Pinch and tilt need single-pointer and no-motion alternatives

- **Draft line:** "Tap to target, pinch to dive in and out, tilt or a
  virtual stick to evade during transit. One thumb must be enough for the
  spine." (Controls and sessions, bullet 1).
- **Finding:** WCAG SC 2.5.1 requires a single-pointer alternative for
  every multipoint or path-based gesture; SC 2.5.4 requires a user
  interface alternative for motion actuation such as tilt. A pinch-only
  dive and a tilt-only evade would fail both. The draft already names a
  virtual stick as an alternative to tilt, which satisfies 2.5.4 if either
  input alone suffices, but pinch still needs a button or tap-hold dive
  alternative to satisfy 2.5.1.
- **Evidence:** [Touch research](../touch-and-input/documents/research.md)
  `TCH-S007`, `TCH-S008`.
- **Question for recheck:** confirm the specification adds an explicit
  single-pointer dive control (button or double-tap-hold) alongside pinch.

## C5 — Tap targets in dense starfields need padding and spacing

- **Draft line:** "Dive loop: tap a marker to target it, dive toward it in
  real time" (Gameplay, Spine, bullet 2).
- **ADR-adjacent fact:** `PICK_PIXELS = 24.0` in `style.rs` is the current
  hover and click radius.
- **Finding:** Apple requires at least 44 by 44 pt hit regions, Material
  recommends 48 by 48 dp with at least 8 dp separation, and WCAG 2.5.8 sets
  a 24 by 24 CSS px floor. The current 24 px pick radius meets only the
  WCAG floor. In a dense field, neighbouring 24 px circles overlap unless
  spacing or disambiguation is specified.
- **Evidence:** [Touch research](../touch-and-input/documents/research.md)
  `TCH-S001`, `TCH-S002`, `TCH-S003`.
- **Question for recheck:** confirm the touch specification pads small
  markers to 44 pt or 48 dp hit areas with a spacing or cycling rule for
  overlaps, rather than treating 24 px as the shipped target.

## C6 — HUD numerals below 11 pt equivalent will fail legibility

- **Draft line:** "HUD is part of the look: scale bar, level label,
  distance, the four meters. Strong typography, minimal chrome."
  (Visual style, bullet 3).
- **Finding:** Apple HIG sets 11 pt as the iOS minimum text size and warns
  against thin weights at small sizes. Meter numbers, distance readouts,
  and scale-bar labels drawn smaller, or in proportional figures that
  jitter as digits change, will be unreadable on a phone at arm's length.
  The draft names no minimum size and no numeral policy.
- **Evidence:** [HUD research](../hud-and-typography/documents/research.md)
  `HUD-S006`, `HUD-S007`, `HUD-S008`.
- **Question for recheck:** confirm a minimum type size token at or above
  the 11 pt equivalent and tabular figures for every changing number.

## C7 — Planet biome tints are stale after ADR 0016

- **ADR clause:** ADR 0015, "planets keep their biome colors".
- **Finding:** ADR 0016 supersedes ADR 0014 and narrows the planet-colours
  clause of ADR 0015: "`universe-core` drops the whole climate layer" and
  the open planet builds "one flat-tone unlit body (`PLANET_TONE`)".
  Any guide text that still describes per-biome planet colours contradicts
  the accepted stack.
- **Evidence:** [Foundations](../foundations/documents/research.md)
  `FOU-S013`; ADR 0016 text.
- **Question for recheck:** none for the draft (the draft already says
  "flat two-tone silhouettes for surfaces"). This entry guards the new
  topic against repeating the superseded clause.

## C8 — Fixed Powers-of-Ten pacing must not become the dive pacing

- **Draft line:** "References: the current build, Powers of Ten, Out
  There, Kingdom Two Crowns, FTL." (Visual style, bullet 4).
- **Finding:** Powers of Ten moves at one power of ten per 10 seconds
  outward and about one per 2 seconds on the return. That pacing serves a
  9-minute narrated film. A playable dive needs player-controlled,
  distance-scaled, interruptible flights (Cesium `flyTo` auto-duration,
  van Wijk and Nuij optimal zoom-pan, Material duration bands). Copying
  the film rate would make long legs tedious and short legs rushed.
- **Evidence:** [References](../references/documents/research.md)
  `REF-S001`; [Motion](../motion/documents/research.md) `MOT-S007`,
  `MOT-S008`, `MOT-S009`.
- **Question for recheck:** confirm the motion specification treats Powers
  of Ten as a scale-narration model only, with its own duration and easing
  tokens.

## References

- [Foundations sources](../foundations/documents/sources.md)
- [Markers and forms sources](../markers-and-forms/documents/sources.md)
- [Motion sources](../motion/documents/sources.md)
- [HUD and typography sources](../hud-and-typography/documents/sources.md)
- [Touch and input sources](../touch-and-input/documents/sources.md)
- [Accessibility sources](../accessibility/documents/sources.md)
- [Tokens sources](../tokens/documents/sources.md)
- [References sources](../references/documents/sources.md)

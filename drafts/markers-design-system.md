# Markers follow the design system

Author: Claude (agent-authored draft, requested by the owner in session on 2026-10-10). Scope chosen by the owner: markers plus the tap-target rules from #444; the tone curve stays as it is. The content comes from the `guides/design-system/` recommendations (PR #442); the wording is the agent's.

## Resume

Bring every marker on screen in line with the new design system before any game work (#444, on hold) builds on them. Markers keep the indicators-only look (ADR 0002, 0017). What changes is how readable they are: no line or dot too thin or too dim to see, no colour that disappears for colour-blind players, portals that can be told apart from populations at a glance, a selection that does not rely on colour, and a pick that hits what you meant in a crowded field.

## Why

- The design-system research (`guides/design-system/documents/critique.md`, C2, C3, C5) found that thin anti-aliased lines read fainter than their colour, that dim red on black vanishes for protanopes, and that the 24 px pick radius meets only the minimum target size.
- Markers are the first thing every later step uses: the game spine taps them, the HUD labels them, hazards sit among them. Fixing them once, with named values, means later work reuses the same values instead of inventing new ones.

## What the owner wants

### Visibility floors

- A minimum line width for every marker line, at or above 2 px; no meaning-carrying 1 px lines.
- A minimum on-screen size for every dot, so the smallest marker stays visible at the form threshold.
- Every marker tint at least 3:1 contrast against black at its dimmest drawn brightness. Dark reds are replaced by a lighter orange-red at low brightness, so red dwarfs and red tints never read as black.

### Brightness ladder

- Three named steps: the entered body (shell) is brightest, children sit just below it, and the context floor sits at least two stops under the children so it never competes.

### Portals versus populations

- Every portal carries three cues: its body form, a portal mark at the child's true position, and a text label. Populations stay plain dots, without a mark or a label.
- The difference must survive in black and white: if the picture is shown without colour, portals and populations are still told apart by shape.

### Shape and emphasis

- Each level keeps one family of shapes, so the code is learned once. Shape says what kind of thing a marker is; size and brightness say how important it is. Importance is never shown by changing the shape.

### Selection and hover

- The selected marker gets an outline ring at the minimum line width plus a halo around its label. Hover is a brighter version of the same ring. Neither is shown by a colour change alone.

### Picking (tap targets from #444)

- On touch, the pick area is 44 pt (iOS) / 48 dp (Android). With the mouse, 24 px stays the minimum.
- When pick areas overlap, the most important marker wins: the portal on the way to the destination first (#423), then the nearest. Clicking or tapping again on the same spot cycles to the next marker under it.

### Named values

- Every value above (line width, dot size, tints, brightness steps, pick sizes) is a named value in one place, named by role (shell, child, context floor, outline, focus) rather than by colour. `style.rs` uses them; it no longer defines them.

## Not in scope

- The tone curve and exposure stay as ADR 0015 decided (AcesFitted, no LUT feature). The guide's suggestion to switch to TonyMcMapface needs its own notion and ADR.
- Touch input itself does not exist yet; only the pick rule and its touch size are set here, ready for when it arrives.
- The HUD, meters and hazards (#444).

## Relation to existing decisions

- ADR 0002 and 0017 (indicators only, body and portal forms) stay; this notion only adds floors, cues and names on top.
- ADR 0015's tint table is kept in spirit, with the dark-red values replaced where they fail the contrast floor.
- Determinism and goldens: these are render-side changes. The headless `--verify` output should stay byte-identical; capture pictures will change.

## Open questions for clarification

1. Labels: should every portal on screen carry a text label, or only portals above a size on screen (plus the selected and hovered one), to avoid clutter in dense fields?
2. What does a label say: the generated name, the level kind ("galaxy", "star"), or both?
3. Portal mark: one shape for every level (for example a small ring), or one per level?
4. Cycling on repeated clicks: does it cycle through every marker under the pointer, or only the portals?
5. Contrast floor: is a 3:1 floor acceptable even if it makes the dimmest context markers visibly brighter than today?

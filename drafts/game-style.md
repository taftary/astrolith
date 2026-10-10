# Game style: real-time expedition survival in a luminous abstract universe

Author: Claude (agent-authored draft, requested by the owner in session on 2026-10-09). The decisions below were chosen by the owner from a shortlist; the wording is the agent's.

Revised 2026-10-10 after a recheck against the `guides/design-system/` topic (PR #442, `guides/design-system/documents/critique.md`). The owner chose the recommended option for every point in session. The changes are the "Revised" notes below and the "Decisions after recheck" section, which replaces the open questions.

## Resume

Astrolith is a mobile-first, real-time survival and strategy game. You are one traveler in one vessel, flying down the universe ladder from the cosmic web to a single room with finite fuel, oxygen, hull and heat. Every dive is a live decision: where to go, what it will cost, what you risk on the way. Later, reaching a planet opens a "land and hold" phase where you gather, build and survive hazard cycles on the surface before leaving for the next system. The game looks the way the prototype already looks: a luminous abstract instrument, glowing points and lines for everything cosmic, flat two-tone silhouettes for surfaces, vessel and hazards.

## Why this style

- The core fantasy from #35 is "a single, unbroken universe you can travel through, a survival game". In this style the dive is the core verb, not a menu. The thing already built (continuous nested dive, target dive with pass-through, free flight) is the game, not a backdrop for it.
- Survival and strategy both come from one source: finite resources across distance. No separate combat or economy system is needed to make the first version play.
- Determinism and store-nothing generation make a run reproducible from seed plus decision log. Saves are tiny, seeds are shareable, suspend/resume is trivial.
- The indicators-only art direction (ADR 0002, 0014, 0015) is kept as the style rather than treated as a placeholder. It is the cheapest thing a phone GPU can draw at 60 fps and it stays readable on a small screen.

## Gameplay

### Spine: real-time expedition flight

- One vessel, four meters: fuel, oxygen, hull, heat. Each drains with distance travelled and with each level crossed; hazards drain them faster.
- Dive loop: tap a marker to target it, dive toward it in real time, pass through the portals on the way (as in ADR 0013), arrive in the next level. Dive out to retreat.
- Hazards are live during transit and must be dodged or endured: debris fields, radiation from hot stars, gravity wells near massive bodies, flares. The first playable ships two of them (see decisions below).
- Strategy is route planning across the ladder: which galaxy, which star, which planet is worth the detour; what to carry; when to turn back. Rich systems cost more to reach and give more.
- Resupply happens at the bottom of the ladder (planet surface, L10 and below) and at rare waypoints on the way. Running out anywhere else loses the leg: the run rewinds to the last resupply point (see decisions below).
- A run is a seed plus the decisions taken. Reaching a chosen destination is the win condition for a run; the first playable can use "reach a habitable planet from the home galaxy" as that destination.

### Later phase: land and hold

- Once L11 to L14 exist (regions, cities, buildings, rooms), landing on a planet opens a real-time surface loop: gather by day, build by night, survive escalating hazard cycles (storms, radiation waves, cold), extract what you came for, leave.
- Reference for the feel: Kingdom Two Crowns (one-input real-time build and survive). This phase is explicitly a second milestone, not part of the first playable.

### Controls and sessions (mobile first)

- Tap to target, pinch to dive in and out, tilt or a virtual stick to evade during transit. One thumb must be enough for the spine.
- Revised (touch): pinch stays the main dive, plus a one-finger dive (on-screen button or double-tap-hold) that works alone. Tilt and virtual stick are both full alternatives for evading; either alone is enough, with adjustable sensitivity and dead zone.
- Revised (tap targets): on touch, tap areas are 44 pt (iOS) / 48 dp (Android); 24 px stays the desktop minimum. When areas overlap, the most important marker wins (the portal on the way to the destination first, then the nearest), and tapping again cycles to the next one.
- Revised (layout): primary controls in the bottom thumb arc inside the safe area; status at the top; nothing that triggers system gestures.
- A leg between two resupply points is 5 to 20 minutes. The game pauses on suspend and resumes exactly where it was.
- Desktop keeps the existing mouse and keyboard controls for development and testing; touch is the design target.

## Visual style: luminous abstract

- Cosmic levels (L1 to L9): glowing points, lines and outlines only, with the per-class tints, emissive billboards, bloom and filmic tonemap already in place. No meshes.
- Surfaces, vessel and hazards: flat two-tone silhouettes and simple fills, extending the L10 planet body already allowed. No lighting model beyond unlit materials.
- Revised (visibility floors): a minimum line width, a minimum point size, and every tint at least 3:1 contrast against black at its dimmest. Dark red is replaced by a lighter orange-red at low brightness.
- HUD is part of the look: scale bar, level label, distance, the four meters. Strong typography, minimal chrome.
- Revised (meters): each meter has an icon, a number and a bar. A low state adds a shape or word ("LOW" or "!"), never a colour change alone.
- Revised (type): minimum text size equal to 11 pt on iOS; every changing number uses fixed-width digits; medium or bold weights only at small sizes.
- Revised (accessibility): follow the system Reduce Motion and text-size settings, with in-game equivalents.
- References: the current build, Powers of Ten, Out There, Kingdom Two Crowns, FTL. Revised: Powers of Ten is a model for presenting scale only, not for dive speed; the dive keeps its own player-controlled pacing.
- Explicit non-choices: photorealistic space sim (cost and scope), pixel art (clashes with continuous zoom), stylized low-poly meshes (would need an ADR superseding indicators-only, and there is no need yet).

## Relation to existing decisions

- #35 and its spec versions list "No gameplay" as a non-goal. This notion does not change #35; it is the next notion after the universe MVP and will get its own Issue.
- The owner decision "desktop-first dev and test until MVP, real-device Android and iOS from MVP" stands. Touch input and real-time hazards are designed for mobile and tested on desktop first.
- No pin or art-direction ADR needs to change for the spine. The land and hold phase may need one for surface silhouettes; that is decided when that phase is specified.

## Decisions after recheck (2026-10-10)

The six former open questions, answered with the recommended option (owner: "finish all with recommendations").

1. **Death model:** running out rewinds to the last resupply point. The run keeps its seed; what was gained since that point is lost. Legs are 5 to 20 minutes on a phone, so losing a whole run to one mistake is too harsh for the first playable. Permadeath can come later as an optional mode.
2. **Resources:** the four meters are the only resources in the first playable. There is no cargo; cargo and what it is for belong to the land-and-hold phase.
3. **Hazards and evade:** the first playable ships two hazards: debris fields (dodge) and radiation near hot stars (endure, drains hull and heat). Gravity wells and flares come later. Evade is tilt or virtual stick, either alone enough (see Controls).
4. **First playable scope:** hazards and resupply on L4 to L10 (galaxy to planet); L1 to L3 are the scenic approach.
5. **Progression:** every run is equal in the first playable. Unlocks and starting bonuses come later.
6. **Platforms:** built and tested on desktop first, per the owner decision "desktop-first dev and test until MVP". The touch design is accepted only after a real-phone test (one-hand reach, overlap in the densest field, no clash with system gestures), because desktop cannot prove it.

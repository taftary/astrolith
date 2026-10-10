# Continuous camera from planet to room

Author: Claude (agent-authored draft, requested by the owner in session on 2026-10-10 while re-testing #403). The decisions below were chosen by the owner in that session; the wording is the agent's.

## Resume

Navigating between dimensions still feels wrong on the planet-to-room leg (L10 planet, L11 region, L12 city, L13 building, L14 room) even after the #403 corrections made the camera's heading continuous across every open and close. The remaining causes are not transition bugs but missing camera behaviour: the picture rolls when the region frame tilts to the planet's surface, the dive can skim or pass through the planet and the ground, and the planet mesh, air rim, horizon, sky, limb and parent grid switch on and off at the open event instead of being there already. This notion gives the camera one model from the planet down to the room: a carried pose with a local up, a landing approach that bends toward the surface, ground clearance, and context that is previewed before entry, so that going from orbit to a room is one unbroken glide.

## Why

- #403 fixed what the approved spec allowed: the heading, up and travel ray now cross each frame exactly, a glide keeps its heading, the exit re-lock searches the right cell, and the close threshold has real slack. The owner still sees the planet-to-room leg turn bad; the study on #403 (round-2 comment) lists the causes left out of that spec as R4, R5, R7: roll on oriented entry, no ground or planet clearance, level-gated visuals that pop.
- The guides research in `guides/navigation/` (analysis G1 to G10, candidate requirements R1 to R12) records what globe viewers, space simulators and games do about exactly these problems: screen-up blended from global to local by altitude, a separate "land" phase at the end of a fly-to, terrain standoff, and transitions faded by size rather than switched by level.
- The R7 rule in `docs/universes/ladder.md` ("entering or leaving a dimension changes nothing on screen") holds today for marker points only. This notion extends it to everything drawn in the tail.

## What the owner wants

### Up and roll

- The camera's up is carried through every frame change (done in #403) and then settles toward a local up: the open frame's `+Y` everywhere except at L10, where it is the planet-local normal under the camera, blended in by altitude so that far from the planet the planet's axis is up and close to the surface the ground is. Entering a region from the planet then changes nothing about the roll: the carried up already is the region's up.
- Roll never steps. Any change of up is a rate-limited ease, never a snap, including the degenerate case of looking straight down at a region.

### Approach and landing (owner decision: bend to land)

- When a dive is locked on a surface portal (a region on the planet, a city or building on the ground), the final approach bends from the straight line toward the portal's surface normal, so the camera arrives above the region or building looking down at it with the local up already upright, instead of skimming it at a grazing angle. The straight line stays for everything that is not on a surface (galaxies, stars, planets from orbit).
- The bend starts late enough that aiming still feels direct and ends with the camera on the normal at the open distance.

### Ground and planet clearance

- In dive mode the camera never goes under the planet's relief at L10 nor under the ground at L11 to L13; a centre dive with nothing locked stops at a standoff above the surface instead of reaching the planet centre or the ground point.
- Free flight gets the same standoff (today it has none), unless clarification decides otherwise.

### Context that is there before you arrive

- What the next level draws around its content is visible inside the marker before entry and fades in by the marker's angular size, exactly like the children already do: the region's curved ground, horizon and sky inside the planet's region patch; the city's ground and sky inside the region; the building's outline and the room's shell further down. Leaving fades them out the same way.
- The parent's context persists in the child at the right scale: the planet limb and air seen from the region, the region ground seen from the city. The grid curvature previewed inside a marker is the true curvature at that scale (today it is drawn unscaled), and rectangles and boxes follow the patch orientation.
- The L10 planet body does not vanish at the open event; it stays as the limb context the child already draws, with no visible swap.

### Pacing on the tail

- The planet-to-room leg is 6.4 decades; the same rate per decade as the rest of the ladder is kept unless clarification decides the tail should run slower, as the guides pacing notes suggest.

## Relation to existing decisions

- #403 stays the fix for transition continuity; this notion starts after it is accepted or at the owner's word. Nothing here changes the #403 notion.
- Needs one ADR: the camera model (carried pose, local up, bend-to-land, clearance, context fades) supersedes the nearest-portal re-target of ADR 0013 and complements the ADR 0012 amendment made on #403. Indicators-only art direction (ADR 0002, 0016, 0017) is untouched: everything added is lines, rings and fades on what is already drawn.
- Determinism: the headless journey must stay byte-identical, or the change is explained under E-GOLDEN. The bend-to-land only applies to manual dives at surface portals unless clarification decides the autopilot should land too.

## Open questions for clarification

1. Landing height: how far above a region, city or building should the camera arrive at entry (today the open distance is about 3.6 cell units, looking at the centre)?
2. Should the autopilot (Spacebar journey) also bend to land, which changes the headless journey and the goldens, or keep its straight line?
3. Free flight: same ground standoff as the dive, or free to go anywhere as today?
4. At L13, building portals sit inside the building shell: approach from above the roof, or allow the camera to enter the shell?
5. Pacing: keep one rate per decade on the tail, or slow the last decades as Powers of Ten does on the way in?
6. Up policy at L10: blend by altitude (Google Earth style) or carry up and pull it toward the local normal at a bounded rate; the guides analysis leaves this open (G10).

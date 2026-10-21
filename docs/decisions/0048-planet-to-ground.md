# 0048 — Planet to ground: no water, real ground, height levels

- Status: accepted
- Date: 2026-10-21
- Issue: #892 (Spec v2)

## Context

The owner found the ground invisible close to it, especially in dive mode, and that it never came back while waiting. The planet-to-ground transition also needed work (#892 notion).

The check on main 52c1dfe at the start point (lat 40, lon 106.4) found four things:

- The start point is ocean, and no water is drawn. The drawn ground is the relief, here the sea floor about 4.3 km below the mean (old sea-level) radius.
- The #873 no-pass-through ball sat at that mean radius. A real dive (holding the up arrow from 100 m above the floor) was pushed up to 4,289 m above it, about 3 m above an undrawn sea, and stayed there.
- The player spawns on the sea floor (terrain plus 2 m, #799 Q5 A), about 4.3 km under that stop.
- The scale panel read "L10 - distance 10^7 m" at every height.

Owner answers:

- Q1 D: no oceans, lakes or water level for now.
- Q2 A: the old seas keep their shape as dry ground, and everything follows the real ground.
- Q3: A, B and D now; C (more ground detail) is set aside (Q8).
- Q4 B and Q5 A: neutral height levels L11-L14.
- Q6 A and Q7 C: pressing 1 lands you, and still flies to your body first.
- Q9 C: one ground colour.
- Q10 A: the start stays in the basin.

## Decision

- **No sea level for the ground (Q1 D, Q2 A).**
  - The avoid ball sits below the lowest possible ground (`lowest_ground_avoid_radius`: mean radius plus the relief's lower bound). It only keeps the camera out of the planet's core.
  - `clamp_above_ground` (relief-based, unchanged) does the real 3 m stop everywhere: dive, orbit and the player glide.
  - The atmosphere keeps its own mean radius as the air model's reference (density, haze). That is physics, not ground the player meets.
- **Gap from the real ground (Q3 B).** At L10 with the planet's centre as target, the dive and the orbit measure their gap from the ground under the camera (`ground_radius_cells`). Before, they measured it from the centre (radius 0), about 6,400 km.
  - `ground_dive_step` scales the height above the ground, so the dive eases down and rests at 3 m. It has no 1/1000-radius floor (the floor `dive_step` keeps, about 6 km here).
  - The orbit rate falls with the height above the ground.
- **One ground colour (Q9 C).** Every patch vertex, skirts included, takes `RELIEF_GROUND_TINT`, the moons' neutral gray, lit by the sun. The four #799 height-tint tokens are gone from the window; the core bands stay for headless tests.
- **Height levels on the panel (Q3 A, Q4 B, Q5 A).** At the planet the panel follows the rendered camera's height above the real ground, in every mode:
  - From 100 km: L11 "High above the planet".
  - From 1 km: L12 "Above the land".
  - From 10 m: L13 "Close to the ground".
  - Below 10 m: L14 "On the ground".
  - Each band shows "height ... above ground" and a scale: the height's power of ten, rounded.
  - Above 1,000 km and off the planet the panel is as before. These are names for heights; nothing opens below L10.
- **Landing (Q6 A, Q7 C).** No change was needed. Near the body, 1 takes it over where it stands, and the camera eases from the stop over the #754 handover. Far away, 1 flies to the body first.
- **Guides (G1, G2).** Help > Navigation opens with the panel's L11-L14 heights. The scale ladder lists the four height rows after L10.

## Tests

- `no_sea_level_for_the_ground`: a dive over a basin 3 km under the old sea level rests 3 m above its floor, below the old sea level.
- `dive_stops_above_the_real_ground`: rests at 3 m over land and over a basin, never inside on the way.
- `dive_slows_near_the_ground`: from 10 km every step is shorter than the last, with no overshoot.
- `ground_has_one_colour`: every vertex of the six root patches has the token colour.
- `ground_levels_follow_the_height` and `panel_follows_the_height`: the bands, names, height line and scale, including from a camera 500 m above the real ground.
- `one_lands_on_your_feet`: one press at the stop lands you, with the handover clock running and no frame covering more than half the move.
- `guides_explain_the_ground_levels`: the G1 opening and the G2 rows in order.
- The #873 test `dive_never_enters_a_planet` now reads "inside" as below the real ground.

## Consequences

- Dives and orbits over the old sea floors reach the ground. The start-point view is the basin floor up close, no longer 4.3 km away.
- Seen from space, the planet is one shaded colour.
- Water, terrain colours and ground detail near the camera remain future work.

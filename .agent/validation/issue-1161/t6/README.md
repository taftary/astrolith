# #1161 T6 (#1208): moons painted by type (Q7 A, Q8 A)

**Code:**
- **Type:** each moon gets its type from `world_type::classify_moon`, from its planet's orbit and star, its own seed and its size. The home moon is `Barren`.
- **Paint:** `world_type::MoonPaint` paints the ball once per open cell, as vertex colours.
  - **Base:** the type's palette.
  - **Seas:** dark seas from a low-frequency field (full on barren moons, faint on others).
  - **Craters:** 120 craters on the sphere, mostly small (size share cubed). Each has a darker floor inside a bright rim, with paler ejecta around.
- **Ball:** goes from 42 to 2,562 points (`MOON_SUBDIVISIONS` 4). The material is white, so the colour comes from the vertices.
- **Not walkable:** moons stay a painted ball with no ground (Q8 A).

**Tests:** `world_type_tests::moons_are_painted_with_seas_and_craters`.
- The home moon's tone spans seas and craters.
- It is grey, and an icy moon is paler.
- The paint is deterministic.
- More than 600 of 4,000 points carry a crater mark.

**Pictures:** `pictures/` (after) and `before-pictures/` (the before program on the same `moon.shots`), side by side in `moon-sheet.png`. Top row: before. Bottom row: after. Columns: from space, close up (cropped), from the planet.
- **Home moon:** now round instead of faceted, grey, with darker seas. Its crater rings are subtle in this half-lit view.
- **From the planet:** the moon is a dot in both.

**Found on the way (not changed here):** a moon's mesh stays where it was when its cell opened. A shot list that jumps back in time therefore looks at an empty place: the before program does the same. `moon.shots` opens the home cell with the moon shots first.

**Checks:** workspace tests and clippy pass.

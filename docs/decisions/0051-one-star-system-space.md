# 0051 - One star system in one space

- Status: accepted
- Date: 2026-10-26
- Issue: #995 (Spec v2)

## Context

The planetary system (L8), the star close-up (L9) and a planet (L10) were three cells of one physical system, swapped through portals when a marker grew past an angle. The owner asked for the levels to keep scaling the way L10 to L14 do on a planet, with a clear rule for where one scale converts to the next.

Owner decisions:
- scope L8 to L10, with L1-L6 later (#998);
- the scale converts at a body's gravity zone;
- no portals inside a system;
- a study first (`go` after the study report);
- true scale with markers (Q1 A);
- L9 as a distance label (Q2 A);
- moons included (Q3 A);
- the HUD names the level by distance and shows the nearest body (Q4 A);
- the system data wins over the ground's own size draw.

## Decision

- **True scale.** `orbits::true_cell` and `true_radius_cell` place planets, belts and the star at true distances and sizes in L8 and L9 cell units. Moons sit at their true orbits and sizes in L10.
- **The gravity zone converts the scale.** `nav::l8_planet_zone` and `planet_zone_m` give a planet's Laplace radius. `Universe::open_planet` opens the star close-up and the planet in one exact origin shift. The dive closes both past `ZONE_LEAVE_FACTOR` (1.2) times the zone. Nothing in a planetary system opens by angle, and planets are not previewed.
- **The journey.** `Universe::journey_target` names the planet in L8, and `Destination::on_the_way` maps the way through the star close-up to it. `DiveMode::Journey` flies to a waypoint two zones out on the planet's morning side, then straight in, so it arrives in daylight with no turn at the open (#780 F1 A).
- **One source of truth.**
  - `terrain::planet_recipe` sizes each planet's ground from its system's `PlanetInfo`.
  - `orbits::planet_slot` gives the home journey's planet the Earth's catalog slot, so the journey, its seeds and the home ground stay the same.
  - Moon periods follow Kepler with the planet's mass.
  - A planet has one name in both views.
- **Drawing.**
  - In L8 the star and the planets stay lit, draw at true size with the marker floor, and bound the near plane (`flight::nearest_body_distance`).
  - A star's glow quad steps aside once it is a body.
  - From inside a planet's cell the star and the other planets draw through the parent cell at their true places.
- **HUD.** `labels::system_level` names L9 within ten star radii. `labels::nearest_line` and `hud::system_hud` show the nearest body.
- **Checks.** The `HANDOVER` verify line measures the open and close jumps, the turn, the distance change and the arrival sun. The study modules (`system_space`, `system_handover`) keep their probes.

## Rules amended

- `docs/universes/ladder.md` R14.
- ADR 0004 (marker-tree nesting): the L8 to L10 chain stays, but inside a planetary system the planet cell opens from its gravity zone, not by angle.
- ADR 0010 (portal and population markers): planets and the star in L8 stay portals for targeting and labels, but never open by angle.
- ADR 0005 (bounded memory): unchanged; the live-cell bound holds.
- `E-GOLDEN`: the golden files were rewritten once (journey without an L9 opening, true-scale L8, the Moon, the `HANDOVER` and `ORBITS` lines). The ground lines are unchanged.

## Consequences

- At true scale a planet is a marker from the system view; it becomes a body only within a few million kilometres (the study: the Sun is 1 px from about 3 AU, Earth from about 4 million km).
- The study measured handovers at 0.000 mm and turns near 1e-15 degrees. On the journey the `HANDOVER` line reads 0.000 mm open and close, with the arrival sun 39 degrees up.
- Planet motion (#981) builds on this space: a body's position becomes a function of time, and a camera inside its zone rides along.
- L1-L7 keep their portals.

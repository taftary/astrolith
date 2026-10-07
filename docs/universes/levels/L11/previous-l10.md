# L10 behind us — Planets and moons as previous level

Loop doc for Issue #228 (Round 1 pilot, refreshed in the Round 2 loop). This page is the bridge
from L10 (Planets and moons) into L11 (Regions of a planet): what
the previous level looks like, what carries over when you zoom in,
and what changes at L11 scale. The part docs from the loop's first
pass now exist — follow the cross-links below.
Entry itself — the visual transition and its effects — lives in
`entry.md`. The dated timeline will run through `lifetime.md`.

## What L10 is and how it looks

L10 is the planet-and-moons scale at 10^6–10^8 m: one planet with its moons filling
the whole view. Its anchor is the Earth equatorial diameter, 1.28 x 10^7 m
(about 12,756 km, or about 7,926 miles across), the NASA planetary fact value.
Our Earth is the example planet: a 4.5 to 4.6-billion-year-old rocky world with
oceans over about 71 percent of its face, air of 78 percent nitrogen and 21 percent
oxygen, and one large Moon going around it at about 384,400 km away.

Inside the view are the parts. Deep down is the interior: the iron core where the
magnetic field is born, wrapped in the rocky mantle that carries heat upward, capped
by the thin crust we stand on. Then comes the face we actually see, the surface —
blue oceans, brown-green continents, white ice at the poles and on high peaks,
with clouds moving above and city lights on the night side. Above that stretches
the atmosphere: layered air from the ground up through clouds and weather to the
thin edge where auroras glow. Around it all is the company: the Moon — about 3,474 km
across, tidally locked so the same face looks back — plus the faint dust and the
small-moon pattern other planets show. And around everything is the shield, the
magnetosphere — the magnetic field reaching into space, bending the solar wind
around the planet into the northern and southern lights, holding the radiation belts.

Target visuals (files in `images/`, credits and licenses at the bottom):

![Global land and ocean topography, green-brown land and blue seas (screensize)](images/earth-bluemarble-nasa.jpg)

Sources: `docs/universes/ladder.md` (R5 anchors, L10 row, L11 row);
`docs/universes/levels/L10/README.md` (L10 part docs
`interior.md`, `surface.md`, `atmosphere.md`, `moons.md`,
`magnetosphere.md`); NASA Earth facts page.

## What carries over into L11

Everything L10 shows at planet scale is still there, only closer.
The round world L10 frames as the whole becomes the ground: the Earth, 12,756 km
across, still turning once in 23.9 hours and circling the Sun in 365.25 days —
its oceans, air, ice, and Moon still around us. The face L10 watches as oceans and
land becomes the terrain under our feet — continents you can cross, mountain ranges
you can trace, plains and basins you can cross in a day. The air L10 counts as a
thin rim becomes the sky above the regions — clouds, weather, and the day-night line
moving across the land. The Moon L10 keeps beside the disk stays far above — about
30 Earths away in a line — still pulling the tides in the seas below. Sources: ladder
L10 and L11 rows; NASA Earth facts page; NASA Moon facts page.

## What changes at this zoom

**From one planet to regions of a planet.** At L10 the view holds
one planet with its moons at 10^6–10^8 m. At L11 each L10 cell opens
into regions at 10^5–10^6 m: areas about 100 to 1,000 km across — the size of
large mountain ranges, big basins, long coastlines, whole countries — the continents,
ranges, plains, waters, and river systems (see `continents.md`,
`mountains.md`, `plains.md`, `oceans.md`, `rivers.md`).

**From a whole disk to a curved ground.** The L10 disk — oceans blue, land
brown-green, clouds white — fills past the edges and becomes landscape: the curve
of the planet flattens into horizon, the terminator becomes local day and night,
the atmosphere rim becomes the sky. A region about 1,000 km across spans roughly
one-twelfth of the planet's width (1.28 x 10^7 m anchor); a 100 km region spans
roughly one-hundredth. Populations (shown points) never open; only portals do.
Sources: ladder L10 and L11 rows; ADR 0010 (portal vs
population).

**From reading a planet to standing over a region.** L10 shows a world — inside,
face, air, Moon, and magnetic shield with its space weather. L11 shows what one
patch of that face holds up close: the rock of the continents, the lift of the
mountain ranges, the flat of the plains, the blue of the seas and the line of the
coasts, the threads of rivers and the still of lakes. The planet is still there,
12,756 km around — but here the story is the region, not the globe. Sources: NASA Earth
facts page; USGS science pages.

## How the parts fit together

One region runs through the planned part docs. The landmasses
(`continents.md`) set the base — continents, cratons, shields; the ranges
(`mountains.md`) set the lift — mountain ranges, highlands, belts; the lowlands
(`plains.md`) set the flat — plains, basins, lowlands, deserts; the waters
(`oceans.md`) set the blue — oceans, seas, coasts, islands at regional scale;
and the threads (`rivers.md`) set the flow — rivers, lakes, valleys, deltas.
The dated version of this story runs through `lifetime.md`, and the dive
that brings you here is in `entry.md`. Sources: L10 part docs
as the pattern; NASA Earth and USGS pages.

## Where to go next

- `entry.md` — the L10-to-L11 dive in plain steps, effects on entry, timing.
- `continents.md`, `mountains.md`, `plains.md` — the base, the lift, the flat.
- `oceans.md`, `rivers.md` — the blue and the flow.
- `lifetime.md` — dated stages from crust and first continents to today's regions.

## Key numbers

- L10 span: 10^6–10^8 m; anchor Earth equatorial diameter 1.28 x 10^7 m (12,756 km / 7,926 miles).
- L11 span: 10^5–10^6 m; regions about 100 to 1,000 km across (continents, countries, mountain ranges per ladder row).
- Earth: oceans cover about 71 percent; mean ocean depth about 3.6 km; air 78 percent nitrogen, 21 percent oxygen; spin 23.9 hours; year 365.25 days.
- Earth–Moon: Moon diameter about 3,474 km; average distance 384,400 km (about 30 Earths in a row); Moon circles Earth about every 27 days; Moon's pull steadies tilt and raises tides.
- L10 to L11 step: a 1,000 km region is about 0.08 of the planet width; a 100 km region about 0.008; region counts per cell fixed in the loop.

Sources: ladder R5 and L11 row; pages named above; NASA object pages.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `earth-bluemarble-nasa.jpg` | Global land and ocean topography, green-brown land and blue seas (screensize) | NASA Earth Observatory / Visible Earth team, frame pinned in-repo | Public domain (NASA) (`https://eoimages.gsfc.nasa.gov/`) |

Note: the pilot audit confirms the file, credit and term before the
loop continues; replacements are separate updates, never silent swaps.

## Sources

- Ladder: `docs/universes/ladder.md` (R5 anchors, L10 and L11 rows, gap note, R8 form)
- L10 reference index: `docs/universes/levels/L10/README.md`
- L10 part docs (pattern for L11 parts): `interior.md`, `surface.md`, `atmosphere.md`, `moons.md`, `magnetosphere.md`
- NASA, Earth facts: `https://science.nasa.gov/earth/facts/`
- NASA, Moon facts: `https://science.nasa.gov/moon/facts/`
- USGS, Earth science pages: `https://www.usgs.gov/science`

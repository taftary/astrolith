# Entering L12 — the dive from regions to cities

Loop doc for Issue #229 (Round 1 pilot, refreshed in the Round 2 loop). This page walks the L11-to-L12
dive in plain steps: what you approach, what changes on entry, and
the numbers behind it. The bridge behind us lives in `previous-l11.md`;
the part docs and the timeline now exist —
the cross-links below say where each sight belongs.

## The approach

You leave the whole region behind and fall toward one patch of its face — a city with its
landscapes about 1 to 100 km across, the size of a large metro, a lake, a valley.
The region thins out into ground: mountains green and parallel, farmland in straight-edged
rectangles, rivers dark and winding, still there, now around you. Far behind lies the region wash:
the coarse land-cover blobs, the broad haze, the city reduced to a dot. Ahead, one marker brightens
among the points: the city portal of this L11 cell, holding one L12 patch — grey built mass below,
suburbs feathering out, roads as pale lines, fields as rectangles, lakes dark and still, valleys
cutting through. Around the point, the city itself swells into view: first the pale patch, then the
street grid, then the field boundaries, then the lake shorelines and valley walls, then the parks
and forest patches inside — with the preview toward the buildings of L13.

Target visual (file in `images/`, credit and license at the bottom):

![City grid meeting farmland grids with canals and hills (screensize)](images/mexicali-city-farmland-nasa.jpg)

Sources: `docs/universes/ladder.md` (L11 and L12 rows, gap note); NASA Earth
Observatory pages; USGS land-cover pages.

## Crossing into L12

The L11–L12 span is crossed through quiet magnification the dive pushes by proximity
to the targeted portal and unwinds the same way, so generation, snapshots and labels
never see a gap. Then the portal opens (angular radius past 0.14 rad, about a third of the
view) and the cell closes back into its marker below 0.10 rad —
pure origin shifts, with the pre-entry preview having already drawn
the interior inside the marker from 0.02 rad up to the opening.

What you see, in order, on this entry:

1. The region thins and goes: the broad patch with its relief shadows and water shapes
   dissolves into the built ground around you — the region reduced to
   city for the landscape.
2. One patch takes everything: a city with its landscapes about 1 to 100 km across, the size of
   a metro or lake, with built mass, fields, and water in one frame.
3. The patch becomes ground: cities pale grey and beige, fields green and brown in rectangles,
   water dark — the curved face, lit on one side.
4. The grids resolve: the street blocks 0.1–1 km, the farm sections 1.6 km with quarter
   cuts at 800 m and 400 m, with canals and rails as thin lines.
5. The relief resolves: valleys as cuts with walls and terraces, hills as green texture,
   lake shores as sharp lines with islands inside.
6. The green shows: parks and riparian corridors as darker green inside the grey, forests on
   the hills, with the preview toward the buildings of L13.
7. The markers fill again: from many region portals per L11 cell to many city portals per L12 cell — this dive continues the beyond-MVP tail.

Sources: ladder gap note and R7 preview angles; ADR 0010; NASA Earth
Observatory pages; USGS pages.

## Effects on entry

Entry is gentle here because the L11-to-L12 leg is short — only about one to two
decades in characteristic size, crossed quietly as the dive falls from 10^5 m toward
10^3 m. On entry, the region markers fade out of the frame, and the coarse blobs of L11
land cover fade with them — they belong to the region view, not to the city. What stays and grows is the targeted city marker:
it swells from the preview size at 0.02 rad up to the open angle at 0.14 rad, drawing
its streets, fields, and waters inside itself before the entry completes. Population
points never open (ADR 0010) — they are shown and counted but never targeted, so
only the true city portal carries you through this entry. At most 6 markers preview
at once, largest on screen first, so the entry view stays calm even when the ground
is crowded. The standard palette reads forest, crops, grass, water, and artificial cover
like roads and buildings; urban imperviousness reads as percent developed surface.

Sources: `docs/universes/ladder.md` (L11 and L12 rows, R7 preview
angles, gap note); ADR 0010; ESA Sentinel-2 pages; USGS land-cover pages.

## Timing

The L11 span (regions 10^5–10^6 m) gives way to
the L12 span (cities 10^3–10^5 m) — about one to two decades
closer in characteristic size. Travel time per rung runs `Δe ·
ln 10 / k`, so this leg runs short; the long four-decade
heliopause-to-Sun run is far behind us. Inside L12, distances read
in fractions of the city: metros tens of km across, lakes single to tens
of km, valleys single km wide — with the region 100 to 1,000 km around,
about 8 light-minutes from the Sun, the Moon 384,400 km out. Earth spins once in
23.9 hours and circles the Sun in 365.25 days; the cities turn with it from day
into night.

Sources: ladder L11 and L12 rows; NASA Earth pages (sizes,
distances, light time); USGS pages (city and field sizes).

## What entry proves

Entry is complete when the region wash is gone, one patch of built ground commands the
view, and the patch shows streets and fields: you count blocks, field edges,
and shorelines, not regions. If you can name the city below as grey mass with suburbs along
it, the day-night change as local light, the road lines and field rectangles, and the lakes
and valleys as dark and cut — you are in L12. The
detailed looks, lifetimes and interactions of each kind will live in the
part docs to come: the cities and towns, the networks and farmland, the landscapes —
start with the reading guide to map each sight to its stage.

Sources: NASA Earth Observatory pages; USGS pages.

## Where to go next

- `previous-l11.md` — the L11 bridge: what carries over, what changes at this zoom.
- `cities.md`, `towns.md`, `networks.md` — the built mass, the spread, the links (planned).
- `farmland.md`, `landscapes.md` — the patchwork and the anchors (planned).
- `lifetime.md` — dated stages from first settlements to today's cities (planned).

## Key numbers

- L11 span: regions 10^5–10^6 m (100 to 1,000 km).
- L12 span: cities 10^3–10^5 m (1 to 100 km); metros 20–100 km; blocks 0.1–1 km; farm sections 1.6 km.
- Portals: many region portals per L11 cell → many city portals per L12 cell (counts fixed in the loop).
- Open angle 0.14 rad, close 0.10 rad, preview from 0.02 rad (at most 6 markers).
- City examples: New York urban area 8,413 km2 (about 100 km across); Los Angeles 4,239 km2 (about 73 km).
- Land-cover palette: forest, crops, grass, water, artificial cover; 10 m Sentinel-2, 30 m NLCD 16-class legend.
- Light travel for context: Moon to Earth about 1.3 s; Sun to Earth about 8 minutes.

Sources: ladder L11/L12 and R7 and gap note; pages named above.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `mexicali-city-farmland-nasa.jpg` | City grid meeting farmland grids with canals and hills (screensize) | Astronaut photograph ISS071-E-131092 (Expedition 71 crew, Nikon Z9 400mm), ISS Crew Earth Observations Facility and Earth Science and Remote Sensing Unit, NASA Johnson Space Center | Public domain (NASA) (`https://science.nasa.gov/earth/earth-observatory/mexicali-baja-california-153234/`) |

Note: the Round 2 audit confirms each file, credit and term before the
loop continues; replacements are separate updates, never silent swaps.

## Sources

- Ladder: `docs/universes/ladder.md` (L11 and L12 rows, R7 preview, gap note)
- NASA, Earth Observatory: `https://science.nasa.gov/earth/earth-observatory/`
- NASA, city views: `https://science.nasa.gov/earth/earth-observatory/monterrey-amid-mountains/`
- USGS, land-cover: `https://www.usgs.gov/centers/eros/science/national-land-cover-database`
- ESA, Sentinel-2: `https://www.esa.int/Applications/Observing_the_Earth/Copernicus/Sentinel-2`

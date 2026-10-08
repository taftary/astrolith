# Farmland — fields, countryside, managed land

One of five L12 parts: farmland and countryside at 1–100 km.
The bridge from L11 lives in `previous-l11.md`; the dive lives in `entry.md`; the timeline in `lifetime.md`.
For the consuming core see [cities](./cities.md); for the fringe spread see [towns](./towns.md);
for the crossing links see [networks](./networks.md); for the frame see [landscapes](./landscapes.md).

## How it looks

Farmland reads as a patchwork of rectangles and squares in vivid greens, golds, and browns; bare
tilled soil is dark brown, pasture and hay lighter uniform green. From air and space the edges are
sharp and straight on a road grid, with center-pivot irrigation circles inscribed in squares and
farmstead clusters as tiny dots. Countryside is fields with pastures, scattered trees and windbreaks,
dirt and paved roads, ditches and canals. USGS classes separate 81 Pasture and Hay (grasses for
grazing and hay, perennial) from 82 Cultivated Crops (annual corn, soy, vegetables, cotton, plus
orchards and tilled land). The Carrot River Valley shows straight-edged fields as an island breaking
up curvy natural contours, green in summer and brown at harvest.

The target visual pins this to a real place. Astronaut photograph ISS071-E-131092, taken
25 May 2024 from the International Space Station with a Nikon Z9 at 400 mm on Expedition 71,
looks over Mexicali in Baja California where the city grid meets United States farm grids along
the border in the Sonoran Desert. The city and its surrounding fields sit in a low valley —
Imperial Valley on the United States side, Mexicali Valley on the Mexico side — on the
San Andreas Fault system between the Salton Sea to the north and the Colorado River Delta
wetlands to the south, with the foothills of the Sierra de Los Cucapah visible at the bottom
of the frame. Sunlight glints off Laguna Mexico, water-treatment ponds, and canals including
the All-American Canal, which carry irrigation water to both sides of the border. South and
east of Mexicali the fields fan outward, shaped by pre-existing wetlands and city
infrastructure, in predominantly cotton and wheat; north of the border the grid is repetitive
with alfalfa, wheat, and sugarcane, plus a solar farm recognizable as small grey rectangles
in parallel rows.

At Landsat scale the same patchwork is a 30 m classification problem. The legacy National Land
Cover Database maps the conterminous United States at 30 m in 16 classes on a modified Anderson
Level II legend across nine epochs (2001, 2004, 2006, 2008, 2011, 2013, 2016, 2019, 2021),
with a change index that visualizes change across all epochs; the program has since moved to
an Annual NLCD release.

A single summer frame hides the rotation. The USDA Cropland Data Layer, built each year from
Landsat-class imagery plus ground truth, maps crop type across the continental United States for
the Agricultural Statistics Board acreage estimates: corn, soy, wheat, cotton, alfalfa, sugarcane,
orchards, and more, with disaster assessment, rotation planning, and fraud checks on top. The
program began in North Dakota in 1997, reached the lower 48 by 2008, and moved from 30 m to
10 m beginning with the 2024 layer (the 2025 layer released 27 February 2026, with a 30 m
resampled version for continuity; Hawaii runs its own 10 m layer). In false color that stacks
near-infrared with visible bands, healthy plants burn bright red and stressed or fallow fields
go black or grey — the Saudi desert-agriculture time series of corn, barley, sorghum, and wheat
harvested mostly in late spring is the cleanest demo of the signature. For the L12 reader the rule
is simple: green means growing, gold means ripe, dark brown means tilled, light uniform green
means pasture or hay, and red in false color means photosynthetically active.

Water has its own signature. The open-source OpenET platform uses Landsat thermal-infrared bands
to estimate field-level evapotranspiration — the Gallo vineyard GRAPEX work mapped water use vine
row by vine row — so irrigation on and off is visible as temperature and vigor change, not just
color. NASA Harvest (global) and NASA Acres (United States) connect these pixels to farm
decisions on water, weather, and insurance.

Target visual (file in `images/`, credit and license at the bottom):

![City grid meeting farmland grids with canals and hills (screensize)](../images/mexicali-city-farmland-nasa.jpg)

Sources: NASA Earth Observatory Mexicali page (ISS071-E-131092, 25 May 2024); USGS NLCD pages;
USDA NASS Cropland Data Layer pages; NASA Landsat agriculture and food-security pages.

## How it changes over time

Green growing crop turns gold ripe, then brown stubble and bare soil after harvest and till; hay
shows mowed strips. Irrigation on and off changes color; pivot circles appear and disappear; fallow
versus planted runs checkerboard year to year. Long term, fields expand and contract, hedgerows are
removed, and urban encroachment converts cropland to housing — tracked by the NLCD change index
2001–2021 in the legacy product and now year by year back to 1985 in the Annual NLCD.

The Annual NLCD (Collection 1.2 adds 2025) maps six products every year at 30 m for the
conterminous United States: land cover on the 16-class Anderson Level II legend, land-cover
change, a confidence index, fractional impervious surface, an impervious descriptor, and
spectral-change day of year. It fuses legacy NLCD and LCMAP methods with geospatial deep learning
in cloud and high-performance computing, validated against 8,360 independent 30 m reference plots.
Because Landsat has run continuously since 1972 with calibrated science-grade visible, near-infrared,
shortwave-infrared, and thermal bands, a difference between years means a difference on the ground:
crop cycles, fallow rotations, and new subdivisions over fields are all legible in the stack.

Season flips the patchwork twice a year. The 1 October 2026 Earth Observatory pair over the
Carrot River Valley (Landsat 9 OLI 2 July 2026 against Landsat 8 OLI 21 September 2026) shows
green summer crops going brown at harvest while the surrounding delta wetlands hold less water:
wet spring delayed planting in the Northwest crop region, then a dry week at The Pas
(1.6 mm against up to 43.9 mm elsewhere) let combines run to about 10 percent of canola and
50 percent of spring wheat by late September. Drought flips it harder. The 8 October 2026
Texas High Plains pair over Lubbock (22 September 2025 against 9 September 2026) shows green
and brown fields with watered playas collapsing to parched brown with mostly dry lakes; the
region abandoned 72 percent of dryland cotton in 2022 and 66 percent in 2011, and about
90 percent of one Lubbock County dryland crop died outright in 2026. Scarcity is drawn on the
ground: irrigating only one-half or one-third of a pivot leaves green semicircles and quarter
circles inside brown squares. At decade scale the same stack watches subdivisions replace brown
cropland with red urban — Treasure Valley around Boise more than doubled past 800,000 as
hay, pasture, corn, and wheat ground turned to streets.

Sources: USGS Annual NLCD pages (Collection 1.2); NASA Landsat program pages;
NASA Earth Observatory Carrot Valley, Texas cotton, Treasure Valley, and Fountain Fire features.

## How it is created

Farmland is created by clearing native grass and forest, then plowing, draining, and leveling, then
fencing with roads on survey lines, then planting monocrop with irrigation and fertilizer. The
Carrot River Valley shows the drainage half of that sentence: too wet for farming in its natural
delta state, it was opened by a 1950s flood-control and drainage project and still farms today
only behind dikes and pumps between the Carrot and Pasquia rivers. In the western United States,
rangeland versus irrigated cropland frames the pattern; the Bureau of Land Management runs its
rangelands under multiple-use and sustained-yield mandates for grazing, watershed, wildlife
habitat, and recreation, keeping the open spaces that shape the character of the West. The Public
Land Survey grid then fixes the result to property lines, and the five-year Census of Agriculture
(released February 2024 for 2022, with the 2023 Irrigation and Water Management follow-up in
October 2024) counts what the grid produced.

Sources: BLM rangeland and cadastral pages; NASA Earth Observatory Carrot Valley page;
USDA NASS Census of Agriculture pages.

## How it interacts with the others

Fields interact as the patchwork that [cities](./cities.md) consume, [towns](./towns.md) border,
[networks](./networks.md) cross, and [landscapes](./landscapes.md) frame. Fields abut roads,
farmsteads, windbreak tree lines, irrigation ditches and canals, pasture, and remnant woods and
wetlands. Soil and water run off to ditches and streams; center pivots draw groundwater. Fences,
hedges, and shelterbelts mark the edge; livestock sits on pasture and hay while machines work
the cropland.

Groundwater is the binding constraint. The High Plains aquifer underlies about 174,000 square miles
across eight states and almost 112 million acres, the principal water source for one of the major
agricultural regions of the United States and drinking water for 2.3 million people; it is one of
the most stressed groundwater supplies in the country. Around Lubbock the water table has dropped
25 to 100 feet since large-scale irrigation began about 1950, over 150 feet in counties to the
north, and in the 2026 drought many wells pumped air. Playa lakes — more than 19,300 small
clay-lined basins in the Texas High Plains — are the recharge path when they function, but about
80 percent are tilled or silted and many no longer recharge; a Texas Tech deep-learning census
found significant decline since 1995 in 15 of 45 counties tracked. The NASA Acres Space for
Agriculture tour (August 2026) and its Farm Innovation Ambassador Team now connect these farms
directly to satellite evapotranspiration, planting, cover-crop, and playa-restoration decisions.

Sources: USGS High Plains aquifer pages; NASA Earth Observatory Texas cotton page;
USGS NLCD pages; BLM pages.

## Typical size and mass

The US Public Land Survey sets the grid: a section is 1 sq mile, 640 acres, about 2.6 km2 with
1.6 km sides; a quarter-section is 160 acres, about 0.65 km2 with 800 m sides. A full section field
block runs about 1.6 by 1.6 km; a quarter field 800 by 800 m; a center-pivot circle inscribed in a
quarter runs about 400 m radius, 50 ha, 125 acres — and a half- or third-watered pivot in a dry
year reads as a green semicircle or quarter inside that square. Mass is a thin flat layer; visual
weight comes from color contrast, not height. For scale, the Carrot River Valley island runs over
40,000 ha (100,000 acres) between two rivers; the High Plains aquifer behind the pivots spans
about 174,000 square miles; and a Texas playa lake is typically under 30 acres — three orders of
magnitude from field block to aquifer.

Sources: BLM cadastral survey pages; NASA Earth Observatory Carrot Valley and Texas cotton pages;
USGS High Plains aquifer pages.

## Example objects

- Example: Carrot River Valley (Pasquia Settlement), over 40,000 ha between the Carrot and Pasquia
  rivers — Manitoba's northernmost farmland in the Rural Municipality of Kelsey, an island of
  straight-edged fields in Saskatchewan River Delta wetlands; mostly canola and spring wheat in
  2025 with canary seed patches, read by Agriculture and Agri-Food Canada's Annual Crop Inventory
  and by the Landsat 9/8 July-to-September 2026 pair; farmed only behind 1950s dikes and pumps.
- Example: Texas High Plains cotton around Lubbock — archetype irrigated-versus-dryland contrast;
  Landsat 8 September 2025 against September 2026 shows the drought flip, half- and third-pivots
  as green semicircles, and dry playas; 72 percent abandonment in 2022, wells pumping air in 2026.
- Example: US section grid, 1.6 km squares — archetype field block with quarter cuts at 800 m.
- Example: Center-pivot quarter, 400 m radius — archetype irrigated circle in a square field.

Sources: NASA Earth Observatory Carrot Valley and Texas cotton pages; BLM pages.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `mexicali-city-farmland-nasa.jpg` | City grid meeting farmland grids with canals and hills (screensize) | Astronaut photograph ISS071-E-131092, ISS Crew Earth Observations Facility, NASA Johnson Space Center | Public domain (NASA) (`https://science.nasa.gov/earth/earth-observatory/mexicali-baja-california-153234/`) |

## Sources

- NASA, Earth Observatory Mexicali: `https://science.nasa.gov/earth/earth-observatory/mexicali-baja-california-153234/`
- NASA, Earth Observatory Carrot Valley: `https://science.nasa.gov/earth/earth-observatory/an-agricultural-island-in-the-saskatchewan-river-delta/`
- NASA, Earth Observatory Texas cotton: `https://science.nasa.gov/earth/earth-observatory/fighting-drought-in-texas-cotton-country/`
- NASA, Landsat: `https://science.nasa.gov/mission/landsat/`
- NASA, Landsat agriculture and food security: `https://science.nasa.gov/mission/landsat/benefits/agriculture-food-security/`
- NASA, Annual NLCD insights: `https://science.nasa.gov/earth/human-dimensions/agriculture/annual-nlcds-insights-rely-on-long-landsat-record/`
- USGS, NLCD: `https://www.usgs.gov/centers/eros/science/national-land-cover-database`
- USGS, Annual NLCD: `https://www.usgs.gov/centers/eros/science/annual-national-land-cover-database`
- USGS, High Plains aquifer: `https://www.usgs.gov/mission-areas/water-resources/science/high-plains-aquifer`
- USDA, NASS Cropland Data Layer: `https://www.nass.usda.gov/Research_and_Science/Cropland/SARS1a.php`
- USDA, NASS 2022 Census of Agriculture: `https://www.nass.usda.gov/Publications/AgCensus/2022/index.php`
- BLM, rangelands and grazing: `https://www.blm.gov/programs/rangelands-and-grazing`
- BLM, cadastral survey: `https://www.blm.gov/programs/lands-and-realty/cadastral-survey`

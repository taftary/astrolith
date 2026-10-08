# Landscapes — lakes, rivers, valleys, forests, deserts, coasts

One of five L12 parts: water and green anchors at city scale (1–100 km).
The bridge from L11 lives in `previous-l11.md`; the dive lives in `entry.md`; the timeline in `lifetime.md`.
For the built core see [cities](./cities.md); for the fringe spread see [towns](./towns.md);
for the crossing links see [networks](./networks.md); for the consumed patchwork see [farmland](./farmland.md).

## How it looks

A lake reads as standing water in an inland basin: dark and smooth against land in natural color,
bright or dark depending on sediment, algae, and sun glint. A lake is where surface runoff and
groundwater seepage accumulate in a low spot relative to the surrounding land [S001]; ponds are
smaller than lakes, reservoirs are human-made lakes, and rivers are fast-moving channels.
Landsat blue and red bands track lake clarity across thousands of otherwise unmonitored lakes [S002];
Sentinel-2's 13 spectral channels add chlorophyll concentration, algal-bloom detection, and turbidity
at 10 m with a 5-day revisit [S003]. Rivers read as winding dark or sediment-pale ribbons that shift
beds and build riffles, bars, floodplains, levees, fans, terraces, deltas, and estuaries in straight,
meandering, and braided patterns. Valleys read as elongate depressions, usually river-drained, in
plain or between hills; canyons are steep-sided erosional valleys. Hills rise with confined summits,
smaller than mountains. Forests read as tree-dominant green texture with vertical layers: conifer stands to
about 30 m, deciduous two-story canopy, rainforest three strata and more. Deserts read as bare tan,
pink-tan, and brown plains with isolated short ranges, alluvial fans, playas, and dunes; the
Monterrey view shows light urban development filling the frame against green parallel ridges, and the
Mexicali view shows the city grid meeting farm grids in a low desert valley between the Salton Sea
and the Colorado River Delta wetlands. Coasts read as the land-water edge: beaches, tidal flats,
estuaries where rivers meet the sea, and delta lobes where sediment supply beats marine reworking.
Parks read as darker green inside grey built area with walkers, boating, and picnic dots.

The Monterrey target visual pins this to a real place. Astronaut photograph ISS075-E-70481, taken
26 August 2026 from the International Space Station, looks over Monterrey — Mexico's second-largest
metropolitan area at 5.3 million people (2020 census) — against the folded limestone ridges of the
Sierra Madre Oriental, deposited in the late Mesozoic and folded about 80–50 million years ago [S004].
The Rio Santa Catarina carves through the mountains onto the semiarid floodplain where the city lies;
usually carrying little water, its channel collects summer-rain runoff and serves as an in-city natural
area. Island-like folded-rock protrusions stand inside the urban mass, including the Sierra Las Mitras
state nature reserve rising about 1,500 m over the city. Landsat saw the same scene in 1999 as a grey
patch at the mountain foothills with a tan river ribbon [S004]; ESA's Sentinel-2 view of March 2023
resolves the Santa Catarina, the foothill city edge, and pivot-irrigated fields south of the ranges
at 10 m [S005].

Target visual (file in `images/`, credit and license at the bottom):

![Monterrey urban development against green mountain ridges (screensize)](../images/towns-monterrey-nasa.jpg)

Sources: Britannica lake, river, valley, hill, forest pages; USGS lakes and rivers pages; ESA water pages.

## How it changes over time

Lakes change fast and are fragile: eutrophication from phosphorus and nitrogen to algal blooms, oxygen
below 3 mg per liter, fish kills; sedimentation and turbidity; level swings; species shifts and
invasives. Rivers shift beds, build riffles, bars, floodplains, levees, fans, terraces, deltas, and
estuaries in straight, meandering, and braided patterns. Valleys deepen vertically then widen
laterally to base level; the Grand Canyon model runs deposition, uplift 70–30 million years ago,
downcutting 5–6 million years, then widening by rain, wind, temperature, chemistry, and tributaries.
Forests transpire, intercept runoff, stabilize slopes, and filter lake inflow.

The satellite record now quantifies the fast half. From 1992 to 2020, 53 percent of the 1,972
largest global lakes lost significant storage; climate change plus human consumption dominate the net
natural-lake decline, sedimentation dominates existing-reservoir decline, and about 2 billion people
live in a drying-lake basin [S006]. GRACE gravity data then caught an abrupt global freshwater drop
after the 2014–2016 El Nino that never fully recovered [S007]. Lake evaporation averages about
1,500 km3 per year and is rising at 3.12 km3 per year, driven 58 percent by evaporation-rate rise
itself [S008]. Western Lake Erie shows the eutrophication pulse year by year: the 2024 Microcystis
bloom started 24 June — the earliest since NOAA tracking began in 2002 — covered about 830 km2 on
13 August (Landsat 9) and peaked near 1,700 km2 on 22 August at severity 4.2 [S009]; the 2025 bloom
was milder at severity 2.4 over about 1,140 km2 (441 square miles), starting early July and collapsing in early
September [S010]. Forests swing harder still: the tropics lost a record 6.7 million hectares of
primary rainforest in 2024 (18 football fields per minute, fire-driven, nearly double 2023), then
eased 36 percent in 2025 to 4.3 million hectares — still 46 percent above the decade-ago level [S011].
The Copernicus Water Bodies product maps inland water monthly at 100–300 m from 2020 onward on
Sentinel-2 MSI, giving the L12 reader a current baseline for any city-scale lake [S012].

Sources: Britannica pages; USGS pages; NPS geology pages; Yao et al. 2023 (Science); Zhao et al. 2022;
NOAA NCCOS 2024–2025 assessments; WRI Global Forest Watch 2024–2025; Copernicus Land Monitoring Service.

## How it is created

Lakes form by tectonism and crustal depression, volcanism and crater collapse, landslides damming,
glaciation carving and damming, fluvial oxbows, wind dunes, beaver dams, and meteorite impact. Water
budget balances inflow (streams, rain, seepage, melt) against outflow, seepage, and evaporation;
closed lakes with no outflow swell and recede with climate. Lake Baikal — the rift-lake archetype at
over 25 million years old and 1,642 m deep — holds its 23,600 km3 in an active continental rift with
sediments over 7 km thick [S013]. Valleys form as V-shaped fluvial cuts, U-shaped glacial troughs by
abrasion, or rift fault troughs; downcutting needs steep slope, large flood volume, and arid exposed
bedrock. The Colorado Plateau uplift 70–30 million years ago set the table; the Colorado River
integrated through older palaeocanyons beginning 5–6 million years ago and has incised steadily since
at 100–200 m per million years, faster in the east [S014]. Hills form by uplift, erosion, glacial
deposits, and cinder cones. Forests need the warmest month above 10 C with over 200 mm per year.
Deserts form where subtropical subsidence, rain shadow, or continental interior position starve the
land of rain: the Sonoran Desert spans about 260,000 km2 of the US Southwest and northwest Mexico
between 23 and 30 N, grading into Mojave, Chihuahuan, and thornscrub edges [S015]. Coasts and deltas
form where rivers meet standing water: deltas grow when sediment supply beats wave and tide reworking
— river-dominated bird-foot (Mississippi), wave-smoothed arcuate (Nile, Niger), tide-channeled
(Fly River) — and abandon into sinking, eroding lobes when the channel shifts [S016].

Sources: Britannica lake, valley, forest pages; USGS pages; NPS Grand Canyon geology pages;
Karlstrom et al. 2014; USGS coastal land-loss overview; NPS Sonoran Desert Network.

## How it interacts with the others

Landscapes interact as the anchors that [cities](./cities.md) sit inside, [towns](./towns.md) border,
[networks](./networks.md) bridge, and [farmland](./farmland.md) drains to. Cities grow around rivers
for flood control, irrigation, power, water, habitat, and recreation — not vice versa; rivers are
100 times more effective than coastal erosion at shaping land. Lakes cool summers and milden winters,
make fog and lake-effect snow, and need nutrient and bacteria monitoring in city parks. Forests filter
inflow and keep lakes clear; lakes shelter forests from fire while felling shore trees by wind.
Valleys carry roads, rails, and rivers together.

Monterrey shows the bargain breaking. In summer 2022, after 15 months of very low rainfall, one of
the city's three supply reservoirs fell below 1 percent capacity and a second to 7 percent; taps ran
dry for 50-plus days in outlying districts while industry and farming negotiated emergency cuts [S017].
Cumbres de Monterrey National Park — a UNESCO Biosphere Reserve in the Sierra Madre Oriental —
supplies about half the metro area's water, so forest cover there is water infrastructure [S005].
South of the border, the Mexicali Valley side shows the older bargain: the Colorado River Delta once
spread over 169,000 ha of cottonwood-willow forest, marsh, and tidal flat at the Sonoran Desert's
western edge, then lost about 80 percent of its wetlands to upstream dams and diversions; binational
pulse flows and restoration sites such as Laguna Grande are rebuilding riparian corridors reach by
reach [S018]. Nutrient runoff closes the loop downstream: over 60 percent of US coastal rivers and
bays are moderately to severely degraded by nitrogen and phosphorus, driving algal blooms, dead zones,
and seagrass loss [S019].

Sources: USGS lakes and rivers pages; NPS pages; ESA Monterrey feature; Sonoran Institute delta
program; USGS nutrient-pollution circular; Euronews Monterrey water reporting 2022.

## Typical size and mass

World lakes hold about 199,000 km3 in total; the Caspian Sea alone holds 78,200 km3, Baikal
23,600 km3 at 1,642 m deep (about 20 percent of fresh surface water, as much as all five North
American Great Lakes combined), Tanganyika 18,900 km3, and Superior 12,070 km3 on an 82,100 km2 surface —
the largest freshwater surface in the world [S020][S001]. The 40 largest lakes hold about four-fifths
of the total. A city lake 1 km across covers about 0.8 km2; 10 km across about 78 km2
with 5–15 m depth. City rivers run 20–300 m wide and 2–12 m deep. Valleys run 0.2–30 km wide,
0.1–1.8 km deep, tens to 446 km long. Hills run 20–300 m relief; forest patches 0.5–5 km.
Desert ecoregions dwarf cities: the Sonoran covers about 260,000 km2, its Colorado Desert subregion
about 28,000 km2, and the Colorado River Delta wetlands about 169,000 ha at the desert's wet edge
[S015][S018]. The Sierra Madre Oriental runs about 1,000 km from the Rio Grande to central Mexico,
with peaks to 3,700 m; the Sierra Las Mitras island ridge inside Monterrey rises about 1,500 m over
the streets [S004]. The Copernicus Water Bodies baseline resolves inland water monthly at 100–300 m,
so any L12 lake wider than a few pixels is individually tracked [S012].

Sources: Britannica pages; USGS pages; Water Encyclopedia lake-volume compilation; ILEC World Lake
Database; NPS Sonoran Desert Network; Copernicus Land Monitoring Service.

## Example objects

- Example: Lake Superior, 563 km long and 258 km wide, 82,100 km2, mean depth 147 m — the largest
  freshwater surface in the world; youngest Great Lake at 10,000–7,500 years.
- Example: Lake Baikal, 636 km long and 79 km wide, 31,722 km2, 23,600 km3, 1,642 m deep — the
  largest freshwater volume, the deepest and oldest (25–30 million years) lake; rift archetype [S013].
- Example: Grand Canyon, 446 km long, up to 29 km wide, 1.8 km deep — length L11, width L12; river
  5–6 million years old on rocks over 2.5 billion years.
- Example: Green Lake Seattle — archetype urban lake for recreation, water supply, and monitoring.
- Example: Sierra Madre Oriental at Monterrey — folded limestone ridges against a 5.3-million-person
  metro; Rio Santa Catarina floodplain plus the 1,500 m Sierra Las Mitras island reserve inside the
  city; water-supply forest in Cumbres de Monterrey National Park [S004][S005].
- Example: Colorado River Delta — 169,000 ha of desert-edge wetland reduced 80 percent by upstream
  dams, now under binational restoration; the wet anchor of the Mexicali Valley view [S018].

Sources: Britannica Lake Superior and Grand Canyon pages; NPS Pictured Rocks and geology pages;
ILEC World Lake Database; NASA Earth Observatory Monterrey pages; Sonoran Institute delta program.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `towns-monterrey-nasa.jpg` | Monterrey urban development against green mountain ridges (screensize) | Astronaut photograph ISS075-E-70481 (26 August 2026), ISS Crew Earth Observations Facility, NASA Johnson Space Center | Public domain (NASA) (`https://science.nasa.gov/earth/earth-observatory/monterrey-amid-mountains/`) |

The companion Mexicali desert-valley view (`mexicali-city-farmland-nasa.jpg`, ISS071-E-131092,
25 May 2024) is credited in [farmland](./farmland.md) and shows the same landscapes theme from the
desert side: city and farm grids in a low Sonoran Desert valley between the Salton Sea and the
Colorado River Delta wetlands.

## Sources

- [S001] USGS Water Science School, Lakes and Reservoirs. Accessed 2026-10-08. URL: `https://www.usgs.gov/water-science-school/science/lakes-and-reservoirs` — Supports: lake definition; Baikal 23,000 km3. Used in: How it looks; Typical size and mass.
- [S002] NASA, Landsat Benefits: Water Resources. Accessed 2026-10-08. URL: `https://science.nasa.gov/mission/landsat/benefits/water-resources` — Supports: Landsat lake-clarity and algal-bloom monitoring; thermal ET. Used in: How it looks.
- [S003] ESA, Sentinel-2 Water Bodies. Accessed 2026-10-08. URL: `https://www.esa.int/Applications/Observing_the_Earth/Copernicus/Sentinel-2/Water_bodies` — Supports: 13-channel chlorophyll/turbidity; 10 m, 5-day revisit. Used in: How it looks.
- [S004] NASA Earth Observatory, Monterrey Amid Mountains (11 Sep 2026) and Monterrey, Mexico (Landsat 1999). Accessed 2026-10-08. URL: `https://science.nasa.gov/earth/earth-observatory/monterrey-amid-mountains/` — Supports: ISS075-E-70481, 5.3M metro, Sierra Madre Oriental folding 80–50 Ma, Santa Catarina, Sierra Las Mitras 1,500 m. Used in: How it looks; Example objects.
- [S005] ESA, Earth from Space: Monterrey, Mexico (Sentinel-2, 5 Mar 2023). Accessed 2026-10-08. URL: `https://www.esa.int/ESA_Multimedia/Images/2023/03/Earth_from_Space_Monterrey_Mexico` — Supports: 10 m city/river/field readout; Cumbres de Monterrey water supply. Used in: How it looks; How it interacts.
- [S006] Yao et al. 2023, Satellites reveal widespread decline in global lake water storage, Science. DOI: `10.1126/science.abo2812` — Supports: 53% of 1,972 largest lakes declined 1992–2020; 2 billion people in drying basins. Used in: How it changes over time.
- [S007] NASA GRACE Tellus, Abrupt drop in global freshwater levels (20 Nov 2024). Accessed 2026-10-08. URL: `https://grace.jpl.nasa.gov/news/218/` — Supports: post-2014 freshwater drop. Used in: How it changes over time.
- [S008] Zhao et al. 2022, Evaporative water loss of 1.42 million global lakes. Accessed 2026-10-08. URL: `https://pmc.ncbi.nlm.nih.gov/articles/PMC9240014` — Supports: 1,500 km3/yr mean evaporation, +3.12 km3/yr trend. Used in: How it changes over time.
- [S009] NASA Earth Observatory, Lake Erie Blooms (13 Aug 2024, Landsat 9). Accessed 2026-10-08. URL: `https://science.nasa.gov/earth/earth-observatory/lake-erie-blooms-153282` — Supports: earliest bloom since 2002; 830 km2 then 1,700 km2. Used in: How it changes over time.
- [S010] NOAA NCCOS, 2025 Lake Erie HAB Seasonal Assessment (Dec 2025). Accessed 2026-10-08. URL: `https://coastalscience.noaa.gov/news/2025-lake-erie-harmful-algal-bloom-seasonal-assessment` — Supports: 2025 severity 2.4 over 441 sq mi. Used in: How it changes over time.
- [S011] WRI Global Forest Watch, Forest loss 2024 and 2025 analyses (UMD GLAD). Accessed 2026-10-08. URL: `https://gfr.wri.org/global-tree-cover-loss-data-2024` and `https://gfr.wri.org/latest-analysis-deforestation-trends` — Supports: 6.7 Mha 2024 record; −36% 2025 to 4.3 Mha. Used in: How it changes over time.
- [S012] Copernicus Land Monitoring Service, Water Bodies 2020–present (100 m / 300 m monthly). Accessed 2026-10-08. DOI: `10.2909/86c14646-c0b6-4b82-a82f-cbb23b331743` — Supports: current global inland-water baseline. Used in: How it changes; Typical size and mass.
- [S013] USGS Fact Sheet, Lake Baikal — Touchstone for Global Change and Rift Studies. Accessed 2026-10-08. URL: `https://pubs.usgs.gov/fs/baikal` — Supports: 23,000 km3, 1,600+ m depth, 25+ Myr, rift sediments. Used in: How it is created; Examples.
- [S014] NPS, Grand Canyon Geology; Karlstrom et al. 2014 (Nature Geoscience). Accessed 2026-10-08. URL: `https://home.nps.gov/grca/learn/nature/grca-geology.htm` — Supports: uplift 70–30 Ma; integration 5–6 Ma; 100–200 m/Myr. Used in: How it is created.
- [S015] NPS Sonoran Desert Network, Ecosystems. Accessed 2026-10-08. URL: `https://www.nps.gov/im/sodn/ecosystems.htm` — Supports: 260,000 km2 extent; subdivisions. Used in: How it is created; Typical size and mass.
- [S016] USGS, Rivers and Deltas sediment-budget overview (OFR 03-337). Accessed 2026-10-08. URL: `https://pubs.usgs.gov/of/2003/of03-337/rivers-deltas.html` — Supports: delta formation vs reworking; lobe cycle. Used in: How it is created.
- [S017] Euronews, Monterrey water queues (13 Jul 2022). Accessed 2026-10-08. URL: `https://www.euronews.com/2022/07/13/monterrey-is-one-of-mexico-s-richest-cities-so-why-are-residents-having-to-queue-for-water` — Supports: reservoir collapse; 50-day outages. Used in: How it interacts.
- [S018] Sonoran Institute, Colorado River Delta Program; USFS delta restoration review. Accessed 2026-10-08. URL: `https://sonoraninstitute.org/card/colorado-river-delta-program` — Supports: 169,000 ha; 80% wetland loss; restoration. Used in: How it interacts; Examples.
- [S019] USGS, Nutrient pollution of coastal rivers, bays, and seas. Accessed 2026-10-08. URL: `https://pubs.usgs.gov/publication/70174406` — Supports: 60%+ degraded; N vs P roles. Used in: How it interacts.
- [S020] Water Encyclopedia / Wikipedia, List of lakes by volume; Guinness World Records, Baikal. Accessed 2026-10-08. URL: `https://en.wikipedia.org/wiki/List_of_lakes_by_volume` — Supports: 199,000 km3 total; Caspian 78,200; Baikal 23,610; Superior 12,070. Used in: Typical size and mass.
- Britannica, Lake Superior: `https://www.britannica.com/place/Lake-Superior-lake-North-America`
- NPS, lake: `https://www.nps.gov/piro/learn/nature/lake-superior.htm`
- USGS, lakes: `https://www.usgs.gov/water-science-school/science/lakes-and-reservoirs`
- Britannica, Grand Canyon: `https://www.britannica.com/place/Grand-Canyon`
- NPS, geology: `https://www.nps.gov/grca/learn/nature/grca-geology.htm`
- ESA, water bodies: `https://www.esa.int/Applications/Observing_the_Earth/Copernicus/Sentinel-2/Water_bodies`

## References

- Lake and water observation: [S001](#sources), [S002](#sources), [S003](#sources), [S012](#sources)
- Monterrey and Sierra Madre Oriental: [S004](#sources), [S005](#sources)
- Lake change 1992–2026: [S006](#sources), [S007](#sources), [S008](#sources), [S009](#sources), [S010](#sources)
- Forests 2024–2025: [S011](#sources)
- Formation and deep time: [S013](#sources), [S014](#sources), [S015](#sources), [S016](#sources)
- Interaction and coasts: [S017](#sources), [S018](#sources), [S019](#sources)
- Lake volumes: [S001](#sources), [S020](#sources)

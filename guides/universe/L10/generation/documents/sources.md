# Sources for L10 generation

External records for the planet and moon generation research. Identifiers
are `GEN-S<nnn>`; cite them inline as `[GEN-S001]`. All were accessed on
2026-10-14. Every entry has a **Limitations** line saying how deeply it
was read: **full text** (the source itself was read for the claims used),
or only a summary, an abstract, or a later paper quoting it. Internal
project references are listed at the end without an identifier.

Full-text reads so far: GEN-S012, GEN-S034, GEN-S039, GEN-S051, GEN-S067,
GEN-S071 (the sources behind the formulas the pages use).

The records are split in two files to keep each readable:
- this file: GEN-S001 to GEN-S031 (first version: noise and terrain,
  craters and relief limits, planet classes, moon systems, lakes,
  plates);
- [research-pass sources](./sources-passes.md): GEN-S032 to GEN-S076
  (lifetime, terrain techniques, climate and look, moon dynamics,
  landforms, and passes A, B and C).

## Noise and terrain synthesis

## GEN-S001 — Simplex noise demystified
- **Title:** Simplex noise demystified
- **Author:** Stefan Gustavson (Linköping University)
- **Published:** 2005-03-22
- **Type:** Technical note
- **URL:** [cgvr.cs.uni-bremen.de/teaching/cg_literatur/simplexnoise.pdf](https://cgvr.cs.uni-bremen.de/teaching/cg_literatur/simplexnoise.pdf)
- **Supports:**
  - Perlin presented simplex noise in SIGGRAPH 2001 and 2002 course notes,
    to fix the limits of classic Perlin noise.
  - Simplex noise costs less in higher dimensions and has no noticeable
    directional artifacts.
- **Used in:** [Height maps](./height-maps.md)
- **Limitations:** Read through search summaries. Perlin's original papers
  (1985, 2002) are cited through it, not read.

## GEN-S002 — Domain warping
- **Title:** Domain warping
- **Author:** Inigo Quilez
- **Publisher:** iquilezles.org
- **Type:** Technical article
- **URL:** [iquilezles.org/articles/warp](https://iquilezles.org/articles/warp)
- **Supports:**
  - Domain warping evaluates `f(p + h(p))` instead of `f(p)`, with fBm
    for both `f` and `h`, which gives organic shapes.
  - The nested form `fbm(p + fbm(p + fbm(p)))`.
- **Used in:** [Height maps](./height-maps.md)
- **Limitations:** Read through search summaries and The Book of Shaders,
  which quotes it.

## GEN-S003 — Musgrave noise types (ridged and hybrid multifractal)
- **Title:** Musgrave Texture Node
- **Author:** Blender Foundation
- **Publisher:** Blender manual 2.80
- **Type:** Software documentation
- **URL:** [docs.blender.org/manual/en/2.80/render/shader_nodes/textures/musgrave.html](https://docs.blender.org/manual/en/2.80/render/shader_nodes/textures/musgrave.html)
- **Supports:**
  - Ridged multifractal: absolute value of the noise forms "canyons", then
    the surface is flipped, giving sharp peaks.
  - Hybrid multifractal: an additive and a multiplicative cascade, so
    rough peaks rise from smoother plains.
  - Offset and gain control where fine detail starts and how strong it is.
- **Used in:** [Height maps](./height-maps.md)
- **Limitations:** An implementation's notes. The original definitions are
  in Ebert, Musgrave, Peachey, Perlin and Worley, *Texturing and Modeling:
  A Procedural Approach* (not read here).

## GEN-S004 — Fast hydraulic erosion simulation on GPU
- **Title:** Fast Hydraulic Erosion Simulation and Visualization on GPU
- **Author:** Xing Mei, Philippe Decaudin, Bao-Gang Hu
- **Publisher:** Pacific Graphics 2007 (PG '07), IEEE, pp. 47-56
- **Published:** 2007
- **Type:** Conference paper
- **URL:** [hgpu.org/?p=2256](https://hgpu.org/?p=2256)
- **Supports:**
  - A grid ("shallow water") erosion model fast enough for interactive
    use, where earlier physical models ran at a few frames per second on
    256 x 256 grids.
- **Used in:** [Height maps](./height-maps.md)
- **Limitations:** Read through the slides and the 2011 follow-up
  [GEN-S005], not the full paper.

## GEN-S005 — Fast hydraulic and thermal erosion on GPU
- **Title:** Fast Hydraulic and Thermal Erosion on GPU
- **Author:** Balázs Jákó, Balázs Tóth
- **Publisher:** Eurographics 2011 short papers, pp. 57-60
- **Published:** 2011
- **Type:** Conference paper
- **URL:** [diglib.eg.org/handle/10.2312/EG2011.short.057-060](https://diglib.eg.org/handle/10.2312/EG2011.short.057-060)
- **Supports:**
  - Hydraulic erosion as an improved form of [GEN-S004], plus thermal
    erosion (slopes collapse to a talus angle).
- **Used in:** [Height maps](./height-maps.md)
- **Limitations:** Read through the abstract page only.

## GEN-S006 — Making worlds: scaling heights
- **Title:** Making Worlds 2 — Scaling heights
- **Author:** Steven Wittens
- **Publisher:** acko.net
- **Type:** Engineering blog post
- **URL:** [acko.net/blog/making-worlds-2-scaling-heights](https://acko.net/blog/making-worlds-2-scaling-heights/)
- **Supports:**
  - A cube projected onto a sphere as the base of a planet terrain, with
    the heights built per face.
- **Used in:** [Height maps](./height-maps.md#sampling-on-a-sphere)
- **Limitations:** A practitioner's write-up. The cube-versus-icosphere
  trade-offs were confirmed only by forum discussions, which are not
  recorded as sources.

## Craters and relief limits

## GEN-S007 — Pike, morphometry of fresh lunar craters
- **Title:** Size-dependence in the shape of fresh impact craters on the Moon
- **Author:** Richard J. Pike
- **Publisher:** In Roddy, Pepin and Merrill (eds.), *Impact and Explosion
  Cratering*, Pergamon
- **Published:** 1977
- **Type:** Book chapter (NASA NTRS record)
- **URL:** [ntrs.nasa.gov/citations/19780057837](https://ntrs.nasa.gov/citations/19780057837)
- **Supports:**
  - Simple lunar craters (under about 15 km) have a depth-to-diameter
    ratio of about 0.2; complex craters 0.1 to 0.01.
  - Complex-crater depth `d = 1.044 D^0.301` (km), as quoted by
    [GEN-S008].
- **Used in:** [Height maps](./height-maps.md#building-block-6-craters)
- **Limitations:** Abstract and secondary quotes only. Another quote puts
  the transition at a 10.6 km rim diameter.

## GEN-S008 — Lunar craters and the simple-to-complex transition
- **Title:** Re-examining the main asteroid belt as the primary source of
  ancient lunar craters
- **Publisher:** arXiv:1408.5304
- **Type:** Research paper (preprint)
- **URL:** [arxiv.org/pdf/1408.5304](https://arxiv.org/pdf/1408.5304)
- **Supports:**
  - Transition diameter falls with gravity: `D_tr = 16533.8 g^-1.0303`
    (m, with `g` in m/s²).
  - Quotes `d = 0.2 D` for simple craters and Pike's complex-crater depth.
- **Used in:** [Height maps](./height-maps.md#building-block-6-craters)
- **Limitations:** Read through search summaries.

## GEN-S009 — Equilibrium size-frequency of small lunar craters
- **Title:** The equilibrium size-frequency distribution of small craters
  reveals the effects of distal ejecta on lunar landscape morphology
- **Publisher:** arXiv:1902.07746
- **Type:** Research paper (preprint)
- **URL:** [arxiv.org/pdf/1902.07746](https://arxiv.org/pdf/1902.07746)
- **Supports:**
  - A saturated crater population has a cumulative frequency close to
    `D^-2`; equilibrium means each new crater erases an old one on average.
- **Used in:** [Height maps](./height-maps.md#building-block-6-craters)
- **Limitations:** Read through search summaries. The exact Neukum
  production function is not reproduced here.

## GEN-S010 — Mountain height and gravity
- **Title:** Calculating the Potato Radius of Asteroids using the Height of
  Mt. Everest
- **Publisher:** arXiv:1511.04297
- **Type:** Research paper (preprint)
- **URL:** [arxiv.org/pdf/1511.04297](https://arxiv.org/pdf/1511.04297)
- **Supports:**
  - Rock strength caps a mountain's height at roughly `1/g`; height times
    gravity is about constant (within about 10 percent) for Venus, Earth
    and Mars. It is an upper bound.
  - Olympus Mons is about 21.9 km on Mars (about 0.38 g).
- **Used in:** [Height maps](./height-maps.md#how-gravity-limits-relief),
  [planet types](./planet-types.md)
- **Limitations:** Preprint. Tectonics, erosion and base width also limit
  real mountains.

## GEN-S011 — The potato radius
- **Title:** The Potato Radius: a Lower Minimum Size for Dwarf Planets
- **Author:** Charles H. Lineweaver, Marc Norman
- **Publisher:** arXiv:1004.1091
- **Published:** 2010
- **Type:** Research paper
- **URL:** [arxiv.org/abs/1004.1091](https://arxiv.org/abs/1004.1091)
- **Supports:**
  - Icy moons and rocky asteroids change from potato shapes to spheres at
    a mean radius of about 200 km (icy) to 300 km (rocky).
- **Used in:** [Moons](./moons.md#round-or-potato),
  [planet types](./planet-types.md)
- **Limitations:** Read through the abstract and search summaries.

## Planet classes

## GEN-S012 — Probabilistic mass-radius relation (Forecaster)
- **Title:** Probabilistic Forecasting of the Masses and Radii of Other Worlds
- **Author:** Jingjing Chen, David Kipping
- **Publisher:** *The Astrophysical Journal* 834, 17; arXiv:1603.08614
- **Published:** 2017
- **Type:** Research paper
- **URL:** [arxiv.org/abs/1603.08614](https://arxiv.org/abs/1603.08614)
- **Supports:**
  - Fitted on 316 objects; Table 1: Terran constant 1.008 Earth radii;
    slopes 0.2790 (Terran), 0.589 (Neptunian), −0.044 (Jovian), 0.881
    (stars); transitions 2.04 Earth masses, 0.414 Jupiter masses, 0.0800
    Sun masses; radius spreads 4.03, 14.6, 7.37 and 4.43 percent.
- **Used in:** [Planet types](./planet-types.md#the-classifier)
- **Limitations:** Full text (Table 1) read. Few planets near one Earth
  mass were known, so the Terran-Neptunian point is the least certain.

## GEN-S013 — The mass-radius relation of exoplanets, revisited
- **Title:** The mass-radius relation of exoplanets, revisited
- **Publisher:** arXiv:2311.12593
- **Published:** 2023
- **Type:** Research paper (preprint)
- **URL:** [arxiv.org/pdf/2311.12593](https://arxiv.org/pdf/2311.12593)
- **Supports:** The quoted transitions and slopes of [GEN-S012].
- **Used in:** [Planet types](./planet-types.md#the-classifier)
- **Limitations:** Read through search summaries.

## GEN-S014 — The radius gap (California-Kepler Survey VII and X)
- **Title:** The California-Kepler Survey VII: Precise Planet Radii
  Leveraging Gaia DR2 Reveal the Stellar Mass Dependence of the Planet
  Radius Gap; and CKS X: The Radius Gap as a Function of Stellar Mass,
  Metallicity, and Age
- **Author:** B. J. Fulton, E. A. Petigura (2018); E. A. Petigura et al. (2022)
- **Publisher:** *The Astronomical Journal*; arXiv:1805.01453 and
  arXiv:2201.10020
- **Type:** Research papers
- **URL:** [arxiv.org/abs/1805.01453](https://www.arxiv.org/abs/1805.01453),
  [arxiv.org/abs/2201.10020](https://arxiv.org/abs/2201.10020)
- **Supports:**
  - Small planets split into super-Earths under 1.5 Earth radii and
    sub-Neptunes from 2.0 to 4.0 Earth radii, with few between.
  - The gap is partly filled, moves with the star's mass and the orbit,
    and fits atmosphere loss (photoevaporation or core-powered loss).
- **Used in:** [Planet types](./planet-types.md#the-classifier)
- **Limitations:** Read through abstracts. The 2017 discovery paper
  (Fulton et al.) is cited through them.

## GEN-S015 — Planetary equilibrium temperature
- **Title:** Planetary equilibrium temperature
- **Publisher:** Wikipedia
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Planetary_equilibrium_temperature](https://en.wikipedia.org/wiki/Planetary_equilibrium_temperature)
- **Supports:**
  - Absorbed starlight (with albedo `A`) equals emitted heat; the factor 4
    is the lit disk against the whole sphere.
- **Used in:** [Planet types](./planet-types.md#the-classifier)
- **Limitations:** Tertiary source for a textbook formula.

## GEN-S016 — The water-ice line (worked problem)
- **Title:** Astronomy 141, homework 2 solutions
- **Author:** B. Scott Gaudi (Ohio State University)
- **Type:** Course material
- **URL:** [astronomy.ohio-state.edu/gaudi.1/AST141/homework2_sol.pdf](https://www.astronomy.ohio-state.edu/gaudi.1/AST141/homework2_sol.pdf)
- **Supports:**
  - With a Sun-like star, zero albedo and water ice at 170 K, the ice line
    is at about 2.7 AU. Water ice sublimates at 150 to 170 K depending on
    what it is mixed with.
- **Used in:** [Planet types](./planet-types.md#the-classifier)
- **Limitations:** Course material. Hayashi (1981) is the usual reference
  and was not read.

## GEN-S017 — Zonal jets and the Rhines scale
- **Title:** Atmospheric Circulation of Terrestrial Exoplanets
- **Author:** A. P. Showman, R. D. Wordsworth, T. M. Merlis, Y. Kaspi
- **Publisher:** arXiv:1306.2418 (chapter in *Comparative Climatology of
  Terrestrial Planets*)
- **Published:** 2013
- **Type:** Review
- **URL:** [arxiv.org/pdf/1306.2418](https://arxiv.org/pdf/1306.2418)
- **Supports:**
  - The Rhines length `L ~ π (U/β)^1/2` measures the width of zonal wind
    bands, so faster spin (larger `β`) gives narrower and more bands.
- **Used in:** [Planet types](./planet-types.md#gas-giants)
- **Limitations:** Read through search summaries.

## Moon systems

## GEN-S018 — Stability of distant satellites
- **Title:** Stability of Distant Satellites of the Giant Planets in the
  Solar System
- **Publisher:** arXiv:0806.2882
- **Type:** Research paper
- **URL:** [ar5iv.arxiv.org/html/0806.2882](https://ar5iv.arxiv.org/html/0806.2882)
- **Supports:**
  - Prograde satellites are stable out to about 0.4-0.5 Hill radii,
    retrograde ones to about 0.7 Hill radii (Hénon; Shen and Tremaine).
  - Regular moons orbit within about 0.05 Hill radii on near-circular,
    prograde, equatorial orbits; irregular moons orbit at about
    0.05-0.6 Hill radii on eccentric, inclined orbits.
- **Used in:** [Moons](./moons.md#where-moons-can-orbit)
- **Limitations:** Read through search summaries. Limits depend on
  eccentricity and method.

## GEN-S019 — Irregular moon
- **Title:** Irregular moon
- **Publisher:** Wikipedia
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Irregular_moon](https://en.wikipedia.org/wiki/Irregular_moon)
- **Supports:**
  - Retrograde orbits are far more common (about 83 percent) among
    irregular moons; no known irregular moon has a semi-major axis beyond
    about 0.47 Hill radii.
- **Used in:** [Moons](./moons.md#where-moons-can-orbit)
- **Limitations:** Tertiary source. Its 0.47 limit differs from the 0.6
  in [GEN-S018].

## GEN-S020 — A common mass scaling for satellite systems of gaseous planets
- **Title:** A common mass scaling for satellite systems of gaseous planets
- **Author:** Robin M. Canup, William R. Ward
- **Publisher:** *Nature* 441, 834-839; DOI 10.1038/nature04860
- **Published:** 2006
- **Type:** Research paper
- **URL:** [boulder.swri.edu/~robin/canupward2006.pdf](https://www.boulder.swri.edu/~robin/canupward2006.pdf)
- **Supports:**
  - The regular moons of Jupiter, Saturn and Uranus each total about
    `10^-4` of their planet's mass, set by inflow against gas-driven
    orbital decay; earlier moon generations were lost.
  - This fraction is two to three orders of magnitude smaller than the
    largest moons of the solid planets (the Moon is about 1/81 of Earth).
- **Used in:** [Moons](./moons.md#how-much-moon)
- **Limitations:** Uranus's system origin is less certain.

## GEN-S021 — Roche limit
- **Title:** Roche limit: definition, formula, physics and examples
- **Publisher:** Science Notes
- **Type:** Educational article
- **URL:** [sciencenotes.org/roche-limit-definition-formula-physics-and-examples](https://sciencenotes.org/roche-limit-definition-formula-physics-and-examples/)
- **Supports:**
  - Fluid satellite: `d ≈ 2.44 R (ρ_planet / ρ_moon)^1/3`; rigid body:
    `d ≈ 1.26 R (ρ_planet / ρ_moon)^1/3`.
  - Material inside the limit spreads into rings (Saturn's main rings lie
    almost entirely inside it).
- **Used in:** [Moons](./moons.md#where-moons-can-orbit)
- **Limitations:** Secondary source. Darwin gives about 2.455 for the
  fluid constant.

## GEN-S022 — Hill sphere
- **Title:** Hill sphere
- **Publisher:** Wikipedia
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Hill_sphere](https://en.wikipedia.org/wiki/Hill_sphere)
- **Supports:** `r_H ≈ a (1 - e) (m / 3M)^1/3` for a planet of mass `m` at
  semi-major axis `a` around a star of mass `M`.
- **Used in:** [Moons](./moons.md#where-moons-can-orbit)
- **Limitations:** Textbook formula. Not opened in this run; listed because
  the moon pages rely on it.

## Lakes

## GEN-S023 — Lake types by origin (Hutchinson's classification)
- **Title:** Lake types, as summarised from G. E. Hutchinson, *A Treatise on
  Limnology*, vol. 1 (1957)
- **Publisher:** Taylor and Francis book chapter
  ([taylorfrancis.com/books/9781482206081/chapters/10.1201/b16108-13](https://www.taylorfrancis.com/books/9781482206081/chapters/10.1201/b16108-13))
  and encyclopedia summaries
- **Type:** Secondary summaries of a classic treatise
- **Supports:**
  - Hutchinson defined 76 lake types by origin, grouped into about 11
    major kinds: tectonic, volcanic, glacial, fluvial, solution,
    landslide, aeolian, shoreline, organic, anthropogenic, and meteorite
    (impact).
- **Used in:** [Lakes](./lakes.md)
- **Limitations:** The treatise itself was not read; summaries differ in
  which major kinds they list.

## GEN-S024 — Priority-Flood depression filling
- **Title:** Priority-Flood: An Optimal Depression-Filling and
  Watershed-Labeling Algorithm for Digital Elevation Models
- **Author:** Richard Barnes, Clarence Lehman, David Mulla
- **Publisher:** *Computers and Geosciences* 62, 117-127; arXiv:1511.04463
- **Published:** 2014
- **Type:** Research paper
- **URL:** [arxiv.org/abs/1511.04463](https://arxiv.org/abs/1511.04463)
- **Supports:**
  - Flooding a height grid inward from its edges with a priority queue
    fills every depression and labels watersheds; `O(n)` for integer and
    `O(n log n)` for floating-point heights.
  - Spill elevations where watersheds meet (as described by later tools).
- **Used in:** [Lakes](./lakes.md#finding-lakes-in-a-height-map)
- **Limitations:** Spill-point detail read through tool documentation.

## GEN-S025 — Lakes and seas of Titan
- **Title:** Lakes of Titan; Cassini's final view of Titan's northern lakes
  and seas
- **Publisher:** Wikipedia; NASA Jet Propulsion Laboratory
- **Type:** Encyclopedia article; agency news release
- **URL:** [en.wikipedia.org/wiki/Lakes_of_Titan](https://en.wikipedia.org/wiki/Lakes_of_Titan),
  [jpl.nasa.gov/news/cassinis-final-view-of-titans-northern-lakes-and-seas](https://www.jpl.nasa.gov/news/cassinis-final-view-of-titans-northern-lakes-and-seas/)
- **Supports:**
  - Methane and ethane lakes and seas cluster near the poles, mostly in
    the north (Kraken Mare largest; Ligeia Mare about 126,000 km²);
    hundreds of small northern lakes; few southern lakes (Ontario Lacus).
- **Used in:** [Lakes](./lakes.md#lakes-on-other-worlds)
- **Limitations:** Sizes and composition vary between sources.

## GEN-S026 — Open and closed basin lakes on Mars
- **Title:** NASA Technical Reports Server abstract on Martian open- and
  closed-basin lakes
- **Publisher:** NASA NTRS
- **Type:** Conference abstract
- **URL:** [ntrs.nasa.gov/citations/20170002037](https://ntrs.nasa.gov/citations/20170002037)
- **Supports:**
  - Closed basins: the source valleys end in the basin. Open basins: the
    inflow filled the lake until it breached and cut an outlet valley.
  - Most valley-fed basins were open, with exceptions such as Gale crater.
- **Used in:** [Lakes](./lakes.md#how-lake-basins-form)
- **Limitations:** Read through search summaries.

## GEN-S027 — Jezero crater lake
- **Title:** Jezero crater: an ancient lake with inlet and outlet valleys
  (Fassett and Head 2005, as described by later work)
- **Publisher:** Brown University news (2015); University of Chicago
  repository (Pliva Vallis study)
- **Type:** News release; research paper
- **URL:** [archive2.news.brown.edu/2007-2015/articles/2015/03/jezero.html](https://archive2.news.brown.edu/2007-2015/articles/2015/03/jezero.html),
  [knowledge.uchicago.edu/record/15248](https://knowledge.uchicago.edu/record/15248)
- **Supports:**
  - Jezero, about 45 km across, was fed by two inlet valleys and drained by
    one outlet valley (Pliva Vallis), so it was an open basin.
- **Used in:** [Lakes](./lakes.md#how-lake-basins-form)
- **Limitations:** Fassett and Head (2005) read only through later
  descriptions.

## Plates

## GEN-S028 — Procedural tectonic planets
- **Title:** Procedural Tectonic Planets
- **Author:** Yann Cortial, Adrien Peytavie, Eric Galin, Eric Guérin
- **Publisher:** *Computer Graphics Forum* 38(2), Eurographics 2019;
  DOI 10.1111/cgf.13614
- **Published:** 2019
- **Type:** Research paper
- **URL:** [diglib.eg.org/handle/10.1111/cgf13614](https://diglib.eg.org/handle/10.1111/cgf13614)
- **Supports:**
  - A procedural (not physics-based) model of plate subduction,
    collisions and rifting that produces continents, ridges, mountain
    ranges and island arcs, with user-steered plate movement.
- **Used in:** [Plates](./plates.md#generating-plates-recommendation)
- **Limitations:** Read through the abstract record only; the full text
  is listed on HAL as hal-02136820.

## GEN-S029 — The Wilson cycle
- **Title:** Wilson Cycles (Historical Geology, Bentley et al.); USGS
  Circular 1264 field-trip guide
- **Publisher:** LibreTexts Geosciences; U.S. Geological Survey
- **Type:** Open textbook; government publication
- **URL:** [geo.libretexts.org — 6.07 Wilson Cycles](https://geo.libretexts.org/Bookshelves/Geology/Historical_Geology_(Bentley_et_al.)/06%3A_Plate_Tectonics/6.07%3A_Wilson_Cycles),
  [pubs.usgs.gov/circ/2004/1264/html/trip1](https://pubs.usgs.gov/circ/2004/1264/html/trip1/)
- **Supports:**
  - Ocean basins open by rifting and close through subduction and
    mountain building, ending in continent-continent collision; the
    Appalachians record repeated cycles.
- **Used in:** [Plates](./plates.md#how-rifts-form)
- **Limitations:** Read through search summaries.

## GEN-S030 — Sea-floor depth and age
- **Title:** An analysis of the variation of ocean floor bathymetry and heat
  flow with age
- **Author:** Barry Parsons, John G. Sclater
- **Publisher:** *Journal of Geophysical Research* 82(5)
- **Published:** 1977
- **Type:** Research paper
- **URL:** [topex.ucsd.edu/geodynamics/parsons_sclater77.pdf](https://topex.ucsd.edu/geodynamics/parsons_sclater77.pdf)
- **Supports:**
  - Depth `≈ 2500 + 350 √t` m for floor younger than about 70 million
    years; `6400 − 3200 exp(−t / 62.8)` m for older floor.
  - Korenaga and Korenaga (2008) later found about 320 m per √(million
    years), about 10 percent less.
- **Used in:** [Plates](./plates.md#how-rifts-form)
- **Limitations:** The paper's text was read through OCR snippets; the
  square-root law is confirmed by later papers.

## GEN-S031 — Isostasy and mountain roots
- **Title:** Isostasy
- **Publisher:** Wikipedia; with an ISTerre (Grenoble) Himalaya exercise
- **Type:** Encyclopedia article; course material
- **URL:** [en.wikipedia.org/wiki/Isostasy](https://en.wikipedia.org/wiki/Isostasy),
  [isterre.fr — High reliefs in NW Himalaya](https://www.isterre.fr/IMG/pdf/High_Relliefs_in_NW_Himalaya.pdf)
- **Supports:**
  - Airy isostasy: a mountain's root is about 5 to 6 times its height
    with typical crust and mantle densities; the crust under the Himalaya
    and southern Tibet reaches about 70 km.
  - Convergent margins are partly held up by horizontal stresses, not in
    full isostatic balance.
- **Used in:** [Plates](./plates.md#how-mountains-form)
- **Limitations:** Tertiary and course sources for a textbook result.

## Internal references

- `crates/universe-core/src/terrain.rs` — today's body recipe and moon
  count (read 2026-10-14).
- `crates/universe-core/src/noise.rs` — the shared deterministic value
  noise.
- [Planet and moon kinds catalog](../../../catalogs/documents/planets.md).

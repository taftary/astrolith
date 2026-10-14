# Sources for L10 generation

External records for the planet and moon generation research. Identifiers
are `GEN-S<nnn>`; cite them inline as `[GEN-S001]`. All were accessed on
2026-10-14. Every entry has a **Limitations** line saying how deeply it
was read: **full text** (the source itself was read for the claims used),
or only a summary, an abstract, or a later paper quoting it. Internal
project references are listed at the end without an identifier.

Full-text reads so far: GEN-S012, GEN-S034, GEN-S039, GEN-S051 (the
sources behind the formulas the pages use).

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

## Lifetime

## GEN-S032 — Lunar cratering chronology
- **Title:** Neukum et al. (2001) chronology function, as given in
  *Early Bombardment of the Moon* (arXiv:2303.17736) and the Cratermaker
  documentation
- **Publisher:** arXiv; Cratermaker project documentation
- **Type:** Research paper (review); software documentation
- **URL:** [arxiv.org/pdf/2303.17736](https://arxiv.org/pdf/2303.17736),
  [cratermaker.readthedocs.io — plot chronology](https://cratermaker.readthedocs.io/en/latest/auto_examples/02-production/2.3-plot_chronology.html)
- **Supports:**
  - `N(1 km) = a (exp(λT) − 1) + bT` per km², with `a = 5.44e-14`,
    `b = 8.38e-4`, `λ = 6.93` (T in billions of years).
  - Not constrained before about 4.1 billion years; almost no dated
    surfaces between 1 and 3 billion years.
- **Used in:** [Lifetime](./lifetime.md#craters-pile-up)
- **Limitations:** Neukum et al. 2001 read through these sources; Marchi
  et al. 2009 give slightly different coefficients.

## GEN-S033 — The cosmic shoreline
- **Title:** The Cosmic Shoreline: The Evidence that Escape Determines
  which Planets Have Atmospheres, and what this May Mean for Proxima
  Centauri B
- **Author:** Kevin J. Zahnle, David C. Catling
- **Publisher:** *The Astrophysical Journal* 843, 122; arXiv:1702.03386
- **Published:** 2017
- **Type:** Research paper
- **URL:** [arxiv.org/abs/1702.03386](https://arxiv.org/abs/1702.03386)
- **Supports:**
  - Bodies with and without atmospheres separate along `I ∝ v_esc^4` in
    insolation against escape velocity, across planets, moons, asteroids,
    Kuiper Belt objects and exoplanets.
- **Used in:** [Lifetime](./lifetime.md#air-is-kept-or-lost)
- **Limitations:** An empirical line; its physical cause (thermal escape,
  impact erosion) is discussed but not settled.

## GEN-S034 — Tidal locking timescale
- **Title:** Tidal locking (Gladman et al. 1996 formula, as quoted)
- **Publisher:** Wikipedia; secondary research papers (for example
  arXiv:1407.1214)
- **Type:** Encyclopedia article; research papers quoting the formula
- **URL:** [en.wikipedia.org/wiki/Tidal_locking](https://en.wikipedia.org/wiki/Tidal_locking),
  [arxiv.org/pdf/1407.1214](https://arxiv.org/pdf/1407.1214)
- **Supports:**
  - `t_lock ≈ ω a^6 I Q / (3 G m_p^2 k2 R^5)` (equation 9 of Gladman et
    al. 1996, *Icarus* 122, after Peale 1977), with `I ≈ 0.4 m_s R^2`.
  - The simplified form `t_lock ≈ 6 × a^6 R μ / (m_s m_p^2) × 10^10`
    years for `Q = 100` and a 12-hour initial spin (Hanslmeier 2018);
    rigidity about `3 × 10^10` N/m² for rock, `4 × 10^9` N/m² for ice.
  - Strong `a^6` sensitivity; results uncertain by factors of ten.
- **Used in:** [Lifetime](./lifetime.md#spins-slow-and-moons-move)
- **Limitations:** Full text of the encyclopedia article read; the
  Gladman et al. paper itself was not.

## GEN-S035 — The brightening Sun
- **Title:** Habitability of the early Earth: liquid water under a faint
  young Sun (arXiv:2007.03423); Changes in Solar Output and in the Earth's
  Atmosphere (Earle, *Environmental Geology*)
- **Publisher:** arXiv; LibreTexts Geosciences
- **Type:** Research paper; open textbook
- **URL:** [arxiv.org/pdf/2007.03423](https://arxiv.org/pdf/2007.03423),
  [geo.libretexts.org — 3.01 Changes in Solar Output](https://geo.libretexts.org/Bookshelves/Geology/Environmental_Geology_(Earle)/03%3A_Climate_Changes_in_Earths_Past/3.01%3A_Changes_in_Solar_Output_and_in_the_Earths_Atmosphere)
- **Supports:**
  - The young Sun settled at about 70 percent of today's output and has
    brightened by a little under 1 percent per 100 million years.
  - Other estimates of early brightness range up to 75-80 percent.
- **Used in:** [Lifetime](./lifetime.md#the-star-brightens)
- **Limitations:** Read through search summaries.

## GEN-S036 — Crater degradation by diffusion
- **Title:** Lunar crater degradation and age (Fassett and Thomson 2014
  framework), NASA LPSC presentation
- **Author:** Caleb I. Fassett and co-authors
- **Publisher:** NASA NTRS
- **Type:** Conference presentation
- **URL:** [ntrs.nasa.gov/api/citations/20210026637/downloads/Fassett_VIPER-DegAge_LPSC_v4_ScanCopy.pdf](https://ntrs.nasa.gov/api/citations/20210026637/downloads/Fassett_VIPER-DegAge_LPSC_v4_ScanCopy.pdf)
- **Supports:**
  - Lunar landforms degrade by linear topographic diffusion; typical
    diffusivity about 5 m² per million years at kilometre scale over the
    last 3 billion years, about 200 times lower than in the western US.
  - After 3 billion years a 1 km crater keeps about half its depth.
- **Used in:** [Lifetime](./lifetime.md#craters-soften)
- **Limitations:** The 2014 paper itself was not read.

## GEN-S037 — The short-lived Martian dynamo
- **Title:** Mars dynamo and core conductivity (Hsieh et al. 2024,
  *Science Advances*)
- **Publisher:** University of Texas repository
- **Type:** Research paper
- **URL:** [jsg.utexas.edu/lin/files/HsiehMarsDynamoFeSSciAd2024.pdf](https://www.jsg.utexas.edu/lin/files/HsiehMarsDynamoFeSSciAd2024.pdf)
- **Supports:**
  - A highly conductive Martian core cooled efficiently and stratified,
    giving a short-lived dynamo (about 0.5-0.8 billion years) that ceased
    about 4 billion years ago.
- **Used in:** [Lifetime](./lifetime.md#the-interior-cools)
- **Limitations:** Popular accounts give other timings; this study is the
  more specific one.

## Terrain techniques

## GEN-S038 — Terrain generation from hydrology
- **Title:** Terrain Generation Using Procedural Models Based on Hydrology
- **Author:** Jean-David Génevaux, Éric Galin, Éric Guérin, Adrien
  Peytavie, Bedřich Beneš
- **Publisher:** *ACM Transactions on Graphics* 32(4), SIGGRAPH 2013
- **Published:** 2013
- **Type:** Research paper
- **URL:** [cs.purdue.edu/homes/bbenes/papers/Genevaux13ToG.pdf](https://www.cs.purdue.edu/homes/bbenes/papers/Genevaux13ToG.pdf)
- **Supports:**
  - A hierarchical drainage network (geometric graph) is built first,
    analysed into watersheds and river types, then terrain is generated by
    blending procedural terrain and river patches, from a sketch and a few
    parameters.
- **Used in:** [Terrain techniques](./terrain-techniques.md#rivers-first)
- **Limitations:** Read through the abstract.

## GEN-S039 — Value noise derivatives
- **Title:** Value noise derivatives
- **Author:** Inigo Quilez
- **Publisher:** iquilezles.org
- **Type:** Technical article
- **URL:** [iquilezles.org/articles/morenoise](https://iquilezles.org/articles/morenoise)
- **Supports:**
  - Analytic derivatives of value noise are faster and more accurate than
    central differences; analytic normals for a whole fractal sum; used for
    the landscape of the *Elevated* demo.
  - The terrain code: 15 octaves, each octave's value divided by
    `1 + dot(d, d)` with `d` the running sum of derivatives; gives
    erosion-like flat and rough areas.
- **Used in:** [Terrain techniques](./terrain-techniques.md#derivative-slope-aware-noise)
- **Limitations:** Full text read.

## GEN-S040 — Cellular texture basis
- **Title:** A Cellular Texture Basis Function
- **Author:** Steven Worley
- **Publisher:** SIGGRAPH 1996
- **Published:** 1996
- **Type:** Conference paper
- **URL:** [cedric.cnam.fr/~cubaud/PROCEDURAL/worley.pdf](https://cedric.cnam.fr/~cubaud/PROCEDURAL/worley.pdf)
- **Supports:**
  - `F1`, `F2`, `Fn`: distances to the nearest feature points; uses for
    flagstones, skin, ice, rock, mountain ranges and craters.
  - `F2 − F1` outlines cell boundaries (as used in later work).
- **Used in:** [Terrain techniques](./terrain-techniques.md#cellular-worley-noise)
- **Limitations:** Abstract and summaries only.

## GEN-S041 — Uplift and stream-power erosion
- **Title:** Large Scale Terrain Generation from Tectonic Uplift and
  Fluvial Erosion (2016); Physically-based Analytical Erosion for fast
  Terrain Generation (2024)
- **Author:** Guillaume Cordonnier and co-authors (2016); Petros Tzathas,
  Boris Gailleton, Philippe Steer, Guillaume Cordonnier (2024)
- **Publisher:** *Computer Graphics Forum* (Eurographics 2016 and 2024)
- **Type:** Research papers
- **URL:** [diglib.eg.org/handle/10.1111/cgf12820](https://diglib.eg.org/handle/10.1111/cgf12820),
  [diglib.eg.org/handle/10.1111/cgf15033](https://diglib.eg.org/handle/10.1111/cgf15033)
- **Supports:**
  - Combining a user-painted uplift map with stream-power erosion on a
    stream graph; the graph becomes a height model by blending landform
    kernels.
  - Analytical solutions of the stream-power law for fast, procedural,
    physically based generation.
- **Used in:** [Terrain techniques](./terrain-techniques.md#analytic-stream-power-terrain)
- **Limitations:** Abstracts only; the full paper was too large to fetch.
  The exponents used in the papers were not confirmed. One open-source
  implementation citing the 2016 paper (Veloren's world generator) uses
  `dh/dt = uplift − k A^m S^n` with `m = 0.4`, `n = 1` as its own choice.

## GEN-S042 — Droplet (particle) hydraulic erosion
- **Title:** Erodr (implementation of Beyer's method); UIUC CS418 erosion
  notes
- **Publisher:** GitHub (henrikglass/erodr); University of Illinois course
  notes
- **Type:** Open-source software; course material
- **URL:** [github.com/henrikglass/erodr](https://github.com/henrikglass/erodr),
  [courses.grainger.illinois.edu/cs418/fa2024/text/erosion.html](https://courses.grainger.illinois.edu/cs418/fa2024/text/erosion.html)
- **Supports:**
  - Particles move downhill, carry sediment up to a capacity and deposit
    it; inertia default about 0.1, capacity factor default about 10.
  - The particle approach was first published by Chiba and colleagues
    (1999).
- **Used in:** [Terrain techniques](./terrain-techniques.md#simulated-erosion)
- **Limitations:** Beyer's thesis itself was not read.

## GEN-S043 — Glacial valley cross-sections
- **Title:** Numerical simulation of the evolution of glacial valley cross
  sections (arXiv:0901.1177); Is the cross-section of a glacial valley a
  parabola? (*Journal of Glaciology*)
- **Publisher:** arXiv; Cambridge University Press
- **Type:** Research papers
- **URL:** [arxiv.org/pdf/0901.1177](https://arxiv.org/pdf/0901.1177)
- **Supports:**
  - `y = a x^b` fits valley flanks: `b` near 1 is V-shaped; about 1.5-2
    is the glacial U shape; Tian Shan values mostly 1.3-2.5.
- **Used in:** [Terrain techniques](./terrain-techniques.md#glacial-valleys-and-fjords)
- **Limitations:** The exponent depends on the bedrock datum used.

## GEN-S044 — Fractal coastlines
- **Title:** How Long Is the Coast of Britain? Statistical Self-Similarity
  and Fractional Dimension
- **Author:** Benoit Mandelbrot
- **Publisher:** *Science* 156(3775)
- **Published:** 1967
- **Type:** Research paper
- **URL:** [gis.humboldt.edu/olm/courses/gsp_510/Articles/Mandelbrot1967.pdf](https://gis.humboldt.edu/olm/courses/gsp_510/Articles/Mandelbrot1967.pdf)
- **Supports:**
  - Coastline length depends on the ruler (Richardson); the west coast of
    Britain has a fractal dimension of about 1.25 (reported 1.24-1.26).
- **Used in:** [Terrain techniques](./terrain-techniques.md#coastlines)
- **Limitations:** Read through tutorials quoting the paper.

## Climate and look

## GEN-S045 — Whittaker biome diagram
- **Title:** Biomes (Whittaker's temperature-precipitation diagram)
- **Publisher:** Marietta College biology department; SERC Carleton
  weather lab
- **Type:** Course material
- **URL:** [academic.marietta.edu/departments/biology/biomes/biome_main.htm](https://academic.marietta.edu/departments/biology/biomes/biome_main.htm),
  [serc.carleton.edu/eslabs/weather/4a.html](https://serc.carleton.edu/eslabs/weather/4a.html)
- **Supports:**
  - Mean annual temperature against annual precipitation predicts the
    biome; borders are approximate.
- **Used in:** [Climate and look](./climate-and-look.md#step-3-biomes)
- **Limitations:** Course summaries; Whittaker (1975) not read; exact
  borders not reproduced.

## GEN-S046 — Köppen climate classification
- **Title:** Köppen climate classification
- **Publisher:** Wikipedia
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Köppen_climate_classification](https://en.wikipedia.org/wiki/K%C3%B6ppen_climate_classification)
- **Supports:**
  - Five groups (A, B, C, D, E); group A has every month at 18 °C or
    more; the arid threshold is `20 × mean annual temperature (°C)` plus
    280, 140 or 0 mm depending on when the rain falls.
- **Used in:** [Climate and look](./climate-and-look.md#step-3-biomes)
- **Limitations:** Tertiary source; versions differ on the steppe border.

## GEN-S047 — Hadley cell width and rotation
- **Title:** Toward a unified theory for the Hadley cell descending and
  ascending edges (arXiv:2112.07731); Singh (2019) Hadley circulation
- **Publisher:** arXiv; Monash University
- **Type:** Research papers
- **URL:** [arxiv.org/pdf/2112.07731](https://arxiv.org/pdf/2112.07731),
  [singh.sci.monash.edu.au/papers/Singh_2019_Hadley_circulation.pdf](https://singh.sci.monash.edu.au/papers/Singh_2019_Hadley_circulation.pdf)
- **Supports:**
  - Rotation rate is a main control of the Hadley cell's extent; the cell
    widens as rotation slows and covers the globe on slow rotators.
  - The winter descending edge scales about as `Ω^-1/2`, the summer
    ascending edge about as `Ω^-2/3`, in idealised models.
- **Used in:** [Climate and look](./climate-and-look.md#step-2-wind-and-rain-belts)
- **Limitations:** Read through abstracts; the Earth three-cell summary
  in the page is general knowledge, labelled as practice.

## GEN-S048 — Photosynthesis around other stars
- **Title:** Spectral Signatures of Photosynthesis II (Kiang et al. 2007,
  *Astrobiology*); The Peak Absorbance Wavelength of Photosynthetic
  Pigments Around Other Stars (Lehmer et al. 2021)
- **Publisher:** *Astrobiology*; *Frontiers in Astronomy and Space
  Sciences*
- **Type:** Research papers
- **URL:** [life.illinois.edu/govindjee/publication/2007_Kiang_etal_2.pdf](https://www.life.illinois.edu/govindjee/publication/2007_Kiang_etal_2.pdf),
  [faculty.washington.edu/dcatling/Lehmer2021_Color_of_Alien_Plants.pdf](https://faculty.washington.edu/dcatling/Lehmer2021_Color_of_Alien_Plants.pdf)
- **Supports:**
  - Pigment absorbance may peak in the blue for F stars, red-orange for K
    stars, and near-infrared for M stars; depends on the star and the
    air's water vapour; under-water organisms limited to shorter
    wavelengths.
- **Used in:** [Climate and look](./climate-and-look.md#step-4-plant-colour-under-other-stars)
- **Limitations:** Abstracts only; plant colours as seen by eye are an
  artistic reading.

## GEN-S049 — Sky colour on Mars
- **Title:** Causes of colour: the Martian sky (WebExhibits); Blue moons
  and Martian sunsets (Ehlers et al. 2014)
- **Publisher:** WebExhibits; Washington University in St. Louis
- **Type:** Educational article; research paper
- **URL:** [webexhibits.org/causesofcolor/14C.html](https://www.webexhibits.org/causesofcolor/14C.html),
  [profiles.wustl.edu/en/publications/blue-moons-and-martian-sunsets](https://profiles.wustl.edu/en/publications/blue-moons-and-martian-sunsets/)
- **Supports:**
  - A dust-free Mars sky would be blue (Rayleigh scattering); dust that
    absorbs blue gives the butterscotch day sky; blue sunsets come from
    near-forward scattering of blue light by micron-sized dust.
- **Used in:** [Climate and look](./climate-and-look.md#step-6-sky-and-air-colour)
- **Limitations:** One popular source contradicts the dust mechanism and
  was not used.

## Moon dynamics

## GEN-S050 — Moon origins (giant impact, capture)
- **Title:** Dynamics of Lunar Formation (Canup 2004, *Annual Review of
  Astronomy and Astrophysics*); A "New Hope" for Moon Formation
  (arXiv:2512.10757)
- **Publisher:** Annual Reviews; arXiv
- **Type:** Review; research paper
- **URL:** [boulder.swri.edu/~robin/canup2004b.pdf](https://www.boulder.swri.edu/~robin/canup2004b.pdf),
  [arxiv.org/pdf/2512.10757](https://arxiv.org/pdf/2512.10757)
- **Supports:**
  - A late giant impact is the leading, dynamically consistent account of
    Earth's Moon; the Moon is too large to have been captured.
  - Triton is thought to have been captured when passing too close to
    Neptune (Agnor and Hamilton 2006, as cited).
- **Used in:** [Moon dynamics](./moon-dynamics.md#how-moons-form)
- **Limitations:** Agnor and Hamilton (2006) read only as cited.

## GEN-S051 — Tidal heating of Io
- **Title:** Melting of Io by tidal dissipation (Peale, Cassen, Reynolds
  1979, *Science*), as quoted in Massive Satellites of Close-In Gas Giant
  Exoplanets (arXiv:0909.0770); Astrophysical Classics: Predicting Tidal
  Heating of Io (Astrobites)
- **Publisher:** NASA NTRS; arXiv; Astrobites
- **Type:** Research paper; secondary summaries
- **URL:** [ntrs.nasa.gov/citations/19790041081](https://ntrs.nasa.gov/citations/19790041081),
  [arxiv.org/pdf/0909.0770](https://arxiv.org/pdf/0909.0770),
  [astrobites.org/2012/08/30/astrophysical-classics-predicting-tidal-heating-of-io](https://astrobites.org/2012/08/30/astrophysical-classics-predicting-tidal-heating-of-io/)
- **Supports:**
  - Forced eccentricity plus tides melt Io; predicted before Voyager.
  - `Ė = (42/19) π ρ² n^5 R^7 e^2 / (μ Q)` for a uniform elastic sphere,
    with Io's `e ≈ 0.0043` and `Q ≈ 100` as fiducial values.
  - Makarov and Efroimsky re-derived the formula, correcting terms.
- **Used in:** [Moon dynamics](./moon-dynamics.md#tidal-heating)
- **Limitations:** Full text of the citing paper (arXiv:0909.0770,
  equations 15 and 16) read; the 1979 paper itself was not.

## GEN-S052 — The Laplace resonance
- **Title:** Tidal evolution into the Laplace resonance and the
  resurfacing of Ganymede (Showman and Malhotra); Bland et al. (2009)
- **Publisher:** University of Arizona; Washington University
- **Type:** Research papers
- **URL:** [experts.arizona.edu — tidal evolution into the Laplace resonance](https://experts.arizona.edu/en/publications/tidal-evolution-into-the-laplace-resonance-and-the-resurfacing-of/),
  [epsc.wustl.edu/~mbland/pubs/Bland_et_al_09.pdf](https://epsc.wustl.edu/~mbland/pubs/Bland_et_al_09.pdf)
- **Supports:**
  - Io, Europa and Ganymede: two 2:1 commensurabilities; forced
    eccentricity drives Io's heating; Ganymede's heating is negligible
    now, but Laplace-like resonances could have pumped its eccentricity to
    about 0.07 and its heating several hundred times higher.
- **Used in:** [Moon dynamics](./moon-dynamics.md#resonances-keep-the-heat-on)
- **Limitations:** Abstracts; the resonance's origin is debated.

## GEN-S053 — Phobos's orbital decay
- **Title:** The fate of Phobos (Agrusa and Michel, arXiv:2602.21912);
  press coverage of Black and Mittal (2015)
- **Publisher:** arXiv; Astronomy magazine
- **Type:** Research paper; science journalism
- **URL:** [arxiv.org/abs/2602.21912v1](https://arxiv.org/abs/2602.21912v1),
  [astronomy.com/science/how-moon-dust-will-put-a-ring-around-mars](https://www.astronomy.com/science/how-moon-dust-will-put-a-ring-around-mars/)
- **Supports:**
  - Phobos orbits inside Mars's synchronous radius, spirals inward (about
    2 cm per year) and should break into a ring.
  - Breakup estimates range from under 10 to about 40 million years; the
    2026 study suggests breakup beyond 2 Mars radii for a weak body.
- **Used in:** [Moon dynamics](./moon-dynamics.md#spin-locking-and-drift)
- **Limitations:** Estimates depend on Phobos's unknown inner strength.

## GEN-S054 — Co-orbital moons of Saturn
- **Title:** Primordial migration of co-orbital satellites as a mechanism
  for the horseshoe orbit of Janus-Epimetheus (arXiv:1905.08839); How
  Janus' Orbital Swap Affects the Edge of Saturn's A Ring?
  (arXiv:1510.00434)
- **Publisher:** arXiv
- **Type:** Research papers
- **URL:** [arxiv.org/pdf/1905.08839](https://arxiv.org/pdf/1905.08839),
  [arxiv.org/pdf/1510.00434](https://arxiv.org/pdf/1510.00434)
- **Supports:**
  - Janus and Epimetheus swap orbits about every four years (horseshoe);
    Telesto and Calypso at Tethys's L4 and L5, Helene and Polydeuces at
    Dione's (tadpole); horseshoe pairs are much rarer.
- **Used in:** [Moon dynamics](./moon-dynamics.md#co-orbital-moons)
- **Limitations:** Read through abstracts.

## GEN-S055 — Young, fading rings of Saturn
- **Title:** Durisen on Saturn's rings and Cassini (Indiana University,
  2023); Saturn's rings are dying (NASA ring-rain estimate, O'Donoghue)
- **Publisher:** Indiana University news; ScienceAlert
- **Type:** University news; science journalism
- **URL:** [astro.indiana.edu/news-events/news/durisen-saturn-rings-cassini-2023.html](https://astro.indiana.edu/news-events/news/durisen-saturn-rings-cassini-2023.html),
  [sciencealert.com/saturn-s-rings-are-dying](https://www.sciencealert.com/saturn-s-rings-are-dying)
- **Supports:**
  - The rings are a few hundred million years old (Cassini meteoroid
    influx, ring mass, inflow to Saturn).
  - Ring rain alone could remove them in about 300 million years; under
    100 million years counting equatorial infall.
- **Used in:** [Moon dynamics](./moon-dynamics.md#rings-appear-and-fade)
- **Limitations:** The young age is debated in the literature; read
  through news summaries.

## Landforms

## GEN-S056 — Volcano types
- **Title:** Principal types of volcanoes (USGS); Types of Volcanoes (NPS);
  Gervais, *Geology* ch. 14
- **Publisher:** U.S. Geological Survey; National Park Service; Macmillan
- **Type:** Government publications; textbook
- **URL:** [pubs.usgs.gov/gip/volc/types.html](https://pubs.usgs.gov/gip/volc/types.html),
  [nps.gov/subjects/volcanoes/types-of-volcanoes.htm](https://www.nps.gov/subjects/volcanoes/types-of-volcanoes.htm)
- **Supports:**
  - Four kinds: cinder cones, composite volcanoes, shield volcanoes, lava
    domes.
  - Shields: fluid basalt, wide and gentle; stratovolcanoes: viscous
    andesite and rhyolite, steep; cinder cones rarely over about 300 m,
    slopes 25-35 degrees.
- **Used in:** [Landforms](./landforms.md#volcanoes)
- **Limitations:** The slope range is from a textbook; the profile
  exponents in the page are recommendations, not measurements.

## GEN-S057 — Stream order and Hack's law
- **Title:** Stream order and Hack's law (Geographical Review of Japan;
  arXiv:physics/0005048; Tarboton 1996)
- **Publisher:** J-STAGE; arXiv; Utah State University
- **Type:** Research papers
- **URL:** [jstage.jst.go.jp/article/grj1925/45/8/45_8_588/_article](https://www.jstage.jst.go.jp/article/grj1925/45/8/45_8_588/_article),
  [arxiv.org/pdf/physics/0005048](https://arxiv.org/pdf/physics/0005048),
  [hydrology.usu.edu/dtarb/TarbotonFractalRiverNetworksJH96.pdf](https://hydrology.usu.edu/dtarb/TarbotonFractalRiverNetworksJH96.pdf)
- **Supports:**
  - Strahler ordering; Horton's roughly constant bifurcation ratio.
  - Hack's law `L = c A^h` with `h` about 0.6 (Hack), 0.568 (Gray), 0.6-0.7
    in real networks.
- **Used in:** [Landforms](./landforms.md#river-networks)
- **Limitations:** The 3-5 bifurcation range used in the page is not
  confirmed by these sources.

## GEN-S058 — River meanders
- **Title:** River channel patterns: braided, meandering and straight
  (Leopold and Wolman 1957, USGS Professional Paper 282-B); River meanders
  (Leopold and Wolman 1960)
- **Publisher:** U.S. Geological Survey; GSA Bulletin
- **Type:** Government publication; research paper
- **URL:** [pubs.usgs.gov/publication/pp282B](https://pubs.usgs.gov/publication/pp282B),
  [pubs.usgs.gov/publication/70185557](https://pubs.usgs.gov/publication/70185557)
- **Supports:**
  - Meander wavelength 7-12 times the channel width; radius of curvature
    2-3 widths; straight reaches rarely exceed about 10 widths.
- **Used in:** [Landforms](./landforms.md#meanders)
- **Limitations:** Read through summaries.

## GEN-S059 — Delta classification
- **Title:** Classification of deltas (Bosboom and Stive, *Coastal
  Dynamics*), after Galloway (1975)
- **Publisher:** LibreTexts Geosciences
- **Type:** Open textbook
- **URL:** [geo.libretexts.org — 2.7.3 Classification of deltas](https://geo.libretexts.org/Bookshelves/Oceanography/Coastal_Dynamics_(Bosboom_and_Stive)/02%3A_Large-scale_geographical_variation_of_coasts/2.07%3A_Process-based_classification/2.7.3%3A_Classification_of_deltas)
- **Supports:**
  - River-, wave- and tide-dominated deltas on a triangle; their shore
    shapes and examples (Mississippi; Nile, São Francisco; Ganges-
    Brahmaputra, Fly).
- **Used in:** [Landforms](./landforms.md#deltas)
- **Limitations:** Galloway (1975) read as summarised.

## GEN-S060 — Karst
- **Title:** Karst (USGS Groundwater, archived); karst landscapes and
  sinkholes (USGS)
- **Publisher:** U.S. Geological Survey
- **Type:** Government web pages
- **URL:** [usgs.gov/media/images/karst-landscapes-are-more-prone-have-land-subsidence-and-sinkholes](https://www.usgs.gov/media/images/karst-landscapes-are-more-prone-have-land-subsidence-and-sinkholes)
- **Supports:**
  - Karst forms by dissolving soluble rock (limestone, dolomite); about 20
    percent of the US land surface is karst; sinkholes form over centuries
    to millennia.
- **Used in:** [Landforms](./landforms.md#karst)
- **Limitations:** No reliable global share was found, so none is given.

## Pass A additions

## GEN-S061 — Earth's hypsometry
- **Title:** Hypsometry (Wikipedia); a 2012 GSA Annual Meeting abstract on
  Earth's modal elevation
- **Publisher:** Wikipedia; Geological Society of America
- **Type:** Encyclopedia article; conference abstract
- **URL:** [en.wikipedia.org/wiki/Hypsometry](https://en.wikipedia.org/wiki/Hypsometry),
  [gsa.confex.com/gsa/2012AM/webprogram/Paper211610.html](https://gsa.confex.com/gsa/2012AM/webprogram/Paper211610.html)
- **Supports:**
  - Earth's heights are bimodal, from light continental against dense
    ocean crust (Wegener 1929); the deep mode is centred near −4,300 m and
    the shallow one near sea level.
- **Used in:** [Height maps](./height-maps.md#tuned-starting-values)
- **Limitations:** Summaries; no exact area shares taken.

## GEN-S062 — Mars's hypsometry
- **Title:** Statistics of Mars' topography from the Mars Orbiter Laser
  Altimeter: slopes, correlations, and physical models
- **Author:** Oded Aharonson, Maria T. Zuber, Daniel H. Rothman
- **Publisher:** *Journal of Geophysical Research* (2001)
- **Type:** Research paper
- **URL:** [resolver.caltech.edu/CaltechAUTHORS:AHAjgre01](https://resolver.caltech.edu/CaltechAUTHORS:AHAjgre01)
- **Supports:**
  - Mars's bimodal hypsometry becomes unimodal when heights are measured
    from the centre of figure, unlike Earth's, which stays bimodal; the
    dichotomy is largely a long-baseline tilt.
- **Used in:** [Height maps](./height-maps.md#tuned-starting-values)
- **Limitations:** Abstract only.

## GEN-S063 — Main-sequence lifetimes
- **Title:** Lifetimes of main sequence stars (University of Colorado
  course text); Red dwarf (Wikipedia)
- **Publisher:** JILA, University of Colorado; Wikipedia
- **Type:** Course material; encyclopedia article
- **URL:** [jila.colorado.edu/~ajsh/courses/astr1120_03/text/chapter5/l5S2.htm](https://jila.colorado.edu/~ajsh/courses/astr1120_03/text/chapter5/l5S2.htm),
  [en.wikipedia.org/wiki/Red_dwarf](https://en.wikipedia.org/wiki/Red_dwarf)
- **Supports:**
  - `L ∝ M^3.5` and `t_MS ≈ 10^10 years × M^-2.5` as an approximation;
    exponents vary with mass.
  - Red dwarfs fuse about 90 percent of their hydrogen; lifetimes of
    trillions of years for 0.1-0.16 Sun masses.
- **Used in:** [Lifetime](./lifetime.md#brightening-by-star-class)
- **Limitations:** Textbook approximations; red-dwarf lifetimes are model
  dependent.

## GEN-S064 — Water loss on planets of young M dwarfs
- **Title:** Extreme Water Loss and Abiotic O2 Buildup on Planets
  Throughout the Habitable Zones of M Dwarfs
- **Author:** Rodrigo Luger, Rory Barnes
- **Publisher:** *Astrobiology* 15(2), 119-143; arXiv:1411.7412
- **Published:** 2015
- **Type:** Research paper
- **URL:** [arxiv.org/abs/1411.7412](https://arxiv.org/abs/1411.7412)
- **Supports:**
  - During the long pre-main-sequence phase, habitable-zone planets of M
    dwarfs can be in runaway greenhouses for several hundred million years
    and lose several Earth oceans, if they formed wet; the loss scales
    with surface gravity.
- **Used in:** [Lifetime](./lifetime.md#brightening-by-star-class)
- **Limitations:** Abstract; the phase length is model dependent (about
  100 million years in some models).

## GEN-S065 — Oceans on rogue planets
- **Title:** The Steppenwolf: A Proposal for a Habitable Planet in
  Interstellar Space
- **Author:** Dorian S. Abbot, Eric R. Switzer
- **Publisher:** *The Astrophysical Journal Letters* (2011); arXiv:1102.1108
- **Type:** Research paper
- **URL:** [arxiv.org/pdf/1102.1108](https://arxiv.org/pdf/1102.1108.pdf)
- **Supports:**
  - Stevenson (1999): a very thick hydrogen atmosphere could keep surface
    water liquid by geothermal heat alone.
  - An Earth-composition rogue planet about 3.5 times Earth's mass could
    keep a subglacial ocean under about 8 km of ice.
- **Used in:** [Lifetime](./lifetime.md#rogue-planets)
- **Limitations:** Abstract and summaries; the 30-40 K surface value in
  the page is derived, not from this source.

## Pass B additions

## GEN-S066 — Greenhouse warming on the planets
- **Title:** Greenhouse effect on other planets (Energy Education,
  University of Calgary); Planetary surface temperatures from first
  principles (arXiv:2305.17694)
- **Publisher:** University of Calgary; arXiv
- **Type:** Educational encyclopedia; research paper
- **URL:** [energyeducation.ca/encyclopedia/Greenhouse_effect_on_other_planets](https://energyeducation.ca/encyclopedia/Greenhouse_effect_on_other_planets),
  [arxiv.org/pdf/2305.17694](https://arxiv.org/pdf/2305.17694)
- **Supports:**
  - Earth 255 K effective against 288 K surface (+33 K); Venus about 220 K
    against 730 K (about +510 K); Mars a few kelvin (212 against 218 K in
    one table, 217 against 240 K in another); Titan 82 against 94 K.
- **Used in:** [Air model](./air-model.md#step-3-greenhouse-warming)
- **Limitations:** Summaries; values depend on the albedo assumed.

## GEN-S067 — The diffusive energy-balance model
- **Title:** Lecture 18: Diffusive energy balance model (ClimateModeling
  courseware)
- **Author:** Brian E. J. Rose (University at Albany)
- **Publisher:** GitHub (brian-rose/ClimateModeling_courseware)
- **Type:** Course material (Jupyter notebook)
- **URL:** [github.com/brian-rose/ClimateModeling_courseware — Lecture18](https://github.com/brian-rose/ClimateModeling_courseware/blob/master/Lectures/Lecture18%20--%20Diffusive%20energy%20balance%20model.ipynb)
- **Supports:**
  - The Budyko-Sellers energy-balance model with diffusive transport;
    `OLR = A + B T` with `A = 210 W/m²`, `B = 2 W/m²/°C` (fitted to
    reanalysis data); albedo `α ≈ 0.354 + 0.25 P2(sin φ)`; diffusivity
    `D = 0.55 W/m²/°C` in the worked example, tuned to observations.
- **Used in:** [Climate and look](./climate-and-look.md#the-energy-balance-model-by-latitude)
- **Limitations:** Full text of the notebook read. North (1975) and
  North et al. (1981) themselves were not read.

## GEN-S068 — Climate of tidally locked planets
- **Title:** Stabilizing cloud feedback dramatically expands the
  habitable zone of tidally locked planets (Yang, Cowan and Abbot 2013,
  *ApJL* 771, L45); A low-order model of water vapour, clouds and thermal
  emission for tidally locked terrestrial planets (Yang and Abbot 2014,
  arXiv:1403.0905)
- **Publisher:** *The Astrophysical Journal Letters*; arXiv
- **Type:** Research papers
- **URL:** [arxiv.org/abs/1307.0515](https://www.arxiv.org/abs/1307.0515),
  [arxiv.org/abs/1403.0905](https://ar5iv.arxiv.org/html/1403.0905)
- **Supports:**
  - Clouds over the substellar region raise the albedo and cool tidally
    locked planets, widening their habitable zone; the night side acts as
    a radiator; models diverge by 20-30 K in global mean temperature
    (Yang et al. 2019 intercomparison).
- **Used in:** [Climate and look](./climate-and-look.md#tidally-locked-worlds)
- **Limitations:** Abstracts and summaries.

## GEN-S069 — Plate tectonics on super-Earths, both sides
- **Title:** Inevitability of Plate Tectonics on Super-Earths (Valencia,
  O'Connell and Sasselov 2007, arXiv:0710.0699); with O'Neill and Lenardic
  (2007) and Stamenković et al. (2010) as summarised in reviews
- **Publisher:** *The Astrophysical Journal* 670, L45; arXiv; review
  arXiv:1212.0113
- **Type:** Research papers and review
- **URL:** [arxiv.org/abs/0710.0699](https://arxiv.org/abs/0710.0699v1),
  [arxiv.org/pdf/1212.0113](https://arxiv.org/pdf/1212.0113)
- **Supports:**
  - Valencia et al.: driving stress grows and plates thin with mass, so
    super-Earths have plates.
  - O'Neill and Lenardic: at most an episodic regime; higher gravity
    locks faults.
  - Stamenković et al. 2010 agree with O'Neill and Lenardic for
    super-Earths; the question is described as unresolved (2013).
- **Used in:** [Plates](./plates.md#what-decides-it-facts)
- **Limitations:** Abstracts and reviews; O'Neill and Lenardic read only
  as summarised.

## GEN-S070 — Water and plate tectonics
- **Title:** Korenaga (2007), on seawater, cracking and lithosphere
  weakening (paper `korenaga07c` on the author's page); Effects of
  rheological stratification and elasticity of lithosphere on subduction
  initiation (*Frontiers in Earth Science* 2022)
- **Publisher:** Yale University; Frontiers
- **Type:** Research papers
- **URL:** [people.earth.yale.edu/sites/default/files/korenaga07c.pdf](https://people.earth.yale.edu/sites/default/files/korenaga07c.pdf),
  [frontiersin.org — feart.2022.988320](https://www.frontiersin.org/journals/earth-science/articles/10.3389/feart.2022.988320/pdf)
- **Supports:**
  - Seawater through cracks makes weak zones (serpentinite) that may
    explain plates instead of a stagnant lid on Earth; surface water is
    likely key to weakening the lithosphere; without an ocean, plate
    motion would likely stop.
  - Venus, Mars and the Moon are generally considered stagnant-lid
    planets.
- **Used in:** [Plates](./plates.md#what-decides-it-facts)
- **Limitations:** Search summaries only; the Korenaga paper's exact
  title was not confirmed in this run.

## Pass C additions

## GEN-S071 — Delta morphology from sediment fluxes
- **Title:** Global-scale human impact on delta morphology has led to net
  land area gain
- **Author:** Jaap H. Nienhuis and co-authors (including Douglas A.
  Edmonds, A. J. F. Hoitink and Torbjörn E. Törnqvist)
- **Publisher:** *Nature* 577, 514-518; DOI 10.1038/s41586-019-1905-9
- **Published:** 2020
- **Type:** Research paper
- **URL:** [par.nsf.gov/servlets/purl/10166405](https://par.nsf.gov/servlets/purl/10166405)
- **Supports:**
  - The fluvial dominance ratio `R = Q_river / Q_wave` (`R ≈ 0` no
    deflection; `0 < R < 1` triangular wave-dominated; `R > 1`
    river-dominated) and the tidal dominance ratio `T = Q_tide / Q_river`
    (`T > 1` tide-dominated, widening channels with flow reversal).
  - About 79 percent of about 11,000 deltas are wave-dominated, about 10
    percent each tide- and river-dominated; most large deltas are tide- or
    river-dominated.
- **Used in:** [Landforms](./landforms.md#deltas)
- **Limitations:** Full text of the accepted manuscript read; the methods'
  flux formulas for `Q_wave` and `Q_tide` were not carried into the page.

## GEN-S072 — Crater rays and secondary craters
- **Title:** Elliott et al. (2018, *Icarus*) on lunar ray lengths; LROC
  image posts on secondary craters
- **Publisher:** Purdue University (Icarus 2018 manuscript); Lunar
  Reconnaissance Orbiter Camera team
- **Type:** Research paper; mission image releases
- **URL:** [eaps.purdue.edu/minton/docs/Icarus 2018 Elliott.pdf](https://eaps.purdue.edu/minton/docs/Icarus%202018%20Elliott.pdf),
  [lroc.im-ldi.com/images/1228](https://lroc.im-ldi.com/images/1228)
- **Supports:**
  - Secondary craters line rays that reach many crater diameters; they are
    irregular, shallow, and form chains.
  - Ray length follows a power law in primary radius.
  - Space weathering darkens rays, thin ones first; ray fading is often
    taken as about 0.75-1.1 billion years, with exceptions.
- **Used in:** [Landforms](./landforms.md#crater-rays-secondary-craters-and-chains)
- **Limitations:** Summaries; the power-law constants were not taken.

## GEN-S073 — Dune types
- **Title:** Types of dunes (USGS *Deserts: Geology and Resources*); Ben
  Gurion University dune pages
- **Publisher:** U.S. Geological Survey; Ben-Gurion University of the Negev
- **Type:** Government publication; university pages
- **URL:** [pubs.usgs.gov/gip/deserts/dunes](https://pubs.usgs.gov/gip/deserts/dunes),
  [in.bgu.ac.il/en/bidr/nic/Pages/what-are-dunes.aspx](https://in.bgu.ac.il/en/bidr/nic/Pages/what-are-dunes.aspx)
- **Supports:**
  - Crescentic (barchan) dunes are the most common on Earth and Mars,
    under one wind with limited sand; linear under two winds; star under
    many wind directions with plenty of sand; parabolic where plants anchor
    them.
- **Used in:** [Landforms](./landforms.md#dunes)
- **Limitations:** No spacing law found; transverse-dune naming differs.

## GEN-S074 — Supervolcanoes
- **Title:** Capturing the Extreme in Volcanology: The Case for the Term
  "Supervolcano" (*Frontiers in Earth Science* 2022); The threat of
  supervolcanoes (AAAS)
- **Publisher:** Frontiers; American Association for the Advancement of
  Science
- **Type:** Review; science article
- **URL:** [frontiersin.org — feart.2022.859237](https://www.frontiersin.org/journals/earth-science/articles/10.3389/feart.2022.859237/epub),
  [aaas.org/threat-supervolcanoes](https://www.aaas.org/threat-supervolcanoes)
- **Supports:**
  - USGS definition: a volcanic centre with a VEI 8 eruption, deposits
    over 1,000 km³.
  - Toba (about 75,000 years ago): about 2,800 km³ dense-rock equivalent;
    its caldera holds the world's largest volcanic lake; supervolcanoes at
    hot spots (Yellowstone) and subduction zones (Toba).
- **Used in:** [Landforms](./landforms.md#supervolcanoes)
- **Limitations:** Summaries; Yellowstone volumes are being revised.

## GEN-S075 — Double planets and barycentres
- **Title:** Double planet
- **Publisher:** Wikipedia
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Double_planet](https://en.wikipedia.org/wiki/Double_planet)
- **Supports:**
  - A binary whose centre of mass lies outside both bodies is often called
    a double planet; Pluto-Charon qualifies, Earth-Moon does not.
- **Used in:** [Moons](./moons.md#how-many-moons)
- **Limitations:** Tertiary source; the mass ratio quoted varies (about
  1/8).

## GEN-S076 — Saturn's ring particles and optical depth
- **Title:** The Smallest Particles in Saturn's A and C Rings (Harbison et
  al., arXiv:1312.2927); Origin and Evolution of Saturn's Ring System
  (arXiv:0912.3017)
- **Publisher:** arXiv
- **Type:** Research papers
- **URL:** [arxiv.org/pdf/1312.2927](https://arxiv.org/pdf/1312.2927),
  [arxiv.org/pdf/0912.3017](https://arxiv.org/pdf/0912.3017)
- **Supports:**
  - Optical depth about 0.1 (C ring) and 1-5 (B ring); particles about a
    centimetre to several metres (about 10 m maximum), power-law index
    about 3.
- **Used in:** [Moons](./moons.md#rings)
- **Limitations:** Summaries; B-ring size fits are weak at high optical
  depth.

## Internal references

- `crates/universe-core/src/terrain.rs` — today's body recipe and moon
  count (read 2026-10-14).
- `crates/universe-core/src/noise.rs` — the shared deterministic value
  noise.
- [Planet and moon kinds catalog](../../../catalogs/documents/planets.md).

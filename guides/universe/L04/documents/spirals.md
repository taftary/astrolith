# Spirals — disk galaxies with arms

Spiral galaxies are flat rotating
islands: a thin star-forming disk with arms around a dense central
bulge, wrapped in a faint halo. Our home galaxy is one. How a spiral
interacts with the other L4 parts — ellipticals it may merge into,
dwarfs it tides apart, nuclei it feeds, halos it builds — runs through
every section below.

This document is the L04 spiral reference: what disks and arms look
like, where spirals sit on the Hubble tuning fork, how bars shape
them, how they grow inside-out and quench, how arms and bars are
built, how they connect to the other parts, how rotation curves weigh
them, and which example objects anchor each claim. For smooth
merger-built giants see [ellipticals](ellipticals.md); for the small
faint majority see [dwarfs](dwarfs.md); for central black holes see
[nuclei](nuclei.md); for stripped stars and gas halos see
[halos](halos.md); for the dated timeline see
[lifetime](lifetime.md).

## Where spirals sit on the tuning fork

Spirals occupy the two right-hand prongs of Edwin Hubble's 1926
tuning-fork diagram: normal spirals (S) on the upper prong, barred
spirals (SB) on the lower prong, with lenticular S0 galaxies bridging
back to the ellipticals on the left. Position along each prong runs
Sa to Sb to Sc (de Vaucouleurs later added Sd): Sa spirals wind
tightly with a large bright bulge and smooth arms, Sc spirals wind
openly with a small faint bulge and clumpy arms resolved into star
clusters and nebulae, Sb sits between. Intermediate grades (Sab, Sbc)
and weak-bar grades (SAB) mark boundary cases, and Sd extends the
sequence to near-bulgeless fragmentary disks. The early-type versus
late-type words are historical only: the fork was once read as an
evolutionary sequence, but galaxy evolution runs through collapse
conditions, mergers, and gas flows, not left to right along the
diagram.

Bars are the rule, not the exception: about two thirds of local
spirals carry one, with visual-strong-bar counts near one third and
weak bars making up the rest — the number moves with method (visual
inspection finds the most, ellipse fitting and Fourier analysis fewer)
and with band, mass, and Hubble type. Strong bars favor massive
early-type spirals; weak bars favor late types. Bars are also a
maturity marker: only about 20 percent of distant spirals show them
against about 65 percent locally, and JWST now traces bar-driven
disks back to redshift 3, when the universe was about 2 billion years
old.

| Grade | Bulge | Arms | Bar note |
|---|---|---|---|
| Sa / SBa | large, bright | tightly wound, smooth | arms from bulge (S) or bar ends and rings (SB) |
| Sb / SBb | medium | middling wind and resolution | most common middle ground |
| Sc / SBc | small, faint | openly wound, clumpy clusters and nebulae | Milky Way near here (barred SBc) |
| Sd / SBd | near-absent | very loose, fragmentary | most light in the arms, not the bulge |

Sources: NASA tuning-fork page; Wikipedia "Hubble sequence"
(Sa/Sb/Sc, SB branch, Milky Way SBc); Wikipedia "Barred spiral
galaxy" (two-thirds local fraction, density-wave bar origin, 2008
redshift evolution); Ann et al. ApJ 872 (2019) strong-versus-weak bar
split; JWST CEERS/PRIMER bar fractions at redshift 1-3 (2023).

## How it looks

Face-on, a spiral is a circular disk traced by arms: dark dust lanes on
the inner edge, pink star-forming knots, blue clusters of young stars
outward — the Whirlpool (M51) is the textbook grand design, its two arms
pumped by companion NGC 5195 gliding behind it for hundreds of millions
of years. Edge-on, the same galaxy is a squashed oval: the disk a sharp
line cut by dust, the bulge a glowing egg above and below it — the
Sombrero (M104) shows a brilliant white core with thick dust lanes from
6 degrees off its plane. The bulge holds older redder stars; the arms
hold the young blue ones. Many carry a bar from the bulge to where the
arms start (census and grades in "Where spirals sit on the tuning
fork" above).

Target visuals (files in `images/`, credits and licenses at the bottom):

![Face-on grand-design spiral M51, the Whirlpool, with companion NGC 5195 (screensize)](../images/whirlpool-hst.jpg)

Sources: ESA/Hubble heic0506a (M51 pumps); NASA face-on IC 5332 and
edge-on UGC 10043 pages (orientation); ESA Sombrero release (M104);
ESAHUBBLE wordbank "spiral galaxy".

## How it changes over time

Spirals grow inside-out: a dense old core first, then higher-spin gas
settles into the disk — JWST caught the earliest case 700 million years
after the Big Bang. Massive young stars die within millions of years as
supernovae, whose blast waves compress nearby gas and restart the cycle.
Growth stalls when the gas runs out or is removed: quenching shuts star
birth down and reddens the disk, and half of all massive galaxies had
quenched by age 3 Gyr. Six billion years ago more than half of today's
spirals still looked peculiar; gas-rich mergers rebuilt them into the
grand designs we see. Andromeda fits that rebuilt story; the Milky Way
has been quieter.

Sources: Cambridge inside-out JADES result; ESA "where did today's
spirals come from"; NASA REQUIEM (early massive dead galaxies); NASA
quenching seminar.

## How a spiral feeds and quenches (second pass)

Star birth in a spiral disk obeys scaling laws. The Kennicutt–Schmidt
relation ties star-formation surface density to gas surface density as
roughly the 1.4 power globally (Kennicutt 1998; confirmed at n = 1.5
for the normal-to-starburst ensemble); against molecular gas alone the
slope is near unity, so molecular gas depletes on a roughly constant
1–3 Gyr clock across environments (PHANGS, 80 nearby galaxies, 2023).
Arms gather but barely accelerate the process: spiral arms carry higher
molecular-gas and star-formation densities than the gaps between them,
yet the star-formation efficiency per molecule rises only slightly in
arms — arms organize star birth more than they trigger it.

Quenching has three kill mechanisms. Internal exhaustion plus active-
nucleus heating shuts massive disks from the inside out (see
[nuclei](nuclei.md)); the halo stops delivering fresh gas ("strangulation");
and cluster infall strips gas outright. Ram pressure is the most
photogenic: Virgo's NGC 4522, racing at ~3,500 km/s relative to the
intracluster gas about 1 Mpc from M87, lost 40 percent of its atomic
hydrogen (1.5 x 10^8 solar masses) to an extraplanar tail with star
birth igniting inside the stripped filaments, its star-forming disk cut
back to 0.35 of its optical radius with peak stripping ~50 Myr ago;
NGC 4402 shows the earlier stage — a kiloparsec molecular plume lifted
off its leading edge at ~4.5 kpc with gas accelerated ~60 km/s downwind,
dense clouds decoupling while diffuse gas strips first. Stripping runs
outside-in on 50–500 Myr timescales, and stripped stars that never fall
back join the intracluster light (see [ellipticals](ellipticals.md)).

Sources: Kennicutt 1998 Schmidt law (n ~ 1.4); de los Reyes and
Kennicutt 2019–2021 (n = 1.41 normals, bimodal starburst offset);
PHANGS Sun et al. 2023 (80-galaxy laws, 1–3 Gyr depletion, 5–10%
per-orbit and 0.5–1% per-free-fall efficiencies); PHANGS–JWST Treasury
2023 (19 spirals, cloud-scale feedback); Kenney et al. 2004, Vollmer et
al. 2006, Abramson et al. 2016 (NGC 4522 stripping, 40% extraplanar);
Cramer et al. 2020 ALMA (NGC 4402 plume, compression zone).

## How it is created

A spiral is born when gas cools and settles into a spinning disk inside
a dark halo, conserving angular momentum. The arms themselves are a
traveling pattern, not fixed tubes: stars, gas and dust pass in and out
like cars through a traffic jam, bunching and compressing as they go —
which is why the pattern survives instead of winding itself up. Gas
entering the wave is compressed by factors of order ten, crosses the
Jeans threshold, and forms stars slightly downstream: dust lanes and
atomic-hydrogen clouds mark the inner edge, young massive stars and
ionized regions light the arm itself, and small old red stars drift out
to fill the disk between arms.

Three mechanisms share the work, and they are not mutually exclusive.
The Lin–Shu quasi-stationary density wave (1964, reviewed by Shu 2016)
treats the pattern as a global disk mode rotating at one pattern speed,
trapped between the inner and outer Lindblad resonances: inside
corotation stars overtake the wave, outside the wave overtakes the
stars. Swing amplification (Goldreich–Lynden-Bell 1965; Julian–Toomre
1966; Toomre 1981) grows open trailing disturbances from noise: shear
stretches a leading ripple into a trailing one while self-gravity and
epicyclic motion feed it, favoring two arms in heavy near-maximum disks
and many arms where the disk is light. Tides and bars drive the rest:
companions and near misses raise two-armed waves directly (M51's arms
are the companion-driven case), and bars pump arms from inside. Modern
reviews conclude that outside barred galaxies most arms are transient
and recurrent swing-amplified patterns rather than permanent fixtures,
with infrared imaging sometimes revealing a smooth two-armed backbone
underneath optical flocculence.

Arm regularity is graded separately from Hubble type on the Elmegreen
arm-class scale: grand-design spirals carry two long continuous
symmetric arms (about 3–10 percent of spirals in deep modern censuses,
higher in older shallow ones), flocculent spirals carry patchy
fragmentary star-formation arms with no symmetry (about 38–50
percent), and multi-armed spirals sit between with two symmetric inner
arms branching outward (the majority, about 59 percent in the SDSS
DR18 census of 5,093 blue spirals). Grand designs run larger
(about 1.5 times the size of flocculents), earlier in Hubble type, more
concentrated, and more often barred; 82 percent of spirals show two
arms in their inner regions, and branching points cluster at 0.2–0.6
of the galactic radius. Isolated spirals keep symmetric arms from disk
modes alone.

Sources: C. C. Lin and F. Shu density-wave theory (1964; Shu ARA&A 54,
2016 six-decades review); Dobbs and Baba 2014 Dawes review (three
mechanisms, transient recurrent arms); Sellwood 2022 spirals review
(swing amplification, arm multiplicity versus disk mass); Wikipedia
"Density wave theory" (traffic-jam picture, Lindblad resonances, star
formation ordering); Elmegreen arm classes (1982, 1987) with 2024
SDSS DR18 census (flocculent 38%, multi-arm 59%, grand-design 3%);
Smith et al. 2024 grand-design versus multi-armed structure study; ESA
"face to face with a spiral's arms" (pattern picture).

## How it interacts with the others

- **With ellipticals:** a major merger scrambles two spirals' orbits into
  an elliptical (see [ellipticals](ellipticals.md)); minor mergers puff
  the disk and feed the bulge instead of destroying it.
- **With dwarfs:** spirals shred infalling dwarfs into streams that join
  the halo (see [halos](halos.md) and [dwarfs](dwarfs.md)); the shredded
  Sagittarius dwarf tightened its orbit in three disk collisions that
  triggered our own star birth.
- **With nuclei:** disk gas funneled inward feeds the central black hole;
  the lit nucleus can then heat or expel the remaining gas and quench the
  disk (see [nuclei](nuclei.md)).
- **With the cluster wind:** falling into a cluster turns the
  intracluster gas into a headwind that strips the disk's gas — Virgo's
  NGC 4522 and NGC 4402 are caught mid-stripping at over 10 million km/h
  (see "How a spiral feeds and quenches" above).

Sources: NASA galaxy evolution and merger pages; ESA ram-pressure
stripping release (NGC 4522/4402); L03 `../../L03/documents/icm.md`.

## Weighing a spiral (second pass)

Spirals are weighed by spin: stars and gas orbit faster than the visible
mass allows — rotation curves stay flat far out instead of falling —
so most of the mass is a dark halo. The history runs from Slipher and
Wolf's 1914 tilted lines through Babcock's 1939 non-falling M31 curve,
Freeman's 1970 disk-peak mismatch, Rogstad–Shostak and Roberts–Rots
radio flats (1972–73), Bosma's 25-galaxy thesis and Rubin–Ford–Thonnard's
10-galaxy optical survey (1978), to Faber–Gallagher 1979 closing the
case: at least 90 percent of a spiral's mass is dark, with mass-to-light
ratios reaching ~20 at large radii. The Milky Way weighs ~1–2 trillion
solar masses in total, only ~60 billion of it in stars; Andromeda's halo
weighs in similar, which is why the pair's 110 km/s fall toward each
other is a two-heavyweight bout. The same flat curves Vera Rubin mapped
in the 1970s now calibrate every spiral mass in the part docs.

Spin also gives distance through the Tully–Fisher ladder: luminosity
(or stellar mass, or full baryonic mass of stars plus gas) scales as a
power of rotation speed. The baryonic form is the tightest — slope
~3.3–4 over six decades of mass with little intrinsic scatter (SPARC,
ALFALFA) — and doubles as a redshift-independent ruler that currently
reads H0 ~ 75 km/s/Mpc. One live tension belongs here: Gaia DR3 rotation
curves bend measurably downward past ~50,000 light-years, and fits to
the bend give total Milky Way masses as low as ~2 x 10^11 solar masses,
several times below stream and globular fits near 10^12 (full method
spread in [halos](halos.md), "A note on disagreeing numbers"). All
methods agree inside ~100,000 light-years; they differ in the
extrapolation to the virial edge.

Sources: Wikipedia "Galaxy rotation curve"; Wikipedia "Milky Way"
(mass); Wikipedia "Andromeda Galaxy" (mass, approach); Rubin 2000
"One hundred years of rotating galaxies" and Bertone–Hooper 2018
history (Babcock, Freeman, Bosma, Rubin–Ford–Thonnard, Faber–
Gallagher); Tully and Fisher 1977; McGaugh et al. 2000 and Lelli et
al. 2016–2019 SPARC baryonic Tully–Fisher (slope ~4, negligible
intrinsic scatter); Ball et al. ApJ 950 (2023) ALFALFA slope
3.30; Gaia DR3 declining-curve analyses 2023–2024 (see
[halos](halos.md)).

## Our spiral at home: the Milky Way (second pass)

Seen from outside, the Milky Way would read as a barred spiral near
SBbc/SBc: a boxy-peanut bulge-bar about 3–5 kpc in half-length (trapped
orbits to ~3.5 kpc, overdensities to ~4.8 kpc where bar meets arm),
angled 20–30 degrees ahead of the Sun–center line, spinning slowly at
33–40 km/s/kpc. Gaia DR3 nails the slow spin twice over: the Hercules
moving group strengthens toward the bar's minor axis exactly as
corotation-resonance models predict, and the bar's radial-velocity
quadrupole mirrors symmetrically past the Galactic center, proving the
bar is bisymmetric and settled. The total stellar disk weighs 3–5
x 10^10 solar masses with a broken-exponential profile (half-light
radius ~5.75 kpc); the Sun sits at ~8.15 kpc inside the disk, in or
beside the Local Arm spur between the Sagittarius-Carina and Perseus
arms.

How many arms wrap that disk is the live debate. The long-standing
four-continuous-arm map (Norma, Scutum-Centaurus, Sagittarius-Carina,
Perseus, from Georgelin–Georgelin 1976 through Reid et al. 2019
BeSSeL maser parallaxes) now faces the Xu et al. 2023 multiple-arm
map: two symmetric inner arms (Perseus and Norma) bifurcating outward
into the Centaurus, Sagittarius, Carina, Outer, and Local arms. The
multiple-arm picture fits the statistics of external SBbc/SBc
galaxies — 54 percent multi-armed, only 6 percent grand-design — where
almost no system shows four arms running center to edge, and 2026
Cepheid tests find young stars consistent with Perseus-plus-Norma as
the dominant pair. The stellar backbone itself looks two-armed even
where gas shows more, an age gradient across arms that favors the
density-wave picture. Either way the disk is not settled: Gaia phase
spirals, warps (the Outer arm warps with the disk), and corrugations
record ongoing shaking by the Sagittarius dwarf and the infalling
Large Cloud.

Sources: Xu et al. ApJ 947 (2023) multiple-arm Milky Way (inner
Perseus-plus-Norma symmetry); Reid et al. 2019 BeSSeL four-arm maser
map; Gaia DR3 Hercules/corotation (Lucchini et al. 2023, 2024);
Vislosky et al. MNRAS 528 (2024) bar length 3–5.2 kpc versus arm
strength; Lucey et al. 2023 dynamical bar ~3.5 kpc; Gaia long-period
variables bar pattern speed 34.1 km/s/kpc and length ~4.0 kpc (2024);
2024 SDSS DR18 arm census (74% of barred spirals two-inner-multi-outer);
2026 Cepheid R19-versus-X23 comparison; Outer-arm red-clump warp study
(A&A 2023).

## What is new (2020–2026)

- **Spirals arrived early.** JWST pushed mature disks far earlier than
  Hubble-era models allowed: the barred Milky-Way progenitor
  ceers-2112 at redshift ~3 (2 Gyr after the Big Bang, bar 3.3 kpc,
  disk settled by redshift 4–5); the giant Big Wheel disk at redshift
  3.25 (half-light radius 9.6 kpc, ~3.7 x 10^11 solar masses, flat
  rotation ~200 km/s on the local Tully–Fisher line); grand-design
  candidate Alaknanda at redshift ~4 (two symmetric arms, 10 kpc,
  63 solar masses/yr); and ultra-massive Zhulong at redshift ~5.2
  (Milky-Way-mass bulge plus 19-kpc disk with arms, only 1 Gyr after
  the Big Bang). Intrinsic spiral fractions run ~40 percent at
  redshift 1–3 (CEERS: observed 48% at redshift 0.75 falling to 8% at
  redshift 2.75, intrinsic ~35–40% after redshifting corrections),
  with density-wave color offsets of 0.2–0.8 kpc measured across arms
  at redshift ~1.5. Cold settled disks and bars were in place by
  redshift 3–5, not redshift 1–2.
- **Bars matured with the disk.** The local bar census settled on
  method-dependence (visual ~63%, ellipse-fitting ~48–56%, Fourier
  ~36%) with strong bars in early types and weak bars in late types;
  JWST CEERS/PRIMER finds ~14–18% barred at redshift 1–3, double the
  HST rate in the same fields — bars were already shaping disks 11 Gyr
  ago, and ceers-2112 shows one assembled in ~400 Myr.
- **Our own arms got counted.** The SDSS DR18 census of 5,093 blue
  spirals (2024) demoted grand designs to ~3 percent and crowned
  multi-armed disks at ~59 percent, with 82 percent showing two inner
  arms — the statistical case behind the Xu et al. two-inner-arm Milky
  Way map used in this document. Gaia DR3 plus Cepheid and red-clump
  mapping sharpened the bar (3–5 kpc, slow 33–40 km/s/kpc) and traced
  the warp into the Outer arm itself.
- **Star formation went cloud-scale.** PHANGS plus JWST mapped ~10,000
  clusters, ~20,000 ionized regions and ~12,000 molecular clouds across
  19 spirals, pinning the molecular depletion clock at 1–3 Gyr and
  showing arms as organizers rather than triggers of star birth.

Sources: Costantin et al. Nature 623 (2023, ceers-2112); Nature
Astronomy Big Wheel 2025 (redshift 3.25 giant disk); Alaknanda A&A
2025 (redshift ~4 grand design); Zhulong A&A 2025 / NOIRLab 2025
(redshift 5.2); Guo et al. ApJL 2024 (CEERS spiral fractions);
Kalita et al. 2025 (COSMOS-Web arm offsets at redshift 1.5);
PHANGS–JWST Treasury 2023; 2024 SDSS DR18 arm census.

## Size, mass, example objects

Typical spirals span 5–100 kiloparsecs and weigh 10^9–10^12 solar masses.

Example objects:

- **Milky Way** — home barred spiral, ~100,000 light-years across,
  100–400 billion stars. Source: NASA "Our Milky Way".
- **Andromeda (M31)** — nearest major spiral, ~2.5 million light-years
  away (~780 kpc), bright disk ~152,000 light-years, ~1 trillion stars,
  tilted 77 degrees, roughly twice the Milky Way's mass. Unlike our
  quiet history, Andromeda swallowed a ~2.5 x 10^10-solar-mass
  companion (M32p, the lost third-largest Local Group member, mass
  ratio ~1:4–1:5) about 2–4 Gyr ago: the event built its rotating inner
  halo and Giant Stellar Stream, fired a disk-wide starburst forming
  about a fifth of its stars, and left its thin and thick disks two to
  three times puffier than ours. Its compact neighbor M32 is likely the
  stripped core left behind. Sources: NASA Messier 31; Wikipedia
  "Andromeda Galaxy"; D'Souza and Bell Nature Astronomy 2 (2018,
  M32p progenitor); Hammer et al. MNRAS 475 (2018, 4:1 merger 1.8–3
  Gyr ago); Bhattacharya et al. 2023–2024 planetary-nebulae wet-merger
  timing; Dierickx et al. ApJ 788 (2014, M31–M32 collision and rings).
- **Triangulum (M33)** — third spiral of the Local Group, ~2.7–3
  million light-years out, ~40 billion stars, half home's size, the
  classic flocculent contrast to M51's grand design: patchy
  star-formation arms with no driving bar or companion.
  Source: NASA Messier 33.
- **Whirlpool (M51)** — grand-design face-on spiral, 25–31 million
  light-years out in Canes Venatici (see disagreeing-numbers note),
  almost face-on with its eastern side tilted ~20 degrees, imaged by
  Hubble ACS in January 2005 for Hubble's 15th anniversary. Companion
  NGC 5195 (a dwarf barred SB0) glides behind it and has crossed its
  plane twice: the first passage raised the two-arm pattern from a
  flocculent disk, the second cut the kinks both arms still show, with
  star-cluster ages peaking near 100 and 500 Myr. The arm assembly line
  runs dust-lane compression to pink star-forming knots to blue outer
  clusters, with a 90-kpc atomic-hydrogen tidal tail trailing west.
  Source: ESA/Hubble heic0506a; NASA Messier 51; Dobbs et al. 2010
  two-passage simulations; 2024 H-alpha/CO resonance-mapping study.
- **Sombrero (M104)** — luminous Sa spiral ~29 million light-years out
  in Virgo, seen 6 degrees off its plane: a brilliant white bulge cut
  by thick dust lanes, the edge-on bookend to M51's face-on view.
  Source: ESA Sombrero release.

## A note on disagreeing numbers

Three numbers on this page move with method and date; read the method
first. **M51's distance** runs from ~7.5 Mpc (2023 Cepheid plus SN
2005cs: 7.59 and 7.34 Mpc, combined 7.50) through ~8.6 Mpc (2016 HST
tip-of-the-red-giant-branch) to heritage 9–10 Mpc values — a ~20
percent spread that rescales every luminosity-based mass and size, so
this page quotes 25–31 million light-years with the method named.
**Milky Way total mass** runs from ~2 x 10^11 (Gaia DR3 declining-curve
fits to small virial radii) through ~6 x 10^11 (escape velocity) to
~10^12 solar masses (streams, globulars, H3 giants); see
[halos](halos.md) for the full method spread. **Andromeda's approach**
reads ~110 km/s Galactocentric radial (HST proper motions, nearly
head-on) while Gaia EDR3/DR3 transverse components (~80 km/s and up)
leave the merger itself uncertain: 2025 four-body integrations give
only ~50 percent odds of any Milky-Way–Andromeda merger in the next 10
Gyr once M33 and the Large Cloud are included.

Sources: Nair et al. A&A 2023 (M51 Cepheid-plus-supernova distance);
McQuinn et al. ApJ 2016 (M51 TRGB 8.58 Mpc); van der Marel et al. 2012
(M31 HST proper motions, radial orbit); Salomon et al. 2021 (Gaia EDR3
transverse); Sawala et al. Nature Astronomy 2025 (50% merger odds).

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `whirlpool-hst.jpg` | M51 Whirlpool face-on spiral with companion (screensize) | NASA, ESA, S. Beckwith (STScI), and the Hubble Heritage Team (STScI/AURA) | CC-BY 4.0 (`https://esahubble.org/images/heic0506a/`) |

## Sources

- Wikipedia, Spiral galaxy: `https://en.wikipedia.org/wiki/Spiral_galaxy`
- Wikipedia, Hubble sequence: `https://en.wikipedia.org/wiki/Hubble_sequence`
- Wikipedia, Barred spiral galaxy: `https://en.wikipedia.org/wiki/Barred_spiral_galaxy`
- Wikipedia, Grand design spiral galaxy: `https://en.wikipedia.org/wiki/Grand_design_spiral_galaxy`
- Wikipedia, Density wave theory: `https://en.wikipedia.org/wiki/Density_wave_theory`
- Wikipedia, Kennicutt-Schmidt law: `https://en.wikipedia.org/wiki/Kennicutt%E2%80%93Schmidt_law`
- Wikipedia, Galaxy rotation curve: `https://en.wikipedia.org/wiki/Galaxy_rotation_curve`
- Wikipedia, Whirlpool Galaxy: `https://en.wikipedia.org/wiki/Whirlpool_Galaxy`
- ESA/Hubble, M51 heic0506a: `https://esahubble.org/images/heic0506a`
- NASA, tuning fork: `https://science.nasa.gov/asset/hubble/the-hubble-tuning-fork-classification-of-galaxies`
- NASA, M51 2005 ACS release: `https://science.nasa.gov/asset/hubble/out-of-this-whirl-the-whirlpool-galaxy-m51-and-companion-galaxy/`
- NASA, Messier 51: `https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-51/`
- NASA, Milky Way: `https://science.nasa.gov/universe/exoplanets/our-milky-way-galaxy-how-big-is-space/`
- NASA, Messier 31: `https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-31/`
- NASA, Messier 33: `https://science.nasa.gov/mission/hubble/science/explore-the-night-sky/hubble-messier-catalog/messier-33`
- Cambridge, inside-out growth (JADES, Nature Astronomy 2024): `https://www.cam.ac.uk/research/news/inside-out-galaxy-growth-observed-in-the-early-universe`
- Shu ARA&A 54 (2016), six decades of density-wave theory: `https://doi.org/10.1146/annurev-astro-081915-023426`
- Dobbs and Baba 2014, Dawes review spiral structures: `https://doi.org/10.1017/S1323358000020337`
- 2024 SDSS DR18 arm census (AJ, flocculent/multi-arm/grand-design): `https://doi.org/10.3847/1538-3881/ad8632`
- Smith et al. 2024, grand-design versus multi-armed structure: `https://doi.org/10.3847/1538-3881/ad46fb`
- Ann et al. ApJ 872 (2019), bar fraction early- versus late-type spirals: `https://doi.org/10.3847/1538-4357/ab0024`
- Xu et al. ApJ 947 (2023), multiple-arm Milky Way: `https://doi.org/10.3847/1538-4357/acc45c`
- Vislosky et al. MNRAS 528 (2024), Gaia DR3 short bar: `https://doi.org/10.1093/mnras/stae036`
- Lucchini et al. 2023–2024, Hercules corotation slow bar: `https://arxiv.org/abs/2305.04981`
- Costantin et al. Nature 623 (2023), barred spiral at redshift 3: `https://doi.org/10.1038/s41586-023-06636-x`
- Big Wheel, Nature Astronomy 2025 (giant disk at redshift 3.25): `https://www.nature.com/articles/s41550-025-02500-2`
- Alaknanda, A&A 2025 (grand design at redshift 4): `https://www.aanda.org/articles/aa/full_html/2025/11/aa51689-24/aa51689-24.html`
- Zhulong, A&A 2025 (ultra-massive spiral at redshift 5.2): `https://www.aanda.org/articles/aa/full_html/2025/04/aa53487-24/aa53487-24.html`
- Guo et al. ApJL 2024, JWST CEERS spiral fractions: `https://doi.org/10.3847/2041-8213/ad43eb`
- PHANGS Sun et al. ApJL 2023, star-formation laws 80 galaxies: `https://doi.org/10.3847/2041-8213/acbd9c`
- PHANGS-JWST Treasury 2023: `https://doi.org/10.3847/2041-8213/acaaae`
- D'Souza and Bell Nature Astronomy 2 (2018), M32p merger: `https://doi.org/10.1038/s41550-018-0533-x`
- Hammer et al. MNRAS 475 (2018), Andromeda 4:1 merger: `https://doi.org/10.1093/mnras/stx3845`
- Kenney et al. 2004 / Vollmer et al. 2006 (NGC 4522 stripping); Cramer et al. 2020 ALMA (NGC 4402 plume): `https://doi.org/10.3847/1538-4357/abaf54`
- Dobbs et al. 2010, M51 two-passage simulations (see M51 resonance-mapping 2024): `https://doi.org/10.3847/1538-4357/ad3541`
- McQuinn et al. ApJ 826 (2016), M51 TRGB distance: `https://doi.org/10.3847/0004-637X/826/1/21`
- Nair et al. A&A 2023, M51 Cepheid-plus-supernova distance: `https://doi.org/10.1051/0004-6361/202346971`
- Van der Marel et al. 2012, M31 HST proper motions: `https://doi.org/10.1088/0004-637X/753/1/9`
- Sawala et al. Nature Astronomy 2025, Milky-Way–Andromeda merger odds: `https://www.nature.com/articles/s41550-025-02563-1`
- L04 bridge: `previous-l3.md`; L04 entry: `entry.md`; L04 ellipticals: `ellipticals.md`; L04 dwarfs: `dwarfs.md`; L04 nuclei: `nuclei.md`; L04 halos: `halos.md`; L04 lifetime: `lifetime.md`

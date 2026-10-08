# Superclusters

Superclusters are the largest
galaxy systems: sprawling complexes of clusters, filaments and walls spanning
hundreds of megaparsecs — mostly unbound, still assembling, and already
dissolving into the accelerated expansion. Template follows the pilot
(`cosmic-web.md`): looks, creation, change over time, interactions, numbers,
examples, target visuals, sources.

## How superclusters look

Superclusters are large flattened or filamentary high-density systems
extending to tens of megaparsecs, surrounding voids in a cellular pattern;
in redshift surveys they appear as aggregates on scales past 100 million
light-years, a sponge of dense regions spaced by voids. Rich clusters trace
them like peaks trace mountain chains: a typical supercluster holds 2-15
rich clusters. Two definitions coexist — density-contour superclusters
(partitioning nodes plus filament sections at a chosen overdensity or
linking length) and velocity-field basins of attraction, where peculiar
velocities point inward on average. In the watershed implementation each
voxel seeds a streamline integrated through the peculiar-velocity grid
(fourth-order Runge-Kutta); all voxels whose streamlines end at the same
attractor belong to one basin, and the time-reversed field gives basins of
repulsion. Typical basin volumes are 10^5-10^6 (Mpc/h)^3, and the
divergence-field version scales linearly with the velocity-divergence
autocorrelation length from ~10 to 100 Mpc/h. Morphology
splits into spiders (one central mass fed by many filaments) and filament
types (mass spread along one massive thread); the richest are elongated,
poor ones mostly spherical. Superclusters host ~15% of all galaxies in ~1%
of volume. Sources: Bahcall lectures; Fairall essay; Einasto 2025 review;
SLOW-IV; Dupuy et al. 2019 partitioning; Dupuy et al. 2020 segmentation;
Penaranda-Rivera et al. divergence definition.

Shapefinder detail: Minkowski-functional K1-K2 tracks show multibranching
filaments as the norm (lengths to ~100 Mpc/h, largest >150 Mpc/h); the
rich-poor divide sits at ~20 Mpc/h size, ~4 x 10^12 h^-2 solar
luminosities, ~5 x 10^15 solar masses. Filament types (e.g. Sloan SCl 126 /
SCl 027, simple few-filament spine) vs spider / multispider types (e.g.
Sloan SCl 111 / SCl 019, many filaments; Corona Borealis multispider at a
three-chain intersection) can share the same overall elongation yet differ
in clumpiness (fourth functional V3); spiders host more star formation and
substructure. Poor systems are mostly spherical and never reach core
densities — their maxima resemble rich-system outskirts.

![Nearby superclusters and voids within a billion light-years, Laniakea in yellow](../images/supercluster-laniakea-context.gif)

The home view in flow form: Laniakea rendered with density in color (red
dense, blue void), galaxies as white dots, white streams flowing inward to
the Great Attractor valley, an orange contour enclosing ~160 Mpc — outside it
galaxies stream elsewhere. Cosmicflows-2 (8,161 distances to z = 0.1)
defined that watershed as a ~160 Mpc (~12,000 km/s) diameter volume of
~10^17 solar masses and ~100,000 galaxies; Cosmicflows-4 re-measures it as
1.9 x 10^6 (Mpc/h)^3 around an attractor at supergalactic Cartesian
[-62, -8, 39] Mpc/h (sgl = 187 deg, sgb = 32 deg, cz = 7,370 km/s),
coincident with the Great Attractor direction. In the grouped CF4
reconstruction Laniakea merges into the larger Shapley basin, and Coma
itself segments as a substructure of Shapley — a reminder that basin
boundaries move with data depth and grouping. By contrast, wall views
(Sloan, CfA2) show thin sheets in redshift space: hundreds of millions of
light-years across, only ~20 deep. Sources: NASA APOD Laniakea; EarthSky
boundaries; NASA large-scale structures; Tully et al. Nature 2014;
Courtois et al. 2023 dynamic cosmography (CF4); Valade et al. 2024 basins.

## How superclusters are created

Superclusters are next up from clusters in the hierarchy, but unlike clusters
they are not virialized. Dark-matter gravity versus dark-energy antigravity
governs their formation from the largest primordial overdensities; high
density cores are older and more evolved than their outskirts. Sheets
collapse first, then filaments, then nodes at the intersections — so a
supercluster is a nested miniature universe of the whole web, with
filaments as the accretion channels shaping even halo orientations.
Occasionally the pattern surprises: the richest nearby superclusters sit in
two huge perpendicular planes hundreds of megaparsecs across (Local
Supercluster plane and Dominant supercluster plane), with a quasi-regular
120-140 Mpc/h spacing whose origin in Lambda-CDM is unclear — Einasto 2025
argues against a BAO origin, favoring primordial dark-matter perturbations.
Sources: Chon et al. definitions; SGW collapsing-cores
paper; A2142 paper; Shapley quenching paper; Einasto 2025 review.

## How superclusters change over time

Superclusters stayed loose and linear until recent times: overdensities of
just a few times the mean, decoupling from the Hubble flow last. Only around
z~0.5 did a few begin turnaround; when matter density fell below twice
dark-energy density (z~0.7), acceleration took over and growth slowed — the
largest bound structures are forming right now, and the richest ones assemble
after the present (Shapley faces two major mergers in forward runs to 119
Gyr). In the far future the web gains contrast: superclusters grow more
spherical and compact, voids emptier — but many will split, with collapsing
cores surrounded by expanding non-collapsing regions; some Sloan
superclusters fall apart into smaller ones. The collapse threshold today is
~7.8x mean matter density (overdensity criterion R for future collapse;
equivalent infall test is d x 49.5 km/s/Mpc — e.g. ~800 km/s required at
the Local Group distance, far above the observed <500 km/s inside 10 Mpc,
so only ~5.5 Mpc around Virgo collapses). High-density cores that survive
carry 0.5-5 x 10^15 suns each; influence (depletion) borders sit at density
contrast ~30-40 and turned around at z ~0.3-0.4. Sources: quasi-spherical paper; A2142 paper;
SLOW-IV; SDSS WHL catalog; Einasto SGW cores; Chon et al. 2015 Table 1;
Einasto 2025 review S6.

Laniakea itself will not survive: at ~0.94x mean density it is no significant
overdensity and fails future-collapse criteria — as a whole it disperses
while only dense subregions collapse. Follow-ups call such velocity basins
"supercluster cocoons" or "watersheds", reserving "supercluster" for the
dense traditional systems inside; proposed term for true survivors:
superstes-clusters. Sources: Chon et al.; Laniakea summary (Einasto 2019).

## How superclusters interact with the rest of the web

A galaxy between structures sits in a gravitational tug-of-war, and peculiar
velocities (observed minus Hubble flow) map the hidden mass: Tully's team
mapped ~8,000 galaxies (Cosmicflows-2) and traced the watershed surface where
flows diverge — that enclosed inward-flow volume is Laniakea, with motions
running downhill into the Great Attractor valley. Laniakea as a whole drifts
toward the Shapley concentration ~650 Mly away (z = 0.048, RA ~198-204 deg,
Dec ~-34 to -29 deg core); the prime suspect for our
motion graduated from the Great Attractor (150 Mly) to Shapley (600 Mly,
2+ dozen rich clusters). Shapley is the densest local mass concentration:
eRASS1 finds 45 member clusters and 2.58 +/- 0.51 x 10^16 solar masses over
~111 Mpc projected length, the richest of 1,338 western-hemisphere systems;
core masses converge at 1.1-2.6 x 10^16 suns within 8-12.4 Mpc/h (density
ratio R ~17-21, already past turnaround in the center), and the central
A3558/A3528 complexes each span ~7.5 Mpc with connectivity ~5 filaments.
Opposite Shapley pulls a void effectively pushes:
the Dipole Repeller and Cold Spot Repeller are repulsion basins, and push
and pull weigh about equally in the CMB dipole (Local Group: 631 km/s).
Dipole Repeller lies ~16,000 +/- 4,500 km/s away (CF2 Wiener filter;
CF3: glon = 94 deg, glat = -16 deg, 14,000 km/s, anti-aligned at
mu = -0.99); Cold Spot Repeller sits at ~23,000 km/s toward the CMB Cold
Spot (glon = 168 deg, glat = -71 deg), likely stacked voids. CF3 unifies
them nearby as one attraction basin (Shapley extension, glon = 306 deg,
15,900 km/s) plus two repulsion basins; CF4 splits the dipole/cold-spot
pair into a single gigantic entity plus Sculptor/Bootes-adjacent basins.
Sources: UH News; Tully et al. (Nature); Dupuy and
Courtois dynamic cosmography; Hoffman et al. Dipole Repeller;
Courtois et al. Cold Spot Repeller; Cosmicflows-3 papers.

## Key numbers

- Extents: observed superclusters to ~150 Mpc/h flattened (largest SDSS
  adaptive volumes exceed that); median SDSS WHL (662 systems, 0.05 < z <
  0.42, linking 20.65 Mpc) ~65 Mpc with median 13 members; eRASS1 X-ray
  (1,338 western-hemisphere systems to z = 0.8, friends-of-friends overdensity f = 10)
  median rich-system length ~33.5 Mpc, mean member separation ~12.9 Mpc,
  longest 127 Mpc projected; density-contour threshold D_th = 5.0 mean
  density is the standard separator (percolation at ~2.5).
- Masses: poor ~10^14 solar masses; typical median ~5.8 x 10^15 (WHL bound
  mass only — intercluster medium at least doubles it); richest
  ~10^16; high-density cores 0.5-5 x 10^15; basins (Laniakea) ~10^17 —
  near mean density, unbound. eRASS1 total masses sum M200 scaled by
  M_tot,Cl = 0.39 + 0.077 x M_tot,SC; 63% have reliability >0.7.
- Share: superclusters ~15% of galaxies in ~1% of volume; 54% of catalogued
  clusters live in one; number density ~10^-6 per cubic Mpc/h^3. Modern
  censuses agree: WHL mocks match observations; Quipu five hold 45% of
  clusters / 30% of galaxies / 25% of matter in 13% of volume; eRASS1 puts
  45% of clusters (3,948 of 8,862) in superclusters.
- Laniakea: ~500 Mly diameter (~160 Mpc), 10^17 suns in 100,000+ galaxies,
  center ~250 Mly away — dispersing, not bound; CF4 volume 1.9 x 10^6
  (Mpc/h)^3 vs 1.7 (CF2 eyeball) and 2.3 (CF2 segmented).
- Shapley core: 2 x 10^15 to 1.3 x 10^16 solar masses within 8 Mpc/h;
  eRASS1 total 2.58 +/- 0.51 x 10^16 over 111 Mpc; collapse radius
  ~11-12.4 Mpc/h; turnaround already reached inside ~6.5-8 Mpc/h.
- "Largest" is definition- and epoch-dependent (see examples).

Sources: Bahcall catalog; SDSS WHL (Sankhyayan et al. 2023); eRASS1 Liu et
al. 2024; Einasto 2025; UH News; Nature Tully; Chon et al.; ESO Messenger
Shapley; Bag et al. SDSS DR12 shape distribution.

## Example objects

- Laniakea (home): "immense heaven", 2014 (Tully et al. Nature; 8,161
  Cosmicflows-2 distances), subsumes Virgo and Hydra-Centaurus, includes
  Great Attractor, Pavo-Indus filament, Antlia Wall; ~160 Mpc / 10^17 suns /
  100,000+ galaxies; neighbors Shapley, Hercules, Coma, Perseus-Pisces —
  all drifting toward Shapley, possibly one greater basin. CF4 confirms the
  watershed (1.9 x 10^6 (Mpc/h)^3) but merges it into Shapley when grouped;
  Coma segments inside Shapley — basin hierarchy, not fixed borders.
- Shapley: found 1930, >8,000 galaxies, core 11 clusters/groups
  (A3552/A3554/A3556/A3558/A3559/A3560/A3562/AS0724/AS0726/SC1327-312/
  SC1329-313, M500 ~1-6 x 10^14 each) within ~260 Mpc^2 at z = 0.033-0.06;
  most massive local concentration (eRASS1: 45 members, 2.58 x 10^16 suns);
  Planck+ROSAT+DSS composite core spans Abell 3558/3562; central 11 Mpc/h
  already collapsing; two major mergers forecast to 119 Gyr.
- Sloan Great Wall complex: 2003 (Gott et al.), ~1.37 Gly visual ridge
  (~420 Mpc; 230 Mpc/h with poor systems, 165 Mpc/h rich only; 433 Mpc at
  z = 0.078), richest nearby system at ~1 Gly distance, lower mass limit
  ~2.5 x 10^16 h^-1 suns; three high-density cores in SCl 027
  (A1650/A1750/A1773, 2.4-5.9 x 10^15 h^-1 suns, 30-57 Mpc/h, D8 > 7) plus
  two in SCl 019, collapsing inside <6.5-8 Mpc/h (<2 x 10^15 suns);
  SCl 126 filament vs SCl 111 multispider; streamlines sink at SCl 126, not
  the densest SCl 111. Single wall vs complex debated; Park et al. say
  Lambda-CDM can make it, Sheth-Diaferio and morphology argue tension with
  Gaussian seeds. Shell wall at 120-130 Mpc/h around Bootes A1795.
- Quipu (2025): 428 Mpc long at z = 0.027-0.065 (mean 0.043), 68 CLASSIX
  clusters, ~2.4 x 10^17 suns — largest confirmed superstructure, a long
  filament with side strands (Inca quipu; Bohringer et al. 2025);
  Shapley in the same census is 90 Mpc / 23 clusters / 0.8 x 10^17 suns but
  the densest (cluster overdensity 3.8 vs 1.4 for Quipu). In the far future
  such superstructures break into several collapsing units: transient
  configurations today, not single bound objects.
- Hyperion proto-supercluster (PSC J1001+0218): z ~2.45 in COSMOS (11 Gly
  lookback, 2.3 Gyr after the Big Bang), ~60 x 60 x 150 comoving Mpc^3,
  ~4.8 x 10^15 suns over 7 density peaks linked by filaments; peaks
  0.09-2.7 x 10^14 suns each, forecast to virialize by z ~0.8-1.6 into a
  Sloan-Wall- or Virgo-like descendant. Largest earliest assembly known at
  that epoch (VUDS/VLT; Cucciati et al. 2018, ESO 1833) — surprisingly
  massive so early, but consistent with Lambda-CDM progenitors seen in
  simulations.
- Hercules-Corona Borealis: 2-3 Gpc putative GRB wall at z ~1.6-2.1
  (2014 Horvath et al.; 2025 Hakkila reanalysis extends radial range to
  z ~0.33-2.43 and ~10 Gly length) — still disputed (Christian 2020
  Monte-Carlo doubts; Ukwatta-Wozniak exposure bias; homogeneity limit
  ~260 Mpc/h vs GRB-tracer allowance ~8.6 Gpc); would dwarf everything if
  real. Treat as candidate, not confirmed superstructure.
- South Pole Wall: announced 2020 (Pomarede, Tully et al., Cosmicflows-3
  ~18,000 velocities), >=1.37 Gly (420 Mpc Apus-to-Funnel; 250 Mpc Apus-Lepus
  plus 170 Mpc bend), 220 Mpc rectilinear section at ~12,000 km/s through
  Chamaeleon, ~500 Mly away — largest contiguous local feature, comparable
  to Sloan at half the distance, skirting Laniakea's southern border opposite
  Shapley; found by velocity behind dust, not redshift. Full extent unknown
  (CF3 radius only 0.05c).

Sources: UH News; Tully et al. Nature 2014; eRASS1 Liu et al. 2024; SGW
Einasto et al. 2016 (A&A 595, A70); Quipu Bohringer et al. A&A 695, A59,
2025; ESO 1833 + Cucciati et al. 2018 Hyperion; Hakkila et al. 2025 HCB
reanalysis + Horvath et al. 2020 review; Pomarede et al. 2020 South Pole
Wall (ApJ 897, 133).

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `supercluster-laniakea-context.gif` | Nearby superclusters + voids within 1 Gly, Laniakea yellow | Richard Powell / Atlas of the Universe | CC-BY-SA 2.5 (`https://creativecommons.org/licenses/by-sa/2.5/`) |

*Figure 1. Nearby superclusters and voids within a billion light-years,
Laniakea highlighted — schematic context for the watershed discussed above.*

Schematic only, not a flow-field map: the scientifically ideal Tully/SDvision
flow visualization is not cleared (NASA-hosted but third-party rights), so
it stays out until written permission exists.

Related parts: [Cosmic web](./cosmic-web.md), [Voids](./voids.md),
[Filaments](./filaments.md), [Walls](./walls.md), [Nodes](./nodes.md),
[Background](./background.md), and the dated [Lifetime](./lifetime.md).

Proposed images (not downloaded — PD status unconfirmed, for iter10 review):
candidate ESO press renders for Quipu (CLASSIX sky shell) and Hyperion
(VLT/VUDS density field) would each need a PD-or-CC source check plus
`supercluster-quipu-context` / `supercluster-hyperion-field` SVG redraws
before copying anything into `../images/`.

## Sources

- Bahcall superclusters: `https://ned.ipac.caltech.edu/level5/Sept01/Bahcall2/Bahcall9.html`
- Fairall essay: `https://ned.ipac.caltech.edu/level5/ESSAYS/Fairall/fairall.html`
- Einasto 2025 review (Universe 11, 167): `https://arxiv.org/pdf/2505.22082v1`
- SLOW-IV: `https://www.aanda.org/articles/aa/full_html/2025/10/aa53421-24/aa53421-24.html`
- Tully et al. (Nature 2014, Laniakea): `https://www.nature.com/articles/nature13674`
- Tully et al. arXiv Laniakea: `https://arxiv.org/abs/1409.0880v1`
- UH News Laniakea: `https://www.hawaii.edu/news/2014/09/03/uh-scientist-maps-supercluster-of-galaxies-names-it-laniakea/`
- NASA APOD Laniakea: `https://apod.nasa.gov/apod/ap140910.html`
- NASA large-scale structures: `https://science.nasa.gov/universe/galaxies/large-scale-structures/`
- Chon et al. definitions (superstes-clusters): `https://www.aanda.org/articles/aa/full_html/2015/03/aa25591-14/aa25591-14.html`
- Quasi-spherical paper: `https://arxiv.org/pdf/2210.13294`
- SDSS WHL catalog (Sankhyayan et al. 2023, 662 systems): `https://ar5iv.labs.arxiv.org/html/2309.06251`
- SDSS DR12 shape distribution (Bag et al.): `https://ar5iv.labs.arxiv.org/html/2111.10253`
- eRASS1 supercluster catalog (Liu et al. 2024, 1,338 systems): `https://www.aanda.org/articles/aa/full_html/2024/03/aa48884-23/aa48884-23.html`
- Quipu discovery (Bohringer et al. A&A 695, A59, 2025): `https://www.aanda.org/articles/aa/full_html/2025/03/aa53582-24/aa53582-24.html`
- SGW complex: `https://www.aanda.org/articles/aa/full_html/2016/11/aa28567-16/aa28567-16.html`
- SGW collapsing cores: `https://arxiv.org/pdf/1608.04988`
- Dupuy and Courtois dynamic cosmography (CF4): `https://www.aanda.org/articles/aa/full_html/2023/10/aa46802-23/aa46802-23.html`
- Dupuy et al. 2019 partitioning (watersheds): `https://ar5iv.labs.arxiv.org/html/1907.06555`
- Dupuy et al. 2020 segmentation: `https://ar5iv.labs.arxiv.org/html/2002.06814`
- Hoffman et al. Dipole Repeller: `https://arxiv.org/abs/1702.02483v1`
- Courtois et al. Cold Spot Repeller (Cosmicflows-3): `https://iopscience.iop.org/article/10.3847/2041-8213/aa88b2`
- South Pole Wall (Pomarede et al. 2020): `https://iopscience.iop.org/article/10.3847/1538-4357/ab9952`
- Shapley core dance (A&A 2025, eRASS1 masses): `https://www.aanda.org/articles/aa/full_html/2025/02/aa51066-24/aa51066-24.html`
- Shapley quenching miniature universe (A&A 2024): `https://www.aanda.org/articles/aa/full_html/2024/09/aa48672-23/aa48672-23.html`
- ESO Hyperion: `https://www.eso.org/public/news/eso1833`
- Hyperion paper (Cucciati et al. A&A 619, A49): `https://www.aanda.org/articles/aa/full_html/2018/11/aa33655-18/aa33655-18.html`
- HCB wall reanalysis (Hakkila et al. 2025, GRB range): `https://arxiv.org/html/2504.05354v1`
- HCB existence review (Horvath et al. 2020): `https://academic.oup.com/mnras/article/498/2/2544/5895980`
- Commons Laniakea.gif: `https://commons.wikimedia.org/wiki/File:Laniakea.gif`

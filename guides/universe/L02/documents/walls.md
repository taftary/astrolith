# Walls and sheets — the pancakes of the web

Walls — also called sheets or
pancakes — are the flattened, quasi-2D component of L2: one axis
collapsed, two still extended, sitting between voids and filaments.
A wall at L02 scale runs hundreds of megaparsecs along its plane but
only a few megaparsecs thick, bounding voids on its faces and feeding
filaments along its ridges. Sibling docs: `filaments.md`, `voids.md`,
`superclusters.md`; bridge in `previous-l1.md`, entry in `entry.md`,
timeline in `lifetime.md`. For the whole-view context and the formal
finder taxonomy see the L01 companion
[Walls](../../L01/documents/walls.md); this page covers walls at full
L02 scale — edge-on vs face-on views, the DTFE look, the named walls
(CfA2, Sloan, South Pole, BOSS), their draining into filaments, and
the 2020-2026 additions.

## What they are and how they look

A wall seen edge-on in a redshift wedge is a thin dense ridge — this is why
the CfA2 discovery slice (Geller and Huchra 1989, Science 246, 897-903)
looks like one long line: at least 500 million light-years long, about
300 million wide, only about 15 million thick (Marcus Chown's summary;
survey cuts move these numbers — see Example objects). Seen face-on the
same object is a diffuse sheet with embedded filaments, groups and
clusters, much lower contrast than a node. That contrast gap is physical:
walls sit near mean density (roughly 1-5x the mean; multiscale-morphology
mean 1+delta ~1.1, median ~0.9) and host mostly low-mass halos, so they
look sparsely populated with widely spaced dim galaxies and are hard to
pick out — in NEXUS+ slices they render as extended tenuous planar
networks, distinct from line-like filaments.

DTFE reconstructions show this directly. The Delaunay Tessellation Field
Estimator (Schaap and van de Weygaert) rebuilds a continuous density
field from discrete galaxy positions in three steps: Delaunay-tessellate
the point set (cells shrink automatically where points crowd, so the mesh
adapts to density and geometry with no user smoothing); estimate density
at each galaxy as the inverse of its surrounding cell volume; linearly
interpolate inside each cell. Walls then appear as faint continuous
membranes joining bright filament spines — the canonical view below, with
the Sloan Great Wall standing out among nearby superclusters. The same
2dF DTFE render is the archetype figure in the L01 companion
[Walls](../../L01/documents/walls.md#how-walls-look).

Target visual (files in `images/`, credits and licenses at the bottom):

![DTFE reconstruction of the inner 2dF survey: density membranes and filament spines with the Sloan Great Wall among nearby superclusters](../images/walls-2df-dtfe-sloan.gif)

*Figure 1. DTFE reconstruction of the inner 2dF Galaxy Redshift Survey —
walls as faint membranes between bright filament spines, the Sloan Great
Wall labelled among nearby superclusters.*

Sources: Geller and Huchra, "Mapping the Universe", Science 246/4932
(1989); NASA Imagine on sheets and voids; NASA APOD 2007 Nov 7;
Wikipedia "Delaunay tessellation field estimator"; Wikipedia "Galaxy
filament" (walls table); Cautun et al., "Evolution of the cosmic web"
(arXiv:1401.7866); Aragon-Calvo et al. 2010 (MMF).

## Example objects

- CfA2 / Coma / Northern Great Wall: the first super-large structure,
  found around 1989 by Margaret Geller and John Huchra in the 2nd CfA
  Redshift Survey ("Mapping the Universe", Science 246/4932). Chown's
  summary: about 300 million light-years wide, 15 million thick, snaking
  at least 500 million light-years; SDSS-cut measurements give 251 Mpc
  long at redshift z = 0.0306 (750 x 250 x 20 million light-years in the
  walls-table compilation), with 500-600 Mly length spreads reflecting
  different boundary and Zone of Avoidance cuts. Holds the Hercules,
  Coma and Leo clusters (Coma near the core; the Coma Supercluster forms
  most of the central "homunculus"); full extent unknown because Milky
  Way gas and dust (the Zone of Avoidance) hide the far side. Sources:
  Wikipedia "CfA2 Great Wall"; NASA Imagine on sheets and voids;
  Wikipedia "Galaxy filament" (walls table); CfA Harvard ZCAT.
- Sloan Great Wall: 1.37 billion light-years long (1.30 x 10^25 m, ~433
  Mpc peer-reviewed), about a billion light-years away in Corvus, Hydra
  and Centaurus at redshift z ~ 0.078; 1.8-2.7 times the CfA2 wall.
  Announced 20 October 2003 by Gott, Juric and colleagues from SDSS
  (Gott et al. 2005, ApJ 624, 463); its richest part is supercluster
  SCl 126 in the highest-density region (Einasto et al. 2011, ApJ 736,
  51). Later work suggests it may be a chance alignment of three
  structures rather than one bound object — and strictly it is not a
  "structure" in the technical sense, since its parts are not
  gravitationally bound together. Sources: Wikipedia "Sloan Great
  Wall"; ladder R5 anchor (1.30 x 10^25 m); NASA APOD 2007 Nov 7.
- South Pole Wall: over 1.37 billion light-years long, nearest light
  ~500 million years old, arcing 200 degrees from Perseus to Telescopium
  over the south celestial pole with the density peak near the pole
  itself. Announced July 2020 by Pomarede, Tully and colleagues from
  Cosmicflows-3 peculiar velocities (ApJ 897:133) — found through its
  gravity imprint on galaxy motions, not direct counts, because dust and
  patchy southern coverage hid it; Pomarede: "found thanks to its
  gravitational influence, imprinted in the velocities". Dense in five
  known places; described as the largest contiguous local-volume feature
  and comparable to the Sloan wall at half the distance. It measurably
  perturbs the local expansion; Tully notes the mapped span fills the
  surveyed domain, so the true extent is still unknown. Sources:
  Wikipedia "South Pole Wall"; University of Hawaii news, 10 Jul 2020;
  Pomarede et al. 2020.
- BOSS Great Wall (2016): supercluster complex at mean redshift
  z ~ 0.47 (~6.8 billion light-years light-travel), ~1 billion
  light-years across, 830+ visible galaxies, ~10,000 Milky-Way masses.
  Two wall components (186 and 173 Mpc/h, superclusters A and B) plus 91
  and 64 Mpc/h superclusters (D and C); identified in BOSS/SDSS by
  Lietzen, Einasto and colleagues, shape verified with Minkowski
  functionals. Richer in dense high-stellar-mass galaxies than the Sloan
  wall; whether it moves together or is being pulled apart by expansion
  is still debated. Sources: Wikipedia "BOSS Great Wall"; Lietzen et
  al. 2016 (A&A 588, L4); Einasto et al. 2017 (A&A 603, A5).
- Quipu context (reported 2025): 68 ROSAT/CLASSIX X-ray clusters,
  ~400-428 Mpc / ~1.3 billion light-years long, ~2 x 10^17 solar masses
  (~200,000 Milky Ways) — the largest reliably characterised nearby
  superstructure, named for the knotted Andean textile. One of five
  CLASSIX superstructures holding ~45% of clusters in ~13% of the local
  volume; its streaming motions perturb Hubble-constant fits. It sets
  the present upper end of the wall/complex scale that L02 examples are
  measured against. Sources: Wikipedia "Quipu (cosmic structure)";
  Bohringer et al. 2025 (A&A 695, A59).
- Nearby wall census: Sculptor Wall (Southern Great Wall), Grus Wall
  and Fornax Wall (hosting the Fornax Cluster) — Sculptor parallel to
  Fornax, Grus perpendicular to both — plus the proposed Centaurus
  (Fornax/Virgo/Local) Great Wall and the Norma/Great Attractor Wall,
  now largely superseded by the Laniakea basin picture. The Coma
  Filament holds the Coma Supercluster and forms part of the CfA2 wall.
  Sources: Wikipedia "Galaxy filament" (walls table); Wikipedia "Coma
  Filament".
- Typical scales: wall lengths of tens to hundreds of megaparsecs (giant
  complexes to ~400+ Mpc), thickness only a few megaparsecs; individual
  wall masses are rarely quoted alone — use supercluster-complex scale
  (~10^16-10^17 solar masses; BOSS ~10^4 Milky Ways, Quipu
  ~2 x 10^17 suns). Sources: Wikipedia "Galaxy filament" (walls table);
  Wikipedia "Laniakea Supercluster".
- Dipole Repeller (related void, not a wall): ~220 Mpc away, the
  underdensity opposite Shapley whose "push" is really missing pull —
  the Local Group moves 631 +/- 20 km/s vs the CMB with Shapley pull and
  repeller dip contributing roughly equally (Hoffman et al. 2017, Nature
  Astronomy 1, 0036). A second Cold Spot Repeller sits near
  ~23,000 km/s. See [voids](./voids.md) and
  [superclusters](./superclusters.md). Sources: Wikipedia "Dipole
  repeller"; Hoffman et al. 2017 (arXiv:1702.02483).

## How they were created

A large ellipsoid collapses fastest along its shortest axis, making a
"pancake" when pressure is negligible and only gravity matters
(Zel'dovich 1970, A&A 5, 84-89) — valid especially before recombination
built hydrogen atoms. Overlapping initial fluctuations then yield "dense
pancakes, filaments, and compact clumps" (Shandarin and Zel'dovich 1989,
Rev Mod Phys 61, 185); plane collapse compresses infalling gas through
shock waves, heating it (Sunyaev and Zel'dovich 1972). The original
pancake picture was top-down (supercluster condensations fragmenting
into protogalaxies), while hierarchical growth needs the truncated
Zel'dovich approximation — cut the small-scale power before displacing
(Pauls and Melott 1995) — with generalisations for a non-zero
cosmological constant; a candidate first pancake was claimed in 1991
from VLA neutral-hydrogen data at redshift 3.4.

In modern deformation-tensor form the eigenvalue count decides the fate:
three positive means cluster, two means filament, one means sheet, none
means void — with a time sequence from walls through filaments to full
collapse. Primordial overdensities favor clusters and filaments,
underdensities favor voids and sheets; tidal shear from the surrounding
megaparsec field stretches the membranes between peaks. Later
refinements (truncated Zel'dovich, adhesion with viscosity) handle the
bottom-up buildup and stream-crossing the first model misses, and void
expansion helps: underdense regions expand faster than average, piling
matter onto boundary shells. Sources: Wikipedia "Zeldovich pancake";
Cautun et al., "Evolution of the cosmic web" (arXiv:1401.7866).

## How they change over time

The early web holds many tenuous sheets pervading most of the volume; by
redshift ~1 most have merged into more prominent filaments, and evolution
slows after z ~ 0.5 with only minor changes to today. Matter flows voids
to walls to filaments to cluster nodes, so each wall's mass share falls
as it drains — particle tracking from z = 2 to z = 0 shows walls lose
over half their mass, mostly to filaments, while voids lose half theirs
to walls and filaments; the wall surface-density distribution shifts to
lower values and the whole wall network shrinks in extent. Apparent
reverse trickles are misidentified tenuous structures, not real backflow.

Treat any single percentage as finder-dependent (density, tidal and
shear finders disagree most on tenuous walls): the MMF inventory gives
mass 28.1/39.2/5.5/27.2% and volume 0.4/8.8/4.9/85.9%
(clusters/filaments/walls/voids), while NEXUS+ on the same box gives
walls ~24% of mass in ~18% of volume; stream-number finders give 10-20%
where others give up to ~50%. All finders agree on prominent walls;
tenuous-wall detection diverges. Gas in walls stays cool: at z = 0 the
warm-hot medium dominates filaments and knots, but cool diffuse gas still
dominates sheets and voids.

Local-flow context: the Local Group moves 631 +/- 20 km/s against the
microwave background, modeled as one Shapley-side attractor plus one
~220 Mpc Dipole Repeller void contributing roughly equally (Hoffman et
al. 2017); the "repulsion" is missing pull from the void side, not a new
force — gravity only attracts, and the underdensity pulls less while
denser directions pull more. Sources: Cautun et al. (arXiv:1401.7866);
Aragon-Calvo et al. 2010; Wikipedia "Dipole repeller"; Wikipedia "Void
(astronomy)"; Martizzi et al. (IllustrisTNG baryons).

## How they interact with the others

Walls feed filaments, which feed nodes — never picture a wall as an
isolated slab. Galaxies are pulled out of voids, cross sheets, join
filaments and fall into node attractors; bright filament ridges and
cluster knots sit inside the sheet plane or stream out of it. Filaments
dominate the dynamics everywhere — inside filaments, across most void
interiors, and inside all wall regions — so even void motion cannot be
understood without the surrounding web. Gravity from a wall is weak and
extended (near mean density, so its pull averages only ~14% of local
gravity in a 300 Mpc/h-box inventory vs ~60% filaments, ~20% voids,
~17% nodes — shares are method- and box-specific, the ranking is the
point); most bound mass lives in filaments and clusters, and node
gravity reaches only its immediate neighborhood. Voids expand faster
than average and push material toward the walls; the Local Void alone
contributes ~240 km/s to the Local Group's motion. For budgeting, walls
carry roughly 14-25% of total mass depending on the finder, sharing most
of the volume with voids — cite the NEXUS tables rather than one number.
Sources: Cautun et al.; Cosmic Web Dynamics 2024 review; Wikipedia "Void
(astronomy)"; Wikipedia "Galaxy filament".

## Key numbers

- Scale: 1 Mpc = 3.26 million light-years = 3.09 x 10^22 m; L02 spans
  10^24-10^26 m (tens-to-hundreds of Mpc).
- CfA2: ~500-750 Mly x ~250-300 Mly x ~15-20 Mly (z = 0.0306; 251 Mpc in
  the SDSS cut; spread is boundary- and Zone-of-Avoidance-dependent).
- Sloan: 1.37 Gly long (~433 Mpc, 1.30 x 10^25 m), redshift ~0.078,
  ~1 Gly away, 1.8-2.7x CfA2; SCl 126 core; not bound (possibly three
  chance-aligned structures).
- South Pole Wall: >1.37 Gly end to end, nearest ~0.5 Gly, Perseus to
  Telescopium 200-degree arc; peak flow signal ~12,000 km/s; largest
  contiguous local-volume feature; full extent unknown.
- BOSS Great Wall: z ~ 0.47, ~1 Gly across, walls of 186 and 173 Mpc/h,
  830+ galaxies, ~10^4 Milky-Way masses.
- Quipu (2025 ceiling): ~400-428 Mpc, ~2 x 10^17 suns, 68 clusters.
- Wall thickness: ~15 Mly galaxy ridge vs 5-8 Mpc/h dark-matter boundary
  (filaments ~2 h^-1 Mpc for scale) — definition-dependent. Density:
  ~1-5x mean. Lengths in the hundreds of Mpc; typical filaments 50-80
  Mpc for contrast.
- Mass share: MMF 5.5% vs NEXUS+ ~24% walls (same box) — always cite the
  finder; gravity share ~14% walls vs ~60% filaments (box-specific).
- Flows: Local Group 631 +/- 20 km/s vs CMB; Dipole Repeller ~220 Mpc
  away, opposite Shapley (anti-aligned); Local Void ~240 km/s of the
  motion.

Sources: pages and papers named above; ladder R5/R6; Gott et al. 2005;
Einasto et al. 2011; Pomarede et al. 2020; Lietzen et al. 2016;
Bohringer et al. 2025; Hoffman et al. 2017.

## Related

- L01 companion: [Walls](../../L01/documents/walls.md) — whole-view
  context, finder taxonomy, South Pole Wall velocity detail.
- Siblings: [filaments](./filaments.md) (threads walls drain into),
  [voids](./voids.md) (cells walls bound, Repeller context),
  [superclusters](./superclusters.md) (basins, Shapley pull, Quipu and
  South Pole Wall entries).
- Entry and timeline: [entry](./entry.md) (2MASS preview of the nearby
  wall-and-void geography), [lifetime](./lifetime.md) (dated spine),
  [previous-l1](./previous-l1.md) (L1-to-L2 key numbers).

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `walls-2df-dtfe-sloan.gif` | DTFE view of inner 2dF survey with Sloan Great Wall | Willem Schaap / 2dF Galaxy Redshift Survey | CC-BY-SA 3.0 (`https://creativecommons.org/licenses/by-sa/3.0/`) |

Note: same survey view as `L01/images/wall-sloan-2df-dtfe.gif`; kept per
level so each level folder is self-contained.

Proposed images (not downloaded — PD status unconfirmed, for a future
pass): a PD-certain SVG L02 wall cross-section (edge-on ridge vs face-on
sheet with embedded filaments and DTFE mesh hint) drawn as vector once a
public-domain source is confirmed; candidate CEA Cosmicflows-3 South Pole
Wall sky renders need a PD-or-CC source check plus a
`walls-southpole-skymap` SVG redraw before copying anything into
`../images/`.

## Sources

- Wikipedia, CfA2 Great Wall: `https://en.wikipedia.org/wiki/CfA2_Great_Wall`
- Wikipedia, Sloan Great Wall: `https://en.wikipedia.org/wiki/Sloan_Great_Wall`
- Wikipedia, South Pole Wall: `https://en.wikipedia.org/wiki/South_Pole_Wall`
- Wikipedia, BOSS Great Wall: `https://en.wikipedia.org/wiki/BOSS_Great_Wall`
- Wikipedia, Quipu (cosmic structure): `https://en.wikipedia.org/wiki/Quipu_%28cosmic_structure%29`
- Wikipedia, Galaxy filament (walls table): `https://en.wikipedia.org/wiki/Galaxy_filament`
- Wikipedia, Coma Filament: `https://en.wikipedia.org/wiki/Coma_Filament`
- UH news on the South Pole Wall: `https://www.hawaii.edu/news/2020/07/10/laniakea-supercluster-mapping/`
- Pomarede et al. 2020 (South Pole Wall, ApJ 897, 133): `https://doi.org/10.3847/1538-4357/ab9952`
- Gott et al. 2005, A Map of the Universe (ApJ 624, 463): `https://arxiv.org/abs/astro-ph/0310571`
- Einasto et al. 2011 (Sloan morphology, ApJ 736, 51): `https://arxiv.org/abs/1105.1632`
- Lietzen et al. 2016 (BOSS discovery, A&A 588, L4): `https://doi.org/10.1051/0004-6361/201628261`
- Einasto et al. 2017 (BOSS morphology, A&A 603, A5): `https://doi.org/10.1051/0004-6361/201629105`
- Bohringer et al. 2025 (Quipu, A&A 695, A59): `https://arxiv.org/abs/2501.19236`
- NASA Imagine, sheets and voids: `https://imagine.gsfc.nasa.gov/features/cosmic/sheets_voids_info.html`
- NASA APOD, Sloan Great Wall: `https://apod.nasa.gov/apod/ap071107.html`
- CfA Harvard ZCAT: `https://lweb.cfa.harvard.edu/~dfabricant/huchra/zcat/`
- Wikipedia, Delaunay tessellation field estimator: `https://en.wikipedia.org/wiki/Delaunay_tessellation_field_estimator`
- Wikipedia, Zeldovich pancake: `https://en.wikipedia.org/wiki/Zeldovich_pancake`
- Wikipedia, Dipole repeller: `https://en.wikipedia.org/wiki/Dipole_repeller`
- Hoffman et al. 2017 (Dipole Repeller, Nature Astronomy 1, 0036): `https://doi.org/10.1038/s41550-016-0036`
- Cautun et al., cosmic web evolution: `https://arxiv.org/abs/1401.7866`
- Aragon-Calvo et al. 2010 (MMF): `https://academic.oup.com/mnras/article/408/4/2163/1419651`
- Cosmic Web Dynamics 2024: `https://arxiv.org/html/2407.16489v1`
- Wikipedia, Void (astronomy): `https://en.wikipedia.org/wiki/Void_%28astronomy%29`
- Commons file page: `https://commons.wikimedia.org/wiki/File:2dfdtfe.gif`

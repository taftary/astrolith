# Superclusters — basins of attraction

A supercluster at L2 scale is not a
bound ball with a shell edge: it is a basin of attraction in the
peculiar-velocity field, galaxies inside streaming toward a common valley
while outside the divide they stream elsewhere. Laniakea, holding the Milky
Way, is the home basin. Sibling docs: `filaments.md`, `walls.md`,
`voids.md`; bridge in `previous-l1.md`, entry in `entry.md`, timeline in
`lifetime.md`.

## What they are and how they look

A supercluster at L02 scale is best defined kinematically: a basin of
attraction in the peculiar-velocity field, the volume enclosed by the
watershed surface where flow streamlines diverge, galaxies inside streaming
toward a common valley while outside the divide they stream elsewhere
(Tully et al., Nature 2014). That definition coexists with the older
density-contour usage (a high-density complex of clusters plus filament
sections above a chosen overdensity or linking length); at L02 the flow
basin is the working definition and the density complex is what is seen
inside it. For the whole-view context and the formal voxel-streamline
(Runge-Kutta) implementation, see the L01 companion
[Superclusters](../../L01/documents/superclusters.md#how-superclusters-look).
Rendered as an orange-bounded volume with white flow lines converging
inward: inside, clusters aligned along filaments drain into one large
well with the Great Attractor region as a broad flat-bottomed valley;
outside, flows head for neighboring valleys.

Target visuals (files in `images/`, credits and licenses at the bottom):

![The universe within 1 billion light-years: local superclusters among tens of millions of galaxies](../images/superclusters-local-1gly.gif)

![Slice of Laniakea in the supergalactic plane: density red (high) to blue (voids), white inward flow streams inside the orange contour, dark blue lines leaving the basin](../images/supercluster-laniakea-slice.jpg)

Sources: Tully et al. 2014 (Nature 513:71); NRAO release; Wikipedia
"Laniakea Supercluster".

## Example objects

- Laniakea (home): ~500 million light-years / ~160 Mpc across, ~100,000
  galaxies, ~10^17 solar masses — including the Local Group, Virgo,
  Hydra-Centaurus, Pavo-Indus and the Great Attractor region. A
  flow-model boundary, not a visible shell; the Milky Way sits in its
  outskirts, not its center. Cosmicflows-4 re-measures the basin at
  1.9 x 10^6 (Mpc/h)^3 around an attractor coincident with the Great
  Attractor direction, but in grouped reconstructions Laniakea merges into
  the larger Shapley basin and Coma segments inside Shapley — basin
  boundaries move with data depth, and Laniakea, Coma and neighbors all
  drift toward Shapley as possible sub-basins. Sources: NRAO release
  (Tully et al., Nature 2014); EurekAlert/UH; Wikipedia "Laniakea
  Supercluster"; Dupuy & Courtois 2023 (A&A 678, A176); Valade et al.
  2024 (Nature Astronomy 8, 1610).
- Shapley Concentration: found in the 1930s by Harlow Shapley as a galaxy
  concentration in Centaurus, center ~650 million light-years away (redshift
  ~0.046), 8,000+ galaxies, the most massive structure within ~1 billion
  light-years at more than ten million billion (10^16) solar masses; its
  collapsing core alone holds ~1.3 x 10^16 solar masses inside ~12.4 Mpc,
  with the central Abell 3558/3562 pair mapped jointly in Sunyaev-Zeldovich
  (Planck), X-ray (ROSAT) and optical light. In the 2025 CLASSIX census it
  spans ~90 Mpc with 23 member clusters but the highest cluster overdensity
  (3.8) of any nearby superstructure. Sources: ESA/Planck; Wikipedia
  "Shapley Supercluster"; Bohringer et al. 2025 (Quipu census).
- Coma Supercluster (SCl 117): redshift 0.023, ~92 Mpc (~300 million
  light-years) away in Coma Berenices — the Coma Cluster (Abell 1656) plus
  Leo (Abell 1367), 3,000+ galaxies in a roughly spherical ~98-Mly volume
  with four ~80-Mpc-long cluster chains, sitting at the center of the CfA2
  Great Wall and part of the Coma Filament; still assembling (multiple
  redshift peaks so not yet relaxed, general inward motion toward the core,
  cluster cores at ~1,000 km/s dispersion with elliptical/lenticular
  populations vs ~300 km/s spiral-rich infalling groups). Sources:
  NASA Extragalactic Database; Gregory & Thompson 1978; Wikipedia "Coma
  Supercluster".
- Quipu (reported 2025): 68 ROSAT/CLASSIX X-ray clusters in the 130-250
  Mpc shell (redshift 0.027-0.065), ~1.3-1.4 billion light-years (428 Mpc,
  over 400 Mpc) long, ~2-2.4 x 10^17 solar masses — the largest reliably
  measured nearby superstructure, named for the Inca knotted-string script
  because it reads as one long filament with side strands. One of five
  CLASSIX superstructures (with Shapley, Serpens-Corona Borealis/Hercules,
  Sculptor-Pegasus) that together hold ~45% of clusters, ~30% of galaxies
  and ~25% of matter in ~13% of the volume; streaming motions from such
  mass to 250 Mpc perturb Hubble-constant fits and imprint an ISW signal
  on the microwave background. Sources: Max Planck Society (Feb 2025);
  Bohringer et al. A&A 695, A59, 2025 (arXiv:2501.19236).
- Hyperion proto-supercluster (PSC J1001+0218): redshift 2.45 (~11 billion
  light-years light-travel, 2.3 Gyr after the Big Bang, under 20% of the
  present age), 60x60x150 Mpc, 4.8 x 10^15 solar masses (~5,000 Milky
  Ways) — assembly caught mid-build in the COSMOS field (Sextans) from
  VUDS/VIMOS mapping of 10,000+ galaxies (3,822 spectroscopic redshifts):
  at least 7 high-density peaks (Theia, Eos, Selene, Helios and kin) linked
  by filaments in loose uniform blobs, unlike the concentrated mass of
  nearby superclusters that had billions of years longer to gather. Largest
  and most massive assembly known that early (Cucciati et al. 2018);
  expected to grow into a Sloan-Wall- or Virgo-like descendant and a
  Lambda-CDM dark-matter test case. Sources: ESO (eso1833); Cucciati et
  al. A&A 619, A49; Wikipedia "Hyperion proto-supercluster".
- South Pole Wall (announced 2020): the largest contiguous local feature,
  found by velocity (Cosmicflows-3, ~18,000 distances) behind southern dust
  rather than by redshift — >=1.37 Gly (~420 Mpc, ~0.11c) end to end from
  Apus through Lepus to the Funnel, with a 250-Mpc Apus-Lepus run plus a
  170-Mpc bend and a 220-Mpc rectilinear section at ~12,000 km/s through
  Chamaeleon; density peak coincident with the celestial South Pole,
  comparable to the Sloan Great Wall at half the distance, skirting
  Laniakea's southern border opposite Shapley, its flows draining toward
  the Shapley concentration. Full extent unknown (CF3 radius only 0.05c).
  Sources: Pomarede et al. 2020 (ApJ 897, 133); Einasto 2025 review
  (Universe 11, 167).

## How they were created

Small groups fall into clusters, clusters align along web filaments into
larger wells — hierarchical assembly, with Hyperion showing it mid-build.
The modern Laniakea definition itself was created kinematically in 2014 by
Tully, Courtois, Hoffman and Pomarede from Cosmicflows peculiar velocities:
subtract Hubble expansion, keep the line-of-sight residuals gravity wrote,
trace streamlines, draw the watershed where flows divide. Later Cosmicflows
work formalized these as segmented attraction/repulsion basins on a
velocity grid — the map keeps the same valleys while the exact divides move
with better data. Sources: Big Think (bound structure); Wikipedia "Hyperion
proto-supercluster"; Tully et al. 2014 (via ADS); watershed update
(arXiv:2305.02339).

## How they change over time

Locally they are still assembling (Coma unrelaxed with multiple redshift
peaks and inward drift; Shapley core already past turnaround and
collapsing, with further major mergers forecast in forward runs).
Globally they are transient under accelerated expansion, which took over
~9 billion years in (~5 billion years ago): unlike clusters,
superclusters like Laniakea are not bound as wholes and are being pulled
apart — simulations show only the densest cores staying bound in the far
future while the whole disperses. Follow-ups therefore call velocity
basins "supercluster cocoons" or "watersheds", reserving "supercluster"
for the dense traditional systems inside; the proposed term for true
far-future survivors is superstes-clusters. For the collapse threshold
(~7.8x mean density today), turnaround redshifts and the far-future
sequence, see the L01 companion
[Superclusters](../../L01/documents/superclusters.md#how-superclusters-change-over-time)
and the dated [lifetime](./lifetime.md). Sources: Big Think; Araya-Melo
et al. 2009; Chon et al. 2015; NASA dark energy.

## How the parts interact

Motion streams along filaments toward attractor valleys and away from
voids. Inside Laniakea everything drifts toward the Great Attractor valley,
a flat-bottomed basin — yet each group keeps its own velocity, so the basin
holds as a flow, not a rigid body, and every group interacts with its
neighbors only weakly by gravity. Larger tug-of-war: the Great Attractor
region itself moves toward Shapley ~650 million light-years off, while a
Dipole Repeller void pushes from the opposite side — repeller push plus
Shapley pull accounting for roughly half each of the Milky Way's
631 km/s dipole motion. The Dipole Repeller sits ~16,000 +/- 4,500 km/s
away, anti-aligned with the CMB dipole (mu = -0.99 in Cosmicflows-3), and
a second Cold Spot Repeller lies near ~23,000 km/s toward the CMB Cold
Spot, likely a stacked-void direction. Cosmicflows-3 unifies the nearby
flow as one Shapley-extension attraction basin plus two repulsion basins.
Sources: NRAO; Wikipedia "Shapley Supercluster"; Hoffman et al. 2017
(arXiv:1702.02483); Courtois et al. 2017 Cold Spot Repeller (ApJL 847:L6);
Sky and Telescope (repeller).

## Key numbers

Catalog medians frame the named examples: 662 SDSS/WHL superclusters
(0.05 < z < 0.42, friends-of-friends on the WHL cluster catalog) have
median mass ~5.8 x 10^15 suns and median size ~65 Mpc, with density
contrast falling as extent^-2 and ~90% of each system's volume
gravitationally influential on its members; mocks match. Automated
watershed segmentation (fourth-order Runge-Kutta streamlines per voxel)
finds typical basin volumes of 10^5-10^6 (Mpc/h)^3 and typical masses
~5 x 10^16 suns/h — Laniakea auto-segmented at 2.3 x 10^6 vs 1.7 x 10^6
(manual CF2 eyeball) (Mpc/h)^3. Morphology splits spider (one central
mass, many filaments) vs filament (mass along one thread) types; richest
systems are elongated, poor ones spherical.

- Laniakea: ~500 Mly / ~160 Mpc; ~100,000 galaxies; ~10^17 suns;
  basin 1.7-2.3 x 10^6 (Mpc/h)^3 (CF2) vs 1.9 x 10^6 (CF4); dispersing.
- Shapley: center ~650 Mly (redshift ~0.046); core ~1.3 x 10^16 suns in ~12.4 Mpc;
  eRASS1 total 2.58 +/- 0.51 x 10^16 suns over ~111 Mpc, 45 members.
- Coma: ~300 Mly away (z = 0.023); ~98 Mly across; 3,000+ galaxies;
  cluster chains ~80 Mpc; cores ~1,000 km/s dispersion.
- Quipu: ~1.3–1.4 Gly (428 Mpc); ~2–2.4 x 10^17 suns; 68 clusters;
  shell z = 0.027-0.065.
- Hyperion: redshift 2.45; 60x60x150 Mpc; 4.8 x 10^15 suns; 7+ peaks.
- South Pole Wall: >=420 Mpc (Apus-Lepus ~250 Mpc + bend ~170 Mpc);
  peak at ~12,000 km/s; largest contiguous local feature.

Sources: NRAO; EurekAlert; ESA; ESO; Max Planck Society; Sankhyayan et
al. 2023 (WHL); Dupuy et al. 2019 (watersheds); papers above.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `superclusters-local-1gly.gif` | Universe within 1 Gly: local superclusters | Richard Powell, Atlas of the Universe | CC-BY-SA 2.5 (`https://creativecommons.org/licenses/by-sa/2.5/`) |
| `supercluster-laniakea-slice.jpg` | Laniakea slice: density + flow streams (1024px) | R. Brent Tully et al. / SDvision, CEA-Saclay (via NRAO press kit) | Same rendering released CC-BY 4.0 on Wikimedia Commons (`https://commons.wikimedia.org/wiki/File:LaniakeaBoundaries.jpg`); NRAO press download `https://public.nrao.edu/news/supercluster-gbt` |

Note: complements (not duplicates) `L01/images/supercluster-laniakea-context.gif`.

Proposed images (not downloaded — PD status unconfirmed, for a future
pass): a PD-certain SVG L02 basin schematic (watershed contour with inward
vs outward streamlines at 10^24–10^26 m) drawn as vector once a
public-domain source is confirmed; candidate ESO press renders for Quipu
(CLASSIX sky shell) and Hyperion (VUDS density field) each need a
PD-or-CC source check plus `supercluster-quipu-context` /
`supercluster-hyperion-field` SVG redraws before copying anything into
`../images/`.

## Sources

- NRAO, Laniakea release: `https://public.nrao.edu/news/supercluster-gbt`
- UH/EurekAlert: `https://www.eurekalert.org/news-releases/500283`
- Tully et al. 2014: `https://ui.adsabs.harvard.edu/abs/2014Natur.513...71T/abstract`
- Tully et al. arXiv: `https://arxiv.org/abs/1409.0880v1`
- Dupuy & Courtois 2023, dynamic cosmography (CF4 watersheds): `https://ar5iv.labs.arxiv.org/html/2305.02339`
- Valade et al. 2024, CF4 basins: `https://arxiv.org/abs/2409.17261`
- Dupuy et al. 2019, watershed partitioning: `https://ar5iv.labs.arxiv.org/html/1907.06555`
- Einasto 2025 review (Universe 11, 167): `https://arxiv.org/abs/2505.22082v1`
- Chon et al. 2015, supercluster definitions: `https://arxiv.org/abs/1502.04584`
- Sankhyayan et al. 2023, SDSS WHL catalog (662 systems): `https://arxiv.org/abs/2309.06251`
- Wikipedia, Laniakea: `https://en.wikipedia.org/wiki/Laniakea_Supercluster`
- ESA, Shapley: `https://sci.esa.int/web/planck/-/57952-shapley-supercluster`
- Wikipedia, Shapley: `https://en.wikipedia.org/wiki/Shapley_Supercluster`
- Wikipedia, Coma: `https://en.wikipedia.org/wiki/Coma_Supercluster`
- Gregory & Thompson 1978, Coma/A1367: `https://ui.adsabs.harvard.edu/abs/1978ApJ...222..784G/abstract`
- Max Planck, Quipu: `https://www.mpg.de/24197951/largest-superstructure-in-the-nearby-universe-unveiled`
- Quipu paper (Bohringer et al. A&A 695, A59, 2025): `https://arxiv.org/abs/2501.19236`
- ESO, Hyperion: `https://www.eso.org/public/news/eso1833`
- Hyperion paper (Cucciati et al. A&A 619, A49): `https://arxiv.org/abs/1806.06073`
- Wikipedia, Hyperion: `https://en.wikipedia.org/wiki/Hyperion_proto-supercluster`
- South Pole Wall (Pomarede et al. 2020, ApJ 897, 133): `https://doi.org/10.3847/1538-4357/ab9952`
- Dipole Repeller: `https://arxiv.org/abs/1702.02483`
- Cold Spot Repeller (Courtois et al. 2017, ApJL 847, L6): `https://doi.org/10.3847/2041-8213/aa88b2`
- NASA, dark energy: `https://science.nasa.gov/dark-energy`
- Commons file page: `https://commons.wikimedia.org/wiki/File:Superclusters_atlasoftheuniverse.gif`

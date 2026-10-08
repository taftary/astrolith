# Filaments

Part doc for Issue #211 (loop iteration 2). Filaments are the thread-like
bridges of the cosmic web: dense rivers of dark matter, gas and galaxies
joining cluster nodes across tens of megaparsecs. Template follows the pilot
(`cosmic-web.md`): looks, creation, change over time, interactions, numbers,
examples, target visuals, sources.

## How filaments look

In redshift surveys filaments appear as thread-like galaxy bridges tens of
megaparsecs long and only 1-2 Mpc thick, forming a network around voids;
statistical filamentarity in SDSS holds to ~80 Mpc/h, longer impressions are
chance alignments. Sources: SDSS DR4 filament study; Bharadwaj et al.

The sharpest observed gas bridge is a 3-million-light-year purple Lyman-alpha
filament linking two quasar-host galaxies 11 billion light-years away (150 h
VLT/MUSE overlaid on Hubble): gas clumped by dark-matter gravity, ~85% of the
matter invisible. A JWST strand at ~830 Myr after the Big Bang shows a
3-million-light-year line of 10 galaxies anchored by a quasar, expected to
become a massive cluster. Sources: ESO potw2504a; STScI news-2023-124.

![Observed Lyman-alpha bridge: two quasar galaxies linked by glowing filament gas, 3 million light-years long](../images/filament-bridge-eso-potw2504a.jpg)

In simulations filaments are dark-matter and gas threads at once: short ones
near nodes, long ones in sparse regions. In the IllustrisTNG rendering
convention brightness is projected baryonic mass density and hue is mean gas
temperature; voids render near-black, filaments yellow-green, halos white,
with hydrodynamical shocks lighting filament boundaries. Sources:
IllustrisTNG media; MIT News gallery.

![Millennium simulation: dark-matter threads zooming into a cluster node](../images/millennium-cosmic-web.jpg)

## How filaments are created

Filaments are born from anisotropic collapse: the tidal field amplifies seed
asymmetries, first one axis collapses into a sheet, then the second axis into
a filament, finally all three into a node. In eigenvalue language a filament
is (+,+,-): collapse on two axes, expansion on the third. Large-scale tidal
fields and density peaks fix the bridges between peaks, confirmed in
essentially all Lambda-CDM N-body runs. Filaments form at wall intersections;
clusters form where filaments intersect, fed by inflow along the spines.
Sources: Cautun et al. (MNRAS 441); "How filaments are woven" review;
caustic-skeleton work (A3 cusps).

## How filaments change over time

The early web is gauze: at redshift 5, ~33-45% of mass sits in sheets, only
~18-30% in filaments and ~2-3% in clusters. Sheets drain into filaments, so
by today more than half the mass is filamentary: the filament share rises
from ~30% at z=2.1 to ~40% at z=0 while sheets shrink. High-mass segments
(mass per length peaking near 10^12 solar masses per Mpc) grow more common
late; typical segment widths grow from ~0.3 Mpc at z=2 to ~1.0-1.5 Mpc at z=0.
Most prominent present filaments connect to massive clusters; most high-z
filaments connect to none. Sources: Zhu and Feng; Cautun et al.
understanding review; Yang et al. widths; MTNG evolution.

Gas heats as filaments fatten: the warm-hot intergalactic medium (WHIM,
10^5-10^7 K) is ~40% of gas at z=0 but only ~10% at z=2.1, and about half of
all WHIM mass lives in filaments at every redshift — filaments are the
missing-baryon reservoir. Sources: Large-Scale Environment II; Martizzi et
al. (IllustrisTNG baryons).

Caveat: observed width evolution (grow vs shrink with time) depends on
estimator, sample and smoothing; dark-matter-only widths (~0.1-0.6 Mpc) are
far smaller than galaxy scale radii (~2-5 Mpc). Always cite the method.
Source: Galaxy Transformation Across the Web.

## How filaments interact with the rest of the web

**Feeding nodes.** Warm gas funnels along filaments into clusters from beyond
~4 virial radii; inflow is anisotropic, aligning cluster major axes with
their filaments. Abell 2744 shows ~8-Mpc hot-gas filaments feeding its core
(~4 x 10^13 solar masses of filament gas) in a node merging 4+ components.
Sources: cosmic gas accretion (IllustrisTNG); PMC baryon content of the web.

**Spinning galaxies up.** Low-mass blue galaxies align their spins with
filaments; high-mass red ones go perpendicular, flipping near 3 x 10^10
solar masses in stars. A 50-Mpc filament at 140 Mly shows coherent spin plus
bulk rotation — gas-rich, dynamically cold, an early-stage structure.
Sources: Horizon-AGN summary; Tempel and Libeskind; Oxford spinning filament.

**Feeding star formation.** Filaments host more star-forming gas than knots;
cold streams feed galaxies from the environment. Near Coma, galaxies get
bluer toward filament spines within ~1 Mpc; the zone of influence reaches
~10 Mpc. Late on, galaxies quench near nodes and filaments as gas supply
fades. Sources: Martizzi et al.; Coma UV catalogue; slime-mold filament work.

**Dominating gravity.** In one 300 Mpc/h-box inventory, filament-induced
forces average ~60% of local gravity and are the largest share in ~97% of
volume (voids ~20%, nodes ~17%, walls ~14%) — shares are method- and
box-specific, the ranking is the point. Filaments rule inside filaments, most
void interiors and all walls; nodes dominate only their immediate
neighborhood. Sources: Cosmic Web Dynamics 2024.

**Detected directly.** Stacking 24,544 SDSS filaments gives 4.4-sigma thermal
Sunyaev-Zel'dovich plus 8.1-sigma CMB-lensing detections; a single 7.2-Mpc
Shapley filament shows 6.1-sigma X-ray WHIM at ~10 million K. Sources:
Tanimura et al.; de Graaff et al.; Migkas et al.

## Key numbers

- Lengths: common 50-80 Mpc; significant to ~80 Mpc/h (SDSS 2D), ~130 Mpc
  (3D); long straight branches exceed 100 Mpc/h.
- Widths: 1-2 Mpc classic; ~1.0-1.5 Mpc at z=0, ~0.3 Mpc at z=2 (TNG
  segments); galaxy scale radii 2-6 Mpc depending on sample.
- Mass share: ~39-50% of cosmic mass in ~6-14% of volume (finder-dependent);
  linear density ~10^12 solar masses per Mpc.
- Connectivity: ~2 filaments at 10^14 solar-mass halos, ~5 above 10^15.

Sources: Coil review; ELUCID statistics; MMF phenomenology; Cosmic Web
Dynamics inventory.

## Example objects

- Sloan Great Wall: 433 Mpc at z=0.078, richest core SCl 126 with tendrils.
- Quipu: >400 Mpc, ~2.4 x 10^17 solar masses, 68 clusters — largest confirmed
  structure (2025).
- CfA2 Great Wall: 251 Mpc long, Coma at its core; Coma supercluster spans
  ~500 deg^2 with Abell 1367 forming at a filament intersection.
- Pisces-Cetus: ~1-billion-light-year filament complex, Abell 85/133 + ~20
  clusters.
- Perseus-Pisces: ~300-million-light-year wall anchored by Perseus A426.
- Hercules-Corona Borealis (~3 Gpc): unconfirmed; would dwarf Quipu if real.

Sources: Wikipedia "Galaxy filament"; Quipu discovery (A&A 2025); SGW
morphology; Coma UV catalogue.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `filament-bridge-eso-potw2504a.jpg` | Lyman-alpha bridge between two quasars | ESO/D. Tornotti et al./Hubble: M. Revalski, P. Francis et al. | CC-BY 4.0 (ESO public images reproducible with credit) |
| `millennium-cosmic-web.jpg` | Millennium dark-matter threads + cluster (shared with pilot) | Volker Springel / Max Planck Institute for Astrophysics | CC-BY-SA 4.0 |

## Sources

- SDSS DR4 filament study: `https://arxiv.org/pdf/astro-ph/0601179v3`
- ESO potw2504a: `https://www.eso.org/public/images/potw2504a`
- ESO copyright: `https://www.eso.org/public/copyright/`
- STScI JWST strand: `https://www.stsci.edu/contents/news-releases/2023/news-2023-124`
- IllustrisTNG media: `https://www.tng-project.org/media`
- Cautun et al., evolution: `https://academic.oup.com/mnras/article/441/4/2923/1213214`
- Cautun et al., understanding: `https://arxiv.org/html/1501.01306v1`
- Zhu and Feng: `https://iopscience.iop.org/article/10.3847/1538-4357/aa61f9`
- Martizzi et al., baryons: `https://academic.oup.com/mnras/article/486/3/3766/5475663`
- Cosmic Web Dynamics 2024: `https://arxiv.org/html/2407.16489v1`
- Tanimura et al. SZ+lensing: `https://arxiv.org/abs/1911.09706`
- de Graaff et al.: `https://inspirehep.net/literature/1627851`
- Migkas et al. Shapley WHIM: `https://arxiv.org/abs/2506.14917`
- Filament spin (Nature Astronomy): `https://www.nature.com/articles/s41550-021-01380-6`
- Oxford spinning filament: `https://www.physics.ox.ac.uk/news/astronomers-spot-one-largest-spinning-structures-ever-found-universe`
- Coma UV catalogue: `https://arxiv.org/pdf/1805.09029`
- Quipu discovery: `https://www.aanda.org/articles/aa/abs/2025/03/aa53582-24/aa53582-24.html`
- ELUCID statistics: `https://academic.oup.com/mnras/article/533/1/1048/7729281`
- NASA slime-mold web: `https://science.nasa.gov/image-detail/amf-f593e7a5-9fc5-4ed8-b02f-673122a4c410`

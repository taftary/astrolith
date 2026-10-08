# ICM — the sea between galaxies

The intracluster medium is L3's
hidden majority: a superheated plasma filling the space between cluster
galaxies, outweighing all the stars several times over, visible only in
X-rays and through its shadow on the Big Bang's afterglow. It holds
most of the cluster's ordinary (baryonic) matter — stars and galaxies
are only a few percent of the total mass. Sibling docs:
`clusters.md`, `groups.md`, `members.md`; bridge in `previous-l2.md`,
entry in `entry.md`, timeline in `lifetime.md`.

## What it is and how it looks

To X-ray eyes a cluster is not galaxies at all — it is a glowing ball of
gas. The ICM is mainly ionized hydrogen and helium at 10–100 megakelvin,
so thin (~10^-3 atoms per cm^3) yet so vast that it holds ~90% of the
cluster's baryons (galaxies hold ~10%) while running only ~5–15% of the
total mass — the rest is dark matter (same convention as
`clusters.md`: ~85–90% dark matter, ~5–15% gas, ~1–2% stars; values vary
per cluster). Free electrons spiraling past ions
radiate thermal bremsstrahlung with an exponential cutoff at hν≈kT,
giving cluster X-ray luminosities of 10^43–10^45 erg/s; temperature,
density, and metallicity come from that continuum plus heavy-element
emission lines (notably iron Kα at 6.7 keV). What you see in
`entry.md`'s Perseus portrait — loops, ripples, plumes — is this sea,
lit by its own heat. The plasma is effectively collisionless by
laboratory standards: particle mean free paths reach ~10^16 m (about a
light-year), yet the sheer volume makes it glow. Toward the cluster
center the density rises to a strong peak while the temperature
typically falls to one-half to one-third of the outer value — the
cool-core signature that sets up the feedback cycle below. Chandra's
field guide puts the same gas at 30–100 million °C, heated violently as
group-scale atmospheres collide and merge over billions of years.

Target visual (files in `images/`, credits and licenses at the bottom):

![Chandra composite of the Bullet Cluster: stripped X-ray gas trailing the lensing mass of two colliding clusters](../images/bullet-chandra.jpg)

Note: the same Bullet figure leads `clusters.md`; here it reads as gas
physics — ram-pressure stripping at cluster scale, the ICM lagging the
dark matter it outweighs in light but not in mass.

Sources: Wikipedia "Intracluster medium"; Sarazin review
(Rev. Mod. Phys. 58.1, via open scholar copy); Trieste ICM X-ray notes.

## How it was created

The gas fell in with everything else and never got to leave. As
protocluster halos collapsed, infalling gas shocked to virial
temperatures of tens of millions of kelvin and stayed there — too hot to
cool quickly, too deep in the dark-matter well to escape. Supernovae in
the member galaxies salted it early: the bulk of the metals went in
around the peak of cosmic star formation (redshift ~2), was already mixed
by redshift 2–3, and shows no big change since redshift 1. Today's
typical massive-cluster metallicity is ~1/3 solar, a blend of
core-collapse plus Type Ia supernovae (12–37% from Ia) — read straight
off the X-ray lines. Averaged over radius the range runs about
one-third to one-half solar; cluster cores are more metal-rich than the
outskirts, and some systems (Centaurus) reach super-solar central
metallicity. Because the cluster's gravity holds supernova ejecta
bound, the ICM keeps a historical record of element production:
measuring metallicity against redshift reads the enrichment history of
the universe's star formation. Sources: ApJ 826:124 (McDonald et al., SPT-SZ
metallicity evolution); ApJ Lett. 811:L25 (outskirts enrichment);
Mantz et al. 2017, MNRAS 472:2877 (metallicity over cosmic time, early
enrichment); Sanders et al. 2016, MNRAS 457:82 (Centaurus deep metals,
sloshing, feedback).

## How it changes over time

The ICM settles into two personalities. Relaxed clusters grow cool cores:
dense centers that should cool and form stars furiously — but mostly
don't, because the central black hole fights back. Cooling gas rains
onto the black hole (cold precipitation when cooling time drops to ~10x
the free-fall time), jets fire, bubbles and shockwaves reheat the inner
~100 kpc, and the cycle repeats. Perseus is the textbook case: buoyant
bubbles (not shocks) carry ~half the injected energy as enthalpy, its
pressure waves ripple outward as sound 57–58 octaves below middle C.
Disturbed clusters — fresh mergers like the Bullet — have their cores
shredded and homogenized instead; Coma (~100 million °C, no cool core)
vs Perseus (cool core, suppressed star formation) is the standing
contrast. Two heaters share the work: the central active nucleus
injecting relativistic jets (visible as X-ray-dark radio lobes carved
into the Perseus glow), and sloshing of the plasma during mergers with
subclusters stirring the core from the outside. In some systems the
inflow also appears deflected by magnetic fields. Early X-ray data
implied hundreds of solar masses per year should be cooling and forming
stars in cluster centers; searches found some cool gas but nowhere near
that much — the missing-cooling problem the feedback cycle resolves.
Sources: ApJ 811:108 (precipitation/feedback cycle);
OSTI 1256639 (Perseus energetics); MIT News 2015 (Coma vs Perseus);
NASA black-hole sonifications.

## How it interacts with the others

The ICM is L3's weather-maker and its scale. As a headwind it strips
infalling spirals into jellyfish (`members.md`); as a weight it betrays
the dark matter — hydrostatic temperature/density profiles demand far
more mass than the luminous gas, the original cluster dark-matter
evidence. As a backlight trick it finds clusters across cosmic time: hot
electrons upscatter Big Bang afterglow photons (Sunyaev-Zel'dovich
effect, decrement below 217 GHz, increment above — the thermal SZ
signature; the far weaker kinematic SZ from bulk motion along the line
of sight separately traces peculiar velocities, first detected
statistically by ACT in 2012) with a surface brightness nearly
independent of redshift, yielding mass-limited catalogs — Planck's PSZ2
(1,653 detections, 1,203 confirmed, the deepest all-sky SZ sample;
selection-function purity above 83%, with a population of low-redshift
X-ray under-luminous clusters that ROSAT-class surveys nearly missed)
and
ACT DR6 (~10,000 confirmed, 1,000+ beyond redshift 1, the largest SZ
catalog to date). SZ selection sees integrated pressure rather than
density-squared emission, which is why it stays nearly distance-blind
where X-ray glow fades. And groups show the threshold: only about half of
nearby groups manage even a faint central X-ray glow — the deep hot sea
is what makes a cluster a cluster. Sources: SZ review (EPJ Conf. 2026);
HEASARC Planck PSZ2; Wikipedia "Galaxy group".

## How it is mapped

Three surveys read the same sea three ways. X-ray telescopes (Chandra,
XMM-Newton) image the glow directly — temperature, density, metals from
lines — but fade with distance; eROSITA's first all-sky survey
(eRASS1, DR1 January 2024) now supplies the wide X-ray counterpart
catalog (~930,000 sources in six months; 12,247 optically confirmed
groups and clusters, the faint-group end ROSAT barely saw — canonical
counts live in `clusters.md`, not repeated here). SZ surveys (Planck, ACT, SPT) see the
shadow instead, nearly distance-blind, building mass-limited catalogs
across cosmic time that double as growth-of-structure probes for
cosmology. High-resolution SZ (ALMA, MUSTANG/Bolocam, SPT) adds the
kinematic channel: bulk velocities of individual mergers such as
MACS J0717.5+3745. And the metals are archaeology: uniform outskirts enrichment
(Virgo and Perseus outskirts solar-like to 1.3 virial radii, ruling
out core-collapse-only enrichment past 6 sigma) dates the salting to the
z~2–3 star-formation peak. Even sound joins in: Perseus pressure waves
sonified 57–58 octaves below middle C. Sources: ApJ Lett. 811:L25;
EPJ Conf. 2026 (SZ cosmology); NASA sonifications.

## What the new X-ray spectrometers changed (2024–2026)

For decades the feedback-and-sloshing picture above was inferred from
images; XRISM's Resolve microcalorimeter (about 30x the energy
resolution of conventional CCD instruments) made the gas motions
themselves measurable through iron-line Doppler shifts — picking up
where the Hitomi mission's brief 2016 Perseus turbulence measurement
left off. Two Nature results anchor the new dynamics:

- Perseus (XRISM Collaboration 2026): velocity dispersion traces a
  V-shape with radius — ~200 km/s near the center (about 35% of the
  local sound speed, driven by the NGC 1275 black hole, itself ~200x
  the Milky Way's central mass), dipping to ~80 km/s, then rising
  again to ~200 km/s in the outskirts where dark-matter-driven
  accretion flows take over. Black-hole stirring and cosmological
  inflow disentangled in one profile, mapped out to ~800,000
  light-years.
- Centaurus (XRISM Collaboration 2025): hot gas in the core streams
  toward us at 130–310 km/s in an oscillation pattern — first direct
  velocity evidence of merger-driven sloshing stirring the core and
  holding off runaway cooling, matching numerical merger simulations.

The implication for this document's feedback section: sloshing is no
longer a hypothesis alongside jets but a measured motion, and the two
drivers dominate at different radii. Sources: XRISM Collaboration 2026,
Nature (Perseus kinematics); XRISM Collaboration 2025, Nature
(Centaurus bulk motion); JAXA/ISAS press releases 2025-02-13 and
2026-02-20.

## Magnetism, radio halos, and stray starlight

The ICM is magnetized, and that matters for both cooling and mapping.
Faraday rotation measures toward embedded and
background radio galaxies give a few microgauss typically, ~10 μG in
places, varying on 100-pc to 10-kpc scales with a power spectrum near
the Kolmogorov slope in some clusters and shallower in others. Where
mergers stir the sea, the turbulence lights up Mpc-scale radio halos
and peripheral relics in cluster centers — the same disturbed systems
whose X-ray cores look ragged and homogenized. Simulations
(TNG-Cluster, 352 systems, 2024) add a redshift trend: lower-mass,
higher-redshift clusters are the most cooling-susceptible, their
progenitors cooler and richer in cool-phase gas, with central-black-hole
kinetic energy the dominant regulator. And JWST opened a side window:
the faint intracluster light of orphan stars drifting between galaxies
traces the dark matter closely enough to serve as a luminous mass map.
Sources: Govoni 2015 (ICM magnetic fields review); Rohr et al. 2024,
MNRAS 536:1226 (TNG-Cluster cooler past); Montes & Trujillo 2018,
MNRAS 482:2838 (intracluster light as dark-matter tracer).

## Key numbers

- Temperature: 10^7–10^8 K (10–100 megakelvin); density ~10^-3 atoms/cm^3.
- Mass split (convention shared with `clusters.md`, varies per cluster): ~85–90% dark matter; ~5–15% ICM; ~1–2% stars and galaxies.
- X-ray power: 10^43–10^45 erg/s; iron line at 6.7 keV; metallicity ~1/3 solar.
- Virgo total: ~1.2 x 10^15 extended bound mass (out to ~2.2 Mpc; Fouqué et al. 2001); X-ray virial (r200) totals run ~1–4 x 10^14 (Urban et al. 2011; Simionescu et al. 2017; Boselli et al. 2018) — the ICM is the ~10%-scale slice per the split above.
- SZ catalogs: Planck PSZ2 1,203 confirmed; ACT DR6 ~10,000 confirmed.
- Perseus feedback: bubbles carry ~half the energy; sound 57–58 octaves below middle C.
- XRISM dynamics: Perseus dispersion V-shape 200 → 80 → 200 km/s to ~800,000 ly; Centaurus core slosh 130–310 km/s.
- Plasma scale: mean free path ~10^16 m (~1 ly); center T falls to 1/2–1/3 of outer value.
- Magnetism: typically a few μG, up to ~10 μG; radio halos mark merger-stirred systems.

Sources: pages named above; Mpc conversions standard.

## Image credits and licenses

| File | Shows | Credit (required) | License / terms |
|---|---|---|---|
| `bullet-chandra.jpg` | Bullet Cluster composite, gas vs lensing mass (720px) | X-ray: NASA/CXC/CfA/M.Markevitch et al.; Optical: NASA/STScI; Magellan/U.Arizona/D.Clowe et al.; Lensing: NASA/STScI; ESO WFI; Magellan/D.Clowe et al. | Chandra/SAO, free with acknowledgement (`https://chandra.harvard.edu/photo/2006/1e0657/`) |

Note: complements (not duplicates) `entry.md`'s Perseus core portrait.

## Sources

- Wikipedia, Intracluster medium: `https://en.wikipedia.org/wiki/Intracluster_medium`
- Wikipedia, Bullet Cluster: `https://en.wikipedia.org/wiki/Bullet_Cluster`
- Chandra, Bullet lithos: `https://www.chandra.harvard.edu/graphics/resources/handouts/lithos/bullet_lithos.pdf`
- HEASARC, Planck PSZ2: `https://heasarc.gsfc.nasa.gov/w3browse/all/plancksz2.html`
- NASA, black-hole sonifications: `https://www.nasa.gov/universe/new-nasa-black-hole-sonifications-with-a-remix`
- MIT News, feedback contrast: `https://news.mit.edu/2015/galaxies-regulate-stars-0304`
- Fouqué et al. 2001, Virgo Tolman-Bondi mass 1.2 x 10^15 out to ~2.2 Mpc (A&A 375:770): `https://doi.org/10.1051/0004-6361:20010833`
- Urban et al. 2011, Virgo X-ray spectroscopy out to the virial radius (M_vir ~1.4e14): `https://arxiv.org/abs/1102.2430`
- Simionescu et al. 2017, Virgo outskirts Suzaku Key Project (M200 = 1.05 x 10^14, r200 = 974 kpc): `https://doi.org/10.1093/mnras/stx919`
- Boselli et al. 2018, VESTIGE Virgo substructure compilation (Cluster A M200 1.4–4.2 x 10^14): `https://www.aanda.org/articles/aa/full_html/2018/06/aa32407-17/aa32407-17.html`
- Chandra field guide, groups and clusters of galaxies (ICM heating, cooling flows, magnetic deflection, SMBH heating): `https://chandra.harvard.edu/xray_sources/galaxy_clusters.html`
- Chandra, Bullet Cluster photo album (gas vs lensing mass, drag on gas, dark matter required): `https://chandra.harvard.edu/photo/2006/1e0657/`
- XRISM Collaboration 2025, Centaurus bulk motion (Nature 10.1038/s41586-024-08561-z; JAXA press 2025-02-13): `https://global.jaxa.jp/press/2025/02/20250213-1_e.html`
- XRISM Collaboration 2026, Perseus kinematics (Nature 10.1038/s41586-025-10017-x; ISAS topic 2026-02-20): `https://www.isas.jaxa.jp/en/topics/004194.html`
- Mantz et al. 2017, ICM metallicity over cosmic time (MNRAS 472:2877): `https://doi.org/10.1093/mnras/stx2200`
- Sanders et al. 2016, Centaurus deep metals/sloshing/feedback (MNRAS 457:82): `https://doi.org/10.1093/mnras/stv2972`
- Rohr et al. 2024, cooler past of the ICM in TNG-Cluster (MNRAS 536:1226): `https://doi.org/10.1093/mnras/stae2536`
- Govoni 2015, ICM magnetic fields (IAU H16:404): `https://doi.org/10.1017/S1743921314011739`
- Montes & Trujillo 2018, intracluster light as dark-matter tracer (MNRAS 482:2838): `https://academic.oup.com/mnras/article/482/2/2838/5142870`
- eROSITA-DE DR1 (eRASS1, released 2024-01-31): `https://erosita.mpe.mpg.de/dr1/index.html`
- Sibling docs: `./clusters.md`, `./groups.md`, `./members.md`, `./entry.md`, `./previous-l2.md`, `./lifetime.md`

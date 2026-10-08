# Universe

Scope: visual and lifetime reference for the 14 universe scale levels
(L01–L14, observable universe to room). Each level has its own subtopic
with pages and reference images. This topic is maintained through the
normal guides process (`research`, `document`, `refresh`).

Overview: the scale ladder runs from 10^26.94 m to 10^0.5 m across 14
named levels — a factor of ~10^26.4 from the observable universe
(~93 billion light-years across, 8.8 x 10^26 m) down to a room a few
metres across. The universe is 13.787 +/- 0.020 billion years old
(Planck 2018); every level carries a dated lifetime page running from
its formation to the present plus its far-future fate. The frozen level
table and anchors live in
[ladder.md](../../../docs/universes/ladder.md); toolchain pins and Bevy features live
in [stack.md](../../../docs/universes/stack.md). This topic holds the descriptive
reference (what each level looks like, how it was created, how it changed
through time, how its parts interact); the canonical rules stay in `docs/`.

Key numbers used across levels: CMB temperature 2.72548 +/- 0.00057 K;
energy contents 4.9% ordinary matter, 26.8% dark matter, 68.3% dark
energy (Planck 2018); up to ~2 trillion galaxies and ~10^24 stars in
the observable volume (see the L01 lifetime page for the dated
timeline these counts hang on).

## How to read this topic

The application is a dive: start outside the observable universe and
move toward ever smaller things. Each level is a cell whose markers
are the next level's cells — fly into a galaxy marker and you are
inside that galaxy's structures. Every level subtopic opens with the
bridge from the previous level and the entry transition, then its
parts, then the dated lifetime that runs through everything.

The MVP dive runs L01–L10 (the autopilot journey ends at the planet);
L11–L14 continue the same ladder past the journey as the beyond-MVP
tail (many regions per planet, many cities per region, many buildings
per city, many rooms per building, then nothing below).

## Realism review

- [realism-review.md](realism-review.md): panel findings, per-level realism
  targets, proposed ladder, sources. Shape-checked by
  `scripts/validation/review_lint.py`. Numbering inside follows the
  Issue #143 proposal (eleven levels L1–L11); see the note at the top
  of that document for the mapping to the canonical R8 ladder.

## Levels

Each rung links to its subtopic README (bridge, parts, lifetime,
target visuals). Scales are the frozen ladder ranges; the Sloan Great
Wall anchor for L2 (1.37 Gly = 1.30 x 10^25 m, Gott et al. 2005) and
the 93 Gly / 8.8 x 10^26 m L1 diameter are confirmed against current
published values.

| Level | Name | Scale | What lives here |
|---|---|---|---|
| [L01](../L01/documents/README.md) | Observable universe | ~10^27 m | Whole cosmic web, CMB setting, dated timeline |
| [L02](../L02/documents/README.md) | Cosmic web | 10^24–10^26 m | Filaments, walls, giant voids, superclusters |
| [L03](../L03/documents/README.md) | Galaxy clusters and groups | ~10^23 m | Rich clusters, poor groups, intracluster medium |
| [L04](../L04/documents/README.md) | Galaxies | 10^20–10^21 m | Spirals, ellipticals, dwarfs, nuclei, halos |
| [L05](../L05/documents/README.md) | Galactic structures | 10^17–10^19 m | Spiral arms, molecular clouds, star clusters |
| [L06](../L06/documents/README.md) | Stellar neighborhood | 10^16–10^17 m | Nearby stars, multiples, dwarfs, local medium |
| [L07](../L07/documents/README.md) | Outer solar system | 10^15–10^16 m | Giant-planet zone, Kuiper Belt, Oort cloud |
| [L08](../L08/documents/README.md) | Planetary system | 10^12–10^13 m | Sun, planets, belts, heliosphere |
| [L09](../L09/documents/README.md) | Stars | ~10^9 m | Stellar interior, surface, corona, activity |
| [L10](../L10/documents/README.md) | Planets and moons | 10^6–10^8 m | Interior, surface, atmosphere, moons, shield |
| [L11](../L11/documents/README.md) | Regions of a planet | 10^5–10^6 m | Continents, mountains, plains, oceans, rivers |
| [L12](../L12/documents/README.md) | Cities and landscapes | 10^3–10^5 m | Cities, towns, networks, farmland, landscapes |
| [L13](../L13/documents/README.md) | Buildings | 10^1–10^2 m | Houses, blocks, commercial, civic, stadiums |
| [L14](../L14/documents/README.md) | Room | 10^0–10^1 m | Shell, furniture, storage, light, layout |

## Catalogs

Kinds of things, across levels — each type tagged observed,
candidate, or theoretical. Levels describe places; catalogs
describe populations.

| Catalog | Holds |
|---|---|
| [Stars](../catalogs/documents/stars.md) | Protostars, main sequence O–M, brown dwarfs, giants, massive stars, variables, white dwarfs, neutron stars, black holes, black dwarfs (theoretical) |
| [Planets](../catalogs/documents/planets.md) | Rocky worlds, super-Earths, ocean worlds, giants, rogues, circumbinaries, dwarf planets, moon types |
| [Rocks](../catalogs/documents/rocks.md) | Igneous, sedimentary, metamorphic, meteorites, asteroid classes, ices, dust and minerals, building stone |
| [Elements](../catalogs/documents/elements.md) | Big Bang, stellar, supernova, and merger forges; abundances; gold, oxygen, carbon portraits; per-level spots |

## Survey context

The cosmic-web levels (L01–L02) rest on published survey maps (SDSS,
2dF, Planck). Readers should know the map is still growing: DESI
completed its planned five-year survey in April 2026 with spectra for
more than 47 million galaxies and quasars (plus 20 million stars),
and its 2025 DR2 results strengthen hints that dark energy may evolve
— context for the L01 background and L02 lifetime pages, not a change
to the frozen ladder. ESA's Euclid (launched July 2023, first data
release March 2025 with 26 million galaxies) is mapping billions of
galaxies out to 10 billion light-years over more than a third of the
sky — the next source of L01–L02 map updates.

## Important pictures

One anchor visual per level — each file lives in its level's `images/`
folder with source, credit, and license recorded in that level's docs.

| Level | Image | Subject |
|---|---|---|
| L01 | [Planck 2013 all-sky map](../L01/images/planck-cmb-2013.jpg) | Cosmic microwave background |
| L02 | [Eight maps of Earth location](../L02/images/earths-location-8maps.jpg) | Our place in the cosmic web |
| L03 | [Virgo wide field](../L03/images/virgo-wide-eso0919c.jpg) | Home galaxy cluster |
| L04 | [Whirlpool galaxy](../L04/images/whirlpool-hst.jpg) | Face-on spiral archetype |
| L05 | [Eagle Nebula](../L05/images/eagle-nebula-eso0926a.jpg) | Star-forming cloud pillars |
| L06 | [Alpha Centauri wide field](../L06/images/alpha-centauri-wide-eso1629i.jpg) | Nearest star system in context |
| L07 | [Pluto heart](../L07/images/pluto-heart-new-horizons.jpg) | Kuiper Belt world close-up |
| L08 | [Solar system illustration](../L08/images/solar-system-illustration-nasa.jpg) | Full planetary system layout |
| L09 | [Sun close-up](../L09/images/sun-closeup-sdo.jpg) | Active stellar surface |
| L10 | [Earth full disk](../L10/images/earth-fulldisk-nasa.jpg) | Journey's-end planet |
| L11 | [Blue marble](../L11/images/earth-bluemarble-nasa.jpg) | Planetary regions from orbit |
| L12 | [City lights](../L12/images/city-lights-nasa.jpg) | Human settlement at night |
| L13 | [Manhattan blocks](../L13/images/blocks-manhattan-nasa.jpg) | Urban blocks from above |
| L14 | [Destiny lab interior](../L14/images/destiny-lab-interior-nasa.jpg) | Inhabited room, 1 m end point |

## Related topics

- Scale ladder (frozen): [ladder.md](../../../docs/universes/ladder.md)
- Stack, pins, budgets: [stack.md](../../../docs/universes/stack.md)
- Architecture codemap: [ARCHITECTURE.md](../../../docs/ARCHITECTURE.md)

Naming: level folders use two digits (`L01`–`L14`); ladder rows use
`L1`–`L14`. Both refer to the same rungs.

## Documentation status

Complete: all 14 level subtopics follow the same shape — bridge from
the previous level, entry transition, part pages, dated lifetime page,
and target visuals with credited sources in `images/`. L01 splits the
top differently (whole-view `cosmic-web.md` plus six part pages) because
it covers the entire observable volume. This
index stays navigation-only; numbers, physics, and history live in the
level pages, and the frozen rules live in `docs/`.

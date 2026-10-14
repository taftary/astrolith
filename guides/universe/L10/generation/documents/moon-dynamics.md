# Moon dynamics — origins, tides, resonances and rings

The [moons](./moons.md) page sets how many moons a planet gets, where
they orbit and what each type looks like. This page adds the **why**: how
moons form, how tides heat, lock and move them, how resonances chain them,
how co-orbital moons share an orbit, and how rings appear and fade. Each
section ends with a generation rule, so a moon system is consistent with
its own history.

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## How moons form

| Origin | What happens | Result to generate |
|---|---|---|
| Giant impact | a large body hits a young planet; debris forms a moon | one big moon for a rocky planet (the Moon), close at first, then receding |
| Disk (co-accretion) | moons grow in the gas and dust disk around a forming giant | regular moons: circular, prograde, equatorial, a mass budget near `10^-4` of the planet [GEN-S020] |
| Capture | a passing body is caught, often from a binary pair | irregular moons: eccentric, inclined, often retrograde (see [moons](./moons.md#where-moons-can-orbit)) |
| Breakup | a moon torn inside the Roche limit, or shattered by an impact | rings, small shepherds, co-orbital pairs |

- **Fact:** the giant-impact model is the leading account of Earth's
  Moon; it explains the system's angular momentum, while a capture of so
  large a moon is unlikely [GEN-S050].
- **Fact:** Triton is thought to have been captured when it passed too
  close to Neptune (Agnor and Hamilton 2006) [GEN-S050].
- **Recommendation:** draw the origin first, then the orbit and type
  from it: impact moons for some rocky planets, disk systems for giants,
  captured irregulars for everyone, and a breakup chance for small inner
  moons. A large captured moon (Triton-like) on a retrograde orbit clears
  the original regular moons near it: give such systems few regular moons.

## Tidal heating

- **Fact:** tides in a moon on an eccentric orbit flex it and turn orbital
  energy into heat. Peale and co-authors predicted Io's melting this way
  in 1979, before Voyager saw its volcanoes; the heating grows with the
  square of the eccentricity, steeply with the moon's size and the
  orbital rate, and inversely with the moon's rigidity and dissipation
  factor `Q` [GEN-S051]. Forms of the formula differ between sources,
  and a re-derivation corrected terms in the classic version [GEN-S051].
- **Fact (scaling to use):** for a homogeneous, incompressible, elastic
  moon, the dissipation rate (Peale and Cassen 1978; Peale et al. 1979;
  Wisdom 2004; equation 15 of the citing paper, read in full) is

  ```text
  Ė = (42/19) π ρ² n^5 R^7 e^2 / (μ Q)
  ```

  with density `ρ`, orbital rate `n`, radius `R`, eccentricity `e` and
  rigidity `μ`. For Io (`e = 0.0043`, `ρ = 3.5 g/cm³`, `R = 1,800 km`,
  period 1.7 days, `μ ≈ 6.5 × 10^11` dyne/cm²) it gives
  `1.6 × 10^21 / Q` erg/s, and Peale and co-authors used `Q = 100`
  [GEN-S051]. Since `n ∝ a^-3/2`, heating falls about as `a^-7.5`
  with the orbit's size at fixed eccentricity.
- **Recommendation:** compute a heating score with this scaling,
  normalised so Io scores 1. Above about 0.1: volcanic (rocky) or ocean
  (icy) type; above about 0.01 for icy moons: a subsurface ocean with a
  cracked surface; below: dead and cratered. This replaces the simple
  score in [moons](./moons.md#moon-types-and-their-recipes).

## Resonances keep the heat on

- **Fact:** Io, Europa and Ganymede are in the Laplace resonance, two 2:1
  commensurabilities (periods 1:2:4); the resonance forces Io's
  eccentricity and so its heating; Ganymede's eccentricity is low now,
  but earlier Laplace-like resonances could have pumped it and heated it
  several hundred times more [GEN-S052].
- **Why it matters:** without a resonance, tides circularise an orbit and
  the heating dies. A resonance keeps the eccentricity forced.
- **Recommendation:** when placing regular moons of a giant, sometimes
  snap neighbours to period ratios of 2:1 (or 3:2), and chain three in a
  1:2:4 Laplace-like set. Only resonant inner moons get a non-zero forced
  eccentricity, and so a high heating score.

## Spin, locking and drift

- **Locking:** most regular moons are tidally locked
  ([lifetime — spins slow](./lifetime.md#spins-slow-and-moons-move)).
- **Outward drift:** a moon orbiting outside its planet's synchronous
  orbit (where the orbit period equals the planet's day) is pushed outward
  by tides, as Earth's Moon is.
- **Inward decay (fact):** a moon inside the synchronous orbit is dragged
  inward. Phobos orbits inside Mars's synchronous radius and spirals in,
  about 2 cm per year; when it reaches its breakup distance it should form
  a ring. Estimates of when range from under 10 to about 40 million years,
  and a 2026 study suggests it may break up farther out than thought
  [GEN-S053].
- **Recommendation:** at generation, compare each regular moon's orbit
  with the planet's synchronous radius
  `a_sync = (G M P_spin² / 4π²)^(1/3)`. Inside it: the moon is
  decaying; if the [age](./lifetime.md) puts it past the breakup distance,
  replace it with a ring. Retrograde captured moons also decay inward.

## Co-orbital moons

- **Fact:** Janus and Epimetheus share an orbit in horseshoe motion,
  swapping places about every four years; Telesto and Calypso sit at the
  L4 and L5 points of Tethys's orbit, and Helene and Polydeuces at those of
  Dione (tadpole motion); horseshoe pairs are much rarer than tadpole
  companions [GEN-S054].
- **Recommendation:** after placing large regular moons, give each a
  small chance of one or two small "Trojan" companions 60 degrees ahead
  and behind on the same orbit, and a rarer chance of a horseshoe pair of
  similar small moons. Draw them as tiny potato-shaped moons.

## Rings appear and fade

- **Formation (fact):** material inside the Roche limit spreads into rings
  [GEN-S021]; a moon that decays inside it becomes a ring (see Phobos
  above).
- **Lifetime (fact):** Cassini data suggest Saturn's rings are only a few
  hundred million years old, and ring material falling into Saturn ("ring
  rain") could remove them in about 300 million years, or under 100
  million years counting material falling at the equator [GEN-S055].
- **Recommendation:** rings are temporary. Give each giant a seeded
  history of ring-making events (a moon breakup or a captured body torn
  apart); at the system's [age](./lifetime.md), a ring older than a
  lifetime of about 100-300 million years has faded. Bright, massive rings
  are therefore rare; faint dusty rings, fed by small moons, are common.

## A moon system in order

Recommendation, extending
[moons — generating a moon, step by step](./moons.md#generating-a-moon-step-by-step):

1. Origins: disk system (giants), impact moon (some rocky planets),
   captures (all), breakup chances.
2. Regular orbits, with some resonance chains.
3. Irregular orbits from captures; a big retrograde capture removes
   nearby regulars.
4. Tides at the system's age: locking, drift outward or decay inward;
   decayed moons become rings.
5. Heating score from eccentricity (forced by resonance) and distance;
   the moon type follows.
6. Co-orbital companions.
7. Rings, faded by age.

## Checks

- Volcanic and ocean moons are close in and (for giants) in resonance.
- Every moon inside the synchronous radius is either decaying or gone.
- Bright massive rings are rare and young.
- Trojan companions sit 60 degrees from their host on the same orbit.

## Where to go next

- [Moons](./moons.md) — counts, limits, moon types.
- [Lifetime](./lifetime.md) — age, locking and recession.
- Background: [L10 moons](../../documents/moons.md).

## References

- [Sources](./sources.md) and [research-pass
  sources](./sources-passes.md): GEN-S020 to GEN-S021, GEN-S050 to
  GEN-S055.

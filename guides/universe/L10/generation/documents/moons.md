# Moons — generating moon systems and each moon type

This page explains how to give a generated planet its moons: how many,
where they orbit, how big they are, what type each one is, and how to
build each type's surface. The descriptive background is in
[L10 moons](../../documents/moons.md) and the
[moon kinds table](../../../catalogs/documents/planets.md#moon-kinds--the-companions-observed).

Labels: **fact** (cited), **practice**, **recommendation** (a proposal for
Astrolith, not a decision).

## Where moons can orbit

Two limits bound every moon system:

- **Hill sphere (outer limit, fact):** the region where the planet's
  gravity beats the star's, `r_H ≈ a (1 - e) (m / 3M)^1/3` [GEN-S022].
  - Prograde moons stay stable out to about 0.4-0.5 `r_H`, retrograde ones
    to about 0.7 `r_H` [GEN-S018].
  - **Regular moons** orbit within about 0.05 `r_H`, on near-circular,
    prograde orbits close to the planet's equator [GEN-S018].
  - **Irregular moons** orbit at about 0.05-0.6 `r_H` on eccentric,
    inclined orbits, mostly retrograde (about 83 percent) [GEN-S018],
    [GEN-S019].
- **Roche limit (inner limit, fact):** inside about
  `2.44 R (ρ_planet / ρ_moon)^1/3` a fluid moon is torn apart; a rigid one
  survives to about `1.26 R (...)^1/3`. Material inside the limit spreads
  into rings [GEN-S021].

Example (for scale): Earth's Hill sphere is about 1.5 million km; the Moon
orbits at 384,400 km, about 0.26 `r_H`.

**Recommendation:** place regular moons between the Roche limit and
0.05 `r_H`; irregular moons between 0.05 and 0.45 `r_H` (the more cautious
end of [GEN-S018] and [GEN-S019]). Space regular moons so that neighbouring
orbits differ by at least a factor of about 1.3 in distance, so they do
not overlap.

## How many moons

Astrolith today draws 0-2 moons for rocky planets under 2 Earth radii, and
up to 28 or 274 for giants (`moon_count_for`, matching the Solar System
counts).

Recommendation — keep the counts, but split them:

| Planet | Regular moons | Irregular moons | Big impact moon |
|---|---|---|---|
| Small rocky (under about 0.5 Earth radii) | 0 | 0-2 captured, tiny | rare |
| Rocky (Earth-like, desert, super-Earth) | 0 | 0-2 captured | about 1 in 4 seeds: one large moon |
| Ice giant | 5-20 | 5-15 | no |
| Gas giant | 4-30 (a few large) | 20-250, tiny | no |
| Hot Jupiter | 0 (moons lost near the star) | 0 | no |
| Dwarf | 0-1 large (collisional) | 0-5 | possible (Pluto-Charon) |

**Binary worlds (fact):** when the common centre of mass of a planet and
its moon lies outside the planet, the pair is often called a double
planet; Pluto and Charon qualify (Pluto has about eight times Charon's
mass), while the Earth-Moon centre of mass lies inside Earth [GEN-S075].
**Recommendation:** after drawing a big impact moon, compute the centre
of mass `d_cm = a × m_moon / (m_planet + m_moon)`; if `d_cm > R_planet`,
mark the pair binary: both bodies orbit a point in space, both are
tidally locked to each other early, and both run the full planet
pipeline.

**Origin rates (estimates):** there are no measured statistics for moon
origins around other stars, so the draw rates are tuning values: for
example an impact moon on about 1 in 4 rocky planets (as above), a
disk system on every giant, 0-2 captures per rocky planet and many per
giant, and a binary on about 1 in 20 impact moons. Label them as tuning
in any in-game text.

## How much moon

- **Giants (fact):** each regular-moon system of Jupiter, Saturn and
  Uranus totals about `10^-4` of its planet's mass [GEN-S020].
- **Rocky planets (fact):** a giant-impact moon can be far heavier relative
  to its planet, two to three orders of magnitude above the giants' ratio
  (the Moon is about 1/81 of Earth) [GEN-S020].

**Recommendation for giants:** draw the total regular-moon mass as
`10^-4 * M_planet` (scatter ±0.3 dex), then split it among the moons with a
steep power law, so that 1-4 moons hold most of the mass and the rest are
small, as in the Galilean and Saturnian systems.

## Round or potato

**Fact:** bodies turn round above a mean radius of about 200 km if icy and
about 300 km if rocky (the "potato radius") [GEN-S011].

**Recommendation:**
- Above the potato radius: a sphere with a [height map](./height-maps.md)
  like a small planet.
- Below it: an irregular shape. Build a sphere, then add very
  low-frequency, high-amplitude 3D noise to the radius itself (amplitude
  10-40 percent of the radius), plus one or two large craters that bite
  into the outline. Phobos and Hyperion are the look to match.
- Near the boundary: blend, with a lower irregular amplitude.

## Moon types and their recipes

Pick the type from the moon's size, its distance from the planet, the
planet type and the ice line (recommendation). The look of each kind
follows the catalog's moon table.

| Moon type | When to pick it | Height map and look |
|---|---|---|
| Dead cratered | default for rocky or icy moons with no heating | saturated craters (`D^-2`), a few dark flooded basins on rocky ones (maria); grey or dirty ice |
| Volcanic | large moon, close to a giant, in an orbital resonance with a neighbour (tidal heating) | no craters (resurfaced); calderas, lava flows, sulfur-yellow to orange plains, plumes |
| Ocean (cracked ice) | icy moon with some tidal heating | few craters; smooth ice crossed by long double ridges and cracks (ridged lines along great circles), maybe plumes near a pole |
| Hazy | large (over about 2,000 km radius), cold, far from the star | an opaque orange haze sphere; under it dunes and methane lakes (draw only if the player goes below the haze) |
| Captured irregular | irregular orbit (see above) | potato shape, dark, heavily cratered |
| Small regular or shepherd | small, near rings | potato or smooth, often with a ridge at the equator |
| Ringed small | rare, small, far out | potato shape with its own thin ring |

Tidal heating, resonances, decay and rings over time are detailed in
[moon dynamics](./moon-dynamics.md).

Tidal heating is the switch between "dead cratered" and the active types
(practice): close orbits around massive planets, with resonances, keep
moons hot. **Recommendation:** use the heating score of
[moon dynamics — tidal heating](./moon-dynamics.md#tidal-heating)
(normalised to Io, falling about as `a^-7.5` with distance, non-zero only
with a forced eccentricity from a resonance), and pick volcanic (rocky) or
ocean (icy) above its thresholds.

## Rings

- **Fact:** material inside the Roche limit spreads into rings; Saturn's
  main rings lie almost all inside it [GEN-S021].
- **Fact (Saturn's rings):** the C ring has an optical depth of about
  0.1 and the B ring 1 to 5; ring particles range from about a
  centimetre (millimetres in parts of the C ring) to several metres,
  about 10 m at most, with a size distribution close to a power law of
  index about 3 [GEN-S076].
- **Recommendation:** give giants a seeded chance of a ring system inside
  their Roche limit (high for Saturn-like, low and faint for the others),
  described per radius by:
  - an optical depth `τ(r)` from 1D noise in radius, with a few broad
    regions (a dense B-like band with `τ` 1-5, faint C-like bands with
    `τ` about 0.1), and gaps cleared at shepherd moons and resonances;
  - a particle size range (1 cm to 10 m, index about 3) that sets how the
    ring scatters light: fine dust lights up when backlit, large blocks
    when lit from the front;
  - drawn opacity `1 − exp(−τ / μ)` with `μ` the cosine of the viewing
    angle, so rings look denser when seen edge-on.

## Generating a moon, step by step

1. From the planet's seed and the moon's index, open the moon's own random
   stream (Astrolith uses `MOON_STREAM_TAG`).
2. Pick regular or irregular, then the orbit within the limits above.
3. Draw the mass from the planet's moon budget; radius from density (ice
   about 1,000-2,000 kg/m³, rock about 3,000-3,500 kg/m³).
4. Decide round or potato from the potato radius.
5. Pick the moon type from the table, using the heating score.
6. Build the height map and palette with the
   [height-map blocks](./height-maps.md), the relief scaled by the moon's
   own gravity.

## Checks for a generated moon system

- Every regular moon lies outside the Roche limit and inside 0.05 `r_H`;
  every irregular one inside 0.45 `r_H`.
- Regular orbits do not overlap.
- Giant regular-moon mass totals about `10^-4` of the planet.
- Moons under the potato radius are not drawn as spheres.
- The same planet seed always gives the same moons.

## Where to go next

- [Height maps](./height-maps.md) — craters and irregular shapes.
- [Planet types](./planet-types.md) — which planet each system belongs to.
- Descriptive pages: [L10 moons](../../documents/moons.md),
  [impacts](../../documents/impacts.md).

## References

- [Sources](./sources.md) and [research-pass sources](./sources-passes.md): GEN-S011, GEN-S018 to GEN-S022, GEN-S075,
  GEN-S076.

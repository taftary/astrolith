# Continuous system space: one star system instead of sub-dimension portals

Document type: research with an analysis section. Facts carry a source
identifier; paragraphs marked *Interpretation* or *Relevance to
Astrolith* are the author's reading and are not facts. Sources are in
[sources.md](./sources.md). Nothing here is a decision: the owner and the
#995 specification decide.

Scope: what it takes to drop the sub-dimension portals between L8
(planetary system), L9 (star close-up) and L10 (planet) and keep scaling
through one continuous star-system space, the way L10 to L14 already work
on the planet. Covered: where a reference frame should change, how games
and engines handle moving planets around a player, the precision budget of
one star-centred frame, and the risks for Astrolith. L1 to L6 are out of
scope (see section 6).

This document extends the
[transitions and hysteresis research](./transitions-and-hysteresis.md),
which covers the angular-size gates that open and close cells today, and
the [multi-scale camera research](./multi-scale-camera.md), which covers
floating origin and camera-relative rendering.

## 1. What a portal does today (facts from the code and the ladder)

- Opening a marker is a pure origin shift: "Entering or leaving a
  dimension changes nothing on screen" (ladder R7), and the brightness of
  the marker and its children cross-fades between 0.02 and 0.14 rad
  ([transitions, section 2.2](./transitions-and-hysteresis.md#22-astroliths-fade-facts-from-the-ladder)).
- A portal's radius is the ladder ratio times half a cell
  (`nest::marker_radius`), the same for every portal of a level; per-object
  sizes are not used for the portal
  ([realism review, section 2.1](../../universe/documents/realism-review.md#21-the-ladder-as-implemented)).
- Only the targeted marker opens (`nav::should_open` at
  `OPEN_ANGLE = 0.14` rad; close at `CLOSE_ANGLE`).
- L8, L9 and L10 are three cells: the planetary system, the star
  close-up "with the same planets", and one planet
  (`docs/universes/ladder.md`; [L9 previous-level page](../../universe/L09/documents/previous-l8.md)).
- Below the planet there are no portals. L11 to L14 are labels picked by
  the height above the ground with a 1.5 times exit band
  ([transitions, section 6.1](./transitions-and-hysteresis.md#61-update-after-632-fact)).

*Interpretation:* the coordinate change at a portal is already invisible.
What the owner experiences as a "sub-dimension portal" is the
combination of target-gated entry, ladder-sized portals instead of real
sizes, and three separate cells for one physical system. The realism
review already called the strict cell chain at these rungs physically
wrong
([realism review, section 6](../../universe/documents/realism-review.md#6-proposed-ladder)).

## 2. Where a reference frame should change: the sphere of influence

- In the patched-conic method a trajectory is computed around one central
  body at a time; at the boundary of a body's sphere of influence (SOI)
  the central body, and with it the reference frame, changes
  [NAV-S031].
- The usual radius is the Laplace radius `r_SOI = a (m / M)^(2/5)`, with
  `a` the orbit's semi-major axis and `m`, `M` the masses; an
  orbit-averaged value is `0.9431 a (m / M)^(2/5)` [NAV-S031][NAV-S033].
  The Hill radius `a (m / 3M)^(1/3)` is the other classical radius
  [NAV-S033].
- Values: Earth about 929,000 km around the Sun (145 Earth radii), the
  Moon about 64,300 km around the Earth (37 Moon radii), Jupiter about
  48.2 million km (687 Jupiter radii) [NAV-S031].
- The boundary is not sharp: Laplace noted the radius can be made
  "considerably larger" without a noticeable loss of accuracy, and the
  most accurate Sun-Earth value lies between 1.2 and 3 Hill radii
  [NAV-S032].

*Relevance to Astrolith (interpretation):* the SOI is a physically
motivated place to switch from a star-centred frame to the planet frame,
and because the boundary is soft, the switch can sit inside a hysteresis
band (enter at one radius, leave at a larger one) without any physical
error, as the gates in the
[transitions research](./transitions-and-hysteresis.md#1-hysteresis-why-open-and-close-need-two-thresholds)
require.

## 3. How games and engines carry a player with moving bodies

| Product | Mechanism | Source |
|---|---|---|
| Kerbal Space Program | Double-precision flight math (0.11), floating origin around the vessel and in scaled space (0.12, 0.17); patched conics with one SOI per body. | [NAV-S019] |
| Kerbal Space Program | Below a per-body altitude, `inverseRotThresholdAltitude`, the game "switches the reference frame" to one that turns with the planet, so craft on the ground rotate with it. | [NAV-S034] |
| Star Citizen | 64-bit world coordinates with camera-relative rendering. | [NAV-S020] |
| Star Citizen | "Zone containers": "within a system a planet could be a zone and a station orbiting that planet another"; local physics grids attach the player to a ship's grid so the player moves with it. | [NAV-S035] (user sandbox page) |
| Star Citizen | 3.0 plan lists a "Planetary Physics Grid to support orbiting and rotating planets". | [NAV-S036] (news recap) |
| Unreal Engine 5 | Large World Coordinates: double-precision data types; default `WORLD_MAX` 88 million km. | [NAV-S037] |
| Godot | Double-precision builds, or origin shifting as the single-precision alternative. | [NAV-S003] |

*Interpretation:* the products agree on one pattern: a large
double-precision frame for open space, and a local frame attached to the
body (or ship) whose zone the player is inside, so the player rides with
that body's motion. Kerbal Space Program shows two nested switches: the
SOI (follow the orbit) and a low altitude (follow the spin).

Not established: no primary source was found for SpaceEngine's or Outer
Wilds' internals; they are not used as evidence here (see the
[topic's documentation status](./README.md#documentation-status)).

## 4. Precision budget of one star-centred frame

`f64` carries 53 significant bits, about 15 to 17 decimal digits
([realism review S26](../../universe/documents/realism-review.md#12-sources)).
The spacing between adjacent doubles near a value `x` is about
`x x 2^-52` (about `x x 2.2e-16`). *Author's arithmetic:*

| Distance from the star | Spacing of `f64` there | Enough for |
|---|---|---|
| 1 AU (1.5e11 m) | about 0.03 mm | anything |
| Heliopause, 120 AU (1.8e13 m, L8 anchor) | about 4 mm | space flight, planet bodies |
| Oort edge, 100,000 AU (1.5e16 m, L7 anchor) | about 3 m | deep space only |
| Planet frame inside the SOI (L10 cell, about 1.3e7 m per unit) | about 1e-9 m | ground, walking ([multi-scale camera, section 4.4](./multi-scale-camera.md#44-update-after-632-fact)) |

Drawing stays camera-relative `f32` (ADR 0026), so the GPU never sees the
large numbers.

*Interpretation:* one star-centred `f64` frame plus the existing planet
frame inside each SOI covers L7 to L14 without a new precision technique.

## 5. Analysis for Astrolith

| # | Observation | Class |
|---|---|---|
| C1 | In one continuous system, planets and orbits must share one scale. At true scale Earth (12,756 km) is about 1/11,700 of an AU across, so from the system view a planet is far below a pixel; viewers keep it findable with markers and labels. | Design trade-off (true scale with markers, or one declared size exaggeration) |
| C2 | Two frame changes are needed, each with a hysteresis band: the planet's SOI (follow its orbit) and a low altitude (follow its spin). | Recommendation |
| C3 | The change touches frozen rules: `E-LADDER-FROZEN` and `ladder.md` (L8 to L10 stop being separate cells), ADR 0004 and ADR 0010 (marker nesting and portals), ADR 0005 (live-cell bound), `E-GOLDEN` (the fixed journey's golden files change once). | Missing information (owner approvals) |
| C4 | Continuity invariants of the [transitions research](./transitions-and-hysteresis.md#3-continuity-invariants-at-a-frame-change) (position, brightness, look, roll, speed, near plane) apply unchanged to the SOI handover and should be measured headlessly. | Recommendation |
| C5 | Planet motion (#981) becomes a property of the system frame: a body's position is a pure function of time, and a player inside its SOI rides along with no per-level position update. | Recommendation |
| C6 | A headless study before any change (precision at the system edge, a jump-free SOI handover, the look of true-scale planets) keeps the risk visible, as the shared-grid study did on #920. | Recommendation |

## 6. Out of scope: L1 to L6

L1 to L6 hold generated populations (up to thousands of markers per
cell) rather than bodies. Streaming them through a continuous zoom needs
an angle-gated, budgeted, unloading octree of the kind Gaia Sky uses
([realism review, FE1 and section 7](../../universe/documents/realism-review.md#7-loading-and-traversal-design-sketch)).
That is a separate, larger item; the portals stay there for now.

## 7. Open questions

- True scale with markers, or one declared size exaggeration (C1)?
- The SOI band: enter at the Laplace radius and leave at what multiple?
- The spin-frame altitude: a fixed height, or a share of the planet's
  radius as for L11 to L13?
- Does L7 (the Oort shells) join in the same step or later?

## References

- [NAV-S003](./sources.md#nav-s003--large-world-coordinates-godot-engine-documentation)
- [NAV-S019](./sources.md#nav-s019--kerbal-space-program-readme-and-changelog-0170)
- [NAV-S020](./sources.md#nav-s020--monthly-studio-report-may-2015-star-citizen)
- [NAV-S031](./sources.md#nav-s031--sphere-of-influence-astrodynamics-wikipedia)
- [NAV-S032](./sources.md#nav-s032--a-dynamical-definition-of-the-sphere-of-influence-of-the-earth)
- [NAV-S033](./sources.md#nav-s033--poliastro-sphere-of-influence-module)
- [NAV-S034](./sources.md#nav-s034--kopernicus-wiki-properties)
- [NAV-S035](./sources.md#nav-s035--star-citizen-wiki-user-sandbox-on-zone-containers-and-physics-grids)
- [NAV-S036](./sources.md#nav-s036--star-citizen-30-detailed-dsogaming)
- [NAV-S037](./sources.md#nav-s037--large-world-coordinates-in-unreal-engine-5)

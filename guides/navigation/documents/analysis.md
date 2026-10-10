# Analysis: Astrolith's camera against the research

Document type: analysis. Everything here is interpretation of the three
research documents and of the code at commit `1364571`; nothing in this
file is a decision. The specification "Continuous camera from planet to
room" is expected to cite the candidate requirements below and accept,
amend, or reject each one.

Research inputs:
[multi-scale-camera.md](./multi-scale-camera.md),
[planet-approach.md](./planet-approach.md),
[transitions-and-hysteresis.md](./transitions-and-hysteresis.md).
Prior project material:
[realism-review.md section 2.3](../../universe/documents/realism-review.md#23-navigation-and-loading),
[section 5.6](../../universe/documents/realism-review.md#56-loading-and-traversal-size-large),
[section 7](../../universe/documents/realism-review.md#7-loading-and-traversal-design-sketch),
and `docs/universes/ladder.md` (R6-R11).

## 1. What the camera does today

| Aspect | Current behaviour | Where |
|---|---|---|
| Frame | Open cell is the origin; `f64` offset in the core, `f32` at the render boundary; child frames are identity-oriented except the L10 to L11 open, which rotates the region so +Y is the patch normal. | ladder R6, `nav.rs`, `frame.rs` |
| Position | Dive mode: always on the line from the open cell's centre to the target; moves only along it. Free flight: 6-DOF with speed from the nearest surface. | `nav.rs`, `flight.rs` |
| Speed | `h' = h * exp(-k dt)` with `KEY_RATE 1.5`, `AUTOPILOT_RATE 1.2`; wheel notch keeps 0.75 of `h`; `h` is the gap to the target's sphere (dive) or to the nearest surface (free). | `nav.rs`, `camera.rs` |
| Orientation | `looking_at(look, Vec3::Y)` in open-cell units every frame; the look point is a first-order lerp (rate 6 per second) toward the target or the cell centre. No roll control. | `camera.rs` |
| Projection | 45 degree vertical field of view, infinite reverse-Z, near plane `0.05 x gap` clamped to `[1e-7, 0.1]`. | `camera.rs`, realism-review 2.3 |
| Gates | Preview 0.02 rad, open 0.14 rad, close 0.10 rad; settle for 8 dives after entry; 5 s travel horizon with a 4-cell floor. | ladder R6/R7, `nav.rs` |
| Fades | Shell and children brightness cross-fade over 0.02-0.14 rad; continuity verified headlessly. | ladder R7, `frame.rs`, `headless.rs` |
| Retarget | On open and close the target is re-locked to the portal along the travel ray; nothing ahead means gliding until a marker is locked. | `input.rs` (#403) |
| Planet context | Planet radius 0.35 cells at L10, carried down as `planet_radius` in `SurfaceContext`; horizon rings and sky arcs are drawn from it. | `terrain.rs`, `nav.rs` |

The realism review's section 7 sketch (pass-through entry, free-flight
mode, 32-cell cap) predates #403 and #394. Pass-through re-lock has since
landed; the free-flight mode exists (`FlightMode::Free`); the 32-cell cap
has not. Section 7 is therefore partly implemented and partly superseded;
this topic does not restate it.

## 2. Gap analysis

Classification follows the guides skill: confirmed defect, likely risk,
missing information, contradiction, design trade-off, alternative
interpretation, recommendation, open question.

| # | Observation | Class | Evidence |
|---|---|---|---|
| G1 | Screen-up jumps from the planet's +Y to the patch normal in one frame at the L10 to L11 open; up to 180 degrees of roll plus a heading jump from the `basis_from_up` reference switch at 0.9. | Confirmed defect candidate (code reading; not yet measured) | planet-approach section 7 |
| G2 | The open/close band is a factor 1.4 in distance (0.146 decades, about 0.22 s of held key); the settle counter was added because the band alone did not hold. | Likely risk | transitions section 1.1 |
| G3 | Look retarget has no angular-rate bound; a 90 degree re-lock swings 90 degrees with a 0.17 s time constant. | Likely risk | transitions section 4 |
| G4 | Dive speed is measured to the portal sphere, not to the planet or ground, so below L10 the perceived speed is set by a 2 percent-of-cell target while a 35 percent-of-cell body fills the view. Free flight measures to the nearest surface, so the two modes feel different on the same leg. | Design trade-off | multi-scale section 4.3 |
| G5 | The autopilot runs at about 1.9 s per decade on every rung, close to the Powers of Ten *return* pace; the planet-to-room leg (6.4 decades) passes in about 12 s. | Design trade-off | multi-scale section 4.2-4.3 |
| G6 | No altitude-gated behaviour exists: no tilt policy, no collision with the planet sphere off the dive line, no minimum standoff from the ground beyond the settle rule. | Missing information (no requirement yet) | planet-approach section 2 and 7 |
| G7 | The headless verify checks position and brightness continuity only; roll, angular rate, and speed continuity are not measured, so G1-G3 cannot be regression-tested today. | Missing information | transitions section 3 |
| G8 | The near plane at `0.05 x gap` will clip nearby real geometry (terrain, buildings) once any is drawn; harmless for gizmos. | Likely risk (future) | multi-scale section 3.4 |
| G9 | Realism-review section 7 and the current code disagree on what is implemented (pass-through yes, 32-cell cap no). | Contradiction (documentation) | section 1 above |
| G10 | Whether to blend up by altitude (Google Earth style) or to carry up and pull it at a bounded rate (standard practice) is open; the two can be combined. | Alternative interpretation | planet-approach section 4.2 |

## 3. Candidate requirements for the specification

Each item is a recommendation with its reason. "Testable by" names the
check the testability preflight could use; the headless replay
(`E-REPLAY-CORE`) is assumed to be the vehicle because it already prints
per-opening continuity lines.

### 3.1 Continuity invariants (from G1, G3, G7)

- R1. The camera's world-space pose (position, forward, up) is identical
  in the frame before and the frame after an open or close, expressed in
  the parent's units. Reason: the pure-origin-shift promise (ladder R7)
  should cover orientation, not only drawn points. Testable by: a verify
  column with the pose error per opening, golden-tested.
- R2. The camera's up vector is carried across frame changes: at an open
  that rotates the child frame, the carried up is mapped through the
  same rotation (`parent_to_child(up, ...)`) so the screen does not
  roll. Reason: G1. Testable by: roll error at each opening in the verify
  line, zero to float precision.
- R3. The look direction's angular rate is bounded (a constant in
  degrees per second, to be chosen) and the up vector is pulled toward
  the policy up (section 3.2) at a bounded rate. Reason: G3; perceived
  velocity is the quantity the eye judges. Testable by: maximum angular
  rate over the journey printed and golden-tested.

### 3.2 Up-vector policy (from G1, G6, G10)

- R4. Define a policy up per altitude band in planet radii: the planet's
  spin axis while the planet reads as a disk, the local surface normal
  once the horizon reads as a line, with a blend band between. The only
  documented external number is Cesium's trackball threshold at 1.175
  radii; the band edges are a design choice. Reason: this is what globe
  viewers do and what the L11-L14 region frames already assume.
  Testable by: up error against the policy at sampled altitudes.
- R5. Where the policy up is parallel to the forward direction, R2's
  carried up decides the roll (no look-at fallback). Reason: Bevy's
  `looking_at` picks an arbitrary right vector in that case [NAV-S029].

### 3.3 Speed and pacing (from G4, G5)

- R6. Below L10, measure the dive gap `h` to the nearest significant
  surface (planet sphere, then ground), not to the target portal, so dive
  and free flight share one speed law. Reason: G4; the ladder's own
  symbol table defines `h` as "camera distance to the nearest significant
  surface or object". Testable by: speed-versus-altitude trace on the
  journey.
- R7. Allow the per-decade time to depend on level (or on `h`), with the
  current 1.9 s per decade as the default; the specification decides
  whether the planet-to-room leg runs slower. Reason: G5; the Powers of
  Ten film uses 10 s per decade outbound and 2 s on the return
  [NAV-S009]. Testable by: journey duration per rung in the verify output.

### 3.4 Gates (from G2)

- R8. Express the open/close hysteresis as a distance ratio (or in
  decades) and widen it from 1.4 to a value the owner chooses; keep the
  settle counter as a second guard only if the wider band proves
  insufficient. Reason: G2; a Schmitt band must exceed the noise, and
  here the "noise" is one wheel notch (factor 1.33) or a short key press.
  Testable by: no open-close oscillation in a scripted back-and-forth leg.
- R9. Keep the 0.02-0.14 rad preview cross-fade and its continuity
  check unchanged. Reason: the research found nothing better than
  angular-size cross-fading; it is the strongest part of the design.

### 3.5 Ground handoff (from G6, G8)

- R10. Add a minimum standoff from the ground in region, city, building,
  and room cells (Cesium uses 1 m and terrain collision below 15 km;
  Astrolith's equivalent is a fraction of the cell). Reason: G6.
  Testable by: minimum `h` over the journey never below the standoff.
- R11. Tie the near plane to the standoff rather than to `0.05 x gap`
  once real geometry is drawn. Reason: G8. Not needed for the indicator
  renderer; record as a dependency.

### 3.6 Documentation (from G9)

- R12. When the specification is approved, add a one-line status note to
  realism-review section 7 saying which parts landed (#403 pass-through,
  free flight) and which are superseded by the new specification. This
  guides topic already links there; the note is the reverse link.

## 4. What this analysis does not settle

- The numeric values: band edges in planet radii, angular-rate caps,
  open/close ratio, per-level pacing. They are owner decisions.
- Whether "landing" is an explicit action (SpaceEngine, Gaia Sky gravity)
  or the natural end of the dive into a room cell.
- Free-flight behaviour near the ground (gravity, walking) is out of
  scope for a camera-continuity specification.

## References

- [NAV-S009](./sources.md#nav-s009--powers-of-ten-film-series-wikipedia)
- [NAV-S029](./sources.md#nav-s029--transform-bevy-0191-api-documentation)

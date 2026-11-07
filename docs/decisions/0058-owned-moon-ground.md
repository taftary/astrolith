# 0058 — One owned ground stack for planets and moons

- Status: accepted
- Date: 2026-11-07
- Issue: #1215 (Spec v1), task #1249

## Context

The owner wants every existing moon reachable by ordinary normal-mode flight,
landable and walkable with its own gravity and world-type ground. The approved
specification preserves generation, navigation, clock semantics, shared settings
and explicit performance/resource bounds. Teleportation and the mini map are
separate #1226 work. Moon paint from #1161 is the distant representation, not a
collision surface or proof of landing.

ADR 0034 gives the planet a patch tree; ADR 0039 supplies contact against drawn
ground; ADR 0050 replaces per-patch GPU meshes with two shared grid meshes and one
height atlas. These paths assume a planet-centred frame. Merely placing the camera
near a moon, changing the radius or painting another ball cannot make its ground
support the player. The nearby moon moves within its containing L10 planet cell
under ADR 0052; that containing cell must not become a new moon portal.

This decision precedes any new moon-ground render resources. It is the design
contract for subsequent implementation tasks, **not a claim that moon ground,
the budgets or the feature validation already pass**.

## Decision

### One active owner, no new navigation level

Generalize the current detailed surface to a permanent owner: the current planet
or one of its generated moons. An owner includes universe, parent identity and,
for a moon, marker identity. A seed or patch coordinate alone is not an owner.
Resolve identity against the existing generator; do not change its distributions,
types, counts, radius domain or shared-clock orbits.

Only one surface is terrain-resolved. Prefer the actual supporting owner, then
the approached owner, then camera-visible detail and background work. Camera
interest cannot steal ground from a supported player. Choose through bounded
transition state/hysteresis, not a new player control. Keep valid coarse cover
while preparing the new surface; an outgoing set may retire safely but must not
be a second visible opaque copy. Distant bodies retain their identities/slots.

Moons remain population markers: no new selection, portal, dive target, moon
autopilot or cross-world saved-view recall. Real flight into surrounding space
does not authorize crossing solid ground; Free flight still passes through it.
Existing Go to player and selected-player dive arrival use the player's owner,
stopping the view 10 m above the relevant planet or moon as specified.

### Explicit coordinates, stable recipes and actual contact

Core owns pure validated owner/recipe data, frame conversions, gravity and
location resolution. Render owns selection/scheduling and engine resources.
`universe-core` remains std-only, with no rendering dependency or new I/O.

Keep containing-cell positions and body-centred metre positions distinct in
types and named conversions. A body's centre comes from the actual shared-clock
snapshot; its local surface axes preserve existing sky-motion conventions, not
a new spin/orbit or tidal-locking simulation. Subtract centre in f64 before
render narrowing. Local tangent frames, patch selection, culling, displayed
geometry and contact use that same owner/frame. A frozen conversion excludes
travel/time advancement and stays within 1 cm position / 0.05° view direction.

Use the existing moon type and paint seed. Share the stable directional crater/
ice features between paint and relief instead of generating a replacement world
on approach. The close recipe adds geometric detail without a type-specific GPU
texture/shader set. Preserve material rules for all generated types and explicit
airless behavior. The home radius is the supplied 1738 km; procedural boundaries
remain 200 km inclusive and 1700 km exclusive, with no landing cutoff.

Ground support uses the displayed mesh/morph through the existing drawn-surface
path, including near-player CPU geometry, not a reference sphere or unrendered
analytic relief. Grounded feet stay within 1 cm. The actual supporting owner is
distinct from nearest-body/strongest-pull identity; use existing mass/radius
gravity rather than fixed Earth gravity. Preserve jump and flight state.

A grounded player's moon-local spot is transported by that owner's shared-clock
motion; pause stops sky time, not walking. Transport is not a frozen coordinate
conversion or self-propelled travel. A still-flying body is not attached to a
moon by detail readiness. Pending seeded placement and genuine airborne motion
remain distinct so loading cannot snap flight onto the moon or parent planet.

### Shared resources, work and retirement

Reuse the existing patch tree/cache, worker/application pipeline and grid atlas,
not a separate plugin/controller/atlas/cache for every moon. These limits are
combined scene limits from Spec v1:

| Resource/work | Bound |
| --- | --- |
| Detailed owner and outgoing lifetime | One active detailed surface cache; at most one bounded outgoing resource set, not a second visible surface. |
| Local scene | Planet plus at most 15 moons; unchanged 32-point parent-system cell and marker caps 500/2,000/5,000. |
| Ground drawing | Two shared grid meshes, at most 32 ground material slots and 64 logical material/mesh groups per view. Existing wet/dry shader specializations only. |
| Coarse drawing | At most 16 local coarse-body groups, two shared proxy materials, 20,000 vertices per body and 32 MiB combined logical CPU/GPU proxy buffers, including the parent seen from the moon. No per-body image or new transparent layer. |
| Ground cache | One shared 8/16/32/64/128 MiB setting, default 64 MiB. Coarsen/evict far detail first; preserve near-player collision and coarse cover. Settled accounted terrain fits the chosen cap. |
| Atlas storage | One shared 1,024² 32-bit paged height atlas; descriptor storage 12/20/36/68/136 MiB respectively. At most one outgoing atlas during resize; combined descriptor ceiling 272 MiB. |
| Queues | At most six queued/in-flight builds per worker, including completed work awaiting application (30 with five workers). |
| Application | At most 100 patch uploads per frame under the existing shared 5/10/15% target-interval time share. Record indivisible-unit overruns, never add a moon allowance. |

Proxy preparation shares the existing loading budget and is cached when bodies
are unchanged, not rebuilt each frame. Preserve coarse cover during worker
builds and retain full CPU ground where contact needs it. Resource keys include
universe/parent/moon, recipe/radius and patch; generation/quality epochs reject
stale results even after departure and return. Cancelled work still counts while
it occupies the shared pipeline. Bound outgoing work/resources while a successor
is prepared; retirement does not permit an accumulating per-visited-body cache.

Main-world removal, render-world residence/removal and physical GPU release are
different facts. Retire/reuse safely through the engine's existing ownership
boundaries; report observable stages without claiming an exact physical release.
Logical occupancy, atlas descriptors and total device allocation are not aliases.

### Sky, quality and persistence

The parent, sun and other moons keep actual shared-clock positions and true
angular sizes when viewed from moon ground. Use the common coordinate snapshot
for coarse drawing, horizon/terrain/body occlusion and sunlight. Do not draw
bodies through solid terrain, double the active body's coarse/fine light, inherit
planet air on an airless moon or introduce new mutual illumination/eclipses.

The same quality controller and Ground memory setting govern either owner.
Rename the approved UI label to Surface detail without changing `planet_detail`
storage, Low/Normal/High, Normal default or Auto/Manual semantics. Auto preserves
the saved Manual choice; collision/gravity cannot be disabled at its floor.

Add validated moon-relative player/view surface records alongside compatible
planet/space records. Resolve their owner and local spot at current/restored time,
not stale cell coordinates. Keep eight slots, mode/look and current-cell recall
scope. Invalid or unresolvable records use safe existing fallback and preserve
unrelated settings. Reset player, Views Reset and Display Reset retain separate
scopes. This is not saved-view teleportation or new destination browsing.

### Rendering exception amendment

Amend `E-RENDER-NO-MESH` and the ground exceptions in ADRs 0016, 0034, 0036 and
0050 only as follows: **the existing bounded L10 shared-grid ground may belong
to a planet or moon, with bounded coarse local bodies including the parent seen
from a moon**. The numeric resource limits above apply jointly. Do not restore
one GPU mesh per patch or create per-moon terrain resources. Existing figure,
billboard and other explicitly recorded exceptions are unchanged. No generic
renderer, new graphics controller, dependency or Rust/Bevy pin change is allowed
by this decision.

## Alternatives considered

- A ground stack for every moon would multiply caches, queues, atlases and
  materials, violating the single-active-surface and shared budget contract.
- Replacing the containing cell with a moon cell would change navigation/recall
  scope; the specification explicitly forbids new navigation levels/portals.
- Sphere-only contact or a new procedural recipe unrelated to paint would fail
  real drawn-ground clearance or stable-feature handover.
- Teleporting a flying body onto ready terrain would fabricate landing and change
  the controls, rather than satisfy ordinary arrival/contact requirements.

## Validation and consequences

Implementation must preserve generation snapshots and exercise every recipe/type,
radius boundaries, seams/poles, frozen conversions and actual drawn contact in
pure/production tests. Real application input must additionally prove flight,
landing, 100 m walk/run/jump, takeoff, owner-following time changes, save/restart
and all eight views. Testability comes from separately validated #1238 mechanisms;
do not add a testing mechanism in this feature or call preparation travel.

Performance acceptance remains Spec v1: fresh matched frozen-baseline controls,
fixed quality then Auto, three serial 30/120-second comparisons, each new ground
mean ≤45 ms and p95 ≤65 ms with explicit stall limits, near-ground/visible-detail
readiness ≤3/10 seconds, and 20 distinct moons/at least five systems with quality/
memory changes and a settled return. The observed default/Normal process peak
must be <1.25 GiB and final settled median at most 64 MiB above the initial one.
Retain sampling gaps, all stalls and unavailable GPU counters honestly. These
bounds are acceptance obligations, not measured outcomes of this ADR.

Main-agent real-window evidence and owner appearance/feel checks remain separate
from independent numerical validation. Existing planet walking, navigation,
lighting, clock, saved views and seeded journey remain regressions to test.
The approved Help additions accompany the finished behavior, not an early claim
that this architecture record makes moons walkable.

Rules: `E-CORE-NO-BEVY`, `E-DEPS-DIRECTION`, `E-RENDER-SCOPE`,
`E-RENDER-NO-MESH`, `E-NEWTYPE`, `E-CAST`, `E-HOT-NOALLOC`, `E-DET-TIERS`,
`E-NO-PANIC`, `E-STDOUT-PROTOCOL`, `E-PIN-ADR`, `E-GOLDEN`.

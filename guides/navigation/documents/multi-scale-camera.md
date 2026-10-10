# Multi-scale cameras: continuous travel across many orders of magnitude

Document type: research. Facts carry a source identifier; paragraphs
marked *Interpretation* or *Relevance to Astrolith* are the author's
reading and are not facts. Recommendations live in the
[analysis](./analysis.md), not here. Sources are in
[sources.md](./sources.md).

Scope: how real engines and viewers keep a camera usable from
intergalactic distances down to a room: position precision (floating
origin), depth precision (reverse-Z, logarithmic depth), motion law
(speed proportional to distance), pacing (Powers of Ten), and
pan-plus-zoom paths. Planet approach and the up-vector problem are in
[planet-approach.md](./planet-approach.md); switching and hysteresis are
in [transitions-and-hysteresis.md](./transitions-and-hysteresis.md).

## 1. The problem in numbers

Astrolith's ladder spans 10^26.94 m to 10^0.7 m, about 26 decades
(`docs/universes/ladder.md`). No single floating-point coordinate system
covers that:

- Single precision keeps about seven significant digits. Below 131,071 m
  from the origin a float still resolves roughly 1 cm steps; one metre
  farther, adjacent centimetre values collapse onto the same stored
  number [NAV-S001].
- The step between representable singles doubles at every power of two:
  about 0.0001 units between 1,024 and 2,048, about 0.0039 between 32,768
  and 65,536. Godot recommends staying within 2,048-4,096 units of the
  origin for a first-person view and within 65,536 for any 3D game before
  switching to double precision [NAV-S003].
- The symptom is jitter: positions snap to the nearest representable
  value as the viewer moves, and the jitter grows as the viewer nears the
  object [NAV-S001][NAV-S003].

Depth has the same problem in a different coordinate. Hardware depth is a
linear remapping of 1/z, so half the depth-buffer values land between the
near plane and twice the near plane [NAV-S006][NAV-S007]. A standard 24-bit
setup resolves about four decades of depth; a logarithmic distribution
handles nine easily [NAV-S006].

## 2. Position: floating origin and relative-to-eye rendering

### 2.1 Floating origin

Thorne (2005) named the technique: keep the viewer at or near the origin
and translate the world instead, so the numbers that matter (what is close
to the camera) stay small [NAV-S002] (as summarized by [NAV-S001]).

Variants in production software [NAV-S001][NAV-S004]:

| Technique | What is done | Where |
|---|---|---|
| Relative to center (RTC) | The model-view translation is computed in double precision as the object centre relative to the viewer, then cast to float. Works while vertices stay within about 131 km of their centre. | STK, AGI Insight3D |
| Relative to eye, CPU (RTE) | Subtract the viewer position from every vertex in double on the CPU; the GPU matrix has rotation only. Correct but re-uploads vertices whenever the viewer moves. | STK (large primitives) |
| GPU RTE | Encode each double as a high and a low float; the vertex shader subtracts the viewer's high and low parts separately and adds the differences. Static buffers, about 1.35 cm worst-case error out to 5.5e11 m. | Insight3D, CesiumJS geometry shaders |

CesiumJS states that its procedurally generated geometry shaders include
"high-precision rendering via Relative to Eye (GPU RTE)" citing Ohlarik
2008 [NAV-S004].

Game engines document the same choice at engine level:

- Godot offers double-precision builds ("large world coordinates") and
  names origin shifting as the single-precision alternative, noting that
  shifting "introduces more complexity to game logic" [NAV-S003].
- Unreal Engine 5 introduced Large World Coordinates (double-precision
  world positions with a camera-relative rendering path); see
  [realism-review S34](../../universe/documents/realism-review.md#12-sources).
- Star Citizen's "Large World" conversion to 64-bit coordinates
  "required all rendering code to be changed to be relative to the camera
  and not simply in absolute world coordinates any more" [NAV-S020].
- Kerbal Space Program moved most flight calculations to double precision
  (0.11), fixed floating-origin jitter at high warp (0.12), and added "a
  floating origin system to the Scaled Space subscene, eliminating the
  visual jittering when viewing distant objects in the map view" (0.17)
  [NAV-S019].

### 2.2 Scaled scenes

Kerbal Space Program renders two scenes: the local physics scene around
the vessel and a "Scaled Space" copy of the planets at a smaller scale for
distant viewing; the 0.15 changelog tweaks a moon's scaled-space size "to
make the terrain/scaled transition more seamless" and 0.15.1 tunes "the
rotating reference frame thresholds when nearing planets, to reduce
terrain mesh jitter" [NAV-S019]. *Interpretation:* the scaled-space
handoff is a reference-frame transition with thresholds, which is the same
design problem as Astrolith's open and close gates (section 3 of
[transitions-and-hysteresis.md](./transitions-and-hysteresis.md)).

### 2.3 Relevance to Astrolith (interpretation)

Astrolith already is a floating-origin design: the open cell is the render
origin, positions are O(1) at every depth, and the parent's siblings are
drawn `1/ratio` cells away (ladder R6). Opening a marker is a pure origin
shift (ladder R7). Each level is effectively its own RTC frame with double
precision on the CPU (`f64` offsets in `universe-core`) and `f32` at the
Bevy boundary (`to_vec3` in `universe-render`). The remaining precision
exposure is therefore not the camera position but the *range of distances
inside one open cell*: the parent's siblings sit up to `1/ratio` cells
away, which is about 1e4 cells for the L8 to L9 rung (ratio 7.76e-5), well
inside single-precision comfort by Godot's table. The jitter risk only
returns if a future change draws grandparents or keeps a planet mesh in
the parent's frame while the camera is in a region cell.

## 3. Depth: reverse-Z, infinite far plane, logarithmic depth

### 3.1 Why 1/z fails across scales

Reed shows the mapping with pictures: distinct depth values bunch near the
near plane, so pulling the near plane in "will make the d range skyrocket
up toward the asymptote of the 1/z curve", while pushing the far plane to
infinity barely matters [NAV-S007]. Outerra reports that a common setup
"strides into the unusable region after its brief 4 decades" [NAV-S006].

### 3.2 Reversed floating-point depth

Mapping near to 1 and far to 0 in a 32-bit float buffer lets the float
format's density near zero cancel the 1/z crowding. In Reed's simulation
reversed-Z with a float buffer produced a zero rate of depth-comparison
errors and made the choice of infinite far plane and of matrix composition
irrelevant to precision [NAV-S007]. Outerra measured reversed 32-bit float
depth as slightly better than a 24-bit logarithmic buffer across nine
decades [NAV-S006]. It needs a 0..1 clip range (DirectX convention, or
`glClipControl` in OpenGL) [NAV-S006][NAV-S007].

### 3.3 Logarithmic depth

Outerra's vertex-shader rewrite `z = log(C*w + 1) / log(C*Far + 1) * w`
gives resolution proportional to distance; with a 10,000 km far plane and
24 bits it resolves about 0.5 mm at 1 m and 5.5 m at 10,000 km for
C = 0.001, and "(almost) gets us rid of the near clip plane" [NAV-S005].
Costs: the value is correct only at vertices, so long triangles near the
camera need a fragment-shader depth write (disabling early-Z), or enough
tessellation [NAV-S005][NAV-S006]. Outerra ships logarithmic depth; a
32-bit logarithmic buffer is about 20 times finer than reversed float
[NAV-S006].

### 3.4 Relevance to Astrolith (interpretation)

`universe-render/src/camera.rs` already uses Bevy's infinite reverse-Z
perspective and rewrites the near plane every frame to `0.05 x gap`
clamped to `[1e-7, 0.1]` cell units, where `gap` is the distance to the
target's surface (dive) or to the nearest surface (free flight). With
reversed float depth the moving near plane is precision-neutral
[NAV-S007]; its real job is clipping. Two consequences worth stating in a
specification:

- Anything closer than `0.05 x gap` is clipped. In a region cell with the
  camera 0.2 cells above the ground, the near plane sits 0.01 cells
  (about 5 km at L11 scale) in front of the camera; nothing is lost today
  because only gizmos are drawn, but a terrain mesh or a nearby building
  would be.
- The 1e-7 floor is half a micron of an L14 room cell and irrelevant; the
  0.1 ceiling only binds when the gap exceeds two cells, as at the root
  start position.

A logarithmic depth buffer is not needed while the renderer draws
indicators in one cell at a time; it becomes relevant only if a future
renderer draws the parent's siblings and the open cell's contents in one
depth pass with real geometry.

## 4. Motion law: speed proportional to distance

### 4.1 The exponential zoom

If the camera's speed is proportional to its distance from the thing it
approaches, `v = k * h`, then `h(t) = h0 * exp(-k t)` and every decade of
approach takes the same time, `ln 10 / k` [ladder core formulas; ladder
R6 "travel time per rung is Δe · ln 10 / k"]. The same law is visible
in the tools surveyed:

- Celestia: the mouse wheel "will change your distance to Earth -- you can
  move light years away, then roll the wheel in the opposite direction to
  get back" [NAV-S018]; the travel key `G` approaches the selection and a
  second `G` approaches closer [NAV-S018].
- SpaceEngine: velocity is set with the wheel or +/- keys with Ctrl
  presets; `G` goes to the selected object and `G` twice goes faster
  [NAV-S017].
- Gaia Sky spacecraft mode: engine power "is a multiplier in steps of
  powers of ten. Low engine power levels allow for Solar System or
  planetary travel, whereas high engine power levels are suitable for
  galactic and intergalactic exploration" [NAV-S016].
- CesiumJS: `zoomFactor` (default 5) multiplies zoom speed, `inertiaZoom`
  0.8 lets zoom coast, `maximumMovementRatio` 0.1 caps any input to a
  tenth of the window per frame "to keep the camera under control in
  low-frame-rate situations" [NAV-S010].

### 4.2 Pacing: Powers of Ten

The Eames film (1977 version, 9 minutes) travels from a picnic to 10^24 m
and back to 10^-16 m [NAV-S008]. The outbound zoom runs at one power of
ten per 10 seconds; the return runs at one power of ten per 2 seconds and
slows back to the original rate for the dive into the hand [NAV-S009].
The start was moved to a Chicago lakefront park so that the journey
"approach[es] the disk of the galaxy at approximate right angles"
[NAV-S008], which keeps the galaxy readable as a disk during the zoom.

### 4.3 Relevance to Astrolith (interpretation)

Astrolith's constants (`universe-core/src/nav.rs`) give:

| Mode | k | Seconds per decade (`ln 10 / k`) | L10 to L14 (6.41 decades) |
|---|---|---|---|
| Held key, `KEY_RATE = 1.5` | 1.5 | 1.54 s | 9.8 s |
| Autopilot, `AUTOPILOT_RATE = 1.2` | 1.2 | 1.92 s | 12.3 s |
| Wheel notch, `WHEEL_FACTOR = 0.75` | n/a | 8 notches per decade | 51 notches |

The autopilot runs close to the film's *fast return* pace (2 s per
decade), five times faster than its contemplative outbound pace (10 s per
decade). The ladder fixes the journey at about 40 s for L1 to L10, so the
pace is a decision, not an accident; but the planet-to-room leg is where
the eye has the most to read (horizon, coast, streets, walls) and it
passes in about ten seconds at the same per-decade rate. Whether the
lower rungs should run slower per decade is a question for the
specification, not a fact.

One more structural point: `v = k * h` measures `h` to the *target's
surface* in dive mode. Below L10 the thing that fills the view is the
planet or the ground, not the small portal marker, so the speed the eye
perceives is set by the distance to the portal (a 2 percent-of-cell
sphere) while the picture is dominated by a 35 percent-of-cell planet
disk. Free flight uses the nearest surface instead
(`nearest_surface_distance` in `universe-core/src/flight.rs`). The two
modes therefore feel different on the same approach.

## 5. Pan plus zoom: the path, not just the speed

Changing where the camera looks while changing scale is a path in a
space-scale diagram: 2D position on one axis, scale on the other
[NAV-S023]. Van Wijk and Nuij define a metric on simultaneous zooming and
panning "based on an estimate of the perceived velocity", call an
animation optimal when it is smooth and efficient under that metric, and
derive an analytic solution with two free parameters, animation speed and
the zoom/pan trade-off, which they tune in a user experiment [NAV-S022].
The qualitative result is well known: a long pan is perceived as smoother
when the camera zooms out, pans, and zooms back in, because the perceived
screen velocity stays bounded.

CesiumJS's `flyTo` applies the same shape to a globe: the flight has a
`maximumHeight` ("the maximum height at the peak of the flight"), a
`pitchAdjustHeight` above which the camera pitches down "to look down,
and keep Earth in viewport", a duration computed from the distance when
not given, and an easing function over time [NAV-S011][NAV-S012].

*Relevance to Astrolith (interpretation):* the dive has no pan; the
camera always sits on the line from the open cell's centre to the target
and only the look point is eased (`nav.look` lerps toward the target with
rate 6 per second in `camera.rs`). When the target changes at a
transition (ray re-lock on entry and exit, #403), the position does not
move but the look direction swings; the van Wijk metric says that swing
is the perceived velocity. Its size is bounded by the re-lock design
(pick the portal ahead along the travel ray), not by a turn-rate limit.

## 6. Survey of tools

Facts only; a blank cell means the source read does not say.

| Tool | Coordinates and precision | Motion law | Transition model | Source |
|---|---|---|---|---|
| CesiumJS | GPU RTE in geometry shaders; geographic quadtree HLOD globe | Wheel zoom with `zoomFactor` and inertia; `flyTo` arc with peak height and easing | Altitude-gated controller behaviour (section 2 of planet-approach) | [NAV-S004][NAV-S010][NAV-S011] |
| Google Earth Pro | | Wheel zoom; "zoom plus automatic tilt" on right-drag | Auto-tilt while zooming in; reset keys `n`, `u`, `r` | [NAV-S013][NAV-S015] |
| SpaceEngine | | Wheel sets velocity; `G` go-to, `Shift-G` land | | [NAV-S017] |
| Gaia Sky | | Focus/free/game/spacecraft modes; engine power in powers of ten; cinematic momentum optional | Game mode adds gravity below a distance threshold | [NAV-S016] |
| Celestia | | Wheel changes distance to the selection; `G` approaches, second `G` closer; orbit vs free look drags | | [NAV-S018] |
| Outerra | | | Logarithmic depth buffer, grass to 10,000 km in one pass | [NAV-S005][NAV-S006] |
| Kerbal Space Program | Double-precision flight math; floating origin in local and scaled space | | Scaled-space handoff with tuned thresholds; camera never upside down; orbital camera keeps orientation across spheres of influence | [NAV-S019] |
| Star Citizen | 64-bit world coordinates, camera-relative rendering | | | [NAV-S020] |
| No Man's Sky | | | Continuous generation from space to populated terrain | [NAV-S021] |
| Cosmographia | SPICE ephemerides | User-controlled vantage point and animation rate | | [NAV-S030] |
| Astrolith (commit 1364571) | Per-level cell frames, `f64` core, `f32` render; open cell is the origin | `v = k * h` to target surface (dive) or nearest surface (free) | Angular-size gates 0.02/0.10/0.14 rad; ray re-lock at entry and exit | internal references in sources.md |

Not covered: Universe Sandbox (no primary source fetched); Elite
Dangerous and Outer Wilds (no primary engineering source found; see
[planet-approach.md](./planet-approach.md#6-games) for what is and is not
claimed).

## 7. Open questions

- Does the per-decade time need to vary with level (slow near the ground,
  fast in deep space), as the film does between outbound and return?
- Should dive speed below L10 be measured to the nearest surface (as in
  free flight) rather than to the target portal?
- Is a near-plane rule tied to the planet surface needed once real
  geometry is drawn in region and city cells?

## References

- [NAV-S001](./sources.md#nav-s001--precisions-precisions-relative-to-center-and-relative-to-eye-rendering)
- [NAV-S002](./sources.md#nav-s002--using-a-floating-origin-to-improve-fidelity-and-performance-of-large-distributed-virtual-worlds)
- [NAV-S003](./sources.md#nav-s003--large-world-coordinates-godot-engine-documentation)
- [NAV-S004](./sources.md#nav-s004--graphics-tech-in-cesium-the-graphics-stack)
- [NAV-S005](./sources.md#nav-s005--logarithmic-depth-buffer-outerra)
- [NAV-S006](./sources.md#nav-s006--maximizing-depth-buffer-range-and-precision-outerra)
- [NAV-S007](./sources.md#nav-s007--depth-precision-visualized)
- [NAV-S008](./sources.md#nav-s008--powers-of-ten-and-the-relative-size-of-things-in-the-universe-eames-office)
- [NAV-S009](./sources.md#nav-s009--powers-of-ten-film-series-wikipedia)
- [NAV-S010](./sources.md#nav-s010--screenspacecameracontroller-cesiumjs-reference)
- [NAV-S011](./sources.md#nav-s011--camera-cesiumjs-reference)
- [NAV-S012](./sources.md#nav-s012--control-the-camera-cesiumjs-tutorial)
- [NAV-S013](./sources.md#nav-s013--use-keyboard-shortcuts-to-navigate-in-google-earth)
- [NAV-S015](./sources.md#nav-s015--why-does-google-earth-keep-tilting-the-view-when-i-zoom-in)
- [NAV-S016](./sources.md#nav-s016--camera-settings-gaia-sky-documentation)
- [NAV-S017](./sources.md#nav-s017--spaceengine-faq)
- [NAV-S018](./sources.md#nav-s018--celestia-readme)
- [NAV-S019](./sources.md#nav-s019--kerbal-space-program-readme-and-changelog-0170)
- [NAV-S020](./sources.md#nav-s020--monthly-studio-report-may-2015-star-citizen)
- [NAV-S021](./sources.md#nav-s021--continuous-world-generation-in-no-mans-sky-gdc-2017)
- [NAV-S022](./sources.md#nav-s022--smooth-and-efficient-zooming-and-panning)
- [NAV-S023](./sources.md#nav-s023--space-scale-diagrams-understanding-multiscale-interfaces)
- [NAV-S030](./sources.md#nav-s030--spice-enhanced-cosmographia-naif)

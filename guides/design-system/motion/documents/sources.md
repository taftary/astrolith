# Sources for motion

External records for the motion subtopic. Identifiers are `MOT-Snnn`;
cite them inline as `[MOT-S001]`. Accessed 2026-10-10 unless noted.

## MOT-S001 — Van Wijk and Nuij zoom-pan

- **Title:** Smooth and efficient zooming and panning
- **Author:** Jarke J. van Wijk, Wim A. A. Nuij (TU Eindhoven)
- **Publisher:** IEEE Symposium on Information Visualization 2003
- **Published:** 2003
- **Accessed:** 2026-10-10
- **Type:** Academic paper
- **Identifier:** DOI 10.1109/INFVIS.2003.1249004
- **URL:** [https://vanwijk.win.tue.nl/zoompan.pdf](https://vanwijk.win.tue.nl/zoompan.pdf)
- **Supports:**
  - RMS-velocity metric; zoom-out, pan, zoom-in optimum; speed and trade-off parameters.
  - MapLibre and Mapbox adoption of the 1.42 curve.
- **Used in:**
  - [Research](./research.md#1-optimal-zoom-and-pan-moves-out-across-then-in)
- **Limitations:**
  - Parameter values from a user experiment; confirm on phones.

## MOT-S002 — Penner easing equations

- **Title:** Easing Equations v1.3
- **Author:** Robert Penner
- **Publisher:** robertpenner.com
- **Published:** 2001-2002
- **Accessed:** 2026-10-10
- **Type:** Technical reference
- **URL:** [https://robertpenner.com/scripts/easing_equations.txt](https://robertpenner.com/scripts/easing_equations.txt)
- **Supports:**
  - Canonical in, out, and in-out tween set over normalised time.
- **Used in:**
  - [Research](./research.md#2-easing-shapes-time-not-path)
- **Limitations:**
  - Reference curves; feel needs playtesting.

## MOT-S003 — Material easing and duration

- **Title:** Easing and duration (Material Design 3)
- **Author:** Google Material Design
- **Publisher:** m3.material.io
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Design guidance
- **URL:** [https://m3.material.io/styles/motion/easing-and-duration](https://m3.material.io/styles/motion/easing-and-duration)
- **Supports:**
  - Standard, decelerate, and accelerate curves; short, medium, long, and extra-long bands; exits shorter than entrances.
- **Used in:**
  - [Research](./research.md#3-material-duration-and-easing-bands)
- **Limitations:**
  - Mobile UI guidance; game flights need distance scaling on top.

## MOT-S004 — Apple motion

- **Title:** Motion (Human Interface Guidelines)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Platform guidance
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/motion](https://developer.apple.com/design/human-interface-guidelines/motion)
- **Supports:**
  - Brief purposeful motion; cancellable; ease-in-out; no sustained oscillation or whole-frame rotation.
- **Used in:**
  - [Research](./research.md#4-apple-motion-brief-interruptible-stationary-frame)
- **Limitations:**
  - General UI guidance; applied here to camera flights.

## MOT-S005 — Cesium flyTo

- **Title:** Camera flyTo and flyHome; EasingFunction; Control the Camera
- **Author:** Cesium GS
- **Publisher:** cesium.com/learn
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** API reference and tutorial
- **URL:** [https://cesium.com/learn/cesiumjs/ref-doc/Camera.html](https://cesium.com/learn/cesiumjs/ref-doc/Camera.html)
- **Supports:**
  - Distance-scaled auto-duration; quadratic in-out example; cancellable flights.
- **Used in:**
  - [Research](./research.md#5-fly-to-auto-duration-with-cancellation)
- **Limitations:**
  - Defaults move between CesiumJS releases.

## MOT-S006 — Unity LOD cross-fade

- **Title:** Make LOD transitions smooth in LOD Group
- **Author:** Unity Technologies
- **Publisher:** Unity Manual (6.x)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [https://docs.unity3d.com/6000.3/Documentation/Manual/lod/lod-transitions-lod-group.html](https://docs.unity3d.com/6000.3/Documentation/Manual/lod/lod-transitions-lod-group.html)
- **Supports:**
  - Fade transition width band and timed cross-fading animation modes.
- **Used in:**
  - [Research](./research.md#6-fade-by-size-with-a-band-not-a-line)
- **Limitations:**
  - Unity terms; the band concept transfers.

## MOT-S007 — Cesium screen-space refinement

- **Title:** 3D Tiles Specification; Cesium3DTileset.maximumScreenSpaceError
- **Author:** Cesium GS, OGC
- **Publisher:** GitHub and Cesium reference docs
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Specification and API reference
- **URL:** [https://github.com/CesiumGS/3d-tiles/tree/main/specification](https://github.com/CesiumGS/3d-tiles/tree/main/specification)
- **Supports:**
  - Refine while screen-space error exceeds budget.
- **Used in:**
  - [Research](./research.md#6-fade-by-size-with-a-band-not-a-line)
- **Limitations:**
  - Tiles framing; the error math parallels angular fades.

## MOT-S008 — Schmitt trigger hysteresis

- **Title:** Understanding Schmitt Triggers (SCEA046B)
- **Author:** Texas Instruments
- **Publisher:** TI application brief
- **Published:** Living brief
- **Accessed:** 2026-10-10
- **Type:** Engineering note
- **URL:** [https://www.ti.com/lit/pdf/scea046](https://www.ti.com/lit/pdf/scea046)
- **Supports:**
  - Dual thresholds reject oscillation on slow or noisy input.
- **Used in:**
  - [Research](./research.md#6-fade-by-size-with-a-band-not-a-line)
- **Limitations:**
  - Electronics framing; the gate principle is general.

## MOT-S009 — Continuous planet-to-room camera

- **Title:** ADR 0019: Continuous camera from planet to room
- **Author:** Astrolith project
- **Publisher:** Repository (`docs/decisions/0019-continuous-camera-planet-to-room.md`)
- **Published:** 2026-10-10
- **Accessed:** 2026-10-10
- **Type:** Internal reference (no identifier)
- **Supports:**
  - R7 extended to the full tail; up blend, landing bend, clearance, context previews, no pops.
- **Used in:**
  - [Research](./research.md#7-continuity-invariants)
- **Limitations:**
  - Internal; owner-feel tuning goes in a correction round.

## MOT-S010 — Navigation topic baseline

- **Title:** Navigation topic: multi-scale camera, planet approach, transitions and hysteresis, analysis
- **Author:** Astrolith guides
- **Publisher:** Repository (`guides/navigation`)
- **Published:** October 2026
- **Accessed:** 2026-10-10
- **Type:** Internal reference (no identifier)
- **Supports:**
  - Canonical camera research with gaps G1 to G10 and requirements R1 to R12.
- **Used in:**
  - [Research](./research.md) throughout
- **Limitations:**
  - Internal; this subtopic cites it and does not restate it.

## Internal references (no identifier)

- `docs/decisions/0012-streaming-memory-bound.md`, read 2026-10-10.
- `docs/decisions/0013-navigation-model.md`, read 2026-10-10.
- `docs/decisions/0018-final-destination-room.md`, read 2026-10-10.

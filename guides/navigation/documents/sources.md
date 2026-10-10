# Sources for the navigation topic

External records for the camera-navigation research. Identifiers are
`NAV-S<nnn>`; cite them inline as `[NAV-S001]`. Every entry was read on the
access date given unless its **Limitations** say otherwise. Internal project
references (code, `docs/`, other guides) are listed at the end without an
identifier.

## NAV-S001 — Precisions, Precisions (relative-to-center and relative-to-eye rendering)
- **Title:** Precisions, Precisions
- **Author:** Deron Ohlarik
- **Publisher:** AGI (Insight3D blog, archived in the DME Component Libraries documentation)
- **Published:** 2008-09-03
- **Accessed:** 2026-10-10
- **Type:** Engineering blog post
- **URL:** [help.agi.com/AGIComponents/html/BlogPrecisionsPrecisions.htm](http://help.agi.com/AGIComponents/html/BlogPrecisionsPrecisions.htm)
- **Supports:**
  - Single-precision jitter: about 1 cm steps are only available below 131,071 m from the origin.
  - Rendering "relative to center" (RTC): translation of the model-view matrix computed in double precision relative to the viewer.
  - Rendering "relative to eye" (RTE) on the CPU, and "GPU RTE" with a double split into two floats and a vertex-shader subtraction.
  - Describes Thorne's floating-origin paper as the reference for the problem.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Pre-dates CesiumJS; the same technique is confirmed in Cesium by [NAV-S004].

## NAV-S002 — Using a floating origin to improve fidelity and performance of large, distributed virtual worlds
- **Title:** Using a floating origin to improve fidelity and performance of large, distributed virtual worlds
- **Author:** Chris Thorne
- **Publisher:** 2005 International Conference on Cyberworlds (CW'05), IEEE
- **Published:** 2005
- **Accessed:** 2026-10-10 (Crossref metadata only)
- **Type:** Academic paper
- **Identifier:** DOI 10.1109/CW.2005.94
- **URL:** [doi.org/10.1109/CW.2005.94](https://doi.org/10.1109/CW.2005.94)
- **Supports:**
  - Origin of the "floating origin" term: keep the viewer at the origin and move the world.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - The full text was not read; the claims used here come from [NAV-S001], which summarizes it. Crossref lists the author as "C. Thome" (a metadata typo).

## NAV-S003 — Large world coordinates (Godot Engine documentation)
- **Title:** Large world coordinates
- **Author:** Godot Engine contributors
- **Publisher:** docs.godotengine.org (Godot 4.7, stable)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [docs.godotengine.org/en/stable/tutorials/physics/large_world_coordinates.html](https://docs.godotengine.org/en/stable/tutorials/physics/large_world_coordinates.html)
- **Supports:**
  - Single-precision step table by distance from the origin (about 0.0001 at 1,024-2,048; about 0.0039 at 32,768-65,536).
  - Recommended single-precision limits for first-person (2,048-4,096) and any 3D game (32,768-65,536).
  - Two remedies: double-precision builds, or origin shifting on single precision.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Engine-specific advice; the step table is generic IEEE 754 arithmetic.

## NAV-S004 — Graphics Tech in Cesium: The Graphics Stack
- **Title:** Graphics Tech in Cesium - The Graphics Stack
- **Author:** Patrick Cozzi
- **Publisher:** Cesium blog
- **Published:** 2015-05-26
- **Accessed:** 2026-10-10
- **Type:** Engineering blog post
- **URL:** [cesium.com/blog/2015/05/26/graphics-tech-in-cesium-stack/](https://cesium.com/blog/2015/05/26/graphics-tech-in-cesium-stack/)
- **Supports:**
  - CesiumJS geometry shaders include "high-precision rendering via Relative to Eye (GPU RTE)" citing Ohlarik 2008.
  - The globe is a geographic quadtree with out-of-core hierarchical level of detail (HLOD).
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Describes Cesium 1.9 (2015); the architecture is still current but details may have moved.

## NAV-S005 — Logarithmic Depth Buffer (Outerra)
- **Title:** Logarithmic Depth Buffer
- **Author:** Brano Kemen (Outerra)
- **Publisher:** Outerra blog
- **Published:** 2009-08-12
- **Accessed:** 2026-10-10
- **Type:** Engineering blog post
- **URL:** [outerra.blogspot.com/2009/08/logarithmic-z-buffer.html](https://outerra.blogspot.com/2009/08/logarithmic-z-buffer.html)
- **Supports:**
  - Vertex-shader formula `z = log(C*w + 1) / log(C*Far + 1) * w`.
  - Resolution table for a 10,000 km far plane and 24-bit buffer; the near clip plane becomes almost unnecessary.
  - Interpolation artifacts on long triangles near the camera and the fragment-depth fix.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Superseded in detail by [NAV-S006].

## NAV-S006 — Maximizing Depth Buffer Range and Precision (Outerra)
- **Title:** Maximizing Depth Buffer Range and Precision
- **Author:** Brano Kemen (Outerra)
- **Publisher:** Outerra blog
- **Published:** 2012-11-28
- **Accessed:** 2026-10-10
- **Type:** Engineering blog post
- **URL:** [outerra.blogspot.com/2012/11/maximizing-depth-buffer-range-and.html](https://outerra.blogspot.com/2012/11/maximizing-depth-buffer-range-and.html)
- **Supports:**
  - A standard 1/z depth buffer is usable over roughly four decades of depth; a logarithmic one handles nine decades easily.
  - Reversed 32-bit floating-point depth is slightly better than a 24-bit logarithmic buffer across nine decades; a 32-bit logarithmic buffer is about 20 times finer.
  - Outerra ships the logarithmic buffer; the reversed float buffer needs the DirectX-style 0..1 mapping.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - OpenGL driver details from 2012 are outdated; the precision analysis is not.

## NAV-S007 — Depth Precision Visualized
- **Title:** Depth Precision Visualized
- **Author:** Nathan Reed
- **Publisher:** NVIDIA Developer (GameWorks blog)
- **Published:** 2015
- **Accessed:** 2026-10-10
- **Type:** Engineering article
- **URL:** [developer.nvidia.com/content/depth-precision-visualized](https://developer.nvidia.com/content/depth-precision-visualized)
- **Supports:**
  - Why hardware depth is a linear remapping of 1/z and why pulling the near plane in wrecks precision.
  - Reversed-Z with a float buffer gives a zero comparison-error rate in the author's simulation and makes infinite far planes and matrix composition precision-neutral.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Simulation uses near = 0.1 and far = 10,000; extreme open-world ranges still need care (stated by the author).

## NAV-S008 — Powers of Ten and the Relative Size of Things in the Universe (Eames Office)
- **Title:** Powers of Ten and the Relative Size of Things in the Universe
- **Author:** Eames Office
- **Publisher:** eamesoffice.com
- **Published:** Film 1977; page undated
- **Accessed:** 2026-10-10
- **Type:** Primary catalogue page of the film
- **URL:** [eamesoffice.com/the-work/powers-of-ten/](https://www.eamesoffice.com/the-work/powers-of-ten/)
- **Supports:**
  - 1977 version adds two powers of ten at each end; start moved to a Chicago lakefront park so the journey meets the galactic disk at about right angles; 9 minutes; National Film Registry.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Does not state the per-decade pacing; see [NAV-S009].

## NAV-S009 — Powers of Ten (film series), Wikipedia
- **Title:** Powers of Ten (film series)
- **Author:** Wikipedia contributors
- **Publisher:** Wikipedia
- **Published:** Revision of 2026-09-21
- **Accessed:** 2026-10-10
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Powers_of_Ten_(film_series)](https://en.wikipedia.org/wiki/Powers_of_Ten_(film_series))
- **Supports:**
  - Outbound zoom: one power of ten per 10 seconds, to 10^24 m; return: one power of ten per 2 seconds, then back to the slow rate into the hand.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Secondary source; the pacing numbers cite Repp (2001), which was not read.

## NAV-S010 — ScreenSpaceCameraController (CesiumJS reference)
- **Title:** ScreenSpaceCameraController
- **Author:** Cesium GS
- **Publisher:** CesiumJS reference documentation
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [cesium.com/learn/cesiumjs/ref-doc/ScreenSpaceCameraController.html](https://cesium.com/learn/cesiumjs/ref-doc/ScreenSpaceCameraController.html)
- **Supports:**
  - Defaults: `minimumZoomDistance` 1 m, `maximumZoomDistance` infinity, `zoomFactor` 5, `inertiaZoom` 0.8, `inertiaSpin` and `inertiaTranslate` 0.9, `maximumMovementRatio` 0.1.
  - Altitude gates: `minimumPickingTerrainHeight` 150,000 m, `minimumCollisionTerrainHeight` 15,000 m, `minimumPickingTerrainDistanceWithInertia` 4,000 m, `minimumTrackBallHeight` 7,500,000 m, each scaled by the ellipsoid radius for other bodies.
  - `enableCollisionDetection` (default true), `maximumTiltAngle` relative to the ellipsoid normal.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Defaults can change between CesiumJS releases; values read on the access date.

## NAV-S011 — Camera (CesiumJS reference)
- **Title:** Camera
- **Author:** Cesium GS
- **Publisher:** CesiumJS reference documentation
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [cesium.com/learn/cesiumjs/ref-doc/Camera.html](https://cesium.com/learn/cesiumjs/ref-doc/Camera.html)
- **Supports:**
  - The camera is position plus an orthonormal basis `view`, `up`, `right = view x up`.
  - `constrainedAxis`: the camera cannot rotate past this axis.
  - `flyTo` options: `duration` (auto from distance when omitted), `maximumHeight` (peak of the flight), `pitchAdjustHeight` (pitch down above it to keep Earth in view), `easingFunction`.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - None noted.

## NAV-S012 — Control the camera (CesiumJS tutorial)
- **Title:** Control the camera
- **Author:** Cesium GS
- **Publisher:** cesium.com/learn
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Tutorial
- **URL:** [cesium.com/learn/cesiumjs-learn/cesiumjs-camera/](https://cesium.com/learn/cesiumjs-learn/cesiumjs-camera/)
- **Supports:**
  - Heading, pitch, and roll are relative to East, North, and Up at the destination.
  - `lookAtTransform` with `eastNorthUpToFixedFrame` locks the camera to a local surface frame.
  - Easing functions such as `QUADRATIC_IN_OUT` shape the flight time.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - None noted.

## NAV-S013 — Use keyboard shortcuts to navigate in Google Earth
- **Title:** Use keyboard shortcuts to navigate in Google Earth
- **Author:** Google
- **Publisher:** Google Earth Help
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Product help page
- **URL:** [support.google.com/earth/answer/148115](https://support.google.com/earth/answer/148115)
- **Supports:**
  - "Zoom plus automatic tilt" on right-drag; `n` resets to north-up, `u` resets to top-down tilt, `r` recenters.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - Google Earth Pro desktop; the web client differs.

## NAV-S014 — Explore the Earth on your computer
- **Title:** Explore the Earth on your computer
- **Author:** Google
- **Publisher:** Google Earth Help
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Product help page
- **URL:** [support.google.com/earth/answer/148186](https://support.google.com/earth/answer/148186)
- **Supports:**
  - Startup view is straight down; tilt is a separate gesture; the compass returns to north-facing.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - No engineering detail.

## NAV-S015 — Why does Google Earth keep tilting the view when I zoom in?
- **Title:** "Why does Google Earth keep tilting the view when I zoom in?"
- **Author:** Mickey Mellen
- **Publisher:** Google Earth Blog (independent)
- **Published:** 2013-09-18
- **Accessed:** 2026-10-10
- **Type:** Blog post
- **URL:** [gearthblog.com/blog/archives/2013/09/google-earth-keep-tilting-view-zoom.html](https://www.gearthblog.com/blog/archives/2013/09/google-earth-keep-tilting-view-zoom.html)
- **Supports:**
  - Google Earth auto-tilts the view while zooming in to show 3D buildings; the option "Do not automatically tilt while zooming" disables it; `U` restores vertical, `R` also restores north.
  - Reader comments report cases where the option did not take effect.
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - Not an official Google source; used only for the existence and rationale of the auto-tilt behaviour.

## NAV-S016 — Camera settings (Gaia Sky documentation)
- **Title:** Camera settings
- **Author:** Antoni Sagristà Sellés (Gaia Sky)
- **Publisher:** Gaia Sky docs (master)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** User documentation
- **URL:** [gaia.ari.uni-heidelberg.de/gaiasky/docs/master/Camera-modes.html](https://gaia.ari.uni-heidelberg.de/gaiasky/docs/master/Camera-modes.html)
- **Supports:**
  - Four camera modes: focus (default, locked to a focus object with optional lock-position and lock-orientation), free, game (WASD plus mouse look, optional gravity near planets), spacecraft (attitude indicator, engine power in powers of ten, stabilize, stop).
  - Cinematic behaviour (acceleration and momentum) versus the default non-cinematic responsive behaviour since 1.5.0.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - None noted.

## NAV-S017 — SpaceEngine FAQ
- **Title:** FAQ
- **Author:** Cosmographic Software LLC
- **Publisher:** spaceengine.org/manual
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Product FAQ
- **URL:** [spaceengine.org/manual/faq/](https://spaceengine.org/manual/faq/)
- **Supports:**
  - Movement with WASD/RF; velocity set by the mouse wheel or +/- with Ctrl presets; `G` goes to the selected object, `G` twice goes faster; `Shift-G` or `Ctrl-G` lands on a planet; middle mouse looks around without changing the fixed aim.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - No engineering detail on the approach curve; SpaceEngine is closed source.

## NAV-S018 — Celestia README
- **Title:** Celestia README
- **Author:** Celestia Development Team
- **Publisher:** GitHub (CelestiaProject/Celestia)
- **Published:** 2023 copyright line; living document
- **Accessed:** 2026-10-10
- **Type:** Project README
- **URL:** [github.com/CelestiaProject/Celestia/blob/master/README.md](https://github.com/CelestiaProject/Celestia/blob/master/README.md)
- **Supports:**
  - The wheel changes the distance to the selected object (light-years away and back); `G` travels toward the selection and a second `G` approaches closer; right-drag orbits the selection, left-drag rotates about the camera.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - The fuller Celestia User's Guide was not fetched (site bot wall).

## NAV-S019 — Kerbal Space Program readme and changelog (0.17.0)
- **Title:** Kerbal Space Program readme.txt (version 0.17.0 changelog)
- **Author:** Squad
- **Publisher:** GitHub mirror (jeffmcfadden/ksp)
- **Published:** 2012
- **Accessed:** 2026-10-10
- **Type:** Product changelog
- **URL:** [github.com/jeffmcfadden/ksp](https://github.com/jeffmcfadden/ksp)
- **Supports:**
  - 0.17: "Added a floating origin system to the Scaled Space subscene, eliminating the visual jittering when viewing distant objects in the map view."
  - 0.12: floating-origin jitter fix at high warp; 0.11: most flight calculations moved to double precision; 0.15: scaled-space scale tweaked "to make the terrain/scaled transition more seamless"; 0.15.1: rotating-reference-frame thresholds tweaked near planets; 0.14: "The flight camera is no longer able to go upside down" and the orbital camera keeps its orientation across sphere-of-influence changes.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - Unofficial mirror of an official file; the KSP wiki (official) was behind a bot wall on the access date.

## NAV-S020 — Monthly Studio Report: May 2015 (Star Citizen)
- **Title:** Monthly Studio Report: May 2015
- **Author:** Cloud Imperium Games
- **Publisher:** Roberts Space Industries comm-link
- **Published:** 2015-06-05
- **Accessed:** 2026-10-10
- **Type:** Developer report
- **URL:** [robertsspaceindustries.com/en/comm-link/transmission/14758-Monthly-Studio-Report-May-2015](https://robertsspaceindustries.com/en/comm-link/transmission/14758-Monthly-Studio-Report-May-2015)
- **Supports:**
  - "Large World" moved the codebase to 64-bit coordinates; "the move to 64 bit required all rendering code to be changed to be relative to the camera and not simply in absolute world coordinates any more"; a zone system with ship-local physics grids.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - Marketing-adjacent developer report; no camera-approach detail.

## NAV-S021 — Continuous World Generation in 'No Man's Sky' (GDC 2017)
- **Title:** Continuous World Generation in 'No Man's Sky'
- **Author:** Innes McKendrick (Hello Games)
- **Publisher:** GDC Vault
- **Published:** 2017
- **Accessed:** 2026-10-10 (session abstract only)
- **Type:** Conference talk
- **URL:** [gdcvault.com/play/1024265/](https://www.gdcvault.com/play/1024265/Continuous_World_Generation_in__No_Man_s_Sky_)
- **Supports:**
  - The game lets a player "transition seamlessly from space down to an interactive and populated terrain" through continuous real-time generation (voxels, polygonization, texturing, population).
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - Only the abstract was read; camera details are not claimed.

## NAV-S022 — Smooth and efficient zooming and panning
- **Title:** Smooth and efficient zooming and panning
- **Author:** Jarke J. van Wijk and Wim A. A. Nuij
- **Publisher:** IEEE Symposium on Information Visualization (InfoVis 2003), pp. 15-22
- **Published:** 2003
- **Accessed:** 2026-10-10 (abstract via the TU/e research portal)
- **Type:** Academic paper
- **Identifier:** DOI 10.1109/INFVIS.2003.1249004
- **URL:** [research.tue.nl/en/publications/smooth-and-efficient-zooming-and-panning](https://research.tue.nl/en/publications/smooth-and-efficient-zooming-and-panning)
- **Supports:**
  - A metric for simultaneous zoom and pan based on perceived velocity; "optimal" means smooth and efficient; an analytic solution with two free parameters (animation speed, zoom/pan trade-off) tuned by a user experiment.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - The parameter values found in the experiment are not quoted here because the full text was not read.

## NAV-S023 — Space-scale diagrams: understanding multiscale interfaces
- **Title:** Space-scale diagrams
- **Author:** George W. Furnas and Benjamin B. Bederson
- **Publisher:** CHI '95, ACM
- **Published:** 1995
- **Accessed:** 2026-10-10 (Crossref metadata only)
- **Type:** Academic paper
- **Identifier:** DOI 10.1145/223904.223934
- **URL:** [doi.org/10.1145/223904.223934](https://doi.org/10.1145/223904.223934)
- **Supports:**
  - The space-scale diagram: a 2D space plotted against a scale axis, on which pan and zoom trajectories become paths; the standard tool for reasoning about multiscale interfaces.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Full text not read; the description relies on the paper's well-known construction and on [NAV-S022], which builds on it.

## NAV-S024 — Pad++: a zooming graphical interface for exploring alternate interface physics
- **Title:** Pad++: a zooming graphical interface for exploring alternate interface physics
- **Author:** Benjamin B. Bederson and James D. Hollan
- **Publisher:** UIST '94, ACM
- **Published:** 1994
- **Accessed:** 2026-10-10 (metadata only)
- **Type:** Academic paper
- **Identifier:** DOI 10.1145/192426.192435
- **URL:** [doi.org/10.1145/192426.192435](https://doi.org/10.1145/192426.192435)
- **Supports:**
  - Pad++ as the reference zoomable user interface (ZUI) with semantic zooming (objects change representation with scale) and portals.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Full text not read; only the concepts the paper is known for are cited.

## NAV-S025 — Pad: an alternative approach to the computer interface
- **Title:** Pad: an alternative approach to the computer interface
- **Author:** Ken Perlin and David Fox
- **Publisher:** SIGGRAPH '93, ACM
- **Published:** 1993
- **Accessed:** 2026-10-10 (Crossref metadata only)
- **Type:** Academic paper
- **Identifier:** DOI 10.1145/166117.166125
- **URL:** [doi.org/10.1145/166117.166125](https://doi.org/10.1145/166117.166125)
- **Supports:**
  - Origin of the infinite zoomable surface and of semantic zooming.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Full text not read.

## NAV-S026 — Smooth view-dependent level-of-detail control and its application to terrain rendering
- **Title:** Smooth view-dependent level-of-detail control and its application to terrain rendering
- **Author:** Hugues Hoppe
- **Publisher:** IEEE Visualization 1998, pp. 35-42 (author's page)
- **Published:** 1998
- **Accessed:** 2026-10-10
- **Type:** Academic paper (abstract and hindsights on the author's page)
- **URL:** [hhoppe.com/proj/svdlod/](https://hhoppe.com/proj/svdlod/)
- **Supports:**
  - Runtime geomorphs "eliminate popping artifacts by smoothly interpolating geometry"; visually smooth at 72 frames per second in the terrain flyover; the author's hindsight that regular representations (geometry clipmaps) are the future for terrain.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - None noted.

## NAV-S027 — LOD Group component reference (Unity Manual)
- **Title:** LOD Group component reference
- **Author:** Unity Technologies
- **Publisher:** Unity Manual (Unity 6.6)
- **Published:** 2026-10-10 build
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [docs.unity3d.com/Manual/class-LODGroup.html](https://docs.unity3d.com/Manual/class-LODGroup.html)
- **Supports:**
  - LOD thresholds are percentages of screen height; fade modes None, Cross Fade, and SpeedTree vertex interpolation; "Fade Transition Width" defines the fraction of a LOD's range used for the cross-fade; "Animate Cross-fading" uses a fixed time after the threshold instead.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - None noted.

## NAV-S028 — Schmitt trigger
- **Title:** Schmitt trigger
- **Author:** Wikipedia contributors
- **Publisher:** Wikipedia
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Encyclopedia article
- **URL:** [en.wikipedia.org/wiki/Schmitt_trigger](https://en.wikipedia.org/wiki/Schmitt_trigger)
- **Supports:**
  - Definition of hysteresis: output goes high above one threshold and low below a different lower threshold; between them the state is retained; noise smaller than the band causes at most one switch.
- **Used in:**
  - [Transitions and hysteresis](./transitions-and-hysteresis.md)
- **Limitations:**
  - Article flagged as needing more citations; only the textbook definition is used.

## NAV-S029 — Transform (Bevy 0.19.1 API documentation)
- **Title:** Transform in bevy::transform::components
- **Author:** Bevy contributors
- **Publisher:** docs.rs
- **Published:** 2026-08-13 (bevy 0.19.1)
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [docs.rs/bevy/0.19.1/bevy/transform/components/struct.Transform.html](https://docs.rs/bevy/0.19.1/bevy/transform/components/struct.Transform.html)
- **Supports:**
  - `look_to` and `looking_at`: "if direction is parallel with up, an orthogonal vector is used as the 'right' direction" (no failure, but an arbitrary roll).
- **Used in:**
  - [Planet approach research](./planet-approach.md)
- **Limitations:**
  - Pinned to the project's Bevy version; behaviour may change in later releases.

## NAV-S030 — SPICE-enhanced Cosmographia (NAIF)
- **Title:** SPICE-enhanced Cosmographia Mission Visualization Tool, Version 4.2
- **Author:** NASA NAIF
- **Publisher:** naif.jpl.nasa.gov
- **Published:** 2022-12 (release announcement)
- **Accessed:** 2026-10-10
- **Type:** Product page
- **URL:** [naif.jpl.nasa.gov/naif/cosmographia.html](https://naif.jpl.nasa.gov/naif/cosmographia.html)
- **Supports:**
  - Cosmographia is a solar-system and mission visualizer driven by SPICE ephemerides with user control of vantage point and animation rate.
- **Used in:**
  - [Multi-scale camera research](./multi-scale-camera.md)
- **Limitations:**
  - No camera-model detail; the User's Guide was not read.

## Internal references (no identifier)

- `docs/universes/ladder.md` (R6, R7, R9, R10, R11 amendments and the gap note), read 2026-10-10.
- `guides/universe/documents/realism-review.md` sections 2.3, 5.6, 7 and sources S31-S34, S38, read 2026-10-10.
- Code on branch commit `1364571` (read-only, 2026-10-10): `crates/universe-render/src/camera.rs`, `crates/universe-render/src/input.rs`, `crates/universe-core/src/nav.rs`, `crates/universe-core/src/frame.rs`, `crates/universe-core/src/flight.rs`, `crates/universe-core/src/terrain.rs`. Line numbers are not cited because they move; module and item names are.

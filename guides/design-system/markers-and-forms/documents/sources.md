# Sources for markers and forms

External records for the markers-and-forms subtopic. Identifiers are
`MRK-Snnn`; cite them inline as `[MRK-S001]`. Accessed 2026-10-10 unless
noted. Internal references are listed at the end.

## MRK-S001 — Bertin visual variables

- **Title:** Symbol Design: Visual Order and Categories (GEOG 486, after Bertin 1967)
- **Author:** Cary Anderson, Penn State Department of Geography
- **Publisher:** Penn State (courses.ems.psu.edu)
- **Published:** Living course pages
- **Accessed:** 2026-10-10
- **Type:** Course notes synthesising Bertin, MacEachren and others
- **URL:** [https://courses.ems.psu.edu/geog486/node/893](https://courses.ems.psu.edu/geog486/node/893)
- **Supports:**
  - Point, line, and area symbol dimensionalities.
  - Size and value for order and quantity; shape and hue for nominal category.
- **Used in:**
  - [Research](./research.md#1-point-line-and-area-symbols)
- **Limitations:**
  - Teaching synthesis; original Bertin (1967, 1983) not re-read here.

## MRK-S002 — Esri symbolization

- **Title:** Symbolization; Point symbols (ArcGIS Pro)
- **Author:** Esri
- **Publisher:** Esri documentation
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Product documentation
- **URL:** [https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/symbolization.html](https://doc.esri.com/en/arcgis-pro/latest/help/mapping/layer-properties/symbolization.html)
- **Supports:**
  - Shape for qualitative, size for quantitative; hue plus shape for categories.
  - Point markers with colour, size, angle, halo, stroke and fill effects.
- **Used in:**
  - [Research](./research.md#1-point-line-and-area-symbols)
- **Limitations:**
  - GIS product framing; the variable mapping transfers to game markers.

## MRK-S003 — Halo legibility and depiction rules

- **Title:** US Topo Cartographic Specifications; Use halos to make maps more readable
- **Author:** US Geological Survey; Kenneth Field and Esri Basemaps team
- **Publisher:** USGS; Esri Newsroom (ArcUser)
- **Published:** Living standards and article
- **Accessed:** 2026-10-10
- **Type:** Standard and engineering article
- **URL:** [https://www.esri.com/about/newsroom/arcuser/polishing-your-halo](https://www.esri.com/about/newsroom/arcuser/polishing-your-halo)
- **Supports:**
  - Point, linear, and polygon depiction rules including outline-only polygons.
  - Halo colour, width, opacity, and blur kept effective but invisible.
- **Used in:**
  - [Research](./research.md#2-halos-keep-symbols-legible-over-dense-linework)
- **Limitations:**
  - Print and web-map context; widths need device testing for games.

## MRK-S004 — Use of color

- **Title:** Understanding Success Criterion 1.4.1: Use of Color
- **Author:** W3C Accessibility Guidelines Working Group
- **Publisher:** W3C (WCAG 2.2 Understanding)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Standards guidance
- **URL:** [https://www.w3.org/WAI/WCAG22/Understanding/use-of-color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color)
- **Supports:**
  - Colour must not be the only visual means; add shape, text, pattern, or lightness difference.
- **Used in:**
  - [Research](./research.md#3-colour-must-never-be-the-only-code)
- **Limitations:**
  - Understanding text is informative; normative text is the criterion.

## MRK-S005 — Pad and Pad++ semantic zoom

- **Title:** Pad++: A Zooming Graphical Interface (UIST 1994); Zoomable Graphical Interface System (CHI 1995 demo)
- **Author:** Benjamin B. Bederson, James D. Hollan
- **Publisher:** ACM
- **Published:** 1994, 1995
- **Accessed:** 2026-10-10
- **Type:** Academic papers
- **URL:** [http://www.cs.columbia.edu/graphics/courses/csw4170/resources/bedersonHollanUIST94.pdf](http://www.cs.columbia.edu/graphics/courses/csw4170/resources/bedersonHollanUIST94.pdf)
- **Supports:**
  - Semantic zooming and portals as re-representing lenses; smooth pan-zoom interpolation.
- **Used in:**
  - [Research](./research.md#4-semantic-zoom-and-portals)
- **Limitations:**
  - Early systems work; concepts are standard and widely re-confirmed.

## MRK-S006 — Generalized fisheye views

- **Title:** Generalized Fisheye Views
- **Author:** George W. Furnas, Bell Communications Research
- **Publisher:** CHI 1986, ACM
- **Published:** 1986
- **Accessed:** 2026-10-10
- **Type:** Academic paper
- **URL:** [https://dl.acm.org/doi/10.1145/22339.22342](https://dl.acm.org/doi/10.1145/22339.22342)
- **Supports:**
  - Degree of interest: importance minus distance; far views show landmarks.
- **Used in:**
  - [Research](./research.md#4-semantic-zoom-and-portals)
- **Limitations:**
  - Formal model; applied here by analogy to angular thresholds.

## MRK-S007 — Space-scale diagrams

- **Title:** Space-Scale Diagrams: Understanding Multiscale Interfaces
- **Author:** George W. Furnas, Benjamin B. Bederson
- **Publisher:** CHI 1995, ACM
- **Published:** 1995
- **Accessed:** 2026-10-10
- **Type:** Academic paper
- **URL:** [https://dl.acm.org/doi/10.1145/223904.223934](https://dl.acm.org/doi/10.1145/223904.223934)
- **Supports:**
  - Scale as an explicit axis; trajectories and semantic boundaries drawn on it.
- **Used in:**
  - [Research](./research.md#4-semantic-zoom-and-portals)
- **Limitations:**
  - None noted.

## MRK-S008 — Unity LOD cross-fade

- **Title:** Make LOD transitions smooth in LOD Group
- **Author:** Unity Technologies
- **Publisher:** Unity Manual (6.x)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [https://docs.unity3d.com/6000.3/Documentation/Manual/lod/lod-transitions-lod-group.html](https://docs.unity3d.com/6000.3/Documentation/Manual/lod/lod-transitions-lod-group.html)
- **Supports:**
  - Screen-size LOD selection with cross-fade band or timed animation.
- **Used in:**
  - [Research](./research.md#5-lod-cross-fade-by-screen-size-and-hysteresis)
- **Limitations:**
  - Unity pipeline terms; the band concept transfers to gizmo fades.

## MRK-S009 — Cesium screen-space error refinement

- **Title:** 3D Tiles Specification; Cesium3DTileset.maximumScreenSpaceError
- **Author:** Cesium GS, OGC
- **Publisher:** GitHub and Cesium reference docs
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Specification and API reference
- **URL:** [https://github.com/CesiumGS/3d-tiles/tree/main/specification](https://github.com/CesiumGS/3d-tiles/tree/main/specification)
- **Supports:**
  - Refine while screen-space error exceeds budget; root to leaf progression.
- **Used in:**
  - [Research](./research.md#5-lod-cross-fade-by-screen-size-and-hysteresis)
- **Limitations:**
  - Tiles product framing; the error math parallels angular-size fades.

## MRK-S010 — Schmitt trigger hysteresis

- **Title:** Understanding Schmitt Triggers (SCEA046B)
- **Author:** Texas Instruments
- **Publisher:** TI application brief
- **Published:** Living brief
- **Accessed:** 2026-10-10
- **Type:** Engineering note
- **URL:** [https://www.ti.com/lit/pdf/scea046](https://www.ti.com/lit/pdf/scea046)
- **Supports:**
  - Dual thresholds prevent oscillation on slow or noisy input.
- **Used in:**
  - [Research](./research.md#5-lod-cross-fade-by-screen-size-and-hysteresis)
- **Limitations:**
  - Electronics framing; the gate principle is general.

## MRK-S011 — Thomas Was Alone minimalism

- **Title:** Thomas Was Alone
- **Author:** Mike Bithell, Bithell Games
- **Publisher:** Bithell Games
- **Published:** Game 2012
- **Accessed:** 2026-10-10
- **Type:** Game and studio site
- **URL:** [https://www.bithellgames.com/thomas-was-alone](https://www.bithellgames.com/thomas-was-alone)
- **Supports:**
  - Flat coloured rectangles carry character through size, motion, and narration.
- **Used in:**
  - [Research](./research.md#6-flat-two-tone-silhouettes-read-small-and-cost-little)
- **Limitations:**
  - Artistic precedent, not a controlled study.

## MRK-S012 — Alto's flat-shaded minimalism

- **Title:** Alto's Adventure: A Game With Style (80.lv interview)
- **Author:** Team Alto (Snowman, Harry Nesbitt); interview by Kirill Tokarev
- **Publisher:** 80.lv
- **Published:** 2015
- **Accessed:** 2026-10-10
- **Type:** Developer interview
- **URL:** [https://80.lv/articles/altos-adventure-a-game-with-style](https://80.lv/articles/altos-adventure-a-game-with-style)
- **Supports:**
  - Flat-shaded geometry scales crisply, saves texture memory, reads on mobile.
- **Used in:**
  - [Research](./research.md#6-flat-two-tone-silhouettes-read-small-and-cost-little)
- **Limitations:**
  - Studio self-report.

## MRK-S013 — Kingdom Two Crowns silhouette economy

- **Title:** Kingdom Two Crowns and the practical intersection of pixel art and roguelike design
- **Author:** Thomas van den Berg; interview by John Harris
- **Publisher:** Game Developer
- **Published:** Interview, series from 2015
- **Accessed:** 2026-10-10
- **Type:** Developer interview
- **URL:** [https://rawfury.com/games/kingdom-two-crowns](https://rawfury.com/games/kingdom-two-crowns)
- **Supports:**
  - Silhouette minimalism as economical readable style, viewer fills detail.
- **Used in:**
  - [Research](./research.md#6-flat-two-tone-silhouettes-read-small-and-cost-little)
- **Limitations:**
  - Interview context; see references subtopic for the fuller record.

## MRK-S014 — Unity unlit shader

- **Title:** Unlit Shader (URP); Art optimization tips for mobile, part 2
- **Author:** Unity Technologies
- **Publisher:** Unity documentation
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [https://docs.unity.cn/Packages/com.unity.render-pipelines.universal@7.5/manual/unlit-shader.html](https://docs.unity.cn/Packages/com.unity.render-pipelines.universal@7.5/manual/unlit-shader.html)
- **Supports:**
  - Unlit as simplest shading model; opaque preferred over transparent.
- **Used in:**
  - [Research](./research.md#6-flat-two-tone-silhouettes-read-small-and-cost-little)
- **Limitations:**
  - Unity pipeline terms; the cost argument transfers to Bevy unlit fills.

## Internal references (no identifier)

- `docs/decisions/0002-indicators-only-no-meshes.md`, read 2026-10-10.
- `docs/decisions/0010-portal-population-markers.md`, read 2026-10-10.
- `docs/decisions/0017-body-and-portal-forms.md`, read 2026-10-10.

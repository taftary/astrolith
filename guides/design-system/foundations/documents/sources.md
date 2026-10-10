# Sources for foundations

External records for the foundations subtopic. Identifiers are `FOU-Snnn`;
cite them inline as `[FOU-S001]`. Every entry was read via the research
notes on 2026-10-10 unless its Limitations say otherwise. Internal project
references are listed at the end without an identifier.

## FOU-S001 — Bevy bloom

- **Title:** Bloom in bevy::post_process::bloom
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.Bloom.html](https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.Bloom.html)
- **Supports:**
  - Bloom applies to HDR cameras with intensity and threshold controls.
  - Raise emissive to make a mesh glow rather than raising bloom intensity.
- **Used in:**
  - [Research](./research.md#2-hdr-emissive-and-bloom-threshold)
  - [Analysis](./analysis.md#light-and-glow)
- **Limitations:**
  - Behaviour may change in later Bevy releases; pinned to 0.19.1.

## FOU-S002 — Bevy HDR camera

- **Title:** Hdr in bevy::camera
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy 0.19.x)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/latest/bevy/camera/struct.Hdr.html](https://docs.rs/bevy/latest/bevy/camera/struct.Hdr.html)
- **Supports:**
  - HDR intermediate texture widens lighting range; display HDR output unsupported.
  - Bloom requires the HDR path.
- **Used in:**
  - [Research](./research.md#2-hdr-emissive-and-bloom-threshold)
- **Limitations:**
  - None noted.

## FOU-S003 — Bevy bloom prefilter

- **Title:** BloomPrefilter in bevy::post_process::bloom
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.BloomPrefilter.html](https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.BloomPrefilter.html)
- **Supports:**
  - Threshold curve and softness semantics; defaults are physically based.
  - Non-default prefilter emulates an older look and pairs with additive composite.
- **Used in:**
  - [Research](./research.md#2-hdr-emissive-and-bloom-threshold)
- **Limitations:**
  - None noted.

## FOU-S004 — Bevy tonemapping

- **Title:** Tonemapping in bevy::core_pipeline::tonemapping
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/core_pipeline/tonemapping/enum.Tonemapping.html](https://docs.rs/bevy/0.19.1/bevy/core_pipeline/tonemapping/enum.Tonemapping.html)
- **Supports:**
  - TonyMcMapface is the default: neutral, subtle hue shift, brights desaturate.
  - Reinhard variants shift hue without natural desaturation.
- **Used in:**
  - [Research](./research.md#3-filmic-tonemap-and-manual-exposure)
- **Limitations:**
  - Pinned to 0.19.1.

## FOU-S005 — Bevy emissive material

- **Title:** StandardMaterial in bevy::pbr
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/pbr/struct.StandardMaterial.html](https://docs.rs/bevy/0.19.1/bevy/pbr/struct.StandardMaterial.html)
- **Supports:**
  - Emissive adds to lit colour; channels may exceed 1.0; magnitude is strength.
  - Emissive does not light surroundings; exposure weight defaults to zero.
- **Used in:**
  - [Research](./research.md#2-hdr-emissive-and-bloom-threshold)
- **Limitations:**
  - PBR material docs; Astrolith uses gizmos and unlit fills, the emissive principle transfers.

## FOU-S006 — sRGB transfer and relative luminance

- **Title:** Understanding Success Criterion 1.4.3: Contrast (Minimum)
- **Author:** W3C Accessibility Guidelines Working Group
- **Publisher:** W3C (WCAG 2.1 Understanding)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Standards guidance
- **URL:** [https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum)
- **Supports:**
  - sRGB to linear transfer and luminance weights; sRGB encoding assumption.
- **Used in:**
  - [Research](./research.md#1-srgb-authoring-versus-linear-rendering)
- **Limitations:**
  - Understanding document is informative; normative text is the WCAG criterion.

## FOU-S007 — Bevy colour spaces

- **Title:** Representations of colors in various color spaces (bevy_color)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy_color)
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy_color/latest/bevy_color/](https://docs.rs/bevy_color/latest/bevy_color/)
- **Supports:**
  - Srgba for authoring, LinearRgba for lighting, Oklch as perceptual intermediary.
- **Used in:**
  - [Research](./research.md#1-srgb-authoring-versus-linear-rendering)
- **Limitations:**
  - None noted.

## FOU-S008 — Bevy gizmo line config

- **Title:** GizmoLineConfig in bevy_gizmos::config
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy_gizmos 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy_gizmos/0.19.1/bevy_gizmos/config/struct.GizmoLineConfig.html](https://docs.rs/bevy_gizmos/0.19.1/bevy_gizmos/config/struct.GizmoLineConfig.html)
- **Supports:**
  - Width in pixels, 2.0 default; perspective mode; style and joint flags.
  - No engine point size; points are meshes or billboards.
- **Used in:**
  - [Research](./research.md#4-line-weight-and-point-size-legibility)
- **Limitations:**
  - Pinned to 0.19.1.

## FOU-S009 — Bevy exposure

- **Title:** Exposure in bevy_camera
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs (bevy_camera 0.19.1)
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy_camera/0.19.1/bevy_camera/struct.Exposure.html](https://docs.rs/bevy_camera/0.19.1/bevy_camera/struct.Exposure.html)
- **Supports:**
  - ev100 semantics with sunlight, overcast, indoor, Blender presets.
  - Filament physical-camera conversion formula.
- **Used in:**
  - [Research](./research.md#3-filmic-tonemap-and-manual-exposure)
- **Limitations:**
  - None noted.

## FOU-S010 — Filament HDR and exposure

- **Title:** Physically Based Rendering in Filament
- **Author:** Romain Guy, Mathias Agopian and others
- **Publisher:** Google Filament project
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Engine documentation
- **URL:** [https://google.github.io/filament/Filament.md.html](https://google.github.io/filament/Filament.md.html)
- **Supports:**
  - Emissive in linear with exposure compensation; bloom on pre-tonemap HDR.
- **Used in:**
  - [Research](./research.md#2-hdr-emissive-and-bloom-threshold)
- **Limitations:**
  - Filament engine detail; cited for the principle Bevy references.

## FOU-S011 — Tony McMapface

- **Title:** Tony McMapface: A cool-headed display transform
- **Author:** Tomasz Stachowiak
- **Publisher:** GitHub (h3r2tic)
- **Published:** 2023-02
- **Accessed:** 2026-10-10
- **Type:** Project readme
- **URL:** [https://github.com/h3r2tic/tony-mc-mapface](https://github.com/h3r2tic/tony-mc-mapface)
- **Supports:**
  - Neutral display transform intent; grading outside the operator.
- **Used in:**
  - [Research](./research.md#3-filmic-tonemap-and-manual-exposure)
- **Limitations:**
  - Author statement of intent; perceptual claims are design goals.

## FOU-S012 — Exposure stops

- **Title:** Lux-Seconds, Stops, and Exposure Values Explained
- **Author:** Dmitri Juzukov
- **Publisher:** Analog.Cafe
- **Published:** 2023-06-24
- **Accessed:** 2026-10-10
- **Type:** Technical explainer
- **URL:** [https://www.analog.cafe/app/exposure-values-stops-lux-seconds-calculators-definitions](https://www.analog.cafe/app/exposure-values-stops-lux-seconds-calculators-definitions)
- **Supports:**
  - One stop equals doubling; EV100 scale basis.
- **Used in:**
  - [Research](./research.md#3-filmic-tonemap-and-manual-exposure)
- **Limitations:**
  - Photography explainer; the stop math is standard.

## FOU-S013 — Accepted project baseline

- **Title:** ADRs 0002, 0015, 0016 and style.rs
- **Author:** Astrolith project
- **Publisher:** Repository (`docs/decisions`, `crates/universe-render/src/style.rs`)
- **Published:** 2026-10-03 to 2026-10-09
- **Accessed:** 2026-10-10
- **Type:** Internal reference (no identifier)
- **Supports:**
  - Indicators-only baseline, bloom and exposure additions, flat-tone planet, current colour values.
- **Used in:**
  - [Analysis](./analysis.md#current-baseline)
- **Limitations:**
  - Internal; values move with the code.

## Internal references (no identifier)

- `docs/decisions/0002-indicators-only-no-meshes.md`, read 2026-10-10.
- `docs/decisions/0015-star-billboards-bloom-exposure-hud.md`, read 2026-10-10.
- `docs/decisions/0016-bare-planet-and-ladder-to-room.md`, read 2026-10-10.
- `docs/decisions/0017-body-and-portal-forms.md`, read 2026-10-10.
- `crates/universe-render/src/style.rs`, read 2026-10-10.

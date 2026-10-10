# Sources for tokens

External records for the tokens subtopic. Identifiers are `TOK-Snnn`;
cite them inline as `[TOK-S001]`. Accessed 2026-10-10 unless noted.
Internal code is listed at the end as read-only.

## TOK-S001 — W3C design tokens format

- **Title:** Design Tokens Format Module 2025.10 (Final Community Group Report)
- **Author:** W3C Design Tokens Community Group (Chenais, McMahon, Powers, Strom-Awn, Vitan and others)
- **Publisher:** designtokens.org (W3C Community Group)
- **Published:** 2025-10-28 (first stable)
- **Accessed:** 2026-10-10
- **Type:** Community specification
- **URL:** [https://www.designtokens.org/TR/2025.10/format/](https://www.designtokens.org/TR/2025.10/format/)
- **Supports:**
  - Token as name plus value with type and description; dot-joined paths; aliasing.
- **Used in:**
  - [Research](./research.md#1-w3c-token-format)
- **Limitations:**
  - Community report, not a W3C Recommendation; the model is widely adopted.

## TOK-S002 — Style Dictionary

- **Title:** Design Tokens (Style Dictionary)
- **Author:** Style Dictionary (Adobe and community open source)
- **Publisher:** styledictionary.com
- **Published:** Living docs (version 4 and later)
- **Accessed:** 2026-10-10
- **Type:** Tool documentation
- **URL:** [https://styledictionary.com/info/tokens](https://styledictionary.com/info/tokens)
- **Supports:**
  - Platform-agnostic dictionary with nested trees and per-platform transforms.
- **Used in:**
  - [Research](./research.md#2-style-dictionary-translation-precedent)
- **Limitations:**
  - Tool framing; Astrolith applies the pattern manually in Rust.

## TOK-S003 — Material design tokens

- **Title:** Design tokens (Material Design 3)
- **Author:** Google Material Design
- **Publisher:** m3.material.io
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Design guidance
- **URL:** [https://m3.material.io/foundations/design-tokens](https://m3.material.io/foundations/design-tokens)
- **Supports:**
  - Reference, system, and component tiers; role-first stable names.
- **Used in:**
  - [Research](./research.md#3-material-three-tier-naming)
- **Limitations:**
  - Material aesthetic; the tiering transfers.

## TOK-S004 — Apple semantic colours

- **Title:** Color (Human Interface Guidelines); What is New in iOS Design (WWDC19)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living HIG; video 2019-06-04
- **Accessed:** 2026-10-10
- **Type:** Platform guidance and talk
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/foundations/color](https://developer.apple.com/design/human-interface-guidelines/foundations/color)
- **Supports:**
  - Purpose-named colours adapting to light, dark, and contrast; no repurposing.
- **Used in:**
  - [Research](./research.md#4-apple-semantic-colours)
- **Limitations:**
  - Platform guidance; game ships on black first with the same discipline.

## TOK-S005 — Bevy bloom and HDR

- **Title:** Bloom and BloomPrefilter; Hdr (bevy 0.19.1)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs
- **Published:** Living documents, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.Bloom.html](https://docs.rs/bevy/0.19.1/bevy/post_process/bloom/struct.Bloom.html)
- **Supports:**
  - Per-camera bloom on the HDR path; emissive-driven glow.
- **Used in:**
  - [Research](./research.md#5-bevy-constant-mapping)
- **Limitations:**
  - Pinned to 0.19.1.

## TOK-S006 — Bevy tonemapping and exposure

- **Title:** Tonemapping; Exposure (bevy 0.19.1)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs
- **Published:** Living documents, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/core_pipeline/tonemapping/enum.Tonemapping.html](https://docs.rs/bevy/0.19.1/bevy/core_pipeline/tonemapping/enum.Tonemapping.html)
- **Supports:**
  - TonyMcMapface default with LUT feature; ev100 with presets.
- **Used in:**
  - [Research](./research.md#5-bevy-constant-mapping)
- **Limitations:**
  - Pinned to 0.19.1.

## TOK-S007 — Bevy emissive material

- **Title:** StandardMaterial (bevy 0.19.1)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/0.19.1/bevy/pbr/struct.StandardMaterial.html](https://docs.rs/bevy/0.19.1/bevy/pbr/struct.StandardMaterial.html)
- **Supports:**
  - Linear emissive above 1.0; magnitude as strength; exposure weight default zero.
- **Used in:**
  - [Research](./research.md#5-bevy-constant-mapping)
- **Limitations:**
  - PBR framing; the emissive principle transfers to gizmos and billboards.

## TOK-S008 — Bevy gizmo line config

- **Title:** GizmoLineConfig (bevy_gizmos 0.19.1)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs
- **Published:** Living document, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy_gizmos/0.19.1/bevy_gizmos/config/struct.GizmoLineConfig.html](https://docs.rs/bevy_gizmos/0.19.1/bevy_gizmos/config/struct.GizmoLineConfig.html)
- **Supports:**
  - Pixel widths; perspective flag; style and joint options.
- **Used in:**
  - [Research](./research.md#5-bevy-constant-mapping)
- **Limitations:**
  - Pinned to 0.19.1.

## TOK-S009 — Bevy text

- **Title:** TextFont; bevy_text crate (bevy 0.19.1)
- **Author:** Bevy Engine contributors
- **Publisher:** docs.rs
- **Published:** Living documents, build 2026-08-13
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://docs.rs/bevy/latest/bevy/text/struct.TextFont.html](https://docs.rs/bevy/latest/bevy/text/struct.TextFont.html)
- **Supports:**
  - Pixel font sizes per atlas handle; scale-factor multiplication; small-set performance note.
- **Used in:**
  - [Research](./research.md#5-bevy-constant-mapping)
- **Limitations:**
  - Pinned to 0.19.x; Parley migration lineage noted in release notes.

## Internal references (no identifier, read-only)

- `crates/universe-render/src/style.rs`, read 2026-10-10.
- `docs/universes/stack.md` (Bevy pins and features), read 2026-10-10.

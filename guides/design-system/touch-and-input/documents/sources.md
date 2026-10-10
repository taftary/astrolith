# Sources for touch and input

External records for the touch-and-input subtopic. Identifiers are
`TCH-Snnn`; cite them inline as `[TCH-S001]`. Accessed 2026-10-10 unless
noted.

## TCH-S001 — Apple hit regions

- **Title:** Buttons (Human Interface Guidelines)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Platform guidance
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/buttons](https://developer.apple.com/design/human-interface-guidelines/buttons)
- **Supports:**
  - 44 by 44 pt minimum hit region; padded hit areas beyond visible size.
- **Used in:**
  - [Research](./research.md#1-target-sizes)
- **Limitations:**
  - Platform guidance; game fields need an overlap rule on top.

## TCH-S002 — Material touch targets

- **Title:** Touch targets (Material 3 foundations, designing structure)
- **Author:** Google Material Design
- **Publisher:** m3.material.io
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Design guidance
- **URL:** [https://m3.material.io/foundations/designing/structure](https://m3.material.io/foundations/designing/structure)
- **Supports:**
  - 48 by 48 dp with 8 dp separation; glyph may be smaller than target.
- **Used in:**
  - [Research](./research.md#1-target-sizes)
- **Limitations:**
  - General UI guidance; dense starfields need a cycling rule too.

## TCH-S003 — WCAG target size minimum

- **Title:** Understanding SC 2.5.8 Target Size (Minimum)
- **Author:** W3C Accessibility Guidelines Working Group
- **Publisher:** W3C (WCAG 2.2 Understanding)
- **Published:** Living document (criterion new in WCAG 2.2)
- **Accessed:** 2026-10-10
- **Type:** Standards guidance
- **URL:** [https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- **Supports:**
  - 24 by 24 CSS px floor with spacing exception; 44 by 44 best practice.
- **Used in:**
  - [Research](./research.md#1-target-sizes)
- **Limitations:**
  - Understanding text is informative; criterion text is normative.

## TCH-S004 — Apple layout and safe areas

- **Title:** Layout (Human Interface Guidelines)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Platform guidance
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/layout](https://developer.apple.com/design/human-interface-guidelines/layout)
- **Supports:**
  - Full-screen adaptive layout inside safe areas and margins; no fixed widths.
- **Used in:**
  - [Research](./research.md#2-safe-areas-and-system-insets)
- **Limitations:**
  - No fixed margins published; test on devices.

## TCH-S005 — Android edge-to-edge and insets

- **Title:** Display content edge-to-edge; Window insets (Compose)
- **Author:** Android Developers
- **Publisher:** developer.android.com
- **Published:** Living documents (Android 15 enforcement lineage)
- **Accessed:** 2026-10-10
- **Type:** Platform documentation
- **URL:** [https://developer.android.com/develop/ui/views/layout/edge-to-edge](https://developer.android.com/develop/ui/views/layout/edge-to-edge)
- **Supports:**
  - Inset critical content; safe-gesture insets mark OS-priority zones.
- **Used in:**
  - [Research](./research.md#2-safe-areas-and-system-insets)
- **Limitations:**
  - Android version behaviour moves; test on current release.

## TCH-S006 — Pinch as expected zoom

- **Title:** Gestures (Human Interface Guidelines); Material gestures
- **Author:** Apple Developer; Google Material Design
- **Publisher:** developer.apple.com; m3.material.io
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Platform guidance
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/](https://developer.apple.com/design/human-interface-guidelines/)
- **Supports:**
  - Pinch open and close as system-wide zoom; no complex-only gestures.
- **Used in:**
  - [Research](./research.md#3-pinch-is-the-expected-zoom-gesture)
- **Limitations:**
  - Guidance states the norm; alternatives are still required by WCAG.

## TCH-S007 — WCAG pointer and motion actuation

- **Title:** WCAG 2.2 SC 2.5.1 Pointer Gestures, SC 2.5.4 Motion Actuation, SC 2.5.7 Dragging, SC 1.3.4 Orientation
- **Author:** W3C Accessibility Guidelines Working Group
- **Publisher:** W3C Recommendation 2024-12-12
- **Published:** 2024-12-12
- **Accessed:** 2026-10-10
- **Type:** Standard (normative)
- **URL:** [https://www.w3.org/TR/WCAG22/](https://www.w3.org/TR/WCAG22/)
- **Supports:**
  - Single-pointer alternatives for multipoint, path, tilt, and drag; no orientation lock.
- **Used in:**
  - [Research](./research.md#4-pointer-gestures-tilt-and-dragging-need-alternatives)
- **Limitations:**
  - None noted; normative text governs.

## TCH-S008 — Reach and one-handed use

- **Title:** Layout (HIG) and reachability implications; Mobile game UI thumb zones
- **Author:** Apple Developer; H. Idris and Game-Ace synthesis
- **Publisher:** developer.apple.com; h-idris.com and game-ace.com
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Platform guidance plus industry synthesis
- **URL:** [https://h-idris.com/blog/mobile-game-ui-design.html](https://h-idris.com/blog/mobile-game-ui-design.html)
- **Supports:**
  - Primary actions lower-half and reachable; thumb-arc placement; separate orientations.
- **Used in:**
  - [Research](./research.md#5-reach-and-one-handed-use)
- **Limitations:**
  - Apple publishes no zone specification; the zone guidance is industry inference.

## TCH-S009 — Apple accessibility motion and type

- **Title:** Accessibility (HIG); isReduceMotionEnabled; Dynamic Type
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living documents and API
- **Accessed:** 2026-10-10
- **Type:** Platform guidance and API reference
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)
- **Supports:**
  - Honour Reduce Motion; support Dynamic Type to 200 percent; never colour-only.
- **Used in:**
  - [Research](./research.md#6-motion-and-text-scaling)
- **Limitations:**
  - None noted.

## Internal references (no identifier)

- `drafts/game-style.md` (controls lines), read 2026-10-10.
- `crates/universe-render/src/style.rs` (`PICK_PIXELS`), read 2026-10-10.

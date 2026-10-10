# Sources for HUD and typography

External records for the HUD-and-typography subtopic. Identifiers are
`HUD-Snnn`; cite them inline as `[HUD-S001]`. Accessed 2026-10-10 unless
noted.

## HUD-S001 — HUD taxonomy

- **Title:** Game HUD Design: Types, Best Practices and Examples
- **Author:** Argentics studio
- **Publisher:** argentics.io
- **Published:** Living article
- **Accessed:** 2026-10-10
- **Type:** Industry guide
- **URL:** [https://www.argentics.io/game-hud-design](https://www.argentics.io/game-hud-design)
- **Supports:**
  - Diegetic versus non-diegetic versus spatial and meta hybrids.
- **Used in:**
  - [Research](./research.md#1-hud-taxonomy-and-meter-precedent)
- **Limitations:**
  - Studio marketing adjacent; taxonomy is standard.

## HUD-S002 — Game UI Database

- **Title:** Game UI Database 2.0
- **Author:** Edd Coogan (founded by Eddie Stoneham)
- **Publisher:** gameuidatabase.com
- **Published:** Living database, over 1,300 games
- **Accessed:** 2026-10-10
- **Type:** Reference collection
- **URL:** [https://www.gameuidatabase.com/index.php?&tag=83&scrn=133](https://www.gameuidatabase.com/index.php?&tag=83&scrn=133)
- **Supports:**
  - Player Vitals category as meter precedent; touch-screen filtering.
- **Used in:**
  - [Research](./research.md#1-hud-taxonomy-and-meter-precedent)
- **Limitations:**
  - Includes dense desktop HUDs that do not transfer to phones.

## HUD-S003 — Playcentric HUD testing

- **Title:** Game Design Workshop: A Playcentric Approach (5th edition)
- **Author:** Tracy Fullerton
- **Publisher:** USC, gamedesignworkshop.com
- **Published:** 5th edition, living site
- **Accessed:** 2026-10-10
- **Type:** Textbook
- **URL:** [https://www.gamedesignworkshop.com/](https://www.gamedesignworkshop.com/)
- **Supports:**
  - HUD as experience-loop system tested against experience goals.
- **Used in:**
  - [Research](./research.md#1-hud-taxonomy-and-meter-precedent)
- **Limitations:**
  - General design text; applied here to meter glanceability.

## HUD-S004 — Minimal UI talk

- **Title:** Dots That Go for Walks: How to Maximize Minimal UI (GDC 2017)
- **Author:** Margaret Robertson (PlayDots)
- **Publisher:** GDC Vault and YouTube
- **Published:** 2017
- **Accessed:** 2026-10-10
- **Type:** Conference talk
- **URL:** [https://www.youtube.com/watch?v=VY8TsXKRySU](https://www.youtube.com/watch?v=VY8TsXKRySU)
- **Supports:**
  - Maximal readability from minimal elements through hierarchy and motion.
- **Used in:**
  - [Research](./research.md#1-hud-taxonomy-and-meter-precedent)
- **Limitations:**
  - Talk recording; no transcript quoted verbatim.

## HUD-S005 — Colour and contrast minima

- **Title:** Understanding SC 1.4.1 Use of Color; SC 1.4.3 Contrast (Minimum); SC 1.4.11 Non-text Contrast
- **Author:** W3C Accessibility Guidelines Working Group
- **Publisher:** W3C (WCAG 2.2 Understanding)
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Standards guidance
- **URL:** [https://www.w3.org/WAI/WCAG21/Understanding/use-of-color.html](https://www.w3.org/WAI/WCAG21/Understanding/use-of-color.html)
- **Supports:**
  - No colour-only coding; 4.5:1 body, 3:1 large; 3:1 non-text UI.
- **Used in:**
  - [Research](./research.md#2-meters-must-not-rely-on-colour-alone)
- **Limitations:**
  - Understanding texts are informative; criteria are normative.

## HUD-S006 — Scale-bar convention

- **Title:** Scale bars (ArcGIS Pro); d3-geo-scale-bar
- **Author:** Esri; Harry Stevens
- **Publisher:** Esri docs; Observable
- **Published:** Living documents
- **Accessed:** 2026-10-10
- **Type:** Product documentation and notebook
- **URL:** [https://doc.esri.com/en/arcgis-pro/latest/help/layouts/scale-bars.html](https://doc.esri.com/en/arcgis-pro/latest/help/layouts/scale-bars.html)
- **Supports:**
  - Dynamic labelled bars tied to view extent; zoom alone does not convey scale.
- **Used in:**
  - [Research](./research.md#3-scale-bars-are-dynamic-and-labelled)
- **Limitations:**
  - Cartography framing; the labelling rule transfers directly.

## HUD-S007 — Apple typography

- **Title:** Typography (Human Interface Guidelines)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com
- **Published:** Living document (default 17 pt, minimum 11 pt)
- **Accessed:** 2026-10-10
- **Type:** Platform guidance
- **URL:** [https://developer.apple.com/design/human-interface-guidelines/typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- **Supports:**
  - Minimum sizes, weight warnings, per-device legibility testing, scalable text.
- **Used in:**
  - [Research](./research.md#4-mobile-type-floors)
- **Limitations:**
  - Platform guidance; game HUD needs playtesting on top.

## HUD-S008 — Material type scale

- **Title:** Typography type scale tokens (Material Design 3)
- **Author:** Google Material Design
- **Publisher:** m3.material.io
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** Design guidance
- **URL:** [https://m3.material.io/styles/typography/type-scale-tokens](https://m3.material.io/styles/typography/type-scale-tokens)
- **Supports:**
  - Role-based scale with defined sizes and line heights; no ad-hoc sizes.
- **Used in:**
  - [Research](./research.md#4-mobile-type-floors)
- **Limitations:**
  - Material aesthetic; the scale discipline transfers.

## HUD-S009 — Tabular numerals

- **Title:** monospacedDigit (SwiftUI)
- **Author:** Apple Developer
- **Publisher:** developer.apple.com documentation
- **Published:** Living document
- **Accessed:** 2026-10-10
- **Type:** API reference
- **URL:** [https://developer.apple.com/documentation/swiftui/view/monospaceddigit/](https://developer.apple.com/documentation/swiftui/view/monospaceddigit/)
- **Supports:**
  - Fixed-width digits prevent jitter; distinct from a monospace face.
- **Used in:**
  - [Research](./research.md#5-tabular-numerals-stop-jitter)
- **Limitations:**
  - SwiftUI API; the CSS `tabular-nums` equivalent is noted in analysis.

## HUD-S010 — Mobile layout patterns

- **Title:** Mobile game UI design: thumb zones, safe areas; Effective game UI: HUD, menus, accessibility
- **Author:** H. Idris; Game-Ace studio
- **Publisher:** h-idris.com; game-ace.com
- **Published:** Living articles (2025 lineage)
- **Accessed:** 2026-10-10
- **Type:** Industry guides
- **URL:** [https://h-idris.com/blog/mobile-game-ui-design.html](https://h-idris.com/blog/mobile-game-ui-design.html)
- **Supports:**
  - Thumb arc placement; separate portrait and landscape; 44 to 48 px targets.
- **Used in:**
  - [Research](./research.md#7-minimal-chrome-and-thumb-reach)
- **Limitations:**
  - Secondary synthesis; consistent with Apple and Material primaries.

## Internal references (no identifier)

- `drafts/game-style.md` (HUD lines), read 2026-10-10.
- `docs/decisions/0015-star-billboards-bloom-exposure-hud.md`, read 2026-10-10.

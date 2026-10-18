# 0031 — Planet surface cleanup (L10-L14 removal)

- Status: accepted
- Date: 2026-10-18
- Issue: #770

## Context

Issue #770 cleans the L10-L14 planetary surface stack plus buildings/rooms and everything related so the sphere can be recreated from scratch as the cube-sphere engine in `docs/design/cube-sphere-planet-engine.md` (single source, Q3 A). Phase 1 removes only the old L10 surface (icosphere, surface, patch, far ball) and everything L11-L14 (shells, buildings, rooms, landing), keeping all L1-L10 generation (`layouts.rs`, `sysgen.rs`, `orbits.rs`, `Form` variants), the journey backbone, the player work of #754-#756, camera-relative drawing (ADR 0026) and infinite reverse-Z depth, and the L10 guides including `generation/` (Q7 A). Intermediate may be broken until rebuild (decision 3); goldens updated to the cleaned state (decision 4).

## Decision

- **Core:** delete `icosphere.rs`, `surface.rs`, `ground.rs`, `landing.rs`; in `terrain.rs` remove `PLANET_RADIUS_CELL`, `FACET_DEPTH`, `surface_radius`, `surface_floor_radius`, `facet_plane` and region-portal sampling (keep planet record, body recipe, moon sampling); `body::level_ground` returns `None` (single seam for step 6, #779), `walk_levels` removed; remove old-surface parts of `frame.rs` (radius kept as 0.35 literal until T2, then L10-only), `gravity.rs` (point-mass stays), `player.rs` (spot chain/position/heading stay, y=0 floating); `MAX_LEVEL` 14→10, L11-L14 bindings out of `nav.rs`, `nest.rs`, `coords.rs`.
- **Render:** delete `patch.rs`, `sky.rs`; in `planet.rs` remove far ball, air rim, limb preview (moon meshes stay); remove L10 suppress rules in `draw.rs`, `labels.rs`, `hud.rs`; remove L10 ground shots in `screens.rs` (surface-l10.png) and ground tests in `window_tests.rs`/`control_tests.rs`.
- **Harness:** `--verify`/`--capture` legs updated (TAIL-ANCHORS, MILESTONES, CURVATURE cleaned; PORTAL-ON-STRUCTURE passes with no portals), goldens rewritten with `UPDATE_GOLDEN=1`, `universe-verify` parity holds, two runs byte-identical.
- **Settings/docs:** S1 Ground guides row + stored value removed (old files ignore key); G1/G2 gap texts; guides drop code references (knowledge kept, `L10/generation/` byte-identical); `ARCHITECTURE.md` drops removed modules.
- **Rebuild plan:** seven step Issues #774-#780 (precision, cube-sphere, quadtree LOD + debug overlay, cracks/morphing, depth/culling, near-field physics, atmosphere/light), each linking its design-doc section and stating its measured targets (Q8 A).

## Consequences

Supersedes ADR 0030 (true-size travel); marks ADRs 0027-0029 as superseded where they describe the removed surface. The L1-L10 journey, markers, names, autopilot, saved views, dive, free flight and player takeover work; at L10 no planet is drawn, moons stay, the body floats and flies. Levels stop at L10 everywhere; ladder doc frozen (E-LADDER-FROZEN) records the cube-sphere replaces discrete L11-L14 cells with continuous LOD.

# 0028 — Planet levels are shells

- Status: proposed
- Date: 2026-10-14
- Issue: #632 (Spec v2, owner answers Q1-Q6, G1 and G2, Telegram 2026-10-13)

## Context

Since #375 the levels below the planet were cells nested inside it. L11
(a 500 km region), L12 (a 10 km city), L13 (a 32 m building) and L14 (a
5 m room) each opened from a portal at one point on the surface, as
"sub-dimensions" of the planet. The owner found this wrong: to build
buildings and rooms next, every level from the planet down has to be the
whole planet, picked by how high you are, not a small piece of it.

## Decision

1. Nothing opens below the planet. The open-cell chain stops at L10
   (`MAX_OPEN_LEVEL` = 10). An open planet holds no region portals and no
   tail generators exist. L9 planet interiors keep their region portals,
   so L1-L9 stay exactly as before.
2. L11-L14 are shells around the planet centre (`universe_core::shell`).
   On Earth you enter L11 below 1,000 km, L12 below 100 km, L13 below
   1 km, and L14 below 10 m above the ground. L11-L13 scale with the
   planet's radius. L14 is always 10 m, so the player has the same scale
   on every planet. You leave a shell above 1.5 times its height, so
   hovering at a boundary never flickers. Climbing above the L11 leave
   height returns to L10.
3. The shell level lives on `MarkerPath`. It is updated from the camera's
   height above the flat-faced ground on every `set_offset`, so the dive,
   free flight, normal walk and fly, and the replays all follow one rule.
   `Universe::level()` stays the structural level, and
   `Universe::shown_level()` is what the HUD, the title and the readout
   show.
4. Heights are measured with the one metre the body, the squares and the
   planet LOD already use (`body::metre(L10)`). This keeps a 10 m shell
   and a 1.8 m body on the same scale. Everything stays in the L10 cell:
   `f64` resolves about 1e-9 m there, and drawing is camera-relative (ADR
   0026), so no deeper cell is needed for precision.
5. On a planet the dive approaches a rest point by shrinking the height
   and the angle around the planet together (`Universe::approach`). A far
   target is reached over the top, never by skimming the ground. The rest
   point is the player's arrival pose (about 2.7 m up, in L14) when the
   player is the destination, else 5 m straight down from you. Rising
   (`Universe::ascend`) multiplies the height, and the planet closes
   past the close angle as any cell. The ground standoff is 1 m.
6. A level change touches no geometry. The open cell, the render origin
   and the drawn bodies are the same on both sides; only the HUD level,
   readout and title change. The planet's scenery (equator ring, spin
   axis) fades out smoothly across the L11 band (`scenery_fade`).

## Consequences

- Removed: the tail generators and the room, surface-portal landing,
  tail surface frames and cluster tones, the sky rings, haze and ground
  guides (and the Ground guides setting), room outlines, furniture and
  building collisions, tile notes, the ground ruler, and the L10 limb
  preview.
- The player spawns on the planet's ground. A saved player spot or view
  below the planet no longer exists, so it falls back: the player
  restarts at the seeded spot, and a view opens at its planet.
- `--verify` and `--capture` descend through the shells after L10. The
  L1-L9 golden lines are unchanged, L10 loses its six portals, and the
  L11-L14 lines are new (`SHELL`, `SHELLS`).
- Future: buildings and rooms come back in later Issues as content
  standing on the planet's ground in the L13 and L14 shells, not as cells.
- This supersedes the tail-cell parts of ADR 0016, 0018, 0019, 0020, 0024
  and 0025.

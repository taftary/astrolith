# 0019 — Continuous camera from planet to room

- Status: accepted
- Date: 2026-10-10
- Issue: #430 (Spec v1 approved; amends ADR 0018 and the #403 amendment of
  ADR 0012; ADR 0013 unchanged)

## Context

ADR 0018 aims every window dive at a destination room, and the #403
amendment of ADR 0012 keeps heading, up, and the travel ray continuous
across every open and close. The planet-to-room leg (L10 planet, L11
region, L12 city, L13 building, L14 room) still reads wrong: the picture
rolls when the region frame tilts to the planet's surface, dives skim or
pass through the planet and the ground, and the planet mesh, air rim,
horizon, sky, limb, and parent grid switch on and off at the open event
instead of being there already. The R7 rule ("entering or leaving a
dimension changes nothing on screen") holds for marker points only. The
`guides/navigation` research (G1–G10, R1–R12) records the standard answers:
screen-up blended by altitude, a landing phase bending toward the surface
normal, terrain standoff, and transitions faded by size rather than
switched by level.

## Decision

The window camera runs one model from the planet down to the room, all
pure `universe-core` math wired thinly into `universe-render`:

- Local up (`frame::local_up`): the open frame's `+Y` everywhere except at
  L10, where the planet's axis blends into the ground normal under the
  camera by altitude (axis past `UP_BLEND_FAR`, normal below
  `UP_BLEND_NEAR`, smooth in between). The window eases toward it at the
  bounded `UP_RATE`, keeping the previous up when looking straight down,
  so roll never steps and region entry needs no roll.
- Bend to land (`landing::landing_step`, `DiveMode::Landing`): window dives
  fly straight past `BEND_START_MULTIPLE` landing distances, then bend to
  the surface normal (L10 patch normals, ground `+Y` at L11–L13, rooms from
  above the roof) and end on the normal at the open distance (about 3.58
  child units). Off the tail the step is bit-identical to `dive_step`.
- Clearance (`ground::clamp_above_ground`): window dives and window free
  flight never sink under the L10 relief or the L11–L13 ground; a landing
  cap (`landing_floor_cap`, a hair under the open boundary) keeps low
  portals reachable. Centre dives stop at the standoff.
- Context previews (`sky::draw_preview_context`, oriented rects/boxes,
  true-scale grid curvature): the next level's horizon, sky, haze, ground,
  outlines, and shells draw inside its markers, faded by
  `children_brightness` with the same level rules the open cell uses.
- No pops (`sky::parent_context_brightness`, `planet::draw_limb_preview`):
  kept parent context follows the entered marker's own shell curve from
  the mapped camera, so entries and exits change nothing and rest faint;
  the L10 limb and air preview fade in to the kept level before entry, and
  the L10 air rim fades out, masking the mesh despawn. Meshes stay L10-only.

Manual window dives use `Landing`; the autopilot keeps `Targeted` and its
straight line. Replays (`replay_autopilot`, `replay_free_leg`) never touch
the new modes, so `verify.txt` and `capture.txt` stay byte-identical
(`E-REPLAY-CORE`, `E-GOLDEN`). Pacing keeps one rate per decade on the
tail. Indicators-only art direction (ADR 0002, 0016, 0017) is untouched:
everything added is lines, rings, and fades on what is already drawn.

## Consequences

The R7 rule now covers everything drawn on the tail, not only markers
(ladder R12 amendment). `DiveMode::Landing` joins `Passing` (tests only)
and `Targeted` (replays, autopilot) in core; the window chooses by
`input::dive_mode`. Up, landing, clearance, and context constants live in
one place each; owner-feel tuning goes in a correction round. Up and roll,
landing, and ground clearance move off the ADR 0018 consequences list;
ADR 0013 stays accepted unchanged.

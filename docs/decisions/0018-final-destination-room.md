# 0018 — Final destination: the camera always aims at one room

- Status: accepted
- Date: 2026-10-10
- Issue: #423 (Spec v1 approved; amends ADR 0013, which stays accepted
  for free flight, saves, and the headless legs)

## Context

ADR 0013 lets manual dives open every crossed portal in passing, and #403
re-picked the target on every entry and exit along the travel ray inside
a cone of about 20 degrees. When nothing lay in the cone the target was
empty: the camera glided straight with nothing to look at, and a crossed
portal could pull it into another branch. The owner wants navigation to
always have a final target room and always look toward it, even at the
cosmic web, with a click choosing a new room under the clicked marker
(#423 notion and decisions).

## Decision

The window holds a destination: the named marker chain from L1 to a room
(`universe_core::destination::Destination`, a fixed array, `Copy`). Below
a clicked marker it follows the Spacebar journey's seeded picks
(`autopilot_candidates`, `autopilot_marker`) down to L14; a branch with no
portal ends at the deepest marker reached. The start destination is the
journey's own room (`autopilot_path`). The open cell's chain is always a
prefix of it, so the target is re-derived every input frame as the marker
on the way (`Destination::on_the_way`), and on arrival the camera looks at
the room's center. Every window dive is `DiveMode::Targeted` at that
marker: crossed markers off the way never open, and scrolling out keeps
the destination, aiming at the marker just left. A click on the marker
already on the way keeps the destination; leaving free flight aims at the
room below the nearest portal; saved views store the destination. The
travel-ray re-lock (`portal_along_ray`, `RELOCK_CONE_COS`,
`transition_ray`) is removed; the #403 heading and up carry stays.

## Consequences

The headless replays are untouched (`replay_autopilot`, `replay_free_leg`
keep `Targeted` and their own seeded picks), so `verify.txt` and
`capture.txt` stay byte-identical (`E-REPLAY-CORE`, `E-GOLDEN`).
`DiveMode::Passing` stays in core for its tests, but no window input uses
it any more. A click generates up to thirteen cells once, the same work as
`autopilot_path`; the per-frame check reads the path in place without
allocating (`E-HOT-NOALLOC`). Up and roll, landing, and ground clearance
remain the continuous-camera notion's work.

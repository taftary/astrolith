# 0045 — Dive orbit controls: orbit, turn, planet-centre target

- Status: accepted
- Date: 2026-10-20
- Issue: #873 (Spec v2)

## Context

In dive mode the camera only moved along the line of sight (wheel + `DiveIn`/`RiseOut` arrows in `universe-render`, `dive_step` in `universe-core`). The heading eased toward the marker on the way, so the target stayed framed during zoom. There was no lateral move and no voluntary look-away in dive mode; free look lived only in free flight. The owner wants (original notion, plus two follow-ups 2026-10-20): orbit up/down/left/right on the same radius with gaze locked, turn in place while looking at the target, no pass-through when the dive depends on a target behind a planet, planet-centre target from L10 down, and a 10 m move-to-player.

Owner decisions: Q1 WASD + drag orbit in dive (arrows keep zoom); Q2 turn in place, camera does not move; Q3 slide around the body; Q4 planet centre default from L10 on, player only when chosen; Q5 choosing the player only moves to him, then dive works around the centre; Q6 10 m above him; Q7 zoom stops 3 m above ground; Q8 L11-L14 included (still numbered but unreachable, #742).

## Decision

- **Orbit (core, pure).** `dive_orbit` rotates the camera offset around the target centre; distance preserved, gaze locked, up carried round. Rate `k × speed × gap / distance` (`DIVE_ORBIT_RATE`), so screen speed is even at every distance (C2). Optional 3 m floor lifts over hills.
- **Turn (render).** `Navigation.roll` holds a roll around the view axis; right-drag changes it via `advance_roll`, applied in `sync_camera` with `roll_up`. Camera never moves, target stays centred, turn stays.
- **Inputs (render).** WASD orbit in dive; left-drag past 4 px orbits (click without drag still picks); right-drag turns; one-finger touch drag orbits, tap picks. Look sensitivity, invert look and Dive orbit speed apply.
- **No pass-through (core).** Dive, autopilot and orbit steps whose segment would enter the L10 planet slide along it (`slide_around_sphere`, clearance `PLANET_CLEARANCE_SHARE`); never inside (C4). (#892, ADR 0048: the avoid ball now sits below the lowest ground, not at the old sea level, and the centre dive and orbit measure their gap from the real ground.)
- **Planet centre from L10 (core + render).** Default target is the L10 open-cell origin; ground clamp stops zoom at 3 m above relief (`GROUND_CLEARANCE_M`, `clamp_above_ground`, cached frame/relief, no per-step rebuild); orbit uses the same floor. `PlayerSpot::arrival_pose` is 10 m straight above along planet up, looking down (`PLAYER_ARRIVAL_ABOVE_M`, replacing #607 Q3 A); after the glide focus clears so the target is the centre again and zoom-out rises straight up. Replaces #607 Q4 A (player default).
- **Setting S1 + guides G1/G2.** `dive_orbit_speed` 0.5–2.0 default 1.0, saved, Reset; Help > Navigation + Touch texts appended word for word.

## Consequences

- `universe-core`: `dive_orbit` (orbit/turn/slide/ground clamp/clearances), `nav` (cached frame/relief, planet avoid + ground floor), `player` (10 m arrival), `settings` (speed), `guides` (G1/G2).
- `universe-render`: `input` (orbit/turn, thresholds, speed), `camera` (roll), `player` (glide 10 m, focus clears), settings rows + UI, `control_tests` (WASD/drag/turn/touch).
- #607 answers superseded (noted, tests updated): Q3 A behind-and-above → 10 m above; Q4 A player default → planet-centre default.
- Headless: `cargo test` (including `dive_orbit_keeps_distance`, `dive_orbit_speed_scales_with_gap`, `dive_turn_keeps_camera_and_target`, `dive_never_enters_a_planet`, `planet_center_is_the_dive_target`, `player_move_ends_ten_metres_above`, `dive_orbit_speed_setting_round_trips`, `guides_explain_dive_controls`, `touch_drag_orbits_in_dive`) + `VERIFY-OK`; screenshots via shot file, validator takes no pictures.

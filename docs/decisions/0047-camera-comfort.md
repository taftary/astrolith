# 0047 — Camera comfort: dive turn, spaceship free mode, start target

- Status: accepted
- Date: 2026-10-21
- Issue: #884 (Spec v2)

## Context

Dive turn kept spinning after release, free mode was hard to use (pitch
stops top/bottom, planet gravity turned the view), and the start target
was confusing (camera target planet, selected target player).

Owner decisions: Q1 A dive turn stops on release, smooth, no drift; Q2-Q3
A free mode like a spaceship (no pitch limit, full loop, gravity never
turns view, Q/E roll to level); Q4-Q6 B target always planet, player
selected or not, Space starts journey on press, journey flies to planet
then 10 m above selected player, then works around planet centre; Q7 A
Q/E roll, exposure moves E to B; Q8 C P toggles, click elsewhere
deselects.

## Decision

- **Dive turn stops (core + render).** `dive_orbit::ease_roll` eases the
  drawn roll toward the dragged target over `ROLL_TAU_S` (50 ms), snaps
  within 1e-9 rad, no overshoot, holds after release. `Navigation` keeps
  `roll` (target) and `roll_drawn` (drawn); `sync_camera` keeps the
  unrolled base up (`next_up` toward local up) and derives the drawn up
  via `roll_up(base, roll_drawn)` without writing back.
- **Spaceship free mode (core + render).** `flight::free_rotate`
  (yaw around own up, pitch around own right, no clamp, full loop),
  `free_roll` (around forward), `free_right`, `free_step_axes`
  (movement along view axes, surface-distance rate). `Flight` keeps
  `free_forward`/`free_up`; entering free copies the current view
  (dive drawn up), leaving resets dive roll to avoid double. Camera
  copies free pose with no gravity easing. Saved views store free pose
  (forward + up) with old yaw/pitch files still loading.
- **Keys (core + render).** New actions `RollLeft` (Q) and `RollRight`
  (E); `Exposure` moves E to B (Shift+B brighter). `Settings::parse`
  migrates an old `key.exposure = KeyE` (with roll still on E) to B,
  keeping a player's own rebind. Keys page shows them like every action.
- **Target and selection (render).** `PlayerState.selected` (true at
  start) replaces "player is the destination". Destination is always the
  planet's anchored chain when selected (off-way rises back), seeded
  otherwise. Space with selection flies to planet then glides 10 m above
  him (`arrival_pose`), then orbit/zoom work around planet centre with
  him still selected. P toggles, click elsewhere clears (in
  `handle_input`), Go selects, Reset reselects seeded.
- **Setting S1 + guides G1-G3.** `free_roll_speed` 0.5-2.0 default 1.0,
  saved, Reset; `free_roll_rate(speed)` multiplies the base
  `FREE_ROLL_RATE`. Help > Free flight and Navigation ends updated word
  for word; no Guides page names E for exposure.

## Consequences

- `universe-core`: `dive_orbit` (roll easing), `flight` (free
  orientation + movement + roll rate), `keymap` (roll actions, exposure
  B), `settings` (roll speed, free pose, exposure migration), `guides`
  (G1/G2).
- `universe-render`: `input` (free look/roll/movement, P toggle, click
  deselect, handoffs), `camera` (base + drawn roll, free pose),
  `player`/`player_state` (selection, glide on selected), settings rows
  + UI, `views_ui` (free pose), `control_tests` unchanged.
- #873 answers superseded where noted (turn accumulation, free pitch
  clamp + gravity ease, exposure E, player destination); tests updated,
  old file formats still load.
- Headless: `cargo test` (including `dive_turn_stops_on_release`,
  `free_look_loops_and_ignores_gravity`, `free_roll_with_q_and_e`,
  `exposure_moves_to_b`, `space_flies_planet_then_player`,
  `p_toggles_player_selection`, `roll_speed_setting_round_trips`,
  `guides_explain_free_flight_and_selection`) + `VERIFY-OK`; screenshots
  via shot file, validator takes no pictures.

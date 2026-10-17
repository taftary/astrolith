Agent-authored draft at owner request 2026-10-18. Owner asked for a notion about player display and better player controls, starting from what exists today and what needs refactoring.

## What we have today — player display

- Seeded spawn (`crates/universe-core/src/player.rs`): `PlayerSpot` is the named chain of the body's cell plus feet position and yaw. `seeded()` picks a 9-deep chain to L10, a point within `SPAWN_REACH` (0.25) of the cell middle on the ground, with a hashed heading. Same seed gives same spot. `destination()` is anchored; `arrival_pose()` is behind (`ARRIVAL_BACK` 3.0 body heights) and above (`ARRIVAL_UP` 0.6) the eye, looking at it. Saved in settings as `player = seed, x, y, z, yaw, chain`.
- Waiting body: outside normal mode the body waits at the player's spot and is drawn while its cell is open (`draw_body` in `crates/universe-render/src/normal_mode.rs`). When the dive reaches the cell, `player_glide` (`crates/universe-render/src/player.rs`, `GLIDE_RATE` 3.0, done at 1e-3 of a body height) glides the camera to the arrival pose and shows it looking at the body.
- Active body: in normal mode the body is an outline person — 8 gizmo line segments (spine, shoulders, 2 arms, 2 legs, hip bar, facing tick) plus a head sphere, in `FOCUS_TINT`, true size 1.8 m (`body_height`, `body_segments`). No mesh, no lighting response, no animation.
- Cameras: first person from the eye, third person 3.0 back + 0.6 up (`THIRD_BACK`, `THIRD_UP` — same numbers as arrival, defined twice). V toggles; Starting camera setting picks the default (third today). Path offset is the eye, so open/close/preview rules read the body.
- Extras on screen: faint pull arrow toward the strongest gravity (0.8 heights, `Show pull` setting, #559) plus a `pull: <name> <strength>` HUD line; top-left mode reminder (`Dive/Normal/Free - 1 normal - 2 free - 3 dive`); bottom-left scale HUD unchanged.

## What we have today — player controls

Defaults (`crates/universe-core/src/keymap.rs`, all remappable, one key each, swap on conflict):

- 1 normal, 2 free, 3 dive, F switch free/dive, Up dive in, Down rise out, Space autopilot in dive / jump and double-tap fly in normal, P go to player, WASD move, Shift run or boost, C descend while flying, V switch camera, E darker (Shift+E brighter), X debug axes, Esc menu, Left-click pick, Right-drag look, Ctrl+1-8 save view, Alt+1-8 recall view.
- Normal-mode feel (`body.rs`): walk 1.4 m/s, run 4.0, fly 10, boost 30, jump 3.2 m/s (~0.5 m high), all scaled to level units so pace is constant; drone turning default (A/D strafe), plane turning optional (A/D roll and bank-turn); speed steps Shift+1-9 multiply everything (5 = 1x, 1 = 1/16, 9 = 16x).
- Special flows: 1 far away flies the dive to the player first then hands over the body (`takeover`); 1 near takes over in place; leaving normal mode saves the spot (`Player::leave`); Views page has Go to player and Reset player buttons; touch mirrors keys (stick + Jump/Down buttons, #561).

## What needs to be refactored

- Display is placeholder: line-figure + sphere instead of a readable player avatar; active-body path and waiting-body path draw the same figure through different code; third-person offsets defined in two places (`player.rs` arrival vs `normal_mode.rs` camera) and can drift.
- Camera handoff is split: `player_glide`, `camera_pose`, `sync_camera` each own part of the pose; first/third, dive-look easing, and ground-up interact in three files.
- Controls are overloaded and hard to discover: Space does three things (autopilot / jump / fly toggle via double-tap 0.35 s); digits do three things (modes vs speeds vs views with Shift/Ctrl/Alt); 1 behaves differently near vs far; plane vs drone, hold vs toggle dive/look, invert look live in Settings but are invisible in the moment.
- State is scattered: `Player` (spot/seeded/gliding/takeover) + `NormalState` (body/active/third_person/pull) + `Flight` (mode/yaw/pitch) + `Navigation` (destination/focus/target) + `SettingsStore`. Save-on-leave vs follow-on-enter ordering is subtle.
- Physics/mode edges: body starts flying where there is no ground; double-tap vs single-press Space races; walk-into-place opens nothing by design but feels like missing collision; pull arrow + readout duplicate the same pull.
- Ask before spec: what should the player look like (avatar vs marker figure)? Should first/third stay, or one camera? Which overloads stay (double-tap fly, digit modifiers), and what does touch parity require? What is minimal for the refactor — display only, controls only, or the handoff/state cleanup together?

# Real-input scenarios

Opt-in testing interface for #1227 (ADR 0056), extended by #1238 (ADR 0057).
This is not teleportation.
Normal startup installs no scenario driver or observation collector.

## Run on Windows

Build the release app from the checkout being tested:

```powershell
cargo build --locked --release -p universe-app
python scripts/validation/scenario_run.py --scenario scripts/validation/scenarios/planet.scenario --out .agent/validation/issue-1227/20261106-owner-planet --timeout 180
```

Use a **new** output directory on every run. The runner refuses existing
directories, launches the release executable directly (no shell), bounds its
lifetime, and reaps only its own process. It never drives other desktop apps.
The measurement lock is shared with the profiling runner: run these serially.
`--width`, `--height` and `--workers` default to 1600, 900 and 5.

The game uses default settings in memory, without a persistence plugin. The
runner additionally assigns a disposable `APPDATA` below the output directory
and fingerprints the real settings file before and after. An unchanged
fingerprint is required for success. Do not interact with the test window
while it runs: physical input still reaches the normal input handling too.
Vulkan loader defaults match `.cargo/config.toml` (ADR 0042), preserving an
explicit environment override; prototype/profiling switches are disabled.

Evidence is retained for both positive and negative cases:

- `manifest.json`: executable path/SHA-256/mtime, fixture SHA-256, source
  commit and tracked changes, exact arguments, process result, isolation
  fingerprints and final `complete`. The executable hash is authoritative;
  the source commit alone does not prove which source built a binary.
- `fixture.scenario`: exact input copied before launch.
- `stdout.txt`, `stderr.txt`, `process.json`: application logs, exit code,
  PID, elapsed time and timeout/cleanup result.
- `app/scenario.tsv`: streamed setup, action trace, named observations and
  terminal outcome, flushed each frame. Parse failures retain this file too.

Success requires process exit 0, final `SCENARIO-OK` in stdout and the trace,
completed setup/action/terminal records, and unchanged real settings. A closed
window, incomplete run, timeout, crash, missing control or unavailable required
observation cannot succeed. Scan **all** logs as well; an OK does not waive
unexpected engine diagnostics. Existing owner-accepted Vulkan messages are
listed in `validator-capabilities.md`.

The lower-level CLI is `universe-app --scenario FILE NEW_DIRECTORY` followed
optionally by `--size WxH`, `--scale F`, `--build-workers N`. It also uses
in-memory defaults without settings I/O, but the Python runner supplies the
executable fingerprint, process isolation and outer timeout. Invalid CLI usage
exits 2; malformed scenarios and unsuccessful runs exit 1.

## File contract

### Read-only moon catalog (#1238, ADR 0057)

The headless developer command below lists unchanged generated moons, with no
window or settings I/O. No arguments uses the same near-home prefix as
`--list-worlds`; explicit arguments name a universe seed, existing dot-separated
portal prefix (or `root`), planet-search limit (1–4000) and moon-output limit
(1–64000). A full nine-index prefix selects one existing planet.

```powershell
target/release/universe-app.exe --list-moons 42 root 16 64
```

`MOON` tab-separated records retain seed, full host-planet and containing-system
identities, marker, actual type/radius/orbit/period and production paint seed.
Object-kind evidence, catalog-based/procedural instance origin, synthetic/
procedural terrain and supplied/generated property source are separate fields.
Home inputs are supplied reference constants, not newly measured values.
`MOON-BOUNDARY` is separate boundary-only data: the procedural radius domain is
200 km inclusive to 1700 km exclusive. It has **no** generated identity and is not
a sampled moon. A bounded search is not the universe's absolute minimum.
The terminal `MOONS-OK` records limits/count; a nonexistent prefix returns
`MOONS-FAIL` and exit 1, malformed CLI exits 2. Catalog records are not contact,
player travel, persistence or independent capability proof.

### Scenario-v1

UTF-8, at most 128 KiB, at most 2048 action lines. Blank lines and text after
`#` are ignored. Headers are required in this order, once each:

```text
scenario-v1
seed 42
setup planet
timeout 120
observe start
key KeyW down
hold 0.5
key KeyW up
hold 1
expect player.position delta start 0.1 20
observe moved
```

`timeout` bounds the entire window run, including setup, in wall-clock seconds.
Keep the runner's outer timeout larger to retain the app's own terminal reason.
Every duration is finite and in `(0, 1800]`. One ordinary action is issued per
frame; clicks use pointer, press and release phases. Timing is real, not a
fixed-frame replay: compare movement with tolerances, not byte-identical traces.

Fixtures (all use the actual generated universe and real player body):

| `setup` | Context |
| --- | --- |
| `planet` | Home L10 planet, initially 2 m above relief, waits for production drawn-ground settling |
| `planet-other` | Solid L10 planet in a different galaxy from home, same settling rule |
| `near-moon` | Flying at 10 m above an actual moon's reference sphere, not grounded |
| `system` | Within-system space, L8 |
| `interstellar` | Between-system space, L6 |
| `intergalactic` | Between-galaxy space, L3 |

Seed 42 is the checked-in fixture set. Other unsigned seeds are accepted but
may lack a suitable planet, moon or second galaxy and fail setup explicitly.
Celestial time starts at zero and paused; the player and production input are
not paused. Setup opens cells and initializes body/camera context **once**.
`PHASE setup` and `OBSERVE setup` precede `PHASE actions`. No measured action
directly assigns player coordinates or calls an internal gameplay command.

### Actions and checks

| Line | Meaning |
| --- | --- |
| `key KeyW down` / `key KeyW up` | Ordinary keyboard press/release; names from core `BINDING_NAMES`, including `Escape`, `KeyF`, `Shift` |
| `button MouseRight down` / `button MouseRight up` | Ordinary mouse press/release |
| `pointer X Y` | Cursor in logical window pixels, inside the window |
| `motion DX DY` | Relative mouse-look motion in pixels |
| `scroll X Y` | Wheel lines; negative Y scrolls down |
| `click ID` | Resolve one visible laid-out button; normal engine hit-testing owns the press |
| `hold SECONDS` | Leave held inputs alone while the real game updates |
| `observe NAME` | Record a read-only snapshot and a baseline; unique names, maximum 64 |
| `expect FIELD eq VALUE` | Exact string equality |
| `expect FIELD present` | Available observation exists |
| `expect FIELD range MIN MAX` | Inclusive finite numeric bounds |
| `expect FIELD delta BASELINE MIN MAX` | Inclusive Euclidean vector-distance bounds, in the reported units |
| `wait FIELD eq VALUE SECONDS` | Retry a check until met or bounded timeout; also accepts `present`, `range` and `delta` checks |

Identifiers contain letters, digits, dot, dash or underscore (maximum 128).
UI IDs come from actual entity `Name`s with other characters replaced by `-`:
`toggle-Debug axes` becomes `toggle-Debug-axes`. IDs are case-sensitive.
Ambiguous, invisible, clipped/off-window or missing buttons are refused.
Occluded buttons must actually receive the press or the click fails. Targeting
does not assign Bevy `Interaction`, select a page, or run a button callback.
Input messages are injected before the ordinary engine `InputSystems`; UI
focus, key bindings and gameplay handlers process them normally.

Missing values, including the explicit `unavailable` sentinel, cannot satisfy
checks (even `eq unavailable`). Vector deltas require matching `.frame`
metadata and refuse cross-cell subtraction; assert identities separately when
travel changes the reference frame.

## Scenario-v2: selected starts and explicit preparation

V1 is unchanged. V2 keeps the same seed/timeout headers and ordinary input/check
commands, but requires a selected or no-placement start. All selected start
parameters are explicit, with no default body substitution:

```text
scenario-v2
seed 42
setup moon 7.2.7.1.2.0.0.0.5 1 1 1 0 10 86400 normal 20 -15
timeout 120
observe approached
key KeyW down
hold 0.4
key KeyW up
prepare pole moon 7.2.7.1.2.0.0.0.5 1 0 1 0 10 86400 normal 20 -15
observe prepared-pole
```

- `setup moon HOST ID DX DY DZ GAP_M TIME_S MODE YAW_DEG PITCH_DEG`: full
  nine-index host chain, actual moon marker, nonzero finite direction normalized
  in the moon proxy's local untranslated host-cell axes, nonnegative reference-
  sphere gap in metres, nonnegative shared game seconds, `normal`/`free`/`dive`,
  and look degrees in the existing production heading basis. Pitch respects the
  existing normal-mode limit. The initial clock is paused. This does not install
  moon gravity, a moon walking frame, ground or a look-up correction.
- `setup planet HOST DX DY DZ GAP_M TIME_S MODE YAW_DEG PITCH_DEG`: planet-fixed
  direction and offset above production relief, **not** measured collision
  clearance. Solid normal-mode starts still wait for actual drawn-ground settling.
- `setup loaded`: observes ordinary startup without opening a chain, placing the
  player, changing clock/mode or writing preferences. The seed header does not
  replace production-loaded state. In the default memory-only runner this is
  **not** a save/restart test: there is still no production file I/O bridge.
- `prepare NAME moon ...` / `prepare NAME planet ...`: up to 64 unique named
  preparation phases, only in the Universe screen. Release held inputs through
  normal InputSystems before placing once; wait for this scene's readiness before
  the next measured action. `loaded` is an initial start, never a preparation.

The trace marks `PREPARE`, `PHASE preparation`, `PLACEMENT`, the prepared snapshot
and the following `PHASE actions` separately. Initial and later preparation is
explicitly `counts_as_travel=false` and `include_in_resource_peaks=true`. Each
phase remains within the total timeout; it does not clear resource history or
change quality/preferences. A reset's seeded player location remains the real
production seeded location, not a selected test approach. Free/dive release the
prepared body through the ordinary production leave/save-to-store path.

Prepared fixture identity/type/radius/orbit/period/origin/source and requested
direction/basis/gap/time are retained separately from actual player, camera and
clock observations. Requested fixture parameters never stand in for measured
support or contact. Only input-driven travel throughout can support a continuous
journey claim; an explicit preparation suite cannot. Trace protocol remains v1;
`fixture_version=2` identifies the extended fixture grammar.

## Observations

The TSV uses `OBSERVE NAME step=N elapsed_s=T`, then
`VALUE NAME FIELD VALUE` rows. Observations are read after production update,
camera sync, layout and visibility, not copied from expected scenario values.

| Fields | Meaning |
| --- | --- |
| `seed`, `player.id` | Actual universe seed and one-player identity scoped to it |
| `player.path`, `.galaxy`, `.system`, `.planet`, `.level` | Actual marker-chain context; `unavailable` when a containing scope is not applicable |
| `player.position`, `player.position.frame`, `.units` | Authoritative active body's feet or parked spot, in metres in the named seed/open-cell frame |
| `player.active`, `.mode`, `.flying`, `.on_ground`, `.third_person` | Actual player/mode state; flying/grounded flags apply to the active body |
| `player.surface_clearance_m` | Signed feet-to-current-drawn-ground normal distance when that collision surface is available |
| `player.moon`, `player.moon_state`, `moon.reference_sphere_gap_m`, `moon.gap_kind` | Actual nearby moon marker, hovering state from the real flying/grounded flags, and geometric separation from its reference sphere, **not** collision/landing evidence |
| `camera.path`, `camera.position`, `.frame`, `.units` | Rendered camera origin, separate from player coordinates |
| `camera.forward`, `camera.forward.frame` | Rendered camera unit direction; a delta is a direction-vector chord, not degrees |
| `ready` | Camera exists plus applicable fixture ground settling or moon mesh/body readiness |
| `fixture.kind`, `fixture.home.path`, `fixture.other.path` | Prepared fixture and stable planet identities in distinct galaxies |
| `player.region`, `fixture.home.visited`, `fixture.other.visited` | Scenario-local region history, derived from actual player presence, never camera-only browsing or a new discovery system |
| `screen`, `panel.selection`, `panel.expanded.ID`, `panel.scroll`, `panel.list_scroll` | Actual current screen, selected/expanded panel nodes and scroll offsets in logical pixels |
| `ui.ID.visible`, `.actionable`, `.interaction`, `.center` | Unique live UI node visibility, button hit-target status, actual Interaction and center in logical window pixels |
| `settings.labels`, `settings.debug_axes` | Actual settings values; use to check a real toggle result |
| `settings.fullscreen.enabled`, `settings.vsync.enabled` | Actual pending-window guard availability |
| `clock.game_s`, `clock.playing` | Actual production clock, not requested fixture time |
| `fixture.selected.planet`, `.system`, `.moon`, `fixture.moon.*` | Permanent prepared identity and unchanged generated type/size/orbit/provenance, not supporting-body identity |
| `fixture.request.*`, `fixture.direction_basis`, `fixture.placement_origin` | Explicit preparation inputs and their named basis; never measured contact or player-up |

`.actionable` means a visible button hit-target, **not** universal business-rule
availability: a widget may be busy or auto-managed. Check feature-specific
availability/value fields too. A missing/duplicate/hidden UI node has no row;
absence is not reported as a fabricated false value. Panel expansion rows are
emitted only for expanded IDs. Visitation starts empty for each process; reaching
the prepared planet counts as visiting that planet, while hovering near its moon
does not. It does not imply the destination has been visited in the real save.

For future UI features, add read-only fields from their real state in the
observer, after their production systems. Do not add expected state to the
driver. Mini-map browsing, destination selection, confirmation availability and
teleport behavior must be implemented and tested in #1226, not inferred here.

## Representative suite

Run checked-in files under `scripts/validation/scenarios/` serially, each with a
new output directory. Positive: `planet`, `planet-other`, `near-moon`, `system`,
`interstellar`, `intergalactic`, `menu`, `scroll`. `planet` moves the body with W,
then releases it using F and operates mouse-look; the parked player stays still
while the camera direction changes. `menu` observes navigation, selection and
a real Debug axes toggle without moving the player. `scroll` checks a real wheel
change in the Keys panel.

Expected non-success: `missing-control` (absent button), `unmet` (wrong screen),
`malformed` (unknown key, before window launch), `timeout` (unmet bounded wait).
An outer-timeout check can use `--timeout 0.1`: expect `process: timeout`, retained
logs, `complete: false`, and unchanged settings. Reusing an output directory
must fail without modifying existing evidence.

V2 positive procedures: `selected-moon` (seam approach at a chosen shared time,
ordinary body flight then camera look), `preparation-phases` (pole/seam/planet,
held-input release and actual planet walking), `loaded-start` (no-placement
memory-only startup plus ordinary menu input). Repeat `selected-moon` and compare
its initial identity/clock/position records, not wall-clock timing. Negative:
`invalid-moon` (absent permanent marker, before window), `missing-moon-ground`
(required drawn-ground observation unavailable; hovering cannot satisfy it).
These are developer procedures; independent proof is recorded only in the registry.

## Limits

- Windows runner and local real graphics adapter only; no OS-focus automation.
- Moon hovering is proven separately from planet ground: no moon collision,
  standing, landing or walking support is introduced or claimed.
- The reference-sphere moon gap is geometric, not the exact faceted mesh or
  collision distance. It is deliberately labeled, and is not surface clearance.
- No teleport/mini map or persistent discovery system exists in this interface.
- Read `validator-capabilities.md` for independently demonstrated capabilities;
  a main-agent run alone does not establish independent validation.

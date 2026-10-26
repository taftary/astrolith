# 0052 - The sky's clock: moving and spinning bodies

- Status: accepted
- Date: 2026-10-26
- Issue: #981 (Spec v2)

## Context

The star system (ADR 0051) stood still: planets kept their generated orbit angles and the time of day on a planet stayed fixed (#780 Q5 A). The owner asked for a timeline: play, pause and speed buttons at the top middle, planets moving, and the player's place following that movement, with care for performance.

Owner decisions:
- planets go around their star, spin with their day length and tilt, and carry their moons (Q1 C);
- at 1x one Earth day takes ten minutes (Q2 B);
- speeds 1x, 10x, 100x, 1,000x and 10,000x (Q3 A);
- the game starts playing, and pause stops only the sky (Q4 A);
- riding and handovers as settled by #995 (Q5, Q6);
- a home calendar readout, saved on quit (Q7 A);
- the bar hides with the mode button's Display switch and is tap-friendly (Q8 A);
- no frame-time or memory cost (Q9 A);
- keys T, [ and ] (S1), and the Help > Navigation text (G1, G2).

## Decision

- **One clock.** `clock::Clock` holds game seconds, the speed step and play/pause. `GAME_PER_REAL` (144) turns real seconds into game seconds at 1x. The window keeps it in `sky::SkyClock` and ticks it once per frame. Every place is a pure function of game time: `clock::orbit_angle_at` and `clock::spin_angle_at`.
- **Moving in place.** `Universe::advance` rewrites the positions of the open and parent cells' moving bodies (`sky_motion::Mover`, rebuilt only when a cell regenerates). Planets go around their star in L8 and L9 in the ecliptic, prograde (the same turn as their spin). Moons go around their planet in L10.
- **Riding.** An open planet's entry (`frame::Opened`) follows its planet (`MarkerPath::move_entry`), so a camera inside the planet's cell keeps its offset and rides along. Outside every zone the camera stays still; `Universe::sync_zones` opens a planet whose moving zone reaches it, with the same exact origin shift as a dive (ADR 0051).
- **Spin.** `Opened` carries a `spin` angle about a unit `axis` (the planet's pole, tilted from `+Y` toward `+X` by its tilt). Directions and points cross the entry through that turn, so the ground stays put in the planet's cell while the star, the planets and the moons turn in its sky, and the one light (`light::sun_direction`) follows. A planet opened later in the day is turned at once, keeping the camera's place (`MarkerPath::spin_entry` with `keep_place`).
- **Time bar.** `time_bar` draws play or pause, slower, the speed, faster and `Day N, HH:MM` at the top middle (a second row on phone-width windows). Presses resolve by cursor math and claim the click (the mode switch's claim), so the game never picks behind them. `Action::TimePlay`, `TimeSlower` and `TimeFaster` (T, [, ]) are rebindable.
- **Saved time.** The settings file keeps `game_time`. `sky::restore_sky_time` starts the clock from it; `sky::keep_sky_time` writes it back on quit and every 30 real seconds; the settings save runs in `Last`.
- **Checks.** `--verify` adds `RIDE` (a month on the planet, the zone sweeping over a still camera), `ORBIT` (a quarter period and half a moon period) and `CLOCK` (start, one day at 1x, a quarter day's sun turn). The journey and every older line run at time zero, unchanged. Shot files take `time=H`; `--timing` runs the sky at 10,000x.

## Rules amended

- ADR 0051: positions inside a star system become functions of time; the gravity-zone handover is unchanged and is also reached by a moving zone.
- #780 Q5 A (fixed sun): replaced. The sun moves with the planet's spin and orbit.
- `E-GOLDEN`: `verify.txt` gains the `RIDE`, `ORBIT` and `CLOCK` lines; every older line is byte-identical. `capture.txt` is unchanged.

## Consequences

- At 10,000x a day passes in under a tenth of a second, so the light flickers; the speeds are the owner's choice (Q3 A).
- The planet's poles in the game's lat/lon (`+Y`) are not its spin poles once tilted: the sun's path follows the tilted axis, and seasons come with the year.
- Stars, galaxies and L1-L7 stand still; collisions are #993.

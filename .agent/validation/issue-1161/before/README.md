# #1161 T1 (#1203): before evidence

- **Code:** `task/1203-before-tools` at 7005cbd: main plus T2's classifier and the T1 tools. None of them changes what is drawn.
- **Program:** `exe.sha256` (46ee6f4f…). An earlier batch was discarded because its build had picked up unfinished crater code (its fingerprint b7faec5f… differed from a clean rebuild). This batch ran on the clean build.
- **Repeat:** `bash .agent/validation/issue-1161/fixtures/batch.sh before`, then `py .agent/validation/issue-1161/fixtures/tables.py before`.
- **PC and settings:** Intel UHD 620, Vulkan, 60 Hz, AC power on Balanced, idle.

## Worlds
`worlds.txt` is the `--list-worlds` output: 4,000 worlds under the home L3 cell. The fixtures, one per type, are listed in the header of `fixtures/surfaces.shots`:

| fixture | type | chain |
|---|---|---|
| home | Earth-like | 7.2.7.1.2.0.0.0.5 |
| mercury | desert, airless (the only airless planet in the listing) | 7.2.7.1.2.0.0.0.0 |
| desert | desert | 7.2.7.2.3781.1.0.0.0 |
| iron | iron | 7.2.7.9.1294.1.0.0.1 |
| ice | ice | 7.2.7.1.303.1.0.0.1 |
| lava | lava (the only lava world in the listing) | 7.2.7.5.4149.1.0.0.0 |

Each world is shot at its local noon at lat -30 lon 60 (the pole at its own best sun) and at midnight. The times come from the sun's height (`light::sun_direction`).

## Pictures
`pictures/` (`SCREENS-OK files=39`) and `before-sheet.png`. The sheet has one row per world: plain, mountain (3 km up), pole, cube edge (lat 0 lon 45), orbit, night. Its last row is the home moon from space and from the planet.
- **Ground colour:** every type has the same pale, uniform ground (`RELIEF_GROUND_TINT`). No world shows rock, sand, ice, metal or crust.
- **Craters:** Mercury shows its continents and ranges, but no craters.
- **Lava night:** the lava world's night side is dark, with no glow.
- **Home moon:** a smooth grey ball.

## Speed and memory
`tables.md` holds every scenario's interval, render and CPU means, the stalls, the readiness, and the resources and peak working set for each route:
- `surfaces-fixed`: Normal everywhere. It visits all six worlds, so it doubles as the several-world memory route.
- `surfaces-terrain-low` and `surfaces-terrain-high`: S1.
- `surfaces-auto`.

No run has a frame over 100 ms after the warm-up.

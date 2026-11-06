# #1162 before evidence (T1, #1216)

- **Program:** the f943143 release build kept by the preflight. Its SHA-256 is in `exe.sha256` (ffc9fc87…), and the same program is used for the paired runs in T7. `commit.txt` names the T1 branch head the batch ran from; the code in the program is f943143's.
- **Run:** `bash .agent/validation/issue-1162/fixtures/batch.sh before <that program>`.

## Places
`water-home.txt`, `water-ice.txt` and `water-desert.txt` come from `universe-app --list-water <chain>` (T1). They list coast, shallow, deep and polar places by the terrain's sea line at 0 m. The desert world has the same kind of low ground, but it stays dry.

## Pictures
`pictures/`, from the shot list `../fixtures/water.shots`, with a contact sheet in `before-sheet.png`.
- **Home:** coast A from orbit and on the way down (200 km, 30 km, 3 km, 30 m, the beach). Also the coast at night, a shallow place, and a polar sea.
- **Ice world:** a coast and a polar sea.
- **Desert world:** a low coast.

Today no water is drawn. Every view shows bare ground where the sea floor drops away below the sea line.

## Timing and memory
These four runs use the routes in `../fixtures/`:
- `water-fixed`: fixed Normal quality, and also the several-world memory route. It visits the wet worlds (home, ice) and the dry desert world, then visits each one again.
- `water-terrain-low` and `water-terrain-high`: Terrain quality S1.
- `water-auto`: Auto quality.

Each run's manifest reads `complete` true and `settings_unchanged` true, with executable ffc9fc87…. T7 compares these runs with the after runs and repeats both builds in pairs.

## Frozen heights
`crates/universe-core/src/relief_golden_1162.txt` was written from this code with `UPDATE_GOLDEN_1162=1`. It holds 1,500 terrain heights for each of the six #1161 fixture worlds.

---
name: visual-validation
description: Load when a pull request contains any user-visible UI change (part of workflow stage 10). Use to check rendered UI against the spec on the PR preview.
---

# Visual Validation

Required for any user-visible UI change, optional otherwise. Runs inside `technical-validation` before the validator gate, per `docs/workflow.md` (Validation Layers). This product is a native window: there is no PR preview URL. Evidence is real-window screenshots from `cargo run --locked -p universe-app -- --screens <dir>` (the shot list in `crates/universe-render/src/screens.rs`), taken by the main agent at the head commit. The validator takes no pictures (#533), so this check and the owner test are the only eyes on the window. When the change cannot be captured, record per-item `owner-test-only` on the Issue and skip the rest.

## Shot groups

`--screens <dir> <groups>` runs only some shots (#847, ADR 0043). `<groups>` is a comma list of `smoke`, `menus`, `planet`, `sky` and `walking`; with no list, every shot runs. Run the smoke set plus every group the change touches:

| The change touches | Run |
|---|---|
| Menus, settings, guides, the HUD or touch controls | `smoke,menus` |
| The dive's planet views, the patches, the ground's look, LOD or the camera above L9 | `smoke,planet` |
| Light, the atmosphere or the sky | `smoke,sky,planet` |
| The body, walking, the figure or near shadows | `smoke,walking` |
| Rendering in general, the backend, or anything unsure | every shot (no list) |

Name the groups run in the evidence comment.

## Shot files

`--screens <dir> --shots <file>` takes exactly the shots in a text file (#863), in order, and saves each as `<name>.png`. Use it when no built-in shot shows the change, or to capture exactly the views the spec names.

- **Line format.** One shot per line: a name, a level, then `key=value` words. Blank lines and `#` lines are ignored.
- **Place.** On the planet, `lat`/`lon` in degrees (`+Y` is the pole, longitude grows east) and `h` metres above the drawn ground (default 2). Or `x y z`: planet metres at L10, the level's own units above it.
- **Look.** `heading` (compass degrees, 0 north, 90 east) with `tilt` (degrees up), or `dir=a,b,c`.
- **Figure.** `figure` (planet only) stands the walking figure at the spot, with the camera behind it.
- **Daylight.** The sun is fixed, so a spot can be in night. The seeded start spot and the dive's arrival are in the morning (#780 F1 A).
- **Errors.** A wrong line stops the run before the window opens, naming the line.

```text
# name        level  where                          look
cliff-view    L10    lat=12.5 lon=-40 h=2           heading=270 tilt=-5
probe-xyz     L10    x=1200000 y=6500000 z=-300000  dir=0,-1,0
walk-ridge    L10    lat=30 lon=10 figure           heading=90
galaxy-view   L4     x=0.1 y=0.2 z=3                dir=0,0,-1
```

When a PR's pictures come from a shot file, upload the file with the evidence (to the same release) and name it in the PR comment, so the views checked are visible (#863).

## Inputs

- Head commit SHA plus `--screens <dir>` output (PNG shots + manifest), or the `owner-test-only` record link.
- Approved spec: test plan steps + expected results for every user-visible change.

## Steps

1. **Check each change.** For every user-visible change, look at the matching shots and compare what is seen against the spec's expected results. Add a shot to `screens.rs` when no existing one shows the change.
2. **Post the evidence.** Upload the relevant shots to the `validation-evidence` release and post them on the PR with one line each (shot, what it shows, matches the spec or not), naming the commit SHA checked. Reuse the best shots in the owner test steps.
3. **Handle discrepancies.** Fix small discrepancies on the branch directly (new commit, then re-check that item); list any remaining discrepancies explicitly on the PR so `technical-validation` does not proceed silently.
4. **Record and move status.** Reference the PR evidence comment from the parent Issue. This skill moves no status on its own; the transition is handled by `technical-validation`. End with the one-glance summary per `docs/board.md`.

## Outputs

- Screenshot evidence comment on the PR naming the SHA.

## Done criteria

- Every user-visible change checked against the spec's expected results on real-window shots (or `owner-test-only` recorded on the Issue).
- Evidence posted on the PR; discrepancies fixed (re-checked) or explicitly listed.
- Output referenced on the Issue and status handled per `docs/board.md`.

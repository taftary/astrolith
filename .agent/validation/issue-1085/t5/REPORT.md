# #1085 decision report: lightweight rendering investigation

**Short answer:** the frames are slow because of *how they are handed to the screen*, not *how much the game draws*. One fix tested well with VSync on: keeping one frame in flight instead of two. It looks identical and is much faster in many scenes. But it is slower with VSync off and in some Normal scenes, so it is **promising, not ready to switch on**.

## What we looked for

The plan was to make the ground cheaper to draw. The measurements (T2) showed that drawing was not the main cost on this PC. With VSync on, about 32 of a ~40 ms frame was spent **waiting to show the frame**: frames alternated between two and three screen refreshes. The GPU needed only about 12-15 ms.
- Atmosphere quality cost nothing measurable.
- Shadows (+7 ms) and bloom (+3 ms) cost the most, but both are already off by default.

So the candidate became frame delivery.

## What was tried (T3)

Eight ways to change when a frame reaches the screen.
- Only one changed anything: **`inflight1`**, which lets one frame be in flight instead of two.
- On ground with VSync on, it took frames from 39.9 ms to a steady 27.1 ms.
- The driver ignores the other knobs.

## What it does (T4, three to five alternating old/new pairs per scene)

**Pictures:** identical. 19 matched scenes differ by no more than two runs of the old version differ from each other. The scenes: day, dawn, dusk, moonlit and moonless night, first person, orbit, star system, second world, turns and descent.

**VSync on, Low quality: a big win where it counts.**
- Orbit, descent, turns, ground and ground night: about 30-40% faster on average, and 33-50% better on the slow frames (p95). Ground goes from 39.9 to 25.0 ms.
- Universe, system and the second world are unchanged.
- Cost: on ground, ground night and the second world, detail finishes loading about 0.14-0.21 s later (about 1.3 s → 1.45 s). That is just over the limit we set.

**VSync on, Normal quality: mixed.**
- Universe (−23% mean) and orbit (−14% mean) are faster.
- Descent, turns, ground, ground night and the second world are 8-14% slower on average.

**The old version is not steady with VSync on.** Sometimes it locks at about 40 ms per frame; that was every T2 run on ground. Sometimes it runs at about 26 ms, but keeps the main processor thread busy for almost the whole frame. The new version gives the same numbers every run and uses about half the processor time. On Normal it lost to the old version's *good* state.

**With Auto quality** (one check): Auto saw the faster frames and **raised the quality**. The new version still ran turns and ground faster (39.5 → 27.9 ms and 39.9 → 28.9 ms). Orbit kept the same speed at Normal instead of Low.

**VSync off: worse everywhere** (+10% to +30% average). It must never apply with VSync off.

**Long play and switching:**
- 20 minutes: no slowdown over time, with memory within 1%.
- Ten planet/quality switches: 9% faster, with the same mesh and material counts and no growth.

## Distance from the targets (VSync on, average frame time)

| | old | new | 16.7 ms (60 FPS desktop) | 33.3 ms (30 FPS phone) |
|---|---|---|---|---|
| Ground, Low | 39.9 ms | 25.0 ms | still 8 ms away | new is under |
| Turns, Low | 39.9 ms | 23.9 ms | 7 ms away | new is under |
| Ground, Normal | 25.7-39.8 ms (two states) | 29.3 ms | 13 ms away | new is under |
| Second world, Normal | 33.1 ms | 37.1 ms | 20 ms away | both over |
| System (both qualities) | 40.1 ms | 40.1 ms | 23 ms away | both over |

Neither version reaches 60 FPS with VSync on, on this PC.

## Mobile

**No phone or tablet was tested (#1054).** Nothing here shows a frame rate, heat or battery result on a phone.
- Phones' graphics chips usually depend on having more than one frame in flight. So one frame in flight could hurt there even more than it did here with VSync off.
- That is a risk to measure on a real device before any phone use.

## Settings and controls

- With throwaway settings in the real window (your settings file was untouched):
  - a normal launch is the old version;
  - Quality, frame-rate and exposure changes save and survive a restart;
  - Reset Display restores the defaults.
- The developer switch is never saved.
- No setting or help text changed.

## Verdict

**Promising** (Spec v1): better beyond noise in several scenes. **Not recommended for rollout as is:**
- slower with VSync off;
- slower in five Normal scenes against the old version's good state;
- detail loads a little later on Low.

## Next action (your choice)

- **A: one more bounded experiment.** One frame in flight only when VSync is on, and only when the old version is locked at 40 ms (it can detect that from its own wait times). Measured the same way. This targets the wins and avoids the losses seen here.
- **B: stop here.** Record the findings, close the investigation, and keep today's frame delivery.
- **C: something else** you name.

Any rollout would still need your visual confirmation and its own approved scope.

## Try it yourself

On this PC, from the repository folder, after `cargo build --release -p universe-app` on `feat/1085-lightweight-rendering`:
- **Old:** `target\release\universe-app.exe`
- **New:** in PowerShell, `$env:ASTROLITH_PROTOTYPE='inflight1'; target\release\universe-app.exe` (a line `prototype inflight1 in force` confirms it). Close the window and run `Remove-Item Env:ASTROLITH_PROTOTYPE` to go back.
- **Repeat the measurements:** `.agent/validation/issue-1085/t4/batch-t4.sh <log>` (about 7.5 h, with the PC idle). Pictures: `target\release\universe-app.exe --screens <dir> --shots .agent/validation/issue-1085/t5/paired.shots`, once with the switch and once without.

Evidence: `t2/README.md` (diagnosis), `t3/README.md` (prototype), `t4/README.md` (comparison), `t5/README.md` (pictures, real window, criteria).

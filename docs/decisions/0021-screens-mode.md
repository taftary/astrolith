# 0021 — Screens mode: window pictures of every UI screen

- Status: proposed
- Date: 2026-10-11
- Issue: #492 (Spec v1 approved)

## Context

UI work (first #473) needs pictures of the real window so the validator and
the owner can check screens by eye before owner testing. `--capture` is a
software plot of the markers, not the Bevy window, so no existing mode
produces them. `E-STDOUT-PROTOCOL` makes new protocol lines a decision.

## Decision

`universe-app` gains a fourth action, `--screens <dir>`:

- It creates and probes `<dir>` before any window opens; an unwritable
  folder ends the run at once.
- It opens the normal window (`E-SINGLE-WINDOW`) with default settings
  only. The settings file is neither read nor written.
- The window uses `scale_factor_override(1.0)`, so pictures are exactly
  1280×720 (wide) or 640×720 (narrow) whatever the display scaling.
- `universe-render::ScreensPlugin` holds the shot list `SHOTS` (file name,
  screen, size, panel setup), the single place to add a screen. For each
  shot it applies the size, screen, and panel setup, waits a fixed number
  of frames, and requests one `Screenshot` of the primary window. A
  per-shot frame cap turns a stuck shot into a failure, so the mode never
  hangs. It does no file I/O and prints nothing.
- The app driver writes each picture as PNG (Bevy feature `png`), checks
  it has the asked size and more than one colour, then writes
  `screens.txt` with one `<file> <width>x<height>` line per picture.

New protocol lines (stdout, one per run):

- `SCREENS-OK files=<n> dir=<dir>`: `<n>` counts the pictures plus
  `screens.txt`; exit 0.
- `SCREENS-FAIL <reason>`: exit 1.

## Consequences

- The validator gets a proven way to see the real window (recorded in
  `docs/validator-capabilities.md` once proven in stage 10).
- Not run in CI: it needs a real window and a graphics adapter. Unit tests
  cover the shot list, the sequencer, the not-blank check, the list file,
  and the unwritable-folder path.
- No pixel comparison against stored pictures (graphics cards differ).
- `--verify`, `--capture`, and the golden files are unchanged.

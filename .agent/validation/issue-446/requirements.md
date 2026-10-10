# Requirements — Issue #446 (Spec v1)

One criterion per acceptance criterion of approved Spec v1
("Markers follow the design system"). Parent Issue #446, tasks #447-#454.

- AC1 (line floor): every marker line draws at least 2 px wide
  (`LINE_MIN_PX`, applied through `GizmoConfigStore`).
- AC2 (dot floor): every marker dot holds a minimum on-screen diameter
  (`DOT_MIN_PX`); the smallest actionable marker stays visible.
- AC3 (contrast on what you can act on): portals, the selection ring, the
  portal mark, and labels reach at least 3:1 against black at their dimmest
  drawn brightness (`ACTIONABLE_BRIGHTNESS_FLOOR`, `display_tint`).
- AC4 (no vanishing reds): no marker colour reads as black under protanopia
  simulation; dark reds display as a lighter orange-red wherever drawn.
- AC5 (brightness ladder): shell brightest, children one step below, context
  at least two stops under children (`LADDER_SHELL/CHILD/CONTEXT`).
- AC6 (portal mark): every portal shows one small ring at the true position
  of what is inside, the same shape at every level; populations show none.
- AC7 (labels on the portals that matter): a portal labels when large enough
  on screen (`LABEL_MIN_PX`), selected, or hovered; smaller portals and
  populations show none; at most `LABEL_CAP` labels per frame, largest first.
- AC8 (label text): `"kind · name"` (home chain real names: Milky Way, Alpha
  Centauri, Sun, Earth; surface `"<kind> <n>"`; seeded syllable names
  elsewhere, identical every run); kind alone when nameless.
- AC9 (black-and-white test): portals and populations tell apart by shape
  alone (portal mark plus label exist only on portals).
- AC10 (shape stays per level): one form family per level; importance reads
  through size and brightness only.
- AC11 (selection and hover): selection is an outline ring at the line floor
  plus a label halo; hover is a brighter variant of the same ring; neither
  relies on colour alone.
- AC12 (pick size): mouse pick area at least 24 px (`PICK_MOUSE_PX`); touch
  pick sizes defined (`PICK_TOUCH_PT`/`PICK_TOUCH_DP`) for when touch arrives.
- AC13 (pick ranking): destination portal first, then nearest; a population
  is never picked over a portal.
- AC14 (cycling): repeated clicks on the same spot cycle portals in order
  with wrap; moving away resets.
- AC15 (named values): every visual value is named by role in `tokens.rs`
  (core numeric rules in `contrast`/`pick`); drawing code holds no numbers
  of its own.
- AC16 (unchanged headless output): `--verify` stdout and the capture file
  list match the saved references exactly (no `UPDATE_GOLDEN`); captured
  pictures may change.

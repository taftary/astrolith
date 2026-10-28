# #1022 T6 evidence (#1063): internal 3D resolution

Release build, real window, `--timing --size 1920x1080` so that 70 % stays
above the 720-pixel floor (70 % of 1080 is 756). Shot file: `t6.shots`.

- `scale-1.0/`: `ASTROLITH_RENDER_SCALE=1.0`, native.
- `scale-0.7/`: `ASTROLITH_RENDER_SCALE=0.7`: the 3D scene at 1344 x 756,
  stretched to 1920 x 1080; the HUD at native size.
- `smoke-native/`: the built-in `smoke,menus` shots at native, to show the
  native path and the menus are unchanged.

Timing runs let the game clock run, so the daylight differs between the
two folders' planet shots. The space shot (`space-l4.png`) does not depend
on daylight: its markers sit at the same window positions at both scales,
and the line widths match after the gizmo width follows the scale.

The PNGs stay out of git; they go to the `validation-evidence` release with
the pull request. The frame times in `timing.txt` are not a comparison of
the two scales (different daylight and clock); Auto's performance evidence
comes from the route runs once Auto is connected.

# #1160 T4 (#1198): moonlit night

**Cause.** In the baseline the moonlit and moonless nights looked alike: ground means of 3.8/4.0/7.0 and 3.8/4.7/9.4 (sRGB). There were two reasons:
1. **A low moon was weakened twice.** `MoonLight.share` is the light on flat ground, the moon's strength times its height. The moon lamp shone with `share`, and the renderer then applied the angle to the ground again, so the ground got strength × height². At the `moonlit` shot the moon is 13 degrees up (height 0.23) and about a third lit (phase 0.36). The probe showed moon share 0.0206 against a star glow of 0.0200.
2. **The stars' even glow (0.04 at 100%) was as strong as a low moon.**

**Change (within the #981 rule: moons light the night, moonless nights are dark):**
- `MoonLight.strength` is new: the light on a surface facing the moon. `share = strength × height` stays as the value the eyes read.
- The lamp now uses `strength` (`moon_lights.rs`), so the renderer applies the angle once.
- `MOON_FULL_SHARE` goes from 0.5 to 0.8 and `STAR_SHARE` from 0.04 to 0.02.
- The Night light setting is unchanged (S1): 0-100% in steps of 10, default 50%, same saving and Reset, and 0% still gives no night light (existing test `moonlight_tests.rs`, `dark` case).

**Tests**
- New: `a_low_moon_is_not_weakened_twice`.
- The existing moonlight tests pass with the new constants, as does the lamp test (`moon_lights_tests.rs`, now checking `strength`). The workspace tests and clippy pass.

**Pictures** (`night.shots`: moonlit and moonless from `light.shots`, plus `other` at lat 0 where the moon is 30 degrees up)
- `try1` was an intermediate step: a stronger moon without the double-weakening fix. It was darker, showing that the strength alone was not the cause.
- `try2` is final. See `try2-sheet.png`: moonlit before and after, moonless before and after, then other.

Mean ground colour, lower middle of the picture (sRGB):

| shot | before | after |
|---|---|---|
| moonlit (moon 13 degrees up, a third lit) | 3.8, 4.0, 7.0 | 12.4, 9.8, 8.6 |
| moonless | 3.8, 4.7, 9.4 | 1.8, 2.0, 4.4 |
| other (moon 30 degrees up) | not in the baseline | 29.1, 31.9, 44.1 |

- A moon up now makes the ground visible: about 6 times the moonless level, and far more with a higher moon. A moonless night is darker than before. Both stay far below day (100, 92, 85).
- The moon also lights the air now, so a moonlit sky is a dark blue instead of black.

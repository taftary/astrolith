# #1160 T2 (#1196): the dawn and dusk "stripe"

## Finding: a picture-tool fault, not the game's sky
The hard-edged pink or orange band in the #1085 and T1 dusk, dawn and low-sun pictures is **not** the atmosphere. It is the inside of the player figure:
- The shot list's first-person shot (`body`) parks the body at its own place (lat -30, lon 60) and saves that spot.
- Every later shot with no figure at that place (dusk, dawn, low-sun) puts the camera 2 m above the ground, inside the parked figure.
- The figure's faces cover the sky (flat grey), and the "stripe" is real sky seen through a gap between them. That is why its edges are straight and slanted.

Probes, at 800x450 on the release build of main's code:
- `heading.shots` and `heading-sheet.png`: the same place at 16:52 and 8 headings, with no figure shot before. The sky is clean at every heading, with a sun glow.
- `order.shots` and `order-sheet.png`: size, tilt and a shot from orbit before it change nothing. The sky is clean, and dusk shows a warm glow and an orange horizon.
- `order2.shots` and `order2-sheet.png`: day, body (first person), dusk, dusk again, dawn. Every shot after `body` is broken.
- `order3.shots` and `order3-sheet.png`: a third-person figure shot after `body` moves the body away, and the next dusk is clean.
- `rep-a` (day, body, dusk) is broken, and `rep-b` (day, dusk) is clean. This is deterministic.

In play, this needs the camera to fly into your own parked figure, which is the same as walking into any object. No sky change is needed.

## Fix (picture tool only)
- `shot_place::clear_parked_body`: a shot with no figure first sends a body that an earlier shot parked back to its seeded spot, and clears the saved spot.
- Test: `a_shot_without_a_figure_sends_the_parked_body_home`.
- Before and after: `rep-a/dusk.png` was broken; `rep-a-fixed/dusk.png`, the same shot list after the fix, is clean. See `dusk-before-after.png`.

## What this means for #1160
- AC1's "stripe and bright line gone" is met by this tool fix. The real sunset sky already glows warm toward the sun.
- What stays for AC1 is the ground: it stays dark grey at sunset (T3).
- The #1085 and T1 baseline pictures `dusk`, `dawn` and `low-sun` are affected. T7 compares against a baseline retaken with the fixed tool on main's light code, recorded in `../before-fixed-tool/`.
- The game's light code is unchanged by T2, so the T1 route timings stay valid as the baseline.

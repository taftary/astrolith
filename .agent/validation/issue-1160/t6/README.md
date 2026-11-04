# #1160 T6 (#1200): separate light inputs and Help

- **AC5:** `patch_material::GROUND_OWN_LIGHT` (black) is the ground's `emissive`. It is separate from the surface colour (`base_color`), the sun and moon lamps (directional lights) and the even fill (ambient). A later glow sets only this input. Test: `the_ground_keeps_its_light_inputs_separate`.
- **AC11 (G1 yes):** Help > Navigation now reads "Its star lights the side facing it, and the light turns warm at sunrise and sunset; at night ...". Test: `guides_say_the_light_turns_warm_at_sunrise_and_sunset`. The existing guide tests were updated to the new sentence.
- The workspace tests and clippy pass.

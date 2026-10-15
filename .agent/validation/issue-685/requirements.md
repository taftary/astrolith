# Requirements (CONFIRMED) - issue #685

Derived by the main agent from approved Spec v2
(https://github.com/taftary/astrolith/issues/685#issuecomment-6175109848,
owner approval https://github.com/taftary/astrolith/issues/685#issuecomment-6175210654),
with the decisions Q1-Q5 and the candidates S1, S2, G1 (all yes). How the
generated planets look on the real window is judged by the main agent's
`--screens` pictures and the owner test; the validator takes no pictures (#533).

- [ ] C1: Planet look offers Normal, Squares and Generated; Normal and Squares are unchanged; the default is Normal until acceptance (AC1) (verify: contains planet_look_offers_normal_squares_and_generated ... ok in cargo-test.log)
- [ ] C2: The same seed and age give identical heights, types, moons and colours on every run (AC2) (verify: contains generation_is_deterministic_for_seed_and_age ... ok in cargo-test.log)
- [ ] C3: The height at a place does not change when more detail is drawn, and the walked ground equals the drawn ground (AC3) (verify: contains height_is_pure_across_detail_and_ground ... ok in cargo-test.log)
- [ ] C4: Over a fixed list of seeds every planet type and every moon type appears at least once (AC4) (verify: contains every_planet_and_moon_type_appears ... ok in cargo-test.log)
- [ ] C5: Relief stays under each type's peak cap times its gravity scale; Earth-like worlds show two height peaks and airless dwarfs one (AC5) (verify: contains relief_respects_caps_and_hypsometry ... ok in cargo-test.log)
- [ ] C6: Rocky worlds up to about 2 Earth radii keep metre-scale ground detail at their real size; giants show a cloud field, not ground (AC6, the owner's "be careful with the big planets dimension") (verify: contains big_planets_keep_metre_detail ... ok in cargo-test.log)
- [ ] C7: Ranges and trenches only where plates collide, rifts and ridges only where they separate; stagnant-lid worlds have no plate belts (AC7) (verify: contains plate_relief_follows_boundaries ... ok in cargo-test.log)
- [ ] C8: Crater counts grow with surface age, old craters are softer, young ones show rays (AC8) (verify: contains craters_follow_surface_age ... ok in cargo-test.log)
- [ ] C9: Rivers flow downhill to the sea or a closed lake; deltas match their ratios; lakes are flat and at or below their rim (AC9) (verify: contains rivers_deltas_and_lakes_follow_rules ... ok in cargo-test.log)
- [ ] C10: Biome colours follow temperature and rain; deserts in dry belts and rain shadows; ice caps follow cold; no biomes on airless or waterless worlds (AC10) (verify: contains biomes_follow_climate ... ok in cargo-test.log)
- [ ] C11: Mercury-like and Moon-like worlds are airless; Mars-, Venus- and Titan-like worlds keep air in their band (AC11) (verify: contains air_follows_the_shoreline ... ok in cargo-test.log)
- [ ] C12: Older systems give more cratered, more eroded, less active worlds; close moons with a locking time under the age are locked (AC12) (verify: contains lifetime_changes_worlds_with_age ... ok in cargo-test.log)
- [ ] C13: Moons lie between the Roche limit and the Hill limits; small moons are irregular; binaries are marked (AC13) (verify: contains moon_systems_respect_limits ... ok in cargo-test.log)
- [ ] C14: The home planet is generated, and the journey still arrives on the home planet (AC14) (verify: contains the_home_planet_is_generated_and_reached ... ok in cargo-test.log)
- [ ] C15: The Engine lab shows each planet's generated values and its screens list has the type gallery (AC15) (verify: contains the_lab_shows_generated_values_and_gallery ... ok in cargo-test.log)
- [ ] C16: Relief height (0.5x to 2x, default 1x) scales relief and the walked ground; Clouds (on/off, default on) hides the cloud layers; both are saved and restored (AC16, S1, S2) (verify: contains relief_height_and_clouds_settings ... ok in cargo-test.log)
- [ ] C17: Help > Planets shows the G1 text (AC16, G1) (verify: contains the_planets_guide_page_exists ... ok in cargo-test.log)
- [ ] C18: The game is still deterministic overall: the headless check passes with its repeat-run probe (AC2) (verify: contains VERIFY-OK in verify.log)

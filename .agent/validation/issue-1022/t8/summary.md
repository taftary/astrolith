# #1022 T8 evidence (#1065): atmosphere tiers judged

Release build, `--screens <dir> sky` (the built-in sky group: orbit,
100 km, 10 km, 1 km, ground, terminator at sunset, night; 1280 x 720,
the clock held), one run per tier with `ASTROLITH_ATMOSPHERE=low|normal|high`
(sky-table / raymarch samples Low 6/4, Normal 10/8 = today, High 16/12;
table sizes unchanged). The PNGs stay out of git and go to the
`validation-evidence` release with the pull request.

Judged by eye, each tier against Normal (today's picture):

| shot | Low (6/4) | High (16/12) |
|---|---|---|
| sky-orbit | visible jump: the planet looks hazier and washed out, ground shapes fade | same as Normal |
| sky-terminator (sunset) | visible jump: the glow turns pink-magenta instead of orange | small change: the glow is a little wider and more yellow |
| sky-100km | the same horizontal bands as Normal, slightly stronger | the same bands as Normal |
| sky-night | same as Normal | same as Normal |

Verdicts (Spec v4 R3: a tier with obvious banding or a visible jump does
not pass and needs revision):

- **High passes** (its sunset change is small and only visible side by side).
- **Low fails**: a visible jump from orbit and at sunset. Reported to the
  owner with options; the samples are not changed without a spec change.
- **Banding at 100 km is in today's game at every tier**, Normal
  included, so it does not come from the sample counts (likely the fixed
  aerial-view table's 64 slices). Reported as an existing problem.

Bloom: fades over 0.5 s by its intensity (test
`bloom_fades_over_half_a_second`); at zero its passes are skipped.
Shadows switch at once: Bevy's shadow maps have no strength to fade.
Markers: a budget change eases the cutoff over 0.5 s (tests
`a_budget_change_eases_over_half_a_second`,
`each_marker_tier_keeps_its_budget_once_eased`). Terrain: the effective
tier times the memory bias (`terrain_combines_the_tier_and_the_memory_bias`).

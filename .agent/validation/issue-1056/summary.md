# #1056 evidence: orbit paths near the ground and from orbit

Release builds, real window, `--screens <dir> --shots orbit-paths.shots`
(1280 x 720, the clock held). `before/` is `main` at c718a83; `after/` is
`fix/1056-orbit-paths` with the path floor at 0.7. The PNGs stay out of
git and go to the `validation-evidence` release with the pull request.

| shot | before | after |
|---|---|---|
| ground-dawn (2 m, 06:31) | black line from the horizon up | no line; the moons stay |
| h-1km | black line | no line |
| h-10km | black line | a faint line in the moon's colour |
| h-100km | black line | the full path in the moon's colour (green) |
| orbit-start (300 km) | black line | a bright green line |
| orbit-night (300 km, 18:29) | a short black line along the night-side edge | the same, unchanged |

- The ground at dawn, 1 km, 10 km and 100 km match Spec v1 criteria 1 and 2.
- orbit-start matches criterion 3.
- The short black line along the night-side edge in orbit-night is
  identical before and after, so it is not an orbit path. It looks like an
  edge in the planet or atmosphere drawing, outside this item; it is
  reported, not fixed here.
- The first floor tried, 0.4, still drew a dim dark-green line from orbit.
  The kept floor is 0.7, the selected marker's.

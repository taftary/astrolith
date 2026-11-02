# #1128 visual evidence summary (T8)

Spec v3 (owner option B): today's view stays unchanged; the developer layer
adds parts on top, off by default and never in release.

Shot files (in this folder):

- `matrix.shots`: 77 real-window shots in the debug build. They cover:
  - today's view per level with Developer markers off;
  - Developer markers on, far and near, for L1 to L9, plus L10 from orbit,
    from 300 km and on the ground;
  - the L4 host disk and the L7 belts face-on, oblique, edge-on and inside;
  - each of the seven filters alone, at L2 and L7;
  - three window sizes (640x720, 1280x720, 2560x1440);
  - a slow zoom in and back out over the L4 host disk, eight steps each way.
- `release.shots`: 32 of those views, every one asking for Developer
  markers, run with the release build.

Command: `cargo run --locked -p universe-app -- --screens <dir> --shots
matrix.shots` (debug), and `target/release/universe-app --screens <dir>
--shots release.shots` (release).

## Results

- Fresh profile: Developer markers off shows today's view (`off-*`
  shots). The layer classifies and draws nothing while off: the test
  `navigation_survives_all_developer_filters_off` shows zero parts, and
  the drawing returns at once. AC1 and AC12 are also covered by tests.
  Result: MET.
- Complete L1–L10 catalog: MET.
  - Every catalog family appears in its own level's shots:
    - L1: universe boundary and graticule;
    - L2: voids (violet, dotted), walls (ochre), filaments, nodes, clusters
      and groups, and the region context;
    - L3: galaxies and the cluster context;
    - L4: clouds and cloud boundaries, plus the host disk, arms and bar;
    - L5: stars and the host context;
    - L6: the primary star, companions and arcs;
    - L7: the central star, Kuiper belt and scattered disc (tan, dash-dot),
      and the Oort reservoir and system boundary (blue, dotted);
    - L8: planets, true orbits (dashed), belts and the heliopause;
    - L10: moons, moon paths, the equator (green) and the spin axis
      (yellow).
  - L9 shows the L8 families. The registry tests and the far-to-near
    fixtures cover every family.
- Direction and inside: MET.
  - The host disk and the belts draw in full face-on, partly when oblique,
    and not at all edge-on (`dir-l4-edge`, `dir-l7-edge`).
  - Inside a volume, its outline is gone (`dir-l4-inside`, `dir-l7-inside`).
- Slow zoom both ways: MET.
  - `zoom-in-0..7` go from the disk glyph to the disk with arms, then to
    cloud outlines with labels. `zoom-out-0..7` reverse this smoothly.
  - Each part keeps its anchor and colour.
  - The `MARKER-CONTINUITY` verify line reports zero flickers, duplicates,
    anchor jumps and colour jumps.
- Filters: MET. Each `filter-*` shot shows only that class. With
  `labels` alone, nothing new appears (`filter-l7-labels` looks like
  `off-l7`). A label names a part that is drawn, so it needs the part's
  own filter on as well.
  Navigation cues are present in every shot.
- Resolution: MET. The `size-*` shots at the three sizes show the same
  parts at the same stages. The stage follows angular size only (test
  `apparent_size_stages_are_resolution_independent`).
- Monochrome comparison: MET. In the `mono/` copies (dev-l2-near, dev-l7-far,
  dev-l8-near, dir-l7-face), belts (dash-dot),
  boundaries and voids (dotted), orbits (dashed) and walls (solid) differ
  by line pattern, not by hue alone.
- Release exclusion: MET.
  - Every release shot asked for Developer markers, and none shows any.
  - In the release build, `dev-l4-far` and `dev-l7-far` match `off-l4` and
    `off-l7` pixel for pixel (0.000 % of pixels differ).
  - The `dev-l2..l10` release shots match the debug `off` shots at 0.000 %.
  - `dev-l1-far` differs from the debug `off-l1` by 0.170 %: L1 streaming
    differs between builds. Its picture shows no developer part or label.
  - The debug build shows the layer in the same views (0.691 % at L7,
    3.400 % at L4).
- Near the ground: MET. `dev-l10-ground` shows no scaffolding, and the
  paths and references have receded.

Known and unchanged: the white navigation labels overlap each other in
crowded L1 views. They did so before this item, as `off-l1` shows.
Developer labels keep clear of them and of each other.

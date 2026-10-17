Agent-authored draft at owner request in session 2026-10-17. Owner will recreate the sphere from scratch using the notion in drafts/L10toL14.md, including buildings/rooms and everything related, from sphere creation to the last dimension. This draft records the cleanup notion only. No Issue created.

# Clean what was done L10 to L14

Remove the L10–L14 planetary surface stack so the sphere can be recreated from scratch per drafts/L10toL14.md. Cleanup comes first; rebuild starts only after cleanup lands.

## Remove (proposal)

- Core surface: `crates/universe-core/src/icosphere.rs`, `surface.rs` (`SURFACE_DEPTH`, `SurfaceFrame`), `ground.rs` L11–L13 height logic, `terrain.rs` planet-radius/facet parts (`PLANET_RADIUS_CELL`, `FACET_DEPTH`, `surface_radius`/`floor`).
- Frame/shells: `frame.rs` L10–L14 planet-radius/shell parts, `body.rs` `level_ground` L10–L14 bits, `gravity.rs` planet pull parts tied to old surface.
- Render: `universe-render/src/patch.rs` L10 detail window, `planet.rs` far-ball (keep moons only if needed as decoration — decide), L10 suppress rules in `draw.rs`, `labels.rs`, `hud.rs`, L10 ground shots in `screens.rs`/`window_tests.rs`.
- Nav caps: decide whether `nest.rs`/`nav.rs` `MAX_OPEN_LEVEL=10`, `coords.rs` `Level 1..=14`, `destination.rs`, `landing.rs` L10 stop, `flight.rs` scripted legs stay or go. Proposal: keep L1–L10 travel skeleton, remove L11–L14 surface bindings.
- Harness: update `universe-app` verify/capture and `tests/golden/verify.txt` + `capture.txt` plus `universe-verify` parity to the cleaned state.
- Buildings/rooms/everything related: `gen.rs` forms (`Shell`/`Grid`/`Rect`), `layouts.rs`, `sysgen.rs` indicators, `body.rs` room/body placement + `level_ground`/`walk_levels`, `home.rs` fixtures, `destination.rs` chains, doorway/portal bindings, tail remnants, `guides/universe/L10-L14` docs, and any goldens tied to them. Rebuild goes from sphere creation to the last dimension from scratch.
- Docs: new ADR superseding ADR 0030; retire `guides/universe/L11-L14` references; keep `docs/design/cube-sphere-planet-engine.md` as rebuild target.

## Keep

- App shell, camera, travel/place-address backbone, Bevy/WGPU setup, `--verify` harness shape.
- A minimal planet placeholder (simple ball or nothing) so the workspace still builds after cleanup — to be confirmed.

## Decided (owner answers 2026-10-17)

1. Far-ball: delete fully — no planet until rebuild.
2. Levels: keep L1–L10 nav skeleton, remove surface bindings.
3. Intermediate: broken OK until rebuild.
4. Goldens: update during cleanup to the cleaned state.

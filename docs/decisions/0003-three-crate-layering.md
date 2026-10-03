# 0003 — Three-crate layering core render app

- Status: accepted
- Date: 2026-10-03
- Issue: #35 (spec v4), enforced in #85

## Context

Universe logic must run and test without a window, while rendering needs
the engine. One crate cannot serve both without dragging Bevy into
headless checks.

## Decision

Three crates, one direction: `universe-core` (std only, deterministic
logic) <- `universe-render` (Bevy indicators) <- `universe-app` (binary).
`universe-core` is the stable public surface; render exposes exactly its
plugins plus the helpers the app needs; the app exposes nothing
(`E-DEPS-DIRECTION`, `E-CORE-NO-BEVY`, `E-CORE-API`).

## Consequences

Core compiles and tests with no graphics dependency. Layer violations are
CI failures, not review debates.

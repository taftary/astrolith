# 0010 — Portal vs population markers

- Status: accepted
- Date: 2026-10-04
- Issue: #151 (spec v3 approved; amends ADR 0004, which stays accepted for nesting, ratios, and frames)

## Context

ADR 0004 says every marker is the next level's cell. The realism review
(#143) showed this forces nonsense: an Oort cloud holding 32 planetary
systems, a planetary system holding 32 stars, a star holding 32 planets.
Most generated points are context, not continuations.

## Decision

Every marker is either a **portal** (opens the next cell when targeted)
or a **population point** (shown, counted, never opens, highlights, or
takes the target). Portals render at full size and brightness; populations
render smaller and dimmer. Counts come from physical densities
(Binomial draws from the seeded cell hash, integer-only), varying per cell
but identical per seed. Autopilot, preview, open, hover, click, and
snapshots all gate on kind; snapshots prefix portal lines `p` and
population lines `o`; capture marker lines carry `kind=`.

## Consequences

`replay_autopilot` and `--verify` keep the fixed journey (now L1–L10);
golden files were rewritten once with the reason per line class. The
`2 + PREVIEW_CAP` bound is untouched. Content themes (#154–#157) assign
kinds to their new structures under this rule.

# 0006 — Fixed timestep and 60 fps budget

- Status: accepted
- Date: 2026-10-03
- Issue: #35 (owner decisions)

## Context

Simulation results must not depend on frame rate, and the window must stay
smooth. Variable steps make replays diverge; an unbounded frame budget
makes scatter plots slide.

## Decision

Simulation systems run on a fixed timestep (`VERIFY_DT = 1/60 s` headless);
the frame-rate budget is 60 fps, recorded in `docs/universe/stack.md`
(`E-BUDGET`). Frame time is measured behind the profiling feature when
needed, never as a pass/fail criterion.

## Consequences

Replays are deterministic across machines. Performance work measures
against the stated budget instead of impressions.

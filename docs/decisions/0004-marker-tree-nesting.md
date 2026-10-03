# 0004 — Marker-tree nesting

- Status: accepted
- Date: 2026-10-03
- Issue: #58 / #61 (R6)

## Context

Diving into a marker must arrive somewhere meaningful: the next level's
cell, at the true ladder ratio, so the journey L1-L11 is continuous.

## Decision

Every marker is the next level's cell (R6). `universe-core::nest` owns the
nesting, ratios, budgets, and the observer's marker path. Positions convert
between frames through named methods, never arithmetic at the call site
(`E-NEWTYPE`).

## Consequences

`replay_autopilot` and `--verify` replay the same continuous journey. The
open/close inverse (`INVERSE`) checks the round trip on every run.

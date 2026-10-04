# 0005 — Pre-entry preview

- Status: superseded by 0012
- Date: 2026-10-03
- Issue: #63 (R7)

## Context

Opening a marker should never surprise: the target cell's content is
previewed in the frame before opening, bounded in count and cost.

## Decision

Pre-entry preview (R7): at most `PREVIEW_CAP` markers previewed, previewed
child positions equal the open cell's markers within 1e-9, at most
`2 + PREVIEW_CAP` generations alive. `--verify` checks every opening
(`PREVIEW` lines).

## Consequences

Dive targets are verified before entry on every headless run. Preview
budget overruns fail the gate instead of degrading silently.

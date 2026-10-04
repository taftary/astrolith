# 0012 — Streaming memory bound

- Status: accepted
- Date: 2026-10-05
- Issue: #152 (Spec v1 approved; supersedes ADR 0005, which stays on disk as the preview rule's history)

## Context

ADR 0005 bounded live generations at `2 + PREVIEW_CAP` (open cell, parent,
previews). That sufficed for single-line target dives. Issue #152 adds
pass-through entry and free flight under a 32-cell hard cap with a 5-second
travel-time horizon: the working set is now the open chain plus horizon
cells plus previews, which the old bound cannot express.

## Decision

The streaming working set holds at most `STREAM_CAP = 32` live cells: open
chain, horizon, and previews. `StreamLedger` (`universe-core::nav`) is the
bound accounting: one entry per marker chain with its per-cell stream seed
(`path_seed`) and last-seen tick; each sync batch marks the current set,
then unloads longest-unseen-first beyond the cap with deterministic
chain-order tie-breaks. The headless proof asserts the bound at every dive
opening (`alive`, `max 32`) and across the scripted free leg (`alive-max`).
The window mirrors the same ledger, fills previews from background compute
tasks merged in marker order (equal to the synchronous result), and closes
exited cells past the horizon (`HORIZON_SECS = 5.0`, floor
`MIN_HORIZON_DIST = 4.0`, below the angular close distance so slow flight
still sheds cells without fighting the dive). Previews stay capped at 6, a
subset of the 32. Generating horizon cells beyond previews (prefetch along
the velocity for future content) is explicitly future work for #153+.

## Consequences

Golden `verify.txt` rewritten once: `PREVIEW` lines carry the ledger recount
and `max 32` (same journeys, same snapshots), plus the three `FREE-LEG`
lines. `PreviewCache` keeps its 6-preview cap and gains a drain-in-place
merge for background completions. ADR 0005 is superseded by this record.

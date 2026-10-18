# 0030 — True-size travel: window, ball, and the end of the ladder

- Status: superseded by 0031
- Date: 2026-10-17
- Issue: #742

## Context

ADR 0028 drew the planet as a patch-only dome and ADR 0029 completed it
to a hemisphere with a shown rim. The owner notion (#742) redefines the
L10–L14 dimensions as true-size places reached by travel instead of
nested sub-cells: no rescaling anywhere, markers as travel targets that
never open, Spacebar autopilot flight along the landing curve, saved
views teleporting, stable place addresses, and buildings and rooms fully
purged. Locked answers: no new structures (doorways die with the purge),
the ladder ends at L10, G1–G3 as approved with G4 restored to
"There is nothing to open: walk freely."

## Decision

The dive runs L1–L10 and stops at the planet. Below L10 nothing opens:
L11–L14 stay numbered in code but unreachable, and all building, room,
doorway, and furniture generation, collision, outline, and test code is
purged (about 1,900 lines with the dead tail references).

- **Targets.** L10 clicks target but never open; the autopilot keeps
  flying at L10 and lands at standoff instead of idling
  (`DiveMode::Landing` at L10, stop at the landing rest).
- **Addresses.** Places are seed-plus-coordinates addresses
  (`PlayerSpot`, `Destination` place refs); the same seed yields the same
  places on every revisit.
- **Body.** The body keeps one true size everywhere (1.8 m); no level
  holds solid things anymore.
- **Ground.** The hemisphere dissolves: a fixed-size follow detail
  window (`PATCH_RADIUS` cap, `detail_lod` fine/coarse) plus a coarse
  static whole-planet ball (`ball_triangles`, tucked under the lowest
  face), crossfading across a height band (`ball_weight`,
  `detail_fade`) with no pops or holes. The rim dies with the
  hemisphere. The window and the ball share one base tone, so the swap
  changes no hues.
- **Streaming.** The working-set ledger evicts farthest-first
  (recency, then chain order on ties) under the same `STREAM_CAP`.
- **Words.** Guides carry the approved G1–G3 words plus restored G4;
  `ladder.md` gains R13 (the ladder ends at L10) and the L11–L14 rows
  are retired.

## Consequences

This record supersedes ADRs 0028 and 0029 (0024 and 0027 stand
superseded by 0028); they stay on file marked superseded. `E-RENDER-NO-MESH` now reads: one
detail-window mesh plus one coarse far ball (this ADR), plus moons at
L10 per ADR 0016. L10 screenshots change (window edge and ball instead
of the hemisphere and rim); capture goldens are regenerated with the
reasons in the pull request while `--verify` output for L1–L10 stays
byte-identical.

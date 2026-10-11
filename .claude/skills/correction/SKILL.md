---
name: correction
description: Load when owner testing fails or the validator fails (workflow stage 13). Use to run a bounded correction round with new sub-issues and a new fix branch and PR.
---

# Correction

Correction re-enters at Tasks and follows the same path (tasks, implementation, pull request, technical validation, validator gate, merge, owner testing), per `docs/workflow.md` (Correction) and the stage-to-skill table in `docs/workflow.md` (Skills).

## Inputs

- Parent Issue in Needs correction (owner's problem report on the Issue) or a `validator:fail` verdict for the last SHA.
- Existing round counter state on the Issue (owner rounds failed, consecutive validator fails).

## Steps

1. **Round counter.** Read the latest `<!-- correction:round k owner-fails=m validator-fails=v -->` marker on the Issue (never prose). Increment and post the new marker (first correction is round 1, fails carried from history).
2. **Bound check before new work.** After two failed owner-test rounds OR two consecutive validator fails on the same item (the bound in `docs/workflow.md`, Correction): stop. Post a summary comment (what changed each round, last failure reasons + evidence), set `--status-key blocked`, ask the owner for a decision, and do not open new work.
3. **Correction sub-issues.** Create new sub-issues (label `task`) describing the fix with done-criteria; attach as GitHub sub-issues of the parent per `docs/board.md` (numeric `id`, not number; fallback is a `- [ ] #N` task list in the parent body). Owner problem reports stay on the same Issue; they do not become new Issues.
4. **New branch + PR.** Use a new `fix/<N>-<slug>` branch and a new PR for this round (same path: `implementation` → `pull-request` → `technical-validation` → validator gate → merge). Never reuse the merged branch.
5. **Drift check.** If the validator reports spec drift from the original notion, stop and ask the owner before continuing (notion preservation).
6. **Record and move status.** Keep the item in Needs correction while correcting (`--status-key correctionRound`; per Status Transitions it stays there through re-validation and merge). On correction PR merge → In review (via the `pull-request` post-merge steps); on the bound (step 2) → `--status-key blocked` (both verified by re-read). A validator FAIL in any round routes to In progress (`--status-key validatorFail`). End with the one-glance summary per `docs/board.md`.

## Outputs

- Correction sub-issues, new `fix/` branch and PR, validation verdicts, merge record — all linked on the parent Issue.
- Or: bound summary comment + Needs your answer.

## Done criteria

- Correction PR merged → status In review per `docs/board.md`, handed to `present-for-owner-testing`; or bound reached → summary posted, status Needs your answer, owner decision recorded.
- Every output recorded on the Issue.

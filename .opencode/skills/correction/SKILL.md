---
name: correction
description: Load when owner testing fails or the validator fails (workflow stage 13). Use to run a bounded correction round with new sub-issues and a new fix branch and PR.
---

# Correction

Correction re-enters at Tasks and follows the same path (tasks, implementation, pull request, technical validation, validator gate, merge, owner testing), per `docs/workflow.md` (Correction) and `docs/plans/workflow.md` skills table.

## Inputs

- Parent Issue in Needs correction (owner's problem report on the Issue) or a `validator:fail` verdict for the last SHA.
- Existing round counter state on the Issue (owner rounds failed, consecutive validator fails).

## Steps

1. **Round counter.** Read the Issue for the correction round count. Increment and post it (e.g. `Correction round <k>: owner round <m>, consecutive validator fails <v>`). First correction is round 1.
2. **Bound check before new work.** After two failed owner-test rounds OR two consecutive validator fails on the same item: stop. Post a summary comment (what changed each round, last failure reasons + evidence), move status to Needs your answer per `docs/board.md`, ask the owner for a decision, and do not open new work.
3. **Correction sub-issues.** Create new sub-issues (label `task`) describing the fix with done-criteria; attach as GitHub sub-issues of the parent per `docs/board.md` (numeric `id`, not number; fallback is a `- [ ] #N` task list in the parent body). Owner problem reports stay on the same Issue; they do not become new Issues.
4. **New branch + PR.** Use a new `fix/<N>-<slug>` branch and a new PR for this round (same path: `implementation` → `pull-request` → `technical-validation` → validator gate → merge). Never reuse the merged branch.
5. **Drift check.** If the validator reports spec drift from the original notion, stop and ask the owner before continuing (notion preservation).
6. **Record and move status.** Keep the item in Needs correction while correcting (per Status Transitions it stays there through re-validation and merge). On correction PR merge → Ready to test; on the bound (step 2) → Needs your answer. All moves per `docs/board.md`, output recorded on the Issue.

## Outputs

- Correction sub-issues, new `fix/` branch and PR, validation verdicts, merge record — all linked on the parent Issue.
- Or: bound summary comment + Needs your answer.

## Done criteria

- Correction PR merged → status Ready to test per `docs/board.md`, handed to `present-for-owner-testing`; or bound reached → summary posted, status Needs your answer, owner decision recorded.
- Every output recorded on the Issue.

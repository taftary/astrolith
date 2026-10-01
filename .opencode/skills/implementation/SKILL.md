---
name: implementation
description: Load when implementing approved work for a parent Issue (workflow stage 8). Use after spec approval and testability preflight pass to build sub-issues on a feature branch.
---

# Implementation

Implements the approved sub-issues for one parent Issue on a single feature branch, per `docs/workflow.md` (Implementation, Branching and Merge) and `docs/plans/workflow.md` skills table.

## Inputs

- Parent Issue number `#N` with:
  - Original notion verbatim.
  - Approved specification comment (owner `approved`) including goal, acceptance criteria, test plan, testability needs.
  - Testability preflight pass comment, or `owner-test-only approved` comment.
- Sub-issues (label `task`, attached as GitHub sub-issues of the parent) with done-criteria.
- Branch naming rule: `feat/<issue>-<slug>` or `fix/<issue>-<slug>` (must match CI guard `^(feat|fix)/[0-9]+-[a-z0-9-]+$`).

## Steps

1. **Gate first, before any code.** Read the parent Issue:
   - Require an owner `approved` comment on the spec AND a preflight pass comment, OR an `owner-test-only approved` comment.
   - If missing: do not implement. Post a status comment stating what is missing, move status to Needs your answer per `docs/board.md`, record output on the Issue, and stop.
   - Never write the approval words (`approved`, `owner-test-only approved`, `abandoned`) yourself; they are owner-only.
2. **Notion sanity check.** If the notion/spec is incomplete, contradictory, or technically impossible, or planning reveals a choice that changes what the product does: stop, post what is unclear on the Issue, move to Needs your answer per `docs/board.md`, and wait.
3. **Branch.** Create or check out one branch for this round: `feat/<N>-<slug>` for new work, `fix/<N>-<slug>` for a fix round. A correction round after a merge uses a new `fix/` branch (see `correction` skill).
4. **Implement per sub-issue.** Work through sub-issues in plan order. One commit per sub-issue (or more if needed), commit message names the sub-issue. Preserve the original notion; ask before changing it.
5. **Tests.** Create and execute tests per the spec test plan. Run local checks (whatever CI runs plus the repo test command).
6. **Sub-issue keywords.** Sub-issue closes may use auto-close keywords. Never use `Closes #N` / `Fixes #N` / `Resolves #N` for the parent Issue.
7. **Secrets.** Never write credentials or secrets in Issues, comments, or code. Secrets live in the environment/store only.
8. **Record and move status.** Post a summary comment on the parent Issue (branch, commits, sub-issues covered, test results). Move board status per `docs/board.md` (normally stays Working; parent board status does not change while sub-issues are worked). Sub-issues stay off the board.

## Outputs

- Feature branch with all sub-issue work committed and local checks passing.
- Summary comment on the parent Issue.

## Done criteria

- Every sub-issue is implemented and local checks pass.
- Gate evidence (`approved` + preflight pass, or `owner-test-only approved`) was verified before starting.
- Output recorded on the Issue and status moved per `docs/board.md`.

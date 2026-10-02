---
name: implementation
description: Load when implementing approved work for a parent Issue (workflow stage 8). Use after spec approval and testability preflight pass to build sub-issues on a feature branch.
---

# Implementation

Implements the approved sub-issues for one parent Issue on a single feature branch, per `docs/workflow.md` (Implementation, Branching and Merge) and the stage-to-skill table in `docs/workflow.md` (Skills).

## Inputs

- Parent Issue number `#N` with:
  - Original notion verbatim.
  - Approved specification comment (owner `approved`) including goal, acceptance criteria, test plan, testability needs.
  - Testability preflight pass comment, or `owner-test-only approved` comment.
- Sub-issues (label `task`, attached as GitHub sub-issues of the parent) with done-criteria.
- Branch naming rule: `feat/<issue>-<slug>` or `fix/<issue>-<slug>` (must match CI guard `^(feat|fix)/[0-9]+-[a-z0-9-]+$`).

## Steps

1. **Gate first, before any code.** Run `python scripts/gates/spec_gate.py --issue N` and stop on non-zero: it requires the owner approval, the spec sections, and the preflight pass, and prints exactly what is missing. If blocked, post a status comment, set `--status-key blocked`, record output on the Issue, and stop.
   - Require an owner `approved` comment on the spec AND a preflight pass comment, OR an `owner-test-only approved` comment. (Checked by the gate above.)
   - If missing: do not implement. Post a status comment stating what is missing, set `--status-key blocked`, record output on the Issue, and stop.
   - Never write the approval or acceptance words (`approved`, `owner-test-only approved`, `abandoned`, `accept`) yourself; they are owner-only.
2. **Notion sanity check.** If the notion/spec is incomplete, contradictory, or technically impossible, or planning reveals a choice that changes what the product does: stop, post what is unclear on the Issue, set `--status-key blocked`, and wait.
3. **Branch.** Create or check out one branch for this round: `feat/<N>-<slug>` for new work, `fix/<N>-<slug>` for a fix round (see the development tool: `python scripts/sidebar/development.py create-branch --issue N --slug <kebab> --base main`). A correction round after a merge uses a new `fix/` branch (see `correction` skill). Avoid duplicate branches: list first (`python scripts/sidebar/development.py list --issue N`).
4. **Set In progress at pickup and at branch creation.** The parent enters In progress when work starts and stays there through branch creation and commits (it does not flap per sub-issue):
   - `python scripts/sidebar/project.py set-status --issue N --status-key pickedUp`
   - The script reads first (`already set` when correct), adds the issue to the project when missing, retries once, and re-reads to verify. Never fail silently.
5. **Implement per sub-issue.** Work through sub-issues in plan order. One commit per sub-issue (or more if needed), commit message names the sub-issue. Preserve the original notion; ask before changing it.
6. **Tests.** Create and execute tests per the spec test plan. Run local checks (whatever CI runs plus the repo test command). Do not mock or stub the thing under test to make a check pass.
7. **Sub-issue keywords.** Sub-issue closes may use auto-close keywords. Never use `Closes #N` / `Fixes #N` / `Resolves #N` for the parent Issue (check with `python scripts/sidebar/development.py check-body --body "<pr-body>"`).
8. **Secrets.** Never write credentials or secrets in Issues, comments, or code. Secrets live in the environment/store only.
9. **Record and move status.** Post a summary comment on the parent Issue (branch, commits, sub-issues covered, test results). Confirm Status is In progress (set `--status-key pickedUp` when it is not). Sub-issues stay off the board. End with the one-glance summary per `docs/board.md`.

## Outputs

- Feature branch with all sub-issue work committed and local checks passing.
- Summary comment on the parent Issue.

## Done criteria

- Every sub-issue is implemented and local checks pass.
- Gate evidence (`approved` + preflight pass, or `owner-test-only approved`) was verified before starting.
- Status is In progress (verified by re-read), output recorded on the Issue.

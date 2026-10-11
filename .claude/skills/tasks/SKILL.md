---
name: tasks
description: Use when breaking an approved plan into implementable sub-issues at workflow stage 7, after planning and before implementation.
---

# Tasks (from the plan-and-tasks record)

## Inputs

- Plan-and-tasks record (approach, affected areas, sequence, risks, deferred items, plus the task list with done criteria) on the parent Issue.
- `docs/board.md` (sub-issue attach recipe).

## Steps

1. Take the task list with done-criteria from the plan-and-tasks record (planning already wrote both as one record; do not re-plan here).
2. Create one GitHub sub-issue per task with only the label `task`, then attach it to the parent Issue using the recipe in `../../../docs/board.md`: take the sub-issue's numeric `id` via `gh api repos/{owner}/{repo}/issues/{number} --jq .id`, then POST `{"sub_issue_id": <id>}` to `repos/{owner}/{repo}/issues/{parent}/sub_issues`. Fallback when the API is unavailable: a `- [ ] #N` task list in the parent body.
3. Verify none of the sub-issues is on the board: run `python scripts/sidebar/project.py board-check` (fails when any open `task`-labelled Issue is on the board). The parent's board status does not change while sub-issues are worked.
4. Post a summary comment on the parent Issue listing every sub-issue.
5. Final step: record the output on the Issue and confirm Status via `python scripts/sidebar/project.py set-status --issue N --status-key pickedUp` (verified by re-read). End with the one-glance summary per `docs/board.md`.

## Outputs

- Sub-issues (label `task`) with done-criteria, attached as GitHub sub-issues of the parent.
- Summary comment on the parent Issue listing them.

## Done criteria

- Done when all sub-issues exist and none is on the board.

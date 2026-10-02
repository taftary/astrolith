---
name: planning
description: Use when writing the implementation plan for an approved, preflight-passed spec at workflow stage 6, before tasks are created.
---

# Planning

## Inputs

- Approved spec (goal, acceptance criteria, test plan, testability needs) with a recorded preflight pass (or `owner-test-only approved`).
- `docs/workflow.md` (Plan artifact rules).

## Steps

1. Draft the plan owned by the AI: approach, affected areas, implementation sequence, risks, deferred items. The plan does not need owner approval.
2. If planning reveals a notion that is incomplete, contradictory, or technically impossible, stop and ask the owner instead of continuing.
3. If a planning choice changes what the product does, ask the owner on the Issue before continuing. Otherwise proceed without asking.
4. Post the plan as an Issue comment (or a linked `plan.md` under `docs/plans/` only when a comment would be unwieldy).
5. Final step: record the output on the Issue and confirm Status via `python scripts/sidebar/project.py set-status --issue N --status-key pickedUp` (verified by re-read). End with the one-glance summary per `docs/board.md`.

## Outputs

- Plan comment: approach, affected areas, sequence, risks, deferred items.
- Owner question only when a choice changes product behavior.

## Done criteria

- Done when the plan comment is posted on the Issue.

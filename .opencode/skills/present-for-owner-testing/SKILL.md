---
name: present-for-owner-testing
description: Load after a pull request merges (workflow stages 12-13). Use to present the merged result for owner testing and to handle the owner's verdict.
---

# Present for Owner Testing

Product acceptance, not technical review. The owner tests the merged result in the test environment (preview deployment, staging, or local build stated on the Issue), never production, per `docs/workflow.md` (Owner Testing).

## Inputs

- Merged PR (number, head SHA, validator pass link).
- Spec test plan (human steps + expected results).
- Test location (where to test the merged result).

## Steps

1. **Post the testing comment on the Issue in plain, non-technical language:**
   - Where to test (exact URL/build + commit SHA).
   - Steps to follow (the spec test plan).
   - What the owner should see (expected results).
   - How to reply: accept, report a problem, or state abandonment.
2. **Move status to Ready to test** per `docs/board.md`. (Merge already moves here; this skill ensures it.) Never move to Done at this point.
3. **Handle the owner's verdict** (watch the Issue):
   - Accept (confirms implementation matches the original notion) → move to Done per `docs/board.md` and close the Issue.
   - Problem reported → move to Needs correction per `docs/board.md` and hand off to the `correction` skill. The report stays on the same Issue.
   - Owner states abandonment (`abandoned`) → move to Done with label `abandoned` per `docs/board.md`, record the reason on the Issue, and close it. Never write `abandoned` yourself.
4. **Record everything on the Issue.** Comment posted, status moves, verdict and its handling.

## Outputs

- Plain-language testing comment on the Issue.
- Verdict handling (Done closed, or Needs correction handoff).

## Done criteria

- Testing comment posted (where, steps, expected results) and status moved to Ready to test per `docs/board.md`.
- On verdict: accepted → Done (Issue closed); problem → Needs correction (correction handoff); `abandoned` → Done with label.

---
name: specification
description: Use when writing the owner-facing specification and test plan for a parent Issue at workflow stage 4, before any testability preflight or implementation.
---

# Specification

## Inputs

- Parent Issue, including the notion verbatim and the clarification Q&A.
- `docs/workflow.md` (Specification artifact, Testability Check inputs).

## Steps

1. Read the Issue notion verbatim and all clarification answers. Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible. Never change the notion without asking.
2. Write the spec in plain non-technical language as one Issue comment (or a linked `spec.md` under `docs/` only when a comment would be unwieldy) with these sections: goal, non-goals, acceptance criteria, test plan (human steps plus expected results for each step), testability needs (environment, accounts, seed data, third-party services). Number it `Spec v<k>` (first is v1); a new version voids the previous approval and needs a new `approved`.
3. Post an approval request on the Issue in plain language. Approval is an explicit owner comment whose trimmed body is exactly `approved` (record the comment URL); never implied. The AI never writes the approval or acceptance words itself.
4. After approval, write `.agent/validation/issue-<n>/requirements.md` from the approved spec: one criterion per acceptance criterion plus one per owner-expectation sentence, in the owner's words, with the AI adding the `(verify: ...)` probe. Owner approval of the spec counts as confirmation of the criteria text; the owner never sees probe syntax. Commit it with the feature PR (it is part of the specification).
   - Layer-1 criteria must be dischargeable before any verdict exists. Never write a criterion that asserts a pull-request run observation (a step/check conclusion, an annotation, CI greenness at a SHA): such a criterion is unsatisfiable by construction, because the run cannot go green until the verdict the count is built from exists. Run observations are gate-owned (checked by `merge_gate.py` and recorded in the validator's verdict comment) and stay out of the `MET n/n` count. `validate.py` refuses this shape; see `check_criteria_shape`.
5. Final step: record the output on the Issue and set Status via `python scripts/sidebar/project.py set-status --issue N --status-key blocked` (waiting for approval; verified by re-read). End with the one-glance summary per `docs/board.md`.

## Outputs

- Spec comment in plain language: goal, non-goals, acceptance criteria, test plan (human steps + expected results), testability needs.
- Approval request on the Issue.

## Done criteria

- Done only when the owner comments `approved` on the Issue. The spec is then frozen before implementation.

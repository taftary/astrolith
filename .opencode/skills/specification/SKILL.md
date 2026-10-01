---
name: specification
description: Use when writing the owner-facing specification and test plan for a parent Issue at workflow stage 4, before any testability preflight or implementation.
---

# Specification

## Inputs

- Parent Issue, including the notion verbatim and the clarification Q&A.
- `../../../docs/workflow.md` (Specification artifact, Testability Check inputs).

## Steps

1. Read the Issue notion verbatim and all clarification answers. Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible. Never change the notion without asking.
2. Write the spec in plain non-technical language as one Issue comment (or a linked `spec.md` under `../../../docs/` only when a comment would be unwieldy) with these sections: goal, non-goals, acceptance criteria, test plan (human steps plus expected results for each step), testability needs (environment, accounts, seed data, third-party services).
3. Post an approval request on the Issue in plain language. Approval is an explicit owner comment (`approved`), never implied. The AI never writes the approval words itself.
4. Final step: record the output on the Issue and set Status to `Needs your answer` via `python scripts/sidebar/project.py set-status` (waiting for approval; verified by re-read).

## Outputs

- Spec comment in plain language: goal, non-goals, acceptance criteria, test plan (human steps + expected results), testability needs.
- Approval request on the Issue.

## Done criteria

- Done only when the owner comments `approved` on the Issue. The spec is then frozen before implementation.

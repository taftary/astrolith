---
name: clarification
description: Use when taking a new notion, feedback item, or bug from drafts/, owner session text, or an Inbox Issue through workflow stages 1-3 (Issue intake and clarification questions).
---

# Clarification

## Inputs

- A draft file in `../../../drafts/` (any Markdown file except `README.md`), notion text stated by the owner in a session, or an existing Issue in `Inbox`.
- `../../../docs/workflow.md` (Drafts Folder, Project Board sections) and the Skills table in `../../../docs/plans/workflow.md`.

## Steps

1. Identify the input type: draft path, owner session text, or existing Inbox Issue number.
2. If the input is a draft path, check no Issue already names that draft path (search open and closed Issues for the path string). A draft is taken once. If an Issue already names it, use that Issue and do not create a new one.
3. If the draft names an existing Issue, or clearly refers to an item in `Ready to test`, record the content on that Issue instead of opening a new one.
4. If no Issue exists, create it: body holds the notion verbatim, apply the type label (`notion`, `feedback`, or `bug`), and for drafts also record the draft path in the body.
5. Take the draft in an intake commit naming the Issue: remove the file from `../../../drafts/`, or move it under `../../../docs/` when its content must also exist as a repository document (the workflow document is the example). Workflow artifacts never live in `../../../drafts/`.
6. Ask clarification questions when the input is incomplete, contradictory, or technically impossible. Post all questions as one Issue comment. If no questions are needed, post a short no-questions note instead.
7. Final step: record the output on the Issue and move the board status per `../../../docs/board.md` — status to `Needs your answer` when questions were asked, otherwise to `Working`.

## Outputs

- Parent Issue holding the notion verbatim, the type label, and the draft path when it came from a draft.
- One Issue comment with all clarification questions, or a no-questions note.

## Done criteria

- Every owner answer is recorded on the Issue in a decisions comment.
- No taken draft remains in `../../../drafts/`.

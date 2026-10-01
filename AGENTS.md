# Agent Instructions

These rules apply to every agent working in this repository. They implement `docs/workflow.md`, which is the source of truth for the process.

## Instructions

- Preserve the original notion. Ask the owner before changing it.
- Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible.
- A merged pull request moves the item to "Ready to test", never to "Done". The parent Issue is not closed automatically. Never use auto-close keywords for the parent Issue.
- An item cannot move to "Ready to test" without CI green plus a validator pass recorded on the Issue for the exact commit that is merged.
- Implementation does not start until the specification is approved by the owner and the testability preflight passes, unless the owner explicitly approves owner-test-only.
- Credentials and secrets are never written in Issues, comments, or code. Secrets live in the environment/store only; validation checks access without echoing values.
- The validator never writes or edits code in the repository.

## Approval words

The AI never posts `approved`, `owner-test-only approved`, or `abandoned` as a comment. Only the owner writes those words.

## Drafts intake

At the start of a session, offer the untaken drafts in `drafts/` and take one only on the owner's answer.

## References

- Status moves: `docs/board.md`.
- Stage procedures: the skills under `.opencode/skills/` (`clarification`, `specification`, `testability-preflight`, `planning`, `tasks`, `implementation`, `pull-request`, `technical-validation`, `visual-validation`, `correction`, `present-for-owner-testing`).

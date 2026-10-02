# Agent Instructions

These rules apply to every agent working in this repository. They implement `docs/workflow.md`, which is the source of truth for the process.

## Instructions

- Preserve the original notion. Ask the owner before changing it.
- Stop and ask the owner if the notion is incomplete, contradictory, or technically impossible.
- A merged pull request moves the item to "In review", never to "Done". The parent Issue is not closed automatically. Never use auto-close keywords for the parent Issue.
- An item cannot move to "In review" without CI green plus a validator pass recorded on the Issue for the exact commit that is merged.
- Implementation does not start until the specification is approved by the owner and the testability preflight passes, unless the owner explicitly approves owner-test-only.
- Credentials and secrets are never written in Issues, comments, or code. Secrets live in the environment/store only; validation checks access without echoing values.
- The validator never writes or edits code in the repository.

## Approval words

The AI never posts `approved`, `owner-test-only approved`, `abandoned`, or `accept` as a comment. Only the owner writes those words. An approval or acceptance is a comment whose body, trimmed, is exactly the word (or begins with it on its own line). Definitions live in `docs/workflow.md` (Owner Vocabulary) and are mirrored in `.agent/project-config.json`.

## Drafts intake

At the start of a session, offer the untaken drafts in `drafts/` and take one only on the owner's answer. The owner can answer "all". Agent-authored drafts are allowed when the owner asks for them in-session.

## References

- Status moves and the one-glance summary: `docs/board.md`.
- Validator capabilities registry: `docs/validator-capabilities.md`.
- Executable gates: `scripts/gates/` (`spec_gate.py`, `merge_gate.py`, `done_gate.py`); skills call them instead of re-describing the rules in prose.
- Plans live in `docs/plans/`; `docs/plans/workflow.md` is the historical plan for #2, not a live reference.
- Stage procedures: the skills under `.opencode/skills/` (`clarification`, `specification`, `testability-preflight`, `planning`, `tasks`, `implementation`, `pull-request`, `technical-validation`, `visual-validation`, `correction`, `present-for-owner-testing`, plus the `wait-checks` helper used inside `pull-request`).

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

## Harnesses

The repository works with OpenCode and Claude Code. The main session owns the workflow from intake to completion: it follows this file, loads the skill for the current stage, and moves board items per `docs/board.md`. It launches only the `validator`, `guides`, and explore subagents. Subagent definitions and their permissions live in `.opencode/agents/`; the `.claude/agents/` files point to them, and `scripts/agents/opencode_permissions.py` enforces the same permissions in Claude Code.

## Parallel work (worktrees)

Several agents can work at once without touching each other's files (#613). The rules:

- **Ask first.** When you pick up an issue, or start a `chore/` or `guides/` change, ask the owner one question: new worktree, or normal (as today)? Wait for the answer, then record it with `python scripts/sidebar/development.py choose --issue N --mode worktree|normal`.
- **Worktree.** `create-branch` makes the parent branch in its own folder, `../astrolith-work/<N>-<slug>`. That folder has:
  - its own build folder;
  - its own game settings, through its cargo config setting `ASTROLITH_SETTINGS`;
  - the main checkout's agent memory, through a link.

  It refuses when less than 15 GB is free. Work only inside that folder. Task branches are made and merged (`create-branch --kind task`, `merge-task`) inside it, and the tool refuses anywhere else.
- **Normal.** The branch is checked out in the main checkout, as before. Only one piece of normal work at a time; the tool refuses a second while the main checkout holds another branch.
- **Main.** Commits never land on `main` itself: `.githooks/pre-commit` refuses them. Enable it once per clone with `git config core.hooksPath .githooks`.
- **Evidence.** Untracked validation evidence always goes to the main checkout's `.agent/` (`scripts/validation/evidence.py`). Tracked files, such as `requirements.md`, stay on the branch.
- **Launching agents.** `python scripts/sidebar/launch.py --issue N --agent claude|opencode` starts an AI in the issue's worktree, and `--list` shows the running ones. The Telegram agent runs it when the owner says "start #N with claude" (or opencode).
- **Messages to the owner.** Every message starts with `#N:` (`python scripts/sidebar/notify.py --issue N "..."`).
  - The owner answers by replying to it, by starting the answer with `#N:`, or by naming no issue, in which case the answer goes to the agent that asked last.
  - The main Telegram agent passes each answer to `python scripts/sidebar/relay.py [--issue N] "..."`, which posts it on the right issue. The worker agent reads it there.
  - When the worker acts on an answer, it first repeats the issue back: `#N: got it: ...`.

## Approval words

The AI never posts `approved`, `owner-test-only approved`, `abandoned`, or `accept` as a comment. Only the owner writes those words. An approval or acceptance is a comment whose body, trimmed, is exactly the word (or begins with it on its own line). Definitions live in `docs/workflow.md` (Owner Vocabulary) and are mirrored in `.agent/project-config.json`.

## Drafts intake

At the start of a session, offer the untaken drafts in `drafts/` and take one only on the owner's answer. The owner can answer "all". Agent-authored drafts are allowed when the owner asks for them in-session.

## Technical conventions (always on)

- `universe-core` never depends on Bevy or any rendering crate (`E-CORE-NO-BEVY`).
- No `unwrap`, `expect`, or `panic!` outside tests (`E-NO-PANIC`).
- stdout is the machine protocol; the words error, warn, panic, deprecated, FAILED are reserved for real problems (`E-STDOUT-PROTOCOL`).
- Golden files under `tests/golden/` are rewritten only with `UPDATE_GOLDEN=1` and only when the pull request explains the change (`E-GOLDEN`).
- Exact pins (Rust, Bevy) change only through an ADR (`E-PIN-ADR`).
- Before implementing, read `docs/engineering.md`; before asking where something lives, read `docs/ARCHITECTURE.md`.

## References

- Technical rules: `docs/engineering.md`; architecture map: `docs/ARCHITECTURE.md`; decisions: `docs/decisions/`; docs index: `docs/README.md`.
- Status moves and the one-glance summary: `docs/board.md`.
- Validator capabilities registry: `docs/validator-capabilities.md`.
- Executable gates: `scripts/gates/` (`spec_gate.py`, `merge_gate.py`, `done_gate.py`); skills call them instead of re-describing the rules in prose.
- Plans live in `docs/plans/`; `docs/plans/workflow.md` is the historical plan for #2, not a live reference.
- Stage procedures: the skills under `.claude/skills/`, read by both OpenCode and Claude Code (`clarification`, `specification`, `testability-preflight`, `planning`, `tasks`, `implementation`, `pull-request`, `technical-validation`, `visual-validation`, `correction`, `present-for-owner-testing`, plus the `wait-checks` helper used inside `pull-request` and the `guides` helper for the `guides/` knowledge base).

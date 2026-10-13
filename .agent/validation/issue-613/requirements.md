# Requirements (CONFIRMED) - issue #613

Derived by the main agent from approved Spec v4
(https://github.com/taftary/astrolith/issues/613#issuecomment-6145585008,
owner approval https://github.com/taftary/astrolith/issues/613#issuecomment-6145595796),
with the owner's answers Q1-Q7 as revised by R1-R9, the amendments M1-M6,
the owner's choice rule, and P1-P4. One or more criteria per acceptance
criterion. Launching a real Claude or OpenCode session and the Telegram
relay are covered by the owner test.

- [ ] C1: The owner's choice is recorded as a marker and the newest one wins; with a worktree, create-branch makes the parent branch in `../astrolith-work/<N>-<slug>` with long paths, its own game settings and the shared memory link (AC0, AC1) (verify: contains ok worktree-created in python-tests.log)
- [ ] C2: Task branches are made inside the parent's worktree only (AC1) (verify: contains ok task-inside-worktree in python-tests.log)
- [ ] C3: create-branch refuses a worktree when less than 15 GB is free (AC2) (verify: contains ok worktree-disk-floor in python-tests.log)
- [ ] C4: development.py refuses task branches and merge-task outside the parent's worktree (AC3) (verify: contains ok merge-task-refused-outside-worktree in python-tests.log)
- [ ] C5: The pre-commit hook refuses a commit on `main` and allows one on a branch (AC3) (verify: contains ok hook-refuses-main-allows-branch in python-tests.log)
- [ ] C6: Normal work runs as today, and a second normal piece of work is refused while the main checkout is busy (AC0) (verify: contains ok normal-refused-when-busy in python-tests.log)
- [ ] C7: The validator's check worktree is put at the commit, detached, and reused on the next run (AC4) (verify: contains ok check-reused in python-tests.log)
- [ ] C8: remove takes both worktrees and their memory links away (AC5) (verify: contains ok remove-both in python-tests.log)
- [ ] C9: Evidence from a worktree lands in the main checkout's `.agent/`, and an accepted issue's untracked evidence is deleted while `requirements.md` stays (AC6) (verify: contains ok done-cleans-untracked in python-tests.log)
- [ ] C10: The one-time cleanup selects closed issues' evidence and older runs of open issues, never a tracked file (AC6) (verify: contains ok cleanup-selection in python-tests.log)
- [ ] C11: The merge gate's carry-over accepts a clean, non-overlapping update and refuses an overlapping one (AC7, AC14) (verify: contains ok overlap-refused in python-tests.log)
- [ ] C12: A clean update from `main` carries the verdict over (AC14) (verify: contains ok clean-update-carries in python-tests.log)
- [ ] C13: A worktree's cargo config points `ASTROLITH_SETTINGS` inside the worktree (AC9) (verify: contains ok cargo-env-settings in python-tests.log)
- [ ] C14: The game reads `ASTROLITH_SETTINGS` first and is unchanged without it (AC9) (verify: contains the_settings_env_overrides_the_platform_file_when_set ... ok in cargo-test.log)
- [ ] C15: The launcher starts an agent in the issue's worktree, lists it, and refuses a second agent on the same issue (AC13) (verify: contains ok second-agent-refused in python-tests.log)
- [ ] C16: Agent messages are tagged `#N:` and posted on the issue (AC15) (verify: contains ok tag-once in python-tests.log)
- [ ] C16b: An owner answer naming no issue goes to the agent that asked last (or to the issue it names or replies to), and the worker repeats the issue back when it acts (AC15, second half) (verify: contains ok relay-to-last-asker in python-tests.log)
- [ ] C16c: The worker's prompt tells it to repeat the issue back (AC15) (verify: contains ok prompt-repeats-issue-back in python-tests.log)
- [ ] C17: Agents get no permission prompts in `../astrolith-work/` (AC12) (verify: file .claude/settings.json contains ../astrolith-work)
- [ ] C18: `AGENTS.md` describes the rule (AC11) (verify: file AGENTS.md contains ## Parallel work (worktrees))
- [ ] C19: The same seed still gives the same universe on every run (verify: contains VERIFY-OK in verify.log)
